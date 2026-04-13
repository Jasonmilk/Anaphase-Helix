"""Tool Registry - Load and validate tool schemas from YAML."""

import yaml
from typing import Dict, Any, Optional, List
from pydantic import BaseModel


class ToolSchema(BaseModel):
    """Tool schema definition."""
    name: str
    description: str
    aliases: Optional[List[str]] = None
    schema: Dict[str, Any]  # JSON Schema
    handler: Dict[str, Any]  # executor, command, output_mapping
    permissions: List[str]


class ToolRegistry:
    """Tool registry for loading and validating tool schemas."""

    def __init__(self):
        self._tools: Dict[str, ToolSchema] = {}

    def load(self, config_path: str) -> None:
        """Load tool schemas from YAML configuration file."""
        with open(config_path, 'r', encoding='utf-8') as f:
            config = yaml.safe_load(f)

        tools_data = config.get('tools', [])
        for tool_def in tools_data:
            schema = ToolSchema(**tool_def)
            self._tools[schema.name] = schema
            # Register aliases
            if schema.aliases:
                for alias in schema.aliases:
                    self._tools[alias] = schema

    def get(self, name: str) -> Optional[ToolSchema]:
        """Get tool schema by name or alias."""
        return self._tools.get(name)

    def validate(self, name: str, params: Dict[str, Any]) -> bool:
        """
        Validate parameters against tool schema.

        Args:
            name: Tool name or alias
            params: Parameters to validate

        Returns:
            True if valid, False otherwise
        """
        schema = self.get(name)
        if not schema:
            return False

        # Simple validation: check required fields exist
        tool_schema = schema.schema
        required_fields = tool_schema.get('required', [])
        properties = tool_schema.get('properties', {})

        for field in required_fields:
            if field not in params:
                return False

        # Check parameter types (basic validation)
        for param_name, param_value in params.items():
            if param_name in properties:
                expected_type = properties[param_name].get('type')
                if expected_type == 'string' and not isinstance(param_value, str):
                    return False
                elif expected_type == 'integer' and not isinstance(param_value, int):
                    return False
                elif expected_type == 'boolean' and not isinstance(param_value, bool):
                    return False
                elif expected_type == 'array' and not isinstance(param_value, list):
                    return False
                elif expected_type == 'object' and not isinstance(param_value, dict):
                    return False

        return True

    def list_tools(self) -> List[str]:
        """List all registered tool names."""
        return list(self._tools.keys())
