"""Configuration management using pydantic-settings. No hardcoded defaults for model names."""

from pydantic_settings import BaseSettings
from pydantic import Field, ValidationError
from typing import Optional


class Settings(BaseSettings):
    """Helix Anaphase configuration, loaded from .env and environment."""

    # Tuck gateway
    tuck_endpoint: str = Field(
        ...,
        description="Tuck gateway base URL (e.g., http://localhost:8686)"
    )
    tuck_api_key: str = Field(
        ...,
        description="API key for Tuck gateway"
    )

    # Helix-Mind service
    helix_mind_base_url: str = Field(
        "http://localhost:8020",
        description="Helix-Mind microservice base URL"
    )

    # Model routing - explicit env mapping for ANA_ prefix
    left_brain_model: Optional[str] = Field(
        None,
        env="ANA_LEFT_BRAIN_MODEL",
        description="Model for medium priority tasks (e.g., 7B)"
    )
    right_brain_model: Optional[str] = Field(
        None,
        env="ANA_RIGHT_BRAIN_MODEL",
        description="Model for high priority tasks (e.g., 8B)"
    )
    cerebellum_model: Optional[str] = Field(
        None,
        env="ANA_CEREBELLUM_MODEL",
        description="Model for low priority tasks (e.g., 2B)"
    )

    # Embedding
    embedding_model: str = Field(
        "BAAI/bge-small-en",
        env="ANA_EMBEDDING_MODEL",
        description="Local embedding model name"
    )

    # Paths
    hxr_dir: str = Field(
        "./memory_dag/sessions",
        env="ANA_HXR_DIR",
        description="Directory for HXR audit logs"
    )
    gene_lock_path: str = Field(
        "./knowledge_base/l0_gene_lock.md",
        env="ANA_GENE_LOCK_PATH",
        description="Path to L0 gene lock rules"
    )

    # Scheduling weights
    alpha: float = 0.35
    beta: float = 0.35
    gamma: float = 0.20
    delta: float = -0.10

    # Agent Loop
    max_loops: int = Field(
        20,
        description="Maximum iterations per Agent Loop"
    )

    class Config:
        env_file = ".env"
        env_file_encoding = "utf-8"
        extra = "ignore"

    def validate_model_config(self) -> None:
        """Ensure at least one model is configured."""
        if not any([self.left_brain_model, self.right_brain_model, self.cerebellum_model]):
            raise ValueError(
                "At least one of ANA_LEFT_BRAIN_MODEL, ANA_RIGHT_BRAIN_MODEL, "
                "or ANA_CEREBELLUM_MODEL must be set in .env"
            )


def load_config() -> Settings:
    """Load and validate configuration."""
    try:
        settings = Settings()
        settings.validate_model_config()
        return settings
    except ValidationError as e:
        print(f"Configuration error: {e}")
        raise
