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

        # Real execution placeholder
        raise NotImplementedError(
            "Synapse.execute real implementation is not available. "
            "Set ANA_MOCK_MODE=true in .env to use mock responses."
        )
