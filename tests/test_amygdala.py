import pytest
from ana.core.amygdala import Amygdala


def test_amygdala_init(mock_settings):
    amygdala = Amygdala(mock_settings)
    assert amygdala.settings == mock_settings


def test_amygdala_assess_priority_not_implemented(mock_settings):
    amygdala = Amygdala(mock_settings)
    with pytest.raises(NotImplementedError):
        amygdala.assess_priority("test task", "test_trace")


def test_amygdala_evaluate_affect_not_implemented(mock_settings):
    amygdala = Amygdala(mock_settings)
    with pytest.raises(NotImplementedError):
        amygdala.evaluate_affect("test task", "test result", "test_trace")
