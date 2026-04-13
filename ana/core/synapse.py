"""
Anaphase Synapse core: tool interception, schema validation, L0 gene lock checking,
and dual-tag parsing for LLM output.
"""

import re
import json
import asyncio
from string import Template
from typing import Dict, Any, Optional, Tuple

from ana.core.registry import ToolRegistry
from ana.core.gene_lock import GeneLockValidator


class DualTagParser:
    """Parses <reasoning> and <tool_call> tags from LLM output with fallback heuristics."""

    REASONING_PATTERN = re.compile(r"<reasoning>(.*?)</reasoning>", re.DOTALL)
    TOOL_CALL_PATTERN = re.compile(r"<tool_call>(.*?)</tool_call>", re.DOTALL)

    # Enhanced JSON pattern that supports one level of nesting (sufficient for tool params)
    JSON_PATTERN = re.compile(
        r'\{(?:[^{}]|\{[^{}]*\})*"tool"\s*:\s*"[^"]+"\s*,\s*"params"\s*:\s*\{.*?\}\s*\}',
        re.DOTALL
    )

    @classmethod
    def parse(cls, llm_output: str) -> Tuple[Optional[str], Optional[Dict[str, Any]]]:
        """Extract reasoning and tool_call from LLM output.

        Returns:
            Tuple of (reasoning_text, tool_call_dict). Either may be None.
        """
        reasoning_match = cls.REASONING_PATTERN.search(llm_output)
        tool_match = cls.TOOL_CALL_PATTERN.search(llm_output)

        reasoning = reasoning_match.group(1).strip() if reasoning_match else None
        tool_call = None

        if tool_match:
            try:
                tool_call = json.loads(tool_match.group(1).strip())
            except json.JSONDecodeError:
                pass

        # Fallback: if tag parsing failed, try to extract plain JSON from text
        if not tool_call:
            json_match = cls.JSON_PATTERN.search(llm_output)
            if json_match:
                try:
                    tool_call = json.loads(json_match.group(0))
                except json.JSONDecodeError:
                    # Strip markdown code fences and try again
                    cleaned = re.sub(r'```(?:json)?\s*|\s*```', '', json_match.group(0)).strip()
                    try:
                        tool_call = json.loads(cleaned)
                    except json.JSONDecodeError:
                        pass

        # Correct common parameter naming mistakes (model often hallucinates these)
        if tool_call and "params" in tool_call:
            params = tool_call["params"]
            # node_name -> node_id
            if "node_name" in params and "node_id" not in params:
                params["node_id"] = params.pop("node_name")
            # task -> summary (for ana_finish)
            if "task" in params and "summary" not in params:
                params["summary"] = params.pop("task")
            # entity -> entity_id, attributes -> properties (for update_social_graph)
            if "entity" in params and "entity_id" not in params:
                params["entity_id"] = params.pop("entity")
            if "attributes" in params and "properties" not in params:
                params["properties"] = params.pop("attributes")

        return reasoning, tool_call


class Synapse:
    """
    Synapse execution gateway that validates tool calls, enforces gene locks, and executes
    CLI commands on behalf of the Agent Loop.
    """

    def __init__(self, registry: ToolRegistry, gene_lock: GeneLockValidator):
        self.registry = registry
        self.gene_lock = gene_lock

    async def execute(self, tool_name: str, params: Dict[str, Any]) -> Dict[str, Any]:
        """Execute a tool call after validation and safety checks."""
        tool = self.registry.get(tool_name)
        if not tool:
            return {"ok": False, "error": f"Unknown tool: {tool_name}"}

        # 1. Schema validation
        if not self.registry.validate(tool_name, params):
            return {"ok": False, "error": "Parameter validation failed"}

        # 2. L0 gene lock check for sensitive permissions
        if any(p in tool.permissions for p in ["write", "execute", "external_network"]):
            if not self.gene_lock.check(tool_name, params):
                return {
                    "ok": False,
                    "error": "Gene lock violation",
                    "gene_lock_check": "failed"
                }

        # 3. Build CLI command from template
        template = Template(tool.handler.command)
        # Ensure all params are strings for safe substitution
        safe_params = {k: str(v) if not isinstance(v, str) else v for k, v in params.items()}
        command = template.safe_substitute(safe_params)

        # 4. Additional command safety validation (non-gene-lock)
        if any(p in tool.permissions for p in ["write", "execute"]):
            is_safe, reason = self.gene_lock.validate_command_safety(
                command,
                user_confirmed=params.get("_user_confirmed", False)
            )
            if not is_safe:
                return {
                    "ok": False,
                    "error": f"Safety rule violation: {reason}",
                    "gene_lock_check": "failed",
                    "hint": "Use --yes or _user_confirmed=true to explicitly confirm"
                }

        # 5. Execute command asynchronously
        try:
            proc = await asyncio.create_subprocess_shell(
                command,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            stdout, stderr = await asyncio.wait_for(
                proc.communicate(),
                timeout=tool.handler.timeout
            )

            if proc.returncode != 0:
                return {
                    "ok": False,
                    "error": stderr.decode() or "Command failed",
                    "exit_code": proc.returncode
                }

            output = stdout.decode().strip()

            # Parse as JSON if the tool expects JSON output
            if tool.handler.output_mapping and tool.handler.output_mapping.get("format") == "json":
                try:
                    output = json.loads(output)
                except json.JSONDecodeError:
                    # If parsing fails, return raw output with warning
                    return {
                        "ok": True,
                        "data": output,
                        "warning": "Output was not valid JSON"
                    }

            return {"ok": True, "data": output}

        except asyncio.TimeoutError:
            return {"ok": False, "error": f"Timeout after {tool.handler.timeout}s"}
        except Exception as e:
            return {"ok": False, "error": str(e)}
