import pytest
from unittest.mock import Mock
from ana.common import Settings
from ana.registry import ToolRegistry


@pytest.fixture
def trace_id():
    return "test_trace_1234567890"


@pytest.fixture
def epoch_id():
    return "epoch_test_123456"


@pytest.fixture
def mock_settings():
    return Mock(spec=Settings,
        tuck_endpoint="http://localhost:8686",
        tuck_api_key="test",
        helix_mind_endpoint="http://localhost:8020",
        amygdala_model="test-model",
        left_brain_model="test-model",
        right_brain_model="test-model",
        cerebellum_model="test-model",
        budget_total=4096,
        fatigue_line=0.8,
        apoptosis_line=0.95,
        max_duration_seconds=300,
        cognitive_overload_line=10,
    )


@pytest.fixture
def mock_tool_registry():
    return Mock(spec=ToolRegistry)
