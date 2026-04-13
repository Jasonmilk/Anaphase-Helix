"""Test configuration and fixtures for Helix-Ana."""

import shutil
import tempfile
from pathlib import Path

import pytest


@pytest.fixture
def temp_dir():
    """Create a temporary directory for tests.

    Yields:
        Path to temporary directory
    """
    tmpdir = tempfile.mkdtemp()
    yield Path(tmpdir)
    shutil.rmtree(tmpdir, ignore_errors=True)


@pytest.fixture
def sample_config():
    """Sample configuration settings for testing."""
    from ana.core.config import Settings

    return Settings(
        tuck_endpoint="http://test:8686/v1/chat/completions",
        tuck_api_key="test-key",
        helix_mind_endpoint="http://test:8020",
        left_brain_model="test-model-1",
        right_brain_model="test-model-2",
        cerebellum_model="test-model-3",
        embedding_model="test-embedding",
        hxr_dir="./test_sessions",
        gene_lock_path="./test_gene_lock.md",
        log_level="DEBUG",
    )


@pytest.fixture
def mock_httpx_client():
    """Mock HTTPX client for testing API calls."""
    import httpx

    class MockResponse:
        def __init__(self, status_code: int, json_data: dict):
            self.status_code = status_code
            self._json_data = json_data

        def raise_for_status(self):
            if self.status_code >= 400:
                raise httpx.HTTPError(f"HTTP {self.status_code}")

        def json(self):
            return self._json_data

    class MockClient:
        def __init__(self):
            self.is_closed = False

        def get(self, url, params=None):
            return MockResponse(200, {"result": "mocked"})

        def post(self, url, json=None):
            return MockResponse(200, {"result": "mocked"})

        def close(self):
            self.is_closed = True

    return MockClient()
