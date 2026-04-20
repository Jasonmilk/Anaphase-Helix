"""Amygdala module: Priority and affect assessment."""

import json
import re
import httpx

from ana.common import logger, llm_retry, get_settings
from ana.schemas.priority import PriorityAssessment, IntentCategory
from ana.schemas.affect import AffectVector
from ana.schemas.exceptions import TuckRejectionError


class Amygdala:
    """
    Amygdala module responsible for:
    1. Pre-reasoning priority assessment
    2. Post-reasoning 3D affect vector evaluation
    """

    def __init__(self) -> None:
        """Initialize Amygdala with application settings."""
        self.settings = get_settings()

    def _extract_json(self, response: str) -> str:
        """
        Extract pure JSON string from LLM response.

        Handles common patterns:
        - Markdown code blocks: ```json ... ``` or ``` ... ```
        - Raw JSON objects embedded in text.

        Args:
            response: Raw response string from LLM.

        Returns:
            Cleaned JSON string.
        """
        response = response.strip()
        # Try to extract from markdown code block
        match = re.search(r"```(?:json)?\s*(\{.*?\})\s*```", response, re.DOTALL)
        if match:
            return match.group(1).strip()
        # Fallback: find first JSON object
        match = re.search(r"\{.*\}", response, re.DOTALL)
        if match:
            return match.group(0).strip()
        return response

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

        return self._call_tuck_for_priority(task, trace_id)

    def _call_tuck_for_priority(self, task: str, trace_id: str) -> PriorityAssessment:
        """Call Tuck API to assess priority."""
        prompt = self._build_priority_prompt(task)
        response = self._call_tuck(prompt, self.settings.amygdala_model, trace_id)
        return self._parse_priority_response(response)

    def _build_priority_prompt(self, task: str) -> str:
        """Build prompt for priority assessment."""
        return f"""You are Helix's Amygdala, responsible for assessing task priority and intent.

Given the user's task, output a JSON object with:
- priority_score: float between 0.0 and 100.0, indicating urgency/importance.
- intent_category: one of "chat", "knowledge_retrieval", "social_graph_read", "task".

Task: {task}

Output ONLY valid JSON, no other text."""

    def _parse_priority_response(self, response: str) -> PriorityAssessment:
        """Parse LLM response into PriorityAssessment DTO."""
        try:
            json_str = self._extract_json(response)
            data = json.loads(json_str)
            return PriorityAssessment(
                priority_score=float(data.get("priority_score", 50.0)),
                intent_category=IntentCategory(data.get("intent_category", "chat")),
            )
        except (json.JSONDecodeError, KeyError, ValueError) as e:
            logger.error("amygdala.parse_priority.failed", error=str(e), response=response)
            raise TuckRejectionError(f"Failed to parse priority response: {e}")

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

        return self._call_tuck_for_affect(task, result, trace_id)

    def _call_tuck_for_affect(self, task: str, result: str, trace_id: str) -> AffectVector:
        """Call Tuck API to evaluate affect vector."""
        prompt = self._build_affect_prompt(task, result)
        response = self._call_tuck(prompt, self.settings.amygdala_model, trace_id)
        return self._parse_affect_response(response)

    def _build_affect_prompt(self, task: str, result: str) -> str:
        """Build prompt for affect evaluation."""
        return f"""You are Helix's Amygdala, responsible for evaluating emotional affect.

Given the task and the reasoning result, output a JSON object with:
- heliotropism: float between -1.0 (negative) and 1.0 (positive).
- pulse: float between 0.0 (calm) and 1.0 (aroused).
- vigilance: float between 0.0 (relaxed) and 1.0 (alert).

Task: {task}
Result: {result}

Output ONLY valid JSON, no other text."""

    def _parse_affect_response(self, response: str) -> AffectVector:
        """Parse LLM response into AffectVector DTO."""
        try:
            json_str = self._extract_json(response)
            data = json.loads(json_str)
            return AffectVector(
                heliotropism=float(data.get("heliotropism", 0.0)),
                pulse=float(data.get("pulse", 0.5)),
                vigilance=float(data.get("vigilance", 0.1)),
            )
        except (json.JSONDecodeError, KeyError, ValueError) as e:
            logger.error("amygdala.parse_affect.failed", error=str(e), response=response)
            raise TuckRejectionError(f"Failed to parse affect response: {e}")

    def _call_tuck(self, prompt: str, model: str, trace_id: str) -> str:
        """Call Tuck API with given prompt and model."""
        headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {self.settings.tuck_api_key}",
            "X-Trace-Id": trace_id,
        }
        payload = {
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0.0,
        }

        try:
            with httpx.Client(timeout=self.settings.tuck_timeout) as client:
                resp = client.post(
                    f"{self.settings.tuck_endpoint}{self.settings.tuck_chat_path}",
                    headers=headers,
                    json=payload,
                )
                resp.raise_for_status()
                data = resp.json()
                return data["choices"][0]["message"]["content"]
        except httpx.HTTPStatusError as e:
            logger.error("tuck.http_error", status_code=e.response.status_code, trace_id=trace_id)
            raise TuckRejectionError(f"Tuck returned {e.response.status_code}")
        except Exception as e:
            logger.error("tuck.call_failed", error=str(e), trace_id=trace_id)
            raise TuckRejectionError(f"Tuck call failed: {e}")
