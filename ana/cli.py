#!/usr/bin/env python3
"""
ana - Anaphase execution engine: Synapse layer entrypoint for Helix ecosystem.

IMPORTANT: This version uses MANUAL .env parsing to avoid editable install
           and path-resolution bugs. When the project stabilizes and is deployed
           via regular `pip install`, you may restore automatic pydantic-settings
           loading by removing `_load_configuration()` and simply calling `Settings()`.
"""

import asyncio
import json
import os
from pathlib import Path

import click


def _load_configuration():
    """
    Manually parse .env file and return a fully populated Settings instance.

    WHY MANUAL?
    - During development with editable installs (`pip install -e .`), the
      working directory and symlinks often cause pydantic-settings to miss
      the .env file, leading to silent fallback to defaults.
    - This function locates .env relative to THIS SOURCE FILE, guaranteeing
      correct loading regardless of how the CLI is invoked.

    HOW TO RESTORE STANDARD BEHAVIOR:
    1. Remove this entire function.
    2. In `cli()`, replace `_load_configuration()` with `Settings()`.
    3. Ensure `Settings` class has `model_config = SettingsConfigDict(env_file=".env")`.
    """
    from ana.core.config import Settings

    # Locate .env file: start from this file's directory and search upwards.
    project_root = Path(__file__).parent
    env_path = project_root / ".env"
    if not env_path.exists():
        for parent in project_root.parents:
            env_path = parent / ".env"
            if env_path.exists():
                break

    env_vars = {}
    if env_path.exists():
        with open(env_path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                if "=" in line:
                    key, value = line.split("=", 1)
                    key = key.strip()
                    value = value.strip()
                    # Strip quotes if present (common in .env files)
                    if value.startswith('"') and value.endswith('"'):
                        value = value[1:-1]
                    elif value.startswith("'") and value.endswith("'"):
                        value = value[1:-1]
                    env_vars[key] = value
                    os.environ[key] = value

    # Explicit construction avoids any hidden default fallbacks.
    return Settings(
        tuck_endpoint=env_vars.get("TUCK_ENDPOINT", "http://localhost:8686/v1/chat/completions"),
        tuck_api_key=env_vars.get("TUCK_API_KEY", "local"),
        helix_mind_endpoint=env_vars.get("HELIX_MIND_ENDPOINT", "http://localhost:8020"),
        amygdala_model=env_vars.get("ANA_AMYGDALA_MODEL", "qwen2.5:2b"),
        cerebellum_model=env_vars.get("ANA_CEREBELLUM_MODEL", "qwen2.5:2b"),
        left_brain_model=env_vars.get("ANA_LEFT_BRAIN_MODEL", "qwen2.5-coder:7b"),
        right_brain_model=env_vars.get("ANA_RIGHT_BRAIN_MODEL", "deepseek-r1:8b"),
        embedding_model=env_vars.get("ANA_EMBEDDING_MODEL", "BAAI/bge-small-en"),
        mock_mode=env_vars.get("ANA_MOCK_MODE", "false").lower() == "true",
        hxr_dir=env_vars.get("ANA_HXR_DIR", "./memory_dag/sessions"),
        gene_lock_path=env_vars.get("ANA_GENE_LOCK_PATH", "./knowledge_base/l0_gene_lock.md"),
        log_level=env_vars.get("ANA_LOG_LEVEL", "INFO"),
    )


from ana.core.agent_loop import AgentLoop
from ana.core.hxr import HXRLogger


@click.group()
@click.version_option(version="0.1.0")
@click.pass_context
def cli(ctx):
    ctx.ensure_object(dict)
    ctx.obj["config"] = _load_configuration()


@cli.command()
@click.argument("task")
@click.option("--direct", is_flag=True, help="Direct chat mode, bypass tool loop")
@click.option("--json", "output_json", is_flag=True, help="Output in JSON format")
@click.pass_context
def run(ctx, task, direct, output_json):
    """Start Agent Loop to execute a task."""
    config = ctx.obj["config"]
    hxr_dir = getattr(config, "hxr_dir", "./memory_dag/sessions")
    hxr = HXRLogger(hxr_dir)

    loop = AgentLoop(config, hxr, mock_mode=config.mock_mode)

    result = asyncio.run(loop.run(task, direct=direct))

    if output_json:
        click.echo(json.dumps(result, ensure_ascii=False, indent=2))
    else:
        if result.get("ok"):
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
    """Replay inference trace from HXR logs."""
    config = ctx.obj["config"]
    hxr_dir = getattr(config, "hxr_dir", "./memory_dag/sessions")
    hxr_file = Path(hxr_dir) / f"{session_id}.jsonl"
    if not hxr_file.exists():
        click.echo(f"Session {session_id} not found")
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
    """Show system statistics."""
    config = ctx.obj["config"]
    hxr_dir = getattr(config, "hxr_dir", "./memory_dag/sessions")
    sessions_dir = Path(hxr_dir)
    sessions = list(sessions_dir.glob("*.jsonl")) if sessions_dir.exists() else []
    click.echo(f"Total sessions: {len(sessions)}")
    total_steps = 0
    for sess in sessions:
        with open(sess) as f:
            total_steps += sum(1 for _ in f)
    click.echo(f"Total steps: {total_steps}")
    click.echo(f"HXR directory: {hxr_dir}")


if __name__ == "__main__":
    cli()
