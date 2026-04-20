from pathlib import Path
from typing import Dict, Optional
import yaml
from ana.schemas.tool import ToolDefinition
from ana.schemas.exceptions import ContractViolationError


class ToolRegistry:
    def __init__(self, config_path: Path | str):
        self._tools: Dict[str, ToolDefinition] = {}
        self._load(config_path)

    def _load(self, path: Path | str) -> None:
        config_path = Path(path)
        if not config_path.exists():
            return
        with open(config_path, "r", encoding="utf-8") as f:
            data = yaml.safe_load(f) or {}
        for tool_data in data.get("tools", []):
            tool = ToolDefinition(**tool_data)
            self._tools[tool.name] = tool

    def get(self, name: str) -> Optional[ToolDefinition]:
        return self._tools.get(name)

    def list_all(self) -> list[ToolDefinition]:
        return list(self._tools.values())

    def validate_params(self, name: str, params: dict) -> bool:
        tool = self.get(name)
        if not tool:
            raise ContractViolationError(f"Tool '{name}' not registered")
        required = tool.parameters.get("required", [])
        for key in required:
            if key not in params:
                raise ContractViolationError(f"Missing required parameter: {key}")
        return True
