"""Cellrix Bridge：将 HXR 认知日志转换为 Cellrix Manifest JSON。

哲学：Anaphase 只读取自己的认知日志，生成一份声明式 Manifest，
将一切渲染和交互交给 Cellrix Runtime。Anaphase 永不 import cellrix，
Manifest 是两者之间的唯一契约——契约至上，零硬编码。
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


def _parse_events(hxr_path: Path) -> list[dict[str, Any]]:
    """读取 HXR JSONL 文件，返回事件列表。"""
    events: list[dict[str, Any]] = []
    if not hxr_path.exists():
        return events

    with open(hxr_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                try:
                    events.append(json.loads(line))
                except json.JSONDecodeError:
                    continue
    return events


def _extract_metadata(events: list[dict[str, Any]]) -> dict[str, str]:
    """从第一条事件中提取纪元元数据。"""
    if not events:
        return {"task": "unknown", "epoch_id": "unknown", "trace_id": "unknown"}
    first = events[0]
    return {
        "task": first.get("task", "unknown"),
        "epoch_id": first.get("epoch_id", "unknown"),
        "trace_id": first.get("trace_id", "unknown"),
    }


def _step_icon(step: str) -> str:
    """每个 Agent Loop 步骤的图标。"""
    icons: dict[str, str] = {
        "perceive": "👁",
        "assess_priority": "⚖",
        "plan": "🧭",
        "execute": "⚡",
        "reflect": "🪞",
        "validate": "🛡",
        "consolidate": "📦",
        "sleep": "🌙",
    }
    return icons.get(step, "•")


def _build_state_graph(events: list[dict[str, Any]]) -> str:
    """构建紧凑的状态流转图文本。"""
    steps: list[str] = []
    seen: set[str] = set()
    for e in events:
        event_type = e.get("event", "")
        if event_type.startswith("agent_loop."):
            step = event_type.replace("agent_loop.", "")
            if step not in seen:
                seen.add(step)
                steps.append(step)

    meta = _extract_metadata(events)
    lines: list[str] = [
        "Agent Loop State Graph",
        "══════════════════════",
        "",
    ]
    for i, step in enumerate(steps):
        connector = "  └── " if i == len(steps) - 1 else "  ├── "
        icon = _step_icon(step)
        lines.append(f"{connector}{icon} {step}")

    lines.append("")
    lines.append(f"Epoch : {meta['epoch_id']}")
    lines.append(f"Trace : {meta['trace_id'][:16]}…")
    lines.append(f"Task  : {meta['task'][:80]}")
    return "\n".join(lines)


def _build_metrics_panel(events: list[dict[str, Any]]) -> str:
    """构建关键指标面板：优先级、情感向量、代谢状态。"""
    lines: list[str] = ["Key Metrics", "═══════════", ""]
    found_any = False

    for e in events:
        event_type = e.get("event", "")

        if event_type == "agent_loop.assess_priority":
            lines.append(f"Priority Score : {e.get('score', 0):.0f}")
            lines.append(f"Intent        : {e.get('intent', '?')}")
            found_any = True

        elif event_type == "agent_loop.reflect":
            lines.append(f"Heliotropism  : {e.get('heliotropism', 0):+.2f}")
            lines.append(f"Pulse         : {e.get('pulse', 0):.2f}")
            lines.append(f"Vigilance     : {e.get('vigilance', 0):.2f}")
            found_any = True

        elif event_type == "agent_loop.validate":
            passed = e.get("passed", False)
            lines.append(f"Validation    : {'PASSED' if passed else 'FAILED'}")
            found_any = True

        elif event_type == "agent_loop.consolidate":
            used = e.get("used_tokens", 0)
            budget = e.get("budget_total", 1)
            pct = used / budget * 100 if budget else 0
            lines.append(f"Tokens Used   : {used} / {budget} ({pct:.1f}%)")
            bar_len = 20
            filled = int(bar_len * used / budget) if budget else 0
            bar = "█" * filled + "░" * (bar_len - filled)
            lines.append(f"               [{bar}]")
            found_any = True

    if not found_any:
        lines.append("(no metrics captured)")

    lines.append(f"\nTotal Events  : {len(events)}")
    return "\n".join(lines)


def _build_timeline(events: list[dict[str, Any]]) -> str:
    """构建事件时间线文本。"""
    lines: list[str] = ["Event Timeline", "═════════════", ""]
    for e in events:
        event_type = e.get("event", "")
        if not event_type.startswith("agent_loop."):
            continue
        step = event_type.replace("agent_loop.", "")
        icon = _step_icon(step)
        lines.append(f"{icon} {step}")
    return "\n".join(lines)


def build_manifest(hxr_path: Path) -> dict[str, Any]:
    """读取 HXR 日志，生成 Cellrix Manifest dict。

    布局：三面板分形仪表盘。
      - 左上：状态流转图
      - 右上：关键指标
      - 底部：事件时间线（可滚动）

    所有面板均为 STATIC 类型，因为 HXR 日志是不可变快照。
    若需要实时监控，可升级为 REALTIME 类型配合 pipe 源。
    """
    events = _parse_events(hxr_path)
    if not events:
        return {
            "version": "2.0",
            "layout": {
                "direction": "vertical",
                "slots": [{"id": "msg", "weight": 1}],
            },
            "cells": [
                {
                    "id": "empty",
                    "type": "static",
                    "slot": "msg",
                    "content": "没有找到有效的日志条目。",
                }
            ],
        }

    meta = _extract_metadata(events)

    return {
        "version": "2.0",
        "layout": {
            "direction": "vertical",
            "slots": [
                {
                    "id": "top_row",
                    "weight": 1,
                    "layout": {
                        "direction": "horizontal",
                        "slots": [
                            {"id": "state_pane", "weight": 1},
                            {"id": "metrics_pane", "weight": 1},
                        ],
                    },
                },
                {"id": "timeline_pane", "weight": 2},
            ],
        },
        "cells": [
            {
                "id": "state_graph",
                "type": "static",
                "slot": "state_pane",
                "content": _build_state_graph(events),
                "minConstraint": {"width": 30, "height": 8},
            },
            {
                "id": "detail_panel",
                "type": "static",
                "slot": "metrics_pane",
                "content": _build_metrics_panel(events),
                "minConstraint": {"width": 30, "height": 8},
            },
            {
                "id": "event_log",
                "type": "static",
                "slot": "timeline_pane",
                "content": _build_timeline(events),
                "collapseMode": "scroll",
                "minConstraint": {"height": 6},
            },
        ],
    }
