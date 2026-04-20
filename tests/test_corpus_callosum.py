import pytest
from ana.core.corpus_callosum import CorpusCallosumValidator
from ana.schemas.validation import ValidationResult


def test_corpus_callosum_init():
    validator = CorpusCallosumValidator()
    assert validator.settings is not None


def test_validate_mock_mode(monkeypatch, trace_id):
    from ana.common import get_settings
    settings = get_settings()
    monkeypatch.setattr(settings, "mock_mode", True)

    validator = CorpusCallosumValidator()
    result = validator.validate(
        intent="Fetch user data from API",
        action="Execute tool 'api_fetch' with params",
        trace_id=trace_id,
    )

    assert isinstance(result, ValidationResult)
    assert result.passed is True
    assert result.reason is None
    assert result.action == "proceed"
    assert result.flags == {}
    assert result.mismatch_report is None
