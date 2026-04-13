"""Helix-Ana CLI entry point."""

import click
import asyncio
import json
from ana.core.config import load_config
from ana.core.agent_loop import AgentLoop
from ana.core.hxr import HXRLogger


@click.group()
@click.version_option(version="0.1.0")
@click.pass_context
def cli(ctx):
    """ana - Anaphase 执行引擎：Helix 生态的 Harness 层入口"""
    ctx.ensure_object(dict)
    ctx.obj["config"] = load_config()


@cli.command()
@click.argument("task")
@click.option("--direct", is_flag=True, help="直通模式，绕过 LLM")
@click.option("--json", "output_json", is_flag=True, help="JSON 格式输出")
@click.pass_context
def run(ctx, task, direct, output_json):
    """启动 Agent Loop，执行任务"""
    config = ctx.obj["config"]
    hxr = HXRLogger(config.hxr_dir)
    loop = AgentLoop(config, hxr)
    # 正确执行异步主循环
    result = asyncio.run(loop.run(task, direct=direct))
    
    if output_json:
        click.echo(json.dumps(result, ensure_ascii=False))
    else:
        # 根据返回结构智能提取摘要
        summary = result.get("summary") or result.get("reply") or "任务完成"
        click.echo(summary)


@cli.group()
def kb():
    """知识库管理（调用 Helix-Mind API）"""
    pass


@kb.command("fetch")
@click.argument("node_id")
@click.option("--mode", type=click.Choice(["summary", "full"]), default="summary")
@click.option("--json", "output_json", is_flag=True)
@click.pass_context
def kb_fetch(ctx, node_id, mode, output_json):
    """从知识库 DAG 调取节点"""
    # TODO: 调用 Helix-Mind API
    click.echo(f"Fetching node {node_id} in {mode} mode")


@kb.command("query")
@click.argument("keyword")
@click.option("--limit", default=5)
@click.option("--json", "output_json", is_flag=True)
@click.pass_context
def kb_query(ctx, keyword, limit, output_json):
    """语义检索知识库"""
    # TODO: 调用 Helix-Mind API
    click.echo(f"Querying knowledge base for '{keyword}' (limit={limit})")


@cli.command()
@click.argument("session_id")
@click.option("--step", type=int, help="定位到特定步骤")
@click.option("--json", "output_json", is_flag=True)
@click.pass_context
def trace(ctx, session_id, step, output_json):
    """回放推理链路"""
    # TODO: 实现 trace 功能
    click.echo(f"Tracing session {session_id}")


if __name__ == "__main__":
    cli()
