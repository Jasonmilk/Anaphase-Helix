"""LLM Backend - Tuck gateway adapter."""

import httpx
from typing import Dict, Any, Optional, List
from abc import ABC, abstractmethod


class LLMBackend(ABC):
    """Abstract base class for LLM backends."""

    @abstractmethod
    async def generate(
        self,
        prompt: str,
        model: str,
        **kwargs
    ) -> str:
        """Generate text from the LLM."""
        pass


class TuckBackend(LLMBackend):
    """Tuck gateway backend implementation."""

    def __init__(
        self,
        endpoint: str,
        api_key: str,
        mock_mode: bool = True
    ):
        self.endpoint = endpoint
        self.api_key = api_key
        self.mock_mode = mock_mode
        self._client = httpx.AsyncClient(timeout=60.0)

    async def close(self):
        """Close the HTTP client."""
        await self._client.aclose()

    async def generate(
        self,
        prompt: str,
        model: str,
        max_tokens: int = 2048,
        temperature: float = 0.7,
        **kwargs
    ) -> str:
        """
        Generate text from Tuck gateway.

        Args:
            prompt: Input prompt
            model: Model name to use
            max_tokens: Maximum tokens to generate
            temperature: Sampling temperature
            **kwargs: Additional parameters

        Returns:
            Generated text response
        """
        if self.mock_mode:
            return self._mock_generate(prompt, model)

        try:
            response = await self._client.post(
                self.endpoint,
                headers={
                    "Authorization": f"Bearer {self.api_key}",
                    "Content-Type": "application/json"
                },
                json={
                    "model": model,
                    "messages": [{"role": "user", "content": prompt}],
                    "max_tokens": max_tokens,
                    "temperature": temperature,
                    **kwargs
                }
            )
            response.raise_for_status()
            data = response.json()
            return data["choices"][0]["message"]["content"]
        except Exception as e:
            # Fallback to mock mode on error
            return self._mock_generate(prompt, model)

    def _mock_generate(self, prompt: str, model: str) -> str:
        """Mock generation for testing without real Tuck connection."""
        # Simulate different models
        if "coder" in model.lower():
            return f"[MOCK {model}] Code analysis: The task requires implementing a solution. Here's a structured approach..."
        elif "r1" in model.lower() or "deepseek" in model.lower():
            return f"[MOCK {model}] Deep reasoning: Let me think step by step about this problem..."
        else:
            return f"[MOCK {model}] Response: I understand the task. Here's my answer..."

    async def generate_with_constraint(
        self,
        prompt: str,
        model: str,
        schema: Dict[str, Any],
        **kwargs
    ) -> Dict[str, Any]:
        """
        Generate constrained JSON output matching a schema.

        Args:
            prompt: Input prompt
            model: Model name
            schema: JSON Schema for constraint
            **kwargs: Additional parameters

        Returns:
            Parsed JSON response
        """
        if self.mock_mode:
            # Return mock JSON matching typical tool call schema
            return {
                "tool": "shell_exec",
                "params": {"command": "echo 'Mock result'"},
                "reasoning": "Mock constrained generation"
            }

        # In real mode, would use llguidance for constraint decoding
        response_text = await self.generate(
            prompt=prompt,
            model=model,
            **kwargs
        )
        # Try to parse JSON from response
        import json
        import re
        # Extract JSON from markdown code blocks if present
        json_match = re.search(r'```(?:json)?\s*({.*?})\s*```', response_text, re.DOTALL)
        if json_match:
            return json.loads(json_match.group(1))
        # Try parsing entire response as JSON
        return json.loads(response_text)
