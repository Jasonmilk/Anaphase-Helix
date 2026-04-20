import pytest
import asyncio
from datetime import datetime

from ana.core.agent_loop import AgentLoop
from ana.schemas.state import HelixState
from ana.schemas.metabolism import MetabolismState


def test_agent_loop_init():
    loop = AgentLoop()
    assert loop.settings is not None
    assert hasattr(loop, "transitions")
    assert loop.amygdala is not None
    assert loop.prefrontal is not None
    assert loop.synapse is not None
    assert loop.validator is not None


@pytest.mark.asyncio
async def test_run_mock_mode(monkeypatch, trace_id, epoch_id):
    from ana.common import get_settings
    settings = get_settings()
    monkeypatch.setattr(settings, "mock_mode", True)

    metabolism = MetabolismState(
        used_tokens=0,
        budget_total=4096,
        epoch_start_time=datetime.now(),
        max_duration_seconds=300,
        cognitive_overload_line=10,
        fatigue_line=0.8,
        apoptosis_line=0.95,
    )
    state = HelixState(
        epoch_id=epoch_id,
        trace_id=trace_id,
        task="What is the capital of France?",
        created_at=datetime.now(),
        metabolism=metabolism,
        current_step="perceive",
    )

    loop = AgentLoop()
    final_state = await loop.run(state)

    assert final_state.current_step == "sleep"
    assert final_state.priority is not None
    assert final_state.reasoning_draft is not None
    assert final_state.affect is not None
    assert final_state.validation_result is not None
    assert final_state.metabolism.used_tokens == 100
