"""Tuck gateway adapter with flexible model routing."""

import json
from typing import Dict, Any

import httpx
from httpx import HTTPStatusError


class TuckBackend:
    """Async client for Tuck model gateway."""

    def __init__(self, endpoint: str, api_key: str):
        self.endpoint = endpoint.rstrip('/')
        self.api_key = api_key

    async def generate(self, prompt: str, model: str, **kwargs) -> str:
        if not model or not model.strip():
            raise ValueError("Model name cannot be empty")

        url = f"{self.endpoint}/v1/chat/completions"
        payload = {
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": kwargs.get("temperature", 0.7),
            "max_tokens": kwargs.get("max_tokens", 2048),
            "stream": False
        }
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json"
        }

        async with httpx.AsyncClient(
            timeout=httpx.Timeout(300.0, connect=10.0),
            trust_env=False
        ) as client:
            try:
                resp = await client.post(url, json=payload, headers=headers)
                resp.raise_for_status()
                data = resp.json()
                return data["choices"][0]["message"]["content"]
            except HTTPStatusError as e:
                detail = ""
                try:
                    detail = e.response.json().get("detail", "")
                except Exception:
                    pass
                raise HTTPStatusError(
                    f"Tuck gateway error (model '{model}'): {detail or e.response.text}",
                    request=e.request,
                    response=e.response
                ) from e

    async def close(self):
        pass


class MockBackend:
    """Mock backend for testing without real LLM."""

    async def generate(self, prompt: str, model: str, **kwargs) -> str:
        # Simple mock response for amygdala or chat
        if "Amygdala" in prompt:
            return '{"priority_score": 60, "emotion_tag": "neutral", "intent_category": "chat"}'
        return "This is a mock response from Helix (mock mode)."
