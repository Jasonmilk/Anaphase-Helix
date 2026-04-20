import pytest
import asyncio
from datetime import datetime
from ana.core.agent_loop import AgentLoop
from ana.schemas.state import HelixState
from ana.schemas.metabolism import MetabolismState


def test_agent_loop_init(mock_settings):
    loop = AgentLoop(mock_settings)
    assert loop.settings == mock_settings
    assert hasattr(loop, "transitions")


@pytest.mark.asyncio
async def test_agent_loop_run_not_implemented(mock_settings, trace_id, epoch_id):
    loop = AgentLoop(mock_settings)
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
        task="test task",
        created_at=datetime.now(),
        metabolism=metabolism,
    )
    with pytest.raises(NotImplementedError):
        await loop.run(state)
