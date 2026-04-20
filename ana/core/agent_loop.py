"""State-graph driven Agent Loop core."""

from pathlib import Path
from typing import Dict, Callable, Literal

from ana.common import logger, get_settings, add_file_logger
from ana.schemas.state import HelixState
from ana.schemas.reasoning import ReasoningRequest
from ana.schemas.tool import ExecutionRequest

from .amygdala import Amygdala
from .prefrontal import Prefrontal
from .synapse import Synapse
from .corpus_callosum import CorpusCallosumValidator
from ana.registry import ToolRegistry


class AgentLoop:
    """
    State-graph driven main loop for Helix agent.

    Uses declarative transition matrix to define state transitions,
    following the Engineering Manual's requirement of table-driven transitions.
    """

    def __init__(self) -> None:
        self.settings = get_settings()
        self.amygdala = Amygdala()
        self.prefrontal = Prefrontal()
        self.tool_registry = ToolRegistry(self.settings.tools_path)
        self.synapse = Synapse(self.tool_registry)
        self.validator = CorpusCallosumValidator()

        # Declarative transition matrix: current_step -> next_step -> condition
        self.transitions: Dict[
            Literal[
                "perceive", "assess_priority", "plan", "execute",
                "reflect", "consolidate", "sleep"
            ],
            Dict[str, Callable[[HelixState], bool]]
        ] = {
            "perceive": {"assess_priority": lambda s: True},
            "assess_priority": {"plan": lambda s: True},
            "plan": {"execute": lambda s: True},
            "execute": {"reflect": lambda s: True},
            "reflect": {
                "consolidate": lambda s: self._check_fatigue(s),
                "plan": lambda s: not self._check_fatigue(s),
            },
            "consolidate": {
                "sleep": lambda s: self._check_apoptosis(s),
                "plan": lambda s: not self._check_apoptosis(s),
            },
            "sleep": {},
        }

    def _check_fatigue(self, state: HelixState) -> bool:
        """Check if we've hit the fatigue line."""
        usage_ratio = state.metabolism.used_tokens / state.metabolism.budget_total
        return usage_ratio >= state.metabolism.fatigue_line

    def _check_apoptosis(self, state: HelixState) -> bool:
        """Check if we've hit the apoptosis line."""
        usage_ratio = state.metabolism.used_tokens / state.metabolism.budget_total
        return usage_ratio >= state.metabolism.apoptosis_line

    async def run(self, state: HelixState) -> HelixState:
        """
        Run the agent loop until completion.

        The underlying brain modules (Amygdala, Prefrontal) will automatically
        switch between mock and real implementations based on settings.mock_mode.
        This keeps the orchestration layer clean and testable.
        """
        # Persist logs to file for Ana Loom and HXR audit compliance
        log_file = Path(self.settings.hxr_dir) / f"{state.epoch_id}.jsonl"
        add_file_logger(log_file)

        logger.info("agent_loop.start", epoch_id=state.epoch_id, trace_id=state.trace_id)

        # Execute the cognitive loop (same for mock and real modes)
        return await self._run_loop(state)

    async def _run_loop(self, state: HelixState) -> HelixState:
        """Execute the full cognitive loop (used by both mock and real modes)."""
        # Step 1: Perceive (already done - state has task)
        state.current_step = "perceive"
        logger.info("agent_loop.perceive", task=state.task)

        # Step 2: Assess priority (Amygdala)
        state.current_step = "assess_priority"
        priority = self.amygdala.assess_priority(state.task, state.trace_id)
        state.priority = priority
        logger.info(
            "agent_loop.assess_priority",
            score=priority.priority_score,
            intent=priority.intent_category,
        )

        # Step 3: Plan (Prefrontal)
        state.current_step = "plan"
        request = ReasoningRequest(
            task=state.task,
            system_prompt="You are Helix, a digital being.",
            trace_id=state.trace_id,
        )
        draft = self.prefrontal.generate_draft(request)
        state.reasoning_draft = draft
        logger.info("agent_loop.plan", has_tool_call=draft.tool_call is not None)

        # Step 4: Execute (Synapse) if tool call present
        state.current_step = "execute"
        if draft.tool_call:
            exec_request = ExecutionRequest(
                tool_name=draft.tool_call.get("name", ""),
                params=draft.tool_call.get("params", {}),
                trace_id=state.trace_id,
            )
            result = self.synapse.execute(exec_request)
            logger.info("agent_loop.execute", ok=result.ok)
        else:
            logger.info("agent_loop.execute", skipped=True)

        # Step 5: Reflect (Amygdala affect + Corpus Callosum)
        state.current_step = "reflect"
        affect = self.amygdala.evaluate_affect(
            state.task,
            draft.reasoning or draft.final_reply or "",
            state.trace_id,
        )
        state.affect = affect
        logger.info(
            "agent_loop.reflect",
            heliotropism=affect.heliotropism,
            pulse=affect.pulse,
            vigilance=affect.vigilance,
        )

        # Validate intent-execution alignment
        validation = self.validator.validate(
            intent=state.task,
            action=draft.reasoning or draft.final_reply or "",
            trace_id=state.trace_id,
        )
        state.validation_result = validation
        logger.info("agent_loop.validate", passed=validation.passed)

        # Step 6: Consolidate (update metabolism)
        state.current_step = "consolidate"
        # Token consumption is simulated for mock; in real mode, it would be
        # accumulated from actual Tuck API responses. This is a placeholder.
        state.metabolism.used_tokens += 100
        logger.info(
            "agent_loop.consolidate",
            used_tokens=state.metabolism.used_tokens,
            budget_total=state.metabolism.budget_total,
        )

        # Step 7: Sleep - end of epoch
        state.current_step = "sleep"
        logger.info("agent_loop.sleep", epoch_id=state.epoch_id)

        return state
