"""Tests for Synapse module."""

from ana.core.gene_lock import GeneLockValidator
from ana.core.synapse import DualTagParser, Synapse
from ana.core.registry import ToolRegistry


class TestDualTagParser:
    """Test cases for DualTagParser class."""

    def test_parse_single_tag(self):
        """Test parsing a single tool tag."""
        text = 'Some reasoning <tool name="search">{"query": "hello"}</tool>'
        result = DualTagParser.parse(text)

        assert len(result) == 1
        assert result[0] == ("search", '{"query": "hello"}')

    def test_parse_multiple_tags(self):
        """Test parsing multiple tool tags."""
        text = """
        First call: <tool name="fetch">{"id": "1"}</tool>
        Second call: <tool name="search">{"q": "test"}</tool>
        """
        result = DualTagParser.parse(text)

        assert len(result) == 2
        assert result[0] == ("fetch", '{"id": "1"}')
        assert result[1] == ("search", '{"q": "test"}')

    def test_extract_first(self):
        """Test extracting first tool tag."""
        text = '<tool name="tool1">{"a": 1}</tool><tool name="tool2">{"b": 2}</tool>'
        result = DualTagParser.extract_first(text)

        assert result == ("tool1", '{"a": 1}')

    def test_no_tags(self):
        """Test text without tool tags."""
        text = "Just regular text without any tool calls"
        result = DualTagParser.parse(text)

        assert result == []
        assert not DualTagParser.has_tool_tags(text)

    def test_has_tool_tags(self):
        """Test checking for tool tags presence."""
        assert DualTagParser.has_tool_tags('<tool name="x">{}</tool>')
        assert not DualTagParser.has_tool_tags("no tags here")


class TestSynapse:
    """Test cases for Synapse class."""

    def test_init_default(self):
        """Test Synapse initialization with defaults."""
        synapse = Synapse()

        assert synapse.registry is not None
        assert synapse.gene_lock is not None

    def test_init_custom(self):
        """Test Synapse initialization with custom components."""
        registry = ToolRegistry()
        gene_lock = GeneLockValidator()
        synapse = Synapse(registry=registry, gene_lock=gene_lock)

        assert synapse.registry == registry
        assert synapse.gene_lock == gene_lock

    def test_validate_unknown_tool(self):
        """Test validation of unknown tool."""
        synapse = Synapse()
        is_valid, error = synapse.validate_tool_call("nonexistent", {})

        assert not is_valid
        assert "Unknown tool" in error

    def test_parse_llm_response(self):
        """Test parsing LLM response with tool calls."""
        synapse = Synapse()
        text = """
        I need to search for information.
        <tool name="ana_tentacle">{"keyword": "Python"}</tool>
        Then I'll fetch the results.
        """
        result = synapse.parse_llm_response(text)

        assert len(result) == 1
        assert result[0]["tool_name"] == "ana_tentacle"
        assert result[0]["params"] == {"keyword": "Python"}

    def test_parse_invalid_json_params(self):
        """Test parsing with invalid JSON parameters."""
        synapse = Synapse()
        text = '<tool name="tool">invalid json</tool>'
        result = synapse.parse_llm_response(text)

        assert len(result) == 1
        assert result[0]["tool_name"] == "tool"
        assert "error" in result[0]
