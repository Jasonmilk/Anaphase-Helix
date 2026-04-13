"""Configuration management for Helix-Ana."""

from pydantic_settings import BaseSettings
from pydantic import Field


class Settings(BaseSettings):
    """Helix-Ana configuration settings."""

    # Tuck gateway
    tuck_endpoint: str = Field(
        "http://10.0.0.54:8686/v1/chat/completions",
        env="TUCK_ENDPOINT"
    )
    tuck_api_key: str = Field("local", env="TUCK_API_KEY")

    # Helix-Mind
    helix_mind_endpoint: str = Field(
        "http://10.0.0.202:8020",
        env="HELIX_MIND_ENDPOINT"
    )

    # Models
    left_brain_model: str = Field(
        "qwen2.5-coder:7b",
        env="ANA_LEFT_BRAIN_MODEL"
    )
    right_brain_model: str = Field(
        "deepseek-r1:8b",
        env="ANA_RIGHT_BRAIN_MODEL"
    )
    cerebellum_model: str = Field(
        "qwen2.5:2b",
        env="ANA_CEREBELLUM_MODEL"
    )
    embedding_model: str = Field(
        "BAAI/bge-small-en",
        env="ANA_EMBEDDING_MODEL"
    )

    # Paths
    hxr_dir: str = Field(
        "./memory_dag/sessions",
        env="ANA_HXR_DIR"
    )
    gene_lock_path: str = Field(
        "./knowledge_base/l0_gene_lock.md",
        env="ANA_GENE_LOCK_PATH"
    )

    # Scheduling weights
    alpha: float = 0.35
    beta: float = 0.35
    gamma: float = 0.20
    delta: float = -0.10

    class Config:
        env_file = ".env"
        env_file_encoding = "utf-8"


def load_config() -> Settings:
    """Load configuration from environment."""
    return Settings()
