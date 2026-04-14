"""Configuration management for Helix-Ana"""

from pydantic import Field
from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    """Helix-Ana configuration settings."""

    # Tuck gateway
    tuck_endpoint: str = Field(
        default="http://localhost:8686/v1/chat/completions",
        description="Tuck model gateway endpoint",
    )
    tuck_api_key: str = Field(default="local", description="Tuck API key")

    # Helix-Mind
    helix_mind_endpoint: str = Field(
        default="http://localhost:8020",
        description="Helix-Mind memory service endpoint",
    )

    # Model routing - four independent brain regions
    amygdala_model: str = Field(
        default="qwen2.5:2b",
        description="Amygdala: Emotion & risk detection, lightweight fast response",
    )
    cerebellum_model: str = Field(
        default="qwen2.5:2b",
        description="Cerebellum: Action timing & scheduling, low-latency stable execution",
    )
    left_brain_model: str = Field(
        default="qwen2.5-coder:7b",
        description="Left Brain: Logical & code execution, structured rational output",
    )
    right_brain_model: str = Field(
        default="deepseek-r1:8b",
        description="Right Brain: Intuition & semantic understanding, deep thinking",
    )

    # Embedding model
    embedding_model: str = Field(
        default="BAAI/bge-small-en",
        description="Embedding model: pure text-to-vector, NO thinking allowed",
    )

    # Mock mode
    mock_mode: bool = Field(
        default=False,
        description="Enable mock backend for testing (bypasses real LLM calls)",
    )

    # Paths
    hxr_dir: str = Field(
        default="./memory_dag/sessions",
        description="Directory for HXR session logs",
    )
    gene_lock_path: str = Field(
        default="./knowledge_base/l0_gene_lock.md",
        description="Path to L0 gene lock rules",
    )

    # Scheduling weights
    alpha: float = Field(default=0.35, description="Recency weight")
    beta: float = Field(default=0.35, description="Relevance weight")
    gamma: float = Field(default=0.20, description="Frequency weight")
    delta: float = Field(default=-0.10, description="Decay factor")

    # Logging
    log_level: str = Field(default="INFO", description="Logging level")

    class Config:
        extra = "ignore"


def load_config() -> Settings:
    """
    Load configuration. This function is kept for compatibility but is no longer
    responsible for loading .env; that is done in cli.py.
    """
    return Settings()
