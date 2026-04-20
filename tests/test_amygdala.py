import pytest
from ana.core.amygdala import Amygdala
from ana.schemas.priority import PriorityAssessment, IntentCategory
from ana.schemas.affect import AffectVector


def test_amygdala_init():
    amygdala = Amygdala()
    assert amygdala.settings is not None


def test_assess_priority_mock(monkeypatch):
    from ana.common import get_settings
    settings = get_settings()
    monkeypatch.setattr(settings, "mock_mode", True)

    amygdala = Amygdala()
    result = amygdala.assess_priority("test task", "trace_123")

    assert isinstance(result, PriorityAssessment)
    assert result.priority_score == 75.0
    assert result.intent_category == IntentCategory.TASK


def test_evaluate_affect_mock(monkeypatch):
    from ana.common import get_settings
    settings = get_settings()
    monkeypatch.setattr(settings, "mock_mode", True)

    amygdala = Amygdala()
    result = amygdala.evaluate_affect("task", "result", "trace_123")

    assert isinstance(result, AffectVector)
    assert result.heliotropism == 0.6
    assert result.pulse == 0.4
    assert result.vigilance == 0.1
