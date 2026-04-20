import click
from pathlib import Path

from ana.common import (
    configure_logging,
    logger,
    generate_trace_id,
    set_trace_id,
    get_settings,
)


@click.command()
def stats():
    """Show session statistics."""
    configure_logging()
    settings = get_settings()
    
    # Generate trace ID at entry
    trace_id = generate_trace_id()
    set_trace_id(trace_id)
    
    logger.info(
        "command.start",
        command="stats",
        trace_id=trace_id,
    )
    
    # Scan session directory placeholder
    hxr_dir = Path(settings.hxr_dir)
    if not hxr_dir.exists():
        click.echo("No sessions found yet.")
        return
    
    # Statistics calculation placeholder
    click.echo("Not implemented")
    raise NotImplementedError("Session statistics is not implemented yet")
    
    logger.info("command.complete", command="stats")
