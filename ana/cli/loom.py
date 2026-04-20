import click
from pathlib import Path
from ana.common import get_settings
from ana.loom.visualizer import visualize_epoch


@click.command()
@click.argument("epoch_id", required=False)
@click.option("--last", is_flag=True, help="Visualize the most recent epoch")
@click.option(
    "--theme",
    default="ana",
    help="Theme to apply (ana)",
    type=click.Choice(["ana"]),
)
def loom(epoch_id, last, theme):
    """Visualize cognitive process of an epoch (Ana Loom)."""
    settings = get_settings()
    hxr_dir = Path(settings.hxr_dir)

    if last:
        sessions = sorted(hxr_dir.glob("*.jsonl"), key=lambda p: p.stat().st_mtime, reverse=True)
        if not sessions:
            click.echo("No sessions found.")
            return
        epoch_path = sessions[0]
    elif epoch_id:
        epoch_path = hxr_dir / f"{epoch_id}.jsonl"
        if not epoch_path.exists():
            click.echo(f"Epoch {epoch_id} not found.")
            return
    else:
        click.echo("Please specify epoch_id or use --last.")
        return

    visualize_epoch(epoch_path, theme_name=theme)
