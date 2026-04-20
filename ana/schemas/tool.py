"""Tool contracts: Definitions and execution payloads."""

from pydantic import BaseModel
from typing import Dict, Any, Literal, Optional


class ToolDefinition(BaseModel):
    """Schema for tool registration — aligns with OpenAI Function Calling."""
    name: str
    description: str
    parameters: Dict[str, Any]  # JSON Schema
    executor: Literal["cli", "internal"]
    command: Optional[str] = None  # CLI command template
    timeout: int = 30
    permissions: list[Literal["read", "write", "external"]] = []


class ExecutionRequest(BaseModel):
    tool_name: str
    params: Dict[str, Any]
    trace_id: str
    gene_lock_context: Optional[str] = None


class ExecutionResult(BaseModel):
    ok: bool
    data: Optional[Any] = None
    compressed_summary: Optional[str] = None
    error: Optional[str] = None
    exit_code: Optional[int] = None
