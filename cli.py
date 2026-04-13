#!/usr/bin/env python3
"""ana - Anaphase 执行引擎：Helix 生态的 Harness 层入口。"""

import asyncio
import json
import click

from ana.core.config import Settings
from ana.core.agent_loop import AgentLoop
from ana.core.hxr import HXRLogger


@click.group()
@click.version_option(version="0.1.0")
@click.pass_context
def cli(ctx):
    ctx.ensure_object(dict)
    ctx.obj["config"] = Settings()


@cli.command()
@click.argument("task")
@click.option("--direct", is_flag=True, help="直通模式，绕过 LLM")
@click.option("--json", "output_json", is_flag=True, help="JSON 格式输出")
@click.pass_context
def run(ctx, task, direct, output_json):
    """启动 Agent Loop，执行任务"""
    config = ctx.obj["config"]
    hxr_dir = getattr(config, "hxr_dir", "./memory_dag/sessions")
    hxr = HXRLogger(hxr_dir)
    loop = AgentLoop(config, hxr)
    # 关键修正：使用 asyncio.run 执行异步主循环
    result = asyncio.run(loop.run(task, direct=direct))

    if output_json:
        click.echo(json.dumps(result, ensure_ascii=False, indent=2))
    else:
        if result.get("ok"):
            # Chat mode reply has highest priority
            if result.get("reply"):
                click.echo(result["reply"])
            else:
                data = result.get("result", {})
                if isinstance(data, dict) and data.get("summary"):
                    click.echo(data["summary"])
                else:
                    click.echo(f"Task completed (session: {result.get('session_id')})")
        else:
            click.echo(f"Error: {result.get('error', 'Unknown error')}")


@cli.command()
@click.argument("session_id")
@click.option("--json", "output_json", is_flag=True)
@click.pass_context
def trace(ctx, session_id, output_json):
    """回放推理链路"""
    config = ctx.obj["config"]
    hxr_dir = getattr(config, "hxr_dir", "./memory_dag/sessions")
    from pathlib import Path
    hxr_file = Path(hxr_dir) / f"{session_id}.jsonl"
    if not hxr_file.exists():
        click.echo(f"会话 {session_id} 不存在")
        return
    with open(hxr_file) as f:
        records = [json.loads(line) for line in f]
    if output_json:
        click.echo(json.dumps(records, ensure_ascii=False, indent=2))
    else:
        for r in records:
            click.echo(f"[{r['step_id']}] {r['action']} ({r.get('handler', 'unknown')}) "
                       f"{'✅' if r.get('success') else '❌'}")
            if r.get('intent'):
                click.echo(f"   Intent: {r['intent'][:80]}")


@cli.command()
@click.pass_context
def stats(ctx):
    """系统统计"""
    config = ctx.obj["config"]
    hxr_dir = getattr(config, "hxr_dir", "./memory_dag/sessions")
    from pathlib import Path
    sessions_dir = Path(hxr_dir)
    sessions = list(sessions_dir.glob("*.jsonl")) if sessions_dir.exists() else []
    click.echo(f"总会话数：{len(sessions)}")
    total_steps = 0
    for sess in sessions:
        with open(sess) as f:
            total_steps += sum(1 for _ in f)
    click.echo(f"总步骤数：{total_steps}")
    click.echo(f"HXR 目录：{hxr_dir}")


if __name__ == "__main__":
    cli()
