"""Amygdala module: Priority and affect assessment."""

from ana.common import logger, llm_retry, get_settings
from ana.schemas.priority import PriorityAssessment, IntentCategory
from ana.schemas.affect import AffectVector


class Amygdala:
    """
    Amygdala module responsible for:
    1. Pre-reasoning priority assessment
    2. Post-reasoning 3D affect vector evaluation
    """

    def __init__(self) -> None:
        """Initialize Amygdala with application settings."""
        self.settings = get_settings()

    @llm_retry
    def assess_priority(self, task: str, trace_id: str) -> PriorityAssessment:
        """
        Assess task priority and intent category.

        Args:
            task: The user task to assess.
            trace_id: Global trace ID for audit.

        Returns:
            PriorityAssessment with score and intent category.
        """
        logger.info("amygdala.assess_priority.start", task=task, trace_id=trace_id)

        if self.settings.mock_mode:
            return PriorityAssessment(
                priority_score=75.0,
                intent_category=IntentCategory.TASK,
            )

        raise NotImplementedError(
            "Amygdala.assess_priority real implementation is not available. "
            "Set ANA_MOCK_MODE=true in .env to use mock responses."
        )

    @llm_retry
    def evaluate_affect(self, task: str, result: str, trace_id: str) -> AffectVector:
        """
        Evaluate 3D affect vector after reasoning.

        Args:
            task: The original user task.
            result: The reasoning result.
            trace_id: Global trace ID for audit.

        Returns:
            3D AffectVector with heliotropism, pulse, and vigilance.
        """
        logger.info("amygdala.evaluate_affect.start", trace_id=trace_id)

        if self.settings.mock_mode:
            return AffectVector(
                heliotropism=0.6,
                pulse=0.4,
                vigilance=0.1,
            )

        raise NotImplementedError(
            "Amygdala.evaluate_affect real implementation is not available. "
            "Set ANA_MOCK_MODE=true in .env to use mock responses."
        )
