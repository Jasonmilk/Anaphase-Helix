import pytest
from ana.core.prefrontal import Prefrontal
from ana.schemas.reasoning import ReasoningRequest, ReasoningDraft


def test_prefrontal_init():
    prefrontal = Prefrontal()
    assert prefrontal.settings is not None


def test_generate_draft_mock(monkeypatch, trace_id):
    from ana.common import get_settings
    settings = get_settings()
    monkeypatch.setattr(settings, "mock_mode", True)

    prefrontal = Prefrontal()
    request = ReasoningRequest(
        task="What is the capital of France?",
        system_prompt="You are a helpful assistant.",
        trace_id=trace_id,
    )
    result = prefrontal.generate_draft(request)

    assert isinstance(result, ReasoningDraft)
    assert result.reasoning is not None
    assert "Mock reasoning" in result.reasoning
    assert result.tool_call is None
    assert result.final_reply is None
