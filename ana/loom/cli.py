"""Ana Loom CLI — visualize cognitive processes or export Cellrix Manifest."""

import json
from pathlib import Path

import click

from ana.common import get_settings
from ana.loom.cellrix_bridge import build_manifest
from ana.loom.visualizer import visualize_epoch


@click.command()
@click.argument("epoch_id", required=False)
@click.option("--last", is_flag=True, help="Visualize the most recent epoch")
@click.option(
    "--cellrix",
    is_flag=True,
    help="Output Cellrix Manifest JSON instead of Rich rendering.",
)
def loom(epoch_id: str | None, last: bool, cellrix: bool) -> None:
    """Visualize cognitive process of an epoch.

    When --cellrix is set, a valid Cellrix Manifest is written to stdout
    and the Rich renderer is completely bypassed.  This is the zero‑import
    bridge that lets Cellrix discover Anaphase via a CLI subprocess.
    """
    settings = get_settings()
    hxr_dir = Path(settings.hxr_dir)

    if last:
        sessions = sorted(
            hxr_dir.glob("*.jsonl"),
            key=lambda p: p.stat().st_mtime, reverse=True,
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

    # ── Cellrix bridge: pure JSON manifest on stdout ──
    if cellrix:
        manifest = build_manifest(epoch_path)
        click.echo(json.dumps(manifest, indent=2, ensure_ascii=False))
        return

    # Legacy Rich rendering
    visualize_epoch(epoch_path)
