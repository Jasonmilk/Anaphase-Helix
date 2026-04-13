"""任务队列管理 CLI 命令"""
import json

import click

from ana.core.queue import MLFQScheduler, QueueLevel


@click.group()
def queue():
    """任务队列管理"""
    pass


@queue.command("ls")
@click.option("--json", "output_json", is_flag=True, help="JSON 格式输出")
@click.pass_context
def queue_ls(ctx, output_json: bool):
    """列出队列状态"""
    scheduler: MLFQScheduler = ctx.obj.get("scheduler")
    if not scheduler:
        click.echo("调度器未初始化")
        return

    status = {
        "queues": {},
        "waiting_pool": len(scheduler.waiting_pool),
        "active_tasks": len(scheduler.active_tasks),
        "completed_tasks": len(scheduler._completed_tasks),
        "backpressure_mode": scheduler.backpressure.mode
    }

    for level in QueueLevel:
        status["queues"][level.name] = scheduler.queues[level].qsize()

    if output_json:
        click.echo(json.dumps(status, indent=2))
    else:
        click.echo("=== 任务队列状态 ===")
        for level in QueueLevel:
            click.echo(f"{level.name}: {status['queues'][level.name]} 个任务")
        click.echo(f"\n等待池：{status['waiting_pool']} 个任务")
        click.echo(f"活跃任务：{status['active_tasks']} 个")
        click.echo(f"已完成：{status['completed_tasks']} 个")
        click.echo(f"背压模式：{status['backpressure_mode']}")


@queue.command("cancel")
@click.argument("task_id")
@click.option("--cascade/--no-cascade", default=True, help="是否级联取消子任务")
@click.option("--reason", default="USER_REQUEST", help="取消原因")
@click.option("--json", "output_json", is_flag=True, help="JSON 格式输出")
@click.pass_context
async def queue_cancel(ctx, task_id: str, cascade: bool, reason: str, output_json: bool):
    """取消任务"""
    scheduler: MLFQScheduler = ctx.obj.get("scheduler")
    if not scheduler:
        click.echo("调度器未初始化")
        return

    cancelled = await scheduler.cancel_task(task_id, reason, cascade=cascade)

    if output_json:
        click.echo(json.dumps({"cancelled": cancelled}, indent=2))
    else:
        click.echo(f"已取消任务：{task_id}")
        if len(cancelled) > 1:
            click.echo(f"级联取消：{len(cancelled) - 1} 个子任务")
            for tid in cancelled[1:]:
                click.echo(f"  - {tid}")


@queue.command("stats")
@click.option("--json", "output_json", is_flag=True, help="JSON 格式输出")
@click.pass_context
def queue_stats(ctx, output_json: bool):
    """显示调度统计信息"""
    scheduler: MLFQScheduler = ctx.obj.get("scheduler")
    if not scheduler:
        click.echo("调度器未初始化")
        return

    stats = {
        "weights": scheduler.weights,
        "quantum_config": {
            level.name: config
            for level, config in scheduler.quantum_config.items()
        },
        "deadlock_check_interval": scheduler.deadlock_check_interval
    }

    if output_json:
        click.echo(json.dumps(stats, indent=2))
    else:
        click.echo("=== 调度配置 ===")
        click.echo(f"权重：{scheduler.weights}")
        click.echo("\n量子配置:")
        for level, config in scheduler.quantum_config.items():
            click.echo(f"  {level.name}: {config}")
        click.echo(f"\n死锁检测周期：{scheduler.deadlock_check_interval}秒")


# 注册到主 CLI
def init_queue_cli(cli):
    """将 queue 命令组注册到主 CLI"""
    cli.add_command(queue)
