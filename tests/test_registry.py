"""Tests for Tool Registry module."""

import yaml

from ana.core.registry import ToolRegistry


class TestToolRegistry:
    """Test cases for ToolRegistry class."""

    def test_init_empty(self):
        """Test initialization with non-existent config file."""
        registry = ToolRegistry("nonexistent.yaml")
        assert registry.tools == {}

    def test_load_from_yaml(self, temp_dir):
        """Test loading tools from YAML configuration."""
        config_data = {
            "tools": [
                {
                    "name": "test_tool",
                    "description": "A test tool",
                    "parameters": {
                        "type": "object",
                        "properties": {"param1": {"type": "string"}},
                        "required": ["param1"],
                    },
                    "handler": {"executor": "ana_cli", "command": "test {param1}"},
                    "permissions": ["read"],
                }
            ]
        }

        config_path = temp_dir / "tools.yaml"
        with open(config_path, "w") as f:
            yaml.dump(config_data, f)

        registry = ToolRegistry(str(config_path))

        assert "test_tool" in registry.tools
        tool = registry.get("test_tool")
        assert tool is not None
        assert tool.name == "test_tool"
        assert tool.description == "A test tool"

    def test_get_by_alias(self, temp_dir):
        """Test getting tool by alias."""
        config_data = {
            "tools": [
                {
                    "name": "main_tool",
                    "description": "Main tool",
                    "aliases": ["alias1", "alias2"],
                    "parameters": {"type": "object", "properties": {}},
                    "handler": {"executor": "ana_cli", "command": "test"},
                    "permissions": ["read"],
                }
            ]
        }

        config_path = temp_dir / "tools.yaml"
        with open(config_path, "w") as f:
            yaml.dump(config_data, f)

        registry = ToolRegistry(str(config_path))

        # Get by main name
        assert registry.get("main_tool") is not None
        # Get by aliases
        assert registry.get("alias1") is not None
        assert registry.get("alias2") is not None

    def test_validate_success(self, temp_dir):
        """Test successful parameter validation."""
        config_data = {
            "tools": [
                {
                    "name": "validator_tool",
                    "description": "Tool with validation",
                    "parameters": {
                        "type": "object",
                        "properties": {"name": {"type": "string"}},
                        "required": ["name"],
                    },
                    "handler": {"executor": "ana_cli", "command": "test"},
                    "permissions": ["read"],
                }
            ]
        }

        config_path = temp_dir / "tools.yaml"
        with open(config_path, "w") as f:
            yaml.dump(config_data, f)

        registry = ToolRegistry(str(config_path))

        # Valid parameters
        assert registry.validate("validator_tool", {"name": "test"}) is True

    def test_validate_failure(self, temp_dir):
        """Test failed parameter validation."""
        config_data = {
            "tools": [
                {
                    "name": "strict_tool",
                    "description": "Tool with strict validation",
                    "parameters": {
                        "type": "object",
                        "properties": {"count": {"type": "integer"}},
                        "required": ["count"],
                    },
                    "handler": {"executor": "ana_cli", "command": "test"},
                    "permissions": ["read"],
                }
            ]
        }

        config_path = temp_dir / "tools.yaml"
        with open(config_path, "w") as f:
            yaml.dump(config_data, f)

        registry = ToolRegistry(str(config_path))

        # Missing required parameter
        assert registry.validate("strict_tool", {}) is False
        # Wrong type
        assert registry.validate("strict_tool", {"count": "not an int"}) is False

    def test_list_tools(self, temp_dir):
        """Test listing all tools."""
        config_data = {
            "tools": [
                {
                    "name": "tool1",
                    "description": "First tool",
                    "parameters": {"type": "object", "properties": {}},
                    "handler": {"executor": "ana_cli", "command": "test"},
                    "permissions": ["read"],
                },
                {
                    "name": "tool2",
                    "description": "Second tool",
                    "parameters": {"type": "object", "properties": {}},
                    "handler": {"executor": "ana_cli", "command": "test"},
                    "permissions": ["read"],
                },
            ]
        }

        config_path = temp_dir / "tools.yaml"
        with open(config_path, "w") as f:
            yaml.dump(config_data, f)

        registry = ToolRegistry(str(config_path))
        tools_list = registry.list_tools()

        assert len(tools_list) == 2
        names = {t["name"] for t in tools_list}
        assert "tool1" in names
        assert "tool2" in names
