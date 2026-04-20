import pytest
from ana.core.corpus_callosum import CorpusCallosumValidator


def test_corpus_callosum_init(mock_settings):
    validator = CorpusCallosumValidator(mock_settings)
    assert validator.settings == mock_settings


def test_corpus_callosum_validate_not_implemented(mock_settings, trace_id):
    validator = CorpusCallosumValidator(mock_settings)
    with pytest.raises(NotImplementedError):
        validator.validate("test intent", "test action", trace_id)
