"""Unified configuration via pydantic-settings — zero hardcoding.  

Adopts a layered degradation strategy:  
- Mock mode is enabled by default for instant onboarding.  
- Production mode enforces strict vital‑dependency validation (fail‑fast).  
"""

from typing import Optional

from pydantic import Field, model_validator
from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        extra="ignore",
    )

    # ------------------------------------------------------------------
    # Core infrastructure — mandatory in production, optional in mock
    # ------------------------------------------------------------------
    tuck_endpoint: Optional[str] = None
    tuck_api_key: Optional[str] = None
    tuck_timeout: int = Field(30, alias="TUCK_TIMEOUT")
    tuck_chat_path: str = Field("/v1/chat/completions", alias="TUCK_CHAT_PATH")
    helix_mind_endpoint: Optional[str] = None

    # ------------------------------------------------------------------
    # Model routing — aliased for ANA_ prefix
    # ------------------------------------------------------------------
    amygdala_model: str = Field("qwen2.5:2b", alias="ANA_AMYGDALA_MODEL")
    left_brain_model: str = Field("qwen2.5-coder:7b", alias="ANA_LEFT_BRAIN_MODEL")
    right_brain_model: str = Field("deepseek-r1:8b", alias="ANA_RIGHT_BRAIN_MODEL")
    cerebellum_model: str = Field("qwen2.5:2b", alias="ANA_CEREBELLUM_MODEL")
    embedding_model: str = Field("BAAI/bge-small-en", alias="ANA_EMBEDDING_MODEL")

    # ------------------------------------------------------------------
    # Paths — aliased for ANA_ prefix
    # ------------------------------------------------------------------
    hxr_dir: str = Field("./memory_dag/sessions", alias="ANA_HXR_DIR")
    gene_lock_path: str = Field("./config/gene_lock.md", alias="ANA_GENE_LOCK_PATH")
    tools_path: str = Field("./config/tools.yaml", alias="ANA_TOOLS_PATH")

    # ------------------------------------------------------------------
    # Metabolism thresholds — aliased for ANA_ prefix
    # ------------------------------------------------------------------
    budget_total: int = Field(4096, alias="ANA_BUDGET_TOTAL")
    fatigue_line: float = Field(0.8, alias="ANA_FATIGUE_LINE")
    apoptosis_line: float = Field(0.95, alias="ANA_APOPTOSIS_LINE")
    max_duration_seconds: int = Field(300, alias="ANA_MAX_DURATION_SECONDS")
    cognitive_overload_line: int = Field(10, alias="ANA_COGNITIVE_OVERLOAD_LINE")

    # ------------------------------------------------------------------
    # Feature flags and operational settings — aliased for ANA_ prefix
    # ------------------------------------------------------------------
    mock_mode: bool = Field(True, alias="ANA_MOCK_MODE")
    log_level: str = Field("INFO", alias="ANA_LOG_LEVEL")
    enable_affect: bool = Field(True, alias="ANA_ENABLE_AFFECT")
    max_loops: int = Field(20, alias="ANA_MAX_LOOPS")

    # ------------------------------------------------------------------
    # AI Coder Safety Configuration — aliased for ANA_ prefix
    # ------------------------------------------------------------------
    disable_auto_web_search: bool = Field(False, alias="ANA_DISABLE_AUTO_WEB_SEARCH")
    enforce_tool_validation: bool = Field(True, alias="ANA_ENFORCE_TOOL_VALIDATION")
    max_auto_correction_rounds: int = Field(3, alias="ANA_MAX_AUTO_CORRECTION_ROUNDS")

    # ------------------------------------------------------------------
    # Post‑validation: gentle guidance in mock mode, hard‑fail in prod
    # ------------------------------------------------------------------
    @model_validator(mode="after")
    def _validate_critical_config(self) -> "Settings":
        """Perform layered validation: warn on missing deps in mock mode,
        raise an error in production mode."""
        import sys  # kept local for minimal scope

        missing: list[str] = []
        if not self.tuck_endpoint:
            missing.append("TUCK_ENDPOINT")
        if not self.tuck_api_key:
            missing.append("TUCK_API_KEY")
        if not self.helix_mind_endpoint:
            missing.append("HELIX_MIND_ENDPOINT")

        if not self.mock_mode:
            if missing:
                raise ValueError(
                    "Production mode (ANA_MOCK_MODE=false) requires "
                    f"the following environment variables: {', '.join(missing)}.\n"
                    "Please add them to your .env file or inject them via the system environment."
                )
        else:
            if missing:
                print(
                    "Framework is running in [MOCK safe mode] (Zero-Config). "
                    "Missing configs: {}. "
                    "To connect real LLM and memory nodes, configure the "
                    "required environment variables and set ANA_MOCK_MODE=false.".format(
                        ", ".join(missing)
                    ),
                    file=sys.stderr,
                )
            else:
                print(
                    "Environment variables are ready but ANA_MOCK_MODE is still True. "
                    "If this is a production deployment, disable mock mode.",
                    file=sys.stderr,
                )

        return self


# cache instance
_settings: Optional[Settings] = None


def get_settings() -> Settings:
    """Return a singleton Settings instance (lazy load)."""
    global _settings
    if _settings is None:
        _settings = Settings()
    return _settings
