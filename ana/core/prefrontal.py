"""Prefrontal module: Logical reasoning and planning."""

from ana.common import logger, llm_retry, get_settings
from ana.schemas.reasoning import ReasoningRequest, ReasoningDraft


class Prefrontal:
    """Prefrontal cortex module responsible for logical reasoning and planning."""

    def __init__(self) -> None:
        """Initialize Prefrontal with application settings."""
        self.settings = get_settings()

    @llm_retry
    def generate_draft(self, request: ReasoningRequest) -> ReasoningDraft:
        """
        Generate reasoning draft based on input request.

        Args:
            request: ReasoningRequest containing task, memory, etc.

        Returns:
            ReasoningDraft with reasoning, tool call, or final reply.
        """
        logger.info(
            "prefrontal.generate_draft.start",
            task=request.task,
            trace_id=request.trace_id,
        )

        if self.settings.mock_mode:
            return ReasoningDraft(
                reasoning=f"Mock reasoning for task: {request.task}. "
                          "This is a placeholder response generated in mock mode.",
                tool_call=None,
                final_reply=None,
            )

        raise NotImplementedError(
            "Prefrontal.generate_draft real implementation is not available. "
            "Set ANA_MOCK_MODE=true in .env to use mock responses."
        )
