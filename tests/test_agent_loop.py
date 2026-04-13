"""Tests for Agent Loop module."""

import pytest

from ana.core.agent_loop import AgentLoop
from ana.core.hxr import HXRLogger


class TestAgentLoop:
    """Test cases for AgentLoop class."""

    def test_init(self, sample_config):
        """Test AgentLoop initialization."""
        hxr = HXRLogger(sample_config.hxr_dir)
        loop = AgentLoop(sample_config, hxr)

        assert loop.config == sample_config
        assert loop.hxr == hxr
        assert loop.session_id == ""
        assert loop.loop_count == 0

    def test_run_direct_mode_echo(self, sample_config, temp_dir):
        """Test direct mode execution with echo command."""
        hxr_dir = str(temp_dir / "hxr")
        sample_config.hxr_dir = hxr_dir

        hxr = HXRLogger(hxr_dir)
        loop = AgentLoop(sample_config, hxr)

        result = loop.run("echo hello", direct=True)

        assert result["summary"] == "hello"
        assert result["mode"] == "direct"
        assert "session_id" in result
        assert result["loop_count"] >= 1

    def test_run_direct_mode_generic(self, sample_config, temp_dir):
        """Test direct mode with generic task."""
        hxr_dir = str(temp_dir / "hxr")
        sample_config.hxr_dir = hxr_dir

        hxr = HXRLogger(hxr_dir)
        loop = AgentLoop(sample_config, hxr)

        result = loop.run("some task", direct=True)

        assert "Direct execution acknowledged" in result["summary"]
        assert result["mode"] == "direct"

    def test_run_llm_mode(self, sample_config, temp_dir):
        """Test LLM mode execution (placeholder)."""
        hxr_dir = str(temp_dir / "hxr")
        sample_config.hxr_dir = hxr_dir

        hxr = HXRLogger(hxr_dir)
        loop = AgentLoop(sample_config, hxr)

        result = loop.run("test task", direct=False)

        assert "Task processed" in result["summary"]
        assert result["mode"] == "llm"
        assert "session_id" in result

    @pytest.mark.asyncio
    async def test_step(self, sample_config, temp_dir):
        """Test single step execution."""
        hxr_dir = str(temp_dir / "hxr")
        sample_config.hxr_dir = hxr_dir

        hxr = HXRLogger(hxr_dir)
        loop = AgentLoop(sample_config, hxr)

        context = {"task": "test"}
        result = await loop.step(context)

        assert "step_id" in result
        assert result["context"] == context
