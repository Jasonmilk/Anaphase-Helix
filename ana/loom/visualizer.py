"""Visualizer for Ana Loom — render cognitive process from HXR logs."""

import json
from pathlib import Path
from rich.console import Console
from rich.panel import Panel
from rich.table import Table
from rich.text import Text

from ana.loom.themes import get_theme


def visualize_epoch(hxr_path: Path, theme_name: str = "ana") -> None:
    """
    Read HXR JSONL log file and render a terminal-friendly visualization.

    Args:
        hxr_path: Path to the epoch's JSONL log file.
        theme_name: Name of the theme to apply (default "ana").
    """
    theme = get_theme(theme_name)

    console = Console(highlight=False)
    events = []

    if not hxr_path.exists():
        console.print(f"[red]Log file not found: {hxr_path}[/red]")
        return

    with open(hxr_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                try:
                    events.append(json.loads(line))
                except json.JSONDecodeError:
                    continue

    if not events:
        console.print("[yellow]No valid log entries found. The session may have produced no output.[/yellow]")
        return

    # --------------------------------------------------------------------
    # Extract metadata from the first event (must be done before usage)
    # --------------------------------------------------------------------
    first_event = events[0]
    task = first_event.get("task", "unknown")
    epoch_id = first_event.get("epoch_id", "unknown")
    trace_id = first_event.get("trace_id", "unknown")

    # --------------------------------------------------------------------
    # Render header panel with semantic theme colors
    # --------------------------------------------------------------------
    console.print(
        Panel(
            Text.assemble(
                ("🧬 Ana Loom · ", theme["text_secondary"]),
                (epoch_id, theme["thinking"]),
                ("\nTrace: ", theme["text_secondary"]),
                (trace_id, theme["text_primary"]),
                ("\nTask: ", theme["text_secondary"]),
                (task, theme["text_primary"]),
            ),
            border_style=theme["border"],
            style=f"on {theme['bg_dark']}",
            expand=False,
        )
    )

    # --------------------------------------------------------------------
    # Build the main event table
    # --------------------------------------------------------------------
    table = Table(
        show_header=False,
        box=None,
        padding=(0, 1),
        border_style=theme["border"],
        style=theme["text_primary"],
    )
    table.add_column("step", style=theme["text_secondary"], width=12)
    table.add_column("detail", style=theme["text_primary"])

    for event in events:
        event_type = event.get("event", "")

        if event_type == "agent_loop.perceive":
            table.add_row("perceive", f'🧠 "{event.get("task", "")}"')

        elif event_type == "agent_loop.assess_priority":
            score = event.get("score", 0.0)
            intent = event.get("intent", "unknown")
            table.add_row(
                "assess",
                Text.assemble(
                    ("📊 priority=", theme["text_secondary"]),
                    (f"{score:.1f}", theme["highlight"]),
                    (", intent=", theme["text_secondary"]),
                    (intent, theme["thinking"]),
                ),
            )

        elif event_type == "agent_loop.plan":
            has_tool = event.get("has_tool_call", False)
            icon = "🔧" if has_tool else "💭"
            label = "tool call" if has_tool else "reasoning"
            table.add_row("plan", f"{icon} {label}", style=theme["thinking"])

        elif event_type == "agent_loop.execute":
            if event.get("skipped"):
                table.add_row("execute", "⏭️  skipped", style=theme["text_secondary"])
            else:
                ok = event.get("ok", False)
                icon = "✅" if ok else "❌"
                color = theme["success"] if ok else theme["error"]
                table.add_row("execute", f"{icon} executed", style=color)

        elif event_type == "agent_loop.reflect":
            helio = event.get("heliotropism", 0.0)
            pulse = event.get("pulse", 0.0)
            vig = event.get("vigilance", 0.0)
            table.add_row(
                "reflect",
                Text.assemble(
                    ("❤️  helio=", theme["text_secondary"]),
                    (f"{helio:.2f}", theme["affect"]),
                    (", pulse=", theme["text_secondary"]),
                    (f"{pulse:.2f}", theme["affect"]),
                    (", vig=", theme["text_secondary"]),
                    (f"{vig:.2f}", theme["affect"]),
                ),
            )

        elif event_type == "agent_loop.validate":
            passed = event.get("passed", False)
            icon = "✅" if passed else "❌"
            label = "passed" if passed else "failed"
            color = theme["success"] if passed else theme["error"]
            table.add_row("validate", f"{icon} {label}", style=color)

        elif event_type == "agent_loop.consolidate":
            used = event.get("used_tokens", 0)
            budget = event.get("budget_total", 1)
            pct = used / budget * 100 if budget else 0
            table.add_row(
                "consolidate",
                Text.assemble(
                    ("🔋 tokens: ", theme["text_secondary"]),
                    (f"{used}/{budget} ", theme["highlight"]),
                    (f"({pct:.1f}%)", theme["text_secondary"]),
                ),
            )

        elif event_type == "agent_loop.sleep":
            table.add_row("sleep", "🌙 epoch completed", style=theme["success"])

    console.print(table)
    console.print(f"[{theme['text_secondary']}]Total events: {len(events)}[/]")
