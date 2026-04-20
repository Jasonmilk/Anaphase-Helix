"""Synapse module: Tool execution and validation."""

import shlex
import subprocess
from typing import Any, Dict

from ana.common import logger, get_settings
from ana.schemas.tool import ExecutionRequest, ExecutionResult
from ana.schemas.exceptions import ToolExecutionError
from ana.registry import ToolRegistry


class Synapse:
    """
    Synapse module responsible for tool execution with safety rules.

    Implements:
    - shell=False for subprocess
    - parameter escaping
    - timeout enforcement
    - permission validation
    """

    def __init__(self, tool_registry: ToolRegistry) -> None:
        """
        Initialize Synapse with tool registry.

        Args:
            tool_registry: Registry containing all available tools.
        """
        self.settings = get_settings()
        self.tool_registry = tool_registry

    def execute(self, request: ExecutionRequest) -> ExecutionResult:
        """
        Execute a tool with safety rules applied.

        Args:
            request: ExecutionRequest with tool name and parameters.

        Returns:
            ExecutionResult with ok status, data, or error.
        """
        logger.info(
            "synapse.execute.start",
            tool=request.tool_name,
            trace_id=request.trace_id,
        )

        # Validate tool existence
        tool_def = self.tool_registry.get(request.tool_name)
        if tool_def is None:
            return ExecutionResult(
                ok=False,
                error=f"Tool '{request.tool_name}' not registered",
            )

        # Validate parameters against tool schema
        try:
            self.tool_registry.validate_params(request.tool_name, request.params)
        except Exception as e:
            return ExecutionResult(
                ok=False,
                error=f"Tool validation failed: {str(e)}",
            )

        # Mock mode: return simulated success
        if self.settings.mock_mode:
            return ExecutionResult(
                ok=True,
                data=f"Mock execution of '{request.tool_name}' succeeded.",
                compressed_summary=f"Mock summary for {request.tool_name}",
                exit_code=0,
            )

        # Real execution
        return self._execute_real(tool_def, request)

    def _execute_real(
        self, tool_def: Any, request: ExecutionRequest
    ) -> ExecutionResult:
        """Execute CLI tool with safety sandbox rules."""
        if tool_def.executor != "cli":
            return ExecutionResult(
                ok=False,
                error=f"Executor '{tool_def.executor}' not supported yet",
            )

        # Build command with parameter escaping
        try:
            command = self._build_command(tool_def.command, request.params)
        except Exception as e:
            return ExecutionResult(
                ok=False,
                error=f"Command building failed: {str(e)}",
            )

        logger.debug(
            "synapse.execute.command",
            tool=request.tool_name,
            command=command,
            trace_id=request.trace_id,
        )

        # Execute with safety rules
        try:
            result = subprocess.run(
                command,
                shell=False,                     # Mandatory safety rule
                capture_output=True,
                text=True,
                timeout=tool_def.timeout,
                check=False,
            )
        except subprocess.TimeoutExpired as e:
            logger.error(
                "synapse.execute.timeout",
                tool=request.tool_name,
                timeout=tool_def.timeout,
                trace_id=request.trace_id,
            )
            return ExecutionResult(
                ok=False,
                error=f"Tool execution timed out after {tool_def.timeout}s",
            )
        except Exception as e:
            logger.error(
                "synapse.execute.failed",
                tool=request.tool_name,
                error=str(e),
                trace_id=request.trace_id,
            )
            return ExecutionResult(
                ok=False,
                error=f"Tool execution failed: {str(e)}",
            )

        # Process result
        if result.returncode == 0:
            return ExecutionResult(
                ok=True,
                data=result.stdout.strip() or "Success",
                compressed_summary=self._summarize_output(result.stdout),
                exit_code=0,
            )
        else:
            logger.warning(
                "synapse.execute.nonzero_exit",
                tool=request.tool_name,
                exit_code=result.returncode,
                stderr=result.stderr,
                trace_id=request.trace_id,
            )
            return ExecutionResult(
                ok=False,
                error=result.stderr.strip() or f"Exit code {result.returncode}",
                exit_code=result.returncode,
            )

    def _build_command(self, template: str, params: Dict[str, Any]) -> list[str]:
        """
        Build command list from template with parameter escaping.

        Supports placeholder format: {param_name}
        """
        # Replace placeholders with safely quoted values
        formatted = template
        for key, value in params.items():
            placeholder = f"{{{key}}}"
            if placeholder in formatted:
                # Quote each parameter individually
                formatted = formatted.replace(placeholder, shlex.quote(str(value)))

        # Parse into argument list (respects quotes)
        return shlex.split(formatted)

    def _summarize_output(self, output: str, max_length: int = 200) -> str:
        """Create compressed summary of tool output."""
        if not output:
            return ""
        if len(output) <= max_length:
            return output
        return output[:max_length] + "..."
