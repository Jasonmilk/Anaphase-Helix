import pytest
from ana.core.prefrontal import Prefrontal
from ana.schemas.reasoning import ReasoningRequest


def test_prefrontal_init(mock_settings):
    prefrontal = Prefrontal(mock_settings)
    assert prefrontal.settings == mock_settings


def test_prefrontal_generate_draft_not_implemented(mock_settings, trace_id):
    prefrontal = Prefrontal(mock_settings)
    request = ReasoningRequest(
        task="test task",
        system_prompt="test",
        trace_id=trace_id,
    )
    with pytest.raises(NotImplementedError):
        prefrontal.generate_draft(request)
