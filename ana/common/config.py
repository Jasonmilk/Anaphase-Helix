"""Unified configuration via pydantic-settings. Zero hardcoding."""

from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        extra="ignore",
    )

    tuck_endpoint: str
    tuck_api_key: str
    helix_mind_endpoint: str

    amygdala_model: str = "qwen2.5:2b"
    left_brain_model: str = "qwen2.5-coder:7b"
    right_brain_model: str = "deepseek-r1:8b"
    cerebellum_model: str = "qwen2.5:2b"
    embedding_model: str = "BAAI/bge-small-en"

    hxr_dir: str = "./memory_dag/sessions"
    gene_lock_path: str = "./config/gene_lock.md"
    tools_path: str = "./config/tools.yaml"

    budget_total: int = 4096
    fatigue_line: float = 0.8
    apoptosis_line: float = 0.95
    max_duration_seconds: int = 300
    cognitive_overload_line: int = 10

    mock_mode: bool = False
    log_level: str = "INFO"
    enable_affect: bool = True
    max_loops: int = 20

    # AI Coder Safety Configuration
    disable_auto_web_search: bool = False
    enforce_tool_validation: bool = True
    max_auto_correction_rounds: int = 3


_settings: Settings | None = None


def get_settings() -> Settings:
    global _settings
    if _settings is None:
        _settings = Settings()
    return _settings
