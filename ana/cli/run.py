import click
import asyncio
from datetime import datetime

from ana.common import (
    get_settings,
    configure_logging,
    logger,
    generate_trace_id,
    set_trace_id,
    generate_epoch_id,
)
from ana.core.agent_loop import AgentLoop
from ana.schemas.state import HelixState
from ana.schemas.metabolism import MetabolismState


@click.command()
@click.argument("task", type=str)
@click.option("--direct", is_flag=True, help="Direct chat mode without full agent loop")
def run(task, direct):
    """Run a task through the Helix Agent Loop."""
    # Initialize infrastructure
    configure_logging()
    settings = get_settings()
    
    # Generate trace ID at entry
    trace_id = generate_trace_id()
    set_trace_id(trace_id)
    epoch_id = generate_epoch_id()
    
    logger.info(
        "command.start",
        command="run",
        task=task,
        direct=direct,
        trace_id=trace_id,
        epoch_id=epoch_id,
    )
    
    # Initialize metabolism state
    metabolism = MetabolismState(
        used_tokens=0,
        budget_total=settings.budget_total,
        epoch_start_time=datetime.now(),
        max_duration_seconds=settings.max_duration_seconds,
        cognitive_overload_line=settings.cognitive_overload_line,
        fatigue_line=settings.fatigue_line,
        apoptosis_line=settings.apoptosis_line,
    )
    
    # Initialize state
    state = HelixState(
        epoch_id=epoch_id,
        trace_id=trace_id,
        task=task,
        created_at=datetime.now(),
        metabolism=metabolism,
    )
    
    # Run agent loop
    if direct:
        # Direct chat mode placeholder
        raise NotImplementedError("Direct chat mode is not implemented yet")
    else:
        # Full agent loop
        loop = AgentLoop(settings)
        asyncio.run(loop.run(state))
    
    logger.info("command.complete", command="run", epoch_id=epoch_id)
