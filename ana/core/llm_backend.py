"""Tuck gateway adapter with flexible model routing. No hardcoded model names."""

import json
from typing import Dict, Any, Optional

import httpx
from httpx import HTTPStatusError


class TuckBackend:
    """Async client for Tuck model gateway."""

    def __init__(self, endpoint: str, api_key: str):
        # Ensure endpoint does not end with slash
        self.endpoint = endpoint.rstrip('/')
        self.api_key = api_key
        self._client: Optional[httpx.AsyncClient] = None

    async def _get_client(self) -> httpx.AsyncClient:
        if self._client is None:
            self._client = httpx.AsyncClient(
                timeout=httpx.Timeout(60.0),
                headers={
                    "Authorization": f"Bearer {self.api_key}",
                    "Content-Type": "application/json"
                }
            )
        return self._client

    async def generate(self, prompt: str, model: str, **kwargs) -> str:
        """
        Send a completion request to Tuck.

        Args:
            prompt: Full prompt text
            model: Model identifier as understood by Tuck (e.g., "qwen2.5-coder:7b")
            **kwargs: Additional parameters (temperature, max_tokens, etc.)

        Returns:
            Generated text response

        Raises:
            ValueError: If model is empty or invalid
            HTTPStatusError: On HTTP errors
        """
        if not model or not model.strip():
            raise ValueError("Model name cannot be empty")

        # Construct full URL (OpenAI-compatible path)
        url = f"{self.endpoint}/v1/chat/completions"

        payload = {
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": kwargs.get("temperature", 0.7),
            "max_tokens": kwargs.get("max_tokens", 2048),
            "stream": False
        }

        client = await self._get_client()
        try:
            resp = await client.post(url, json=payload)
            resp.raise_for_status()
            data = resp.json()
            # Extract content from OpenAI-compatible response
            return data["choices"][0]["message"]["content"]
        except HTTPStatusError as e:
            # Add context to the error
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
        except KeyError:
            raise ValueError(f"Unexpected response format from Tuck: {data}")

    async def close(self):
        """Close the HTTP client."""
        if self._client:
            await self._client.aclose()
            self._client = None
