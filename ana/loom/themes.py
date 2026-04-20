"""Theme definitions for Ana Loom — Zero hardcoding, contract-first."""

from typing import Dict, Literal, TypedDict, Union


class ThemeColors(TypedDict):
    """Structured theme color contract."""
    bg_dark: str
    border: str
    text_primary: str
    text_secondary: str
    thinking: str
    affect: str
    success: str
    error: str
    highlight: str
    glow_weak: str
    hover_bg: str
    divider: str


# ------------------------------------------------------------------------
# Ana Theme — 克制高阶版 (Cultivation Dish / Thalamus / Corpus Callosum)
# ------------------------------------------------------------------------
ANA_THEME: ThemeColors = {
    "bg_dark": "#080d18",
    "border": "#2a3148",
    "text_primary": "#e2e8f0",
    "text_secondary": "#566385",
    "thinking": "#14a8d8",
    "affect": "#b86ea9",
    "success": "#2e9c58",
    "error": "#b93a3a",
    "highlight": "#d4d288",
    "glow_weak": "rgba(20, 168, 216, 0.12)",
    "hover_bg": "#121a2b",
    "divider": "#1f273b",
}


# Registry of all available themes (extensible without modifying code)
THEME_REGISTRY: Dict[str, ThemeColors] = {
    "ana": ANA_THEME,
    # Additional themes can be registered here
}


def get_theme(name: Literal["ana"] = "ana") -> ThemeColors:
    """Retrieve theme by name. Zero hardcoding, contract-compliant."""
    if name not in THEME_REGISTRY:
        raise ValueError(f"Theme '{name}' not registered. Available: {list(THEME_REGISTRY.keys())}")
    return THEME_REGISTRY[name]
