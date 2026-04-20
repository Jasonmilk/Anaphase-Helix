import pytest
from ana.core.synapse import Synapse
from ana.schemas.tool import ExecutionRequest


def test_synapse_init(mock_settings, mock_tool_registry):
    synapse = Synapse(mock_settings, mock_tool_registry)
    assert synapse.settings == mock_settings
    assert synapse.tool_registry == mock_tool_registry


def test_synapse_execute_not_implemented(mock_settings, mock_tool_registry, trace_id):
    synapse = Synapse(mock_settings, mock_tool_registry)
    request = ExecutionRequest(
        tool_name="test_tool",
        params={},
        trace_id=trace_id,
    )
    with pytest.raises(NotImplementedError):
        synapse.execute(request)
