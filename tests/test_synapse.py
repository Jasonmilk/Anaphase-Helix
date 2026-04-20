import pytest
from unittest.mock import Mock

from ana.core.synapse import Synapse
from ana.schemas.tool import ExecutionRequest, ExecutionResult, ToolDefinition
from ana.registry import ToolRegistry


@pytest.fixture
def mock_tool_registry():
    """Create a mock tool registry with a test tool."""
    registry = Mock(spec=ToolRegistry)
    tool_def = ToolDefinition(
        name="test_tool",
        description="A test tool",
        parameters={"type": "object", "properties": {}, "required": []},
        executor="cli",
        command="echo 'test'",
        timeout=10,
        permissions=["read"],
    )
    registry.get.return_value = tool_def
    registry.validate_params.return_value = True
    return registry


def test_synapse_init(mock_tool_registry):
    synapse = Synapse(mock_tool_registry)
    assert synapse.settings is not None
    assert synapse.tool_registry == mock_tool_registry


def test_execute_mock_mode(monkeypatch, mock_tool_registry, trace_id):
    from ana.common import get_settings
    settings = get_settings()
    monkeypatch.setattr(settings, "mock_mode", True)

    synapse = Synapse(mock_tool_registry)
    request = ExecutionRequest(
        tool_name="test_tool",
        params={"arg": "value"},
        trace_id=trace_id,
    )
    result = synapse.execute(request)

    assert isinstance(result, ExecutionResult)
    assert result.ok is True
    assert result.data is not None
    assert "Mock execution" in result.data
    assert result.exit_code == 0
    assert result.error is None


def test_execute_tool_not_found(mock_tool_registry, trace_id):
    mock_tool_registry.get.return_value = None

    synapse = Synapse(mock_tool_registry)
    request = ExecutionRequest(
        tool_name="unknown_tool",
        params={},
        trace_id=trace_id,
    )
    result = synapse.execute(request)

    assert result.ok is False
    assert "not registered" in result.error
