"""Ana Loom CLI command — visualize cognitive processes."""

import json
import click
from pathlib import Path
from ana.common import get_settings
from ana.loom.visualizer import visualize_epoch


@click.command()
@click.argument("epoch_id", required=False)
@click.option("--last", is_flag=True, help="Visualize the most recent epoch")
@click.option(
    "--cellrix",
    is_flag=True,
    help="Output Cellrix Manifest JSON instead of Rich rendering",
)
def loom(epoch_id, last, cellrix):
    """Visualize Anaphase-Helix cognitive processes (Ana Loom).

    Defaults to Rich terminal rendering. Use --cellrix flag to output
    Cellrix Manifest JSON, which can be piped directly to
    `cellrix preview -` for interactive visualization.
    """
    settings = get_settings()
    hxr_dir = Path(settings.hxr_dir)

    # Resolve epoch path
    if last:
        sessions = sorted(
            hxr_dir.glob("*.jsonl"), key=lambda p: p.stat().st_mtime, reverse=True
        )
        if not sessions:
            click.echo("No sessions found.", err=True)
            return
        epoch_path = sessions[0]
    elif epoch_id:
        epoch_path = hxr_dir / f"{epoch_id}.jsonl"
        if not epoch_path.exists():
            click.echo(f"Epoch {epoch_id} not found.", err=True)
            return
    else:
        click.echo("Please specify epoch_id or use --last.", err=True)
        return

    # Cellrix branch: output Manifest JSON
    if cellrix:
        from ana.loom.cellrix_bridge import build_manifest
        manifest = build_manifest(epoch_path)
        click.echo(json.dumps(manifest, indent=2, ensure_ascii=False))
        return

    # Original branch: Rich rendering
    visualize_epoch(epoch_path)
