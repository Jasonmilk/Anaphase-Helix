"""Prefrontal module: Logical reasoning and planning."""

import json
import re
import httpx

from ana.common import logger, llm_retry, get_settings
from ana.schemas.reasoning import ReasoningRequest, ReasoningDraft
from ana.schemas.exceptions import TuckRejectionError


class Prefrontal:
    """Prefrontal cortex module responsible for logical reasoning and planning."""

    def __init__(self) -> None:
        """Initialize Prefrontal with application settings."""
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
        match = re.search(r"```(?:json)?\s*(\{.*?\})\s*```", response, re.DOTALL)
        if match:
            return match.group(1).strip()
        match = re.search(r"\{.*\}", response, re.DOTALL)
        if match:
            return match.group(0).strip()
        return response

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

        return self._call_tuck_for_reasoning(request)

    def _call_tuck_for_reasoning(self, request: ReasoningRequest) -> ReasoningDraft:
        """Call Tuck API to generate reasoning draft."""
        prompt = self._build_reasoning_prompt(request)
        model = self._select_model(request)
        response = self._call_tuck(prompt, model, request.trace_id)
        return self._parse_reasoning_response(response)

    def _build_reasoning_prompt(self, request: ReasoningRequest) -> str:
        """Build prompt for reasoning."""
        context = ""
        if request.working_memory:
            context += "Working Memory:\n" + "\n".join(request.working_memory) + "\n\n"
        if request.episodic_memory:
            context += "Episodic Memory:\n"
            for mem in request.episodic_memory:
                context += f"- {mem}\n"
            context += "\n"

        prompt = f"""{request.system_prompt}

{context}User Task: {request.task}

You may respond with:
1. Plain reasoning text.
2. A tool call in JSON format: {{"name": "<tool_name>", "params": {{...}}}}
3. A final reply to the user.

Output your response. If tool call, ensure it is valid JSON."""
        return prompt

    def _select_model(self, request: ReasoningRequest) -> str:
        """Select appropriate model based on task complexity (simplified)."""
        return self.settings.left_brain_model

    def _parse_reasoning_response(self, response: str) -> ReasoningDraft:
        """Parse LLM response into ReasoningDraft."""
        response = response.strip()
        # Try to parse as JSON tool call
        try:
            json_str = self._extract_json(response)
            data = json.loads(json_str)
            if "name" in data and "params" in data:
                return ReasoningDraft(
                    reasoning=None,
                    tool_call=data,
                    final_reply=None,
                )
        except (json.JSONDecodeError, ValueError):
            pass

        # Otherwise treat as reasoning or final reply
        if len(response) < 500 and "?" not in response:
            return ReasoningDraft(
                reasoning=None,
                tool_call=None,
                final_reply=response,
            )
        else:
            return ReasoningDraft(
                reasoning=response,
                tool_call=None,
                final_reply=None,
            )

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
            "temperature": 0.7,
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
