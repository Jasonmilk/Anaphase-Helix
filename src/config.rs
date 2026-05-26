//! Configuration loader for Anaphase-Helix.
//!
//! Currently a placeholder. In later versions this will parse
//! config.toml and populate the adapter endpoints, cognitive model
//! settings, and immune thresholds.

/// Top-level configuration struct.
#[derive(Debug, Clone)]
pub struct Config {
    pub anaphase: AnaphaseConfig,
}

#[derive(Debug, Clone)]
pub struct AnaphaseConfig {
    pub mind_endpoint: Option<String>,
    pub flowmodus_endpoint: Option<String>,
    pub tentacle_endpoint: Option<String>,
    pub tuck_endpoint: Option<String>,
    pub callosum_endpoint: Option<String>,
    pub cellrix_endpoint: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            anaphase: AnaphaseConfig::default(),
        }
    }
}

impl Default for AnaphaseConfig {
    fn default() -> Self {
        Self {
            mind_endpoint: None,
            flowmodus_endpoint: None,
            tentacle_endpoint: None,
            tuck_endpoint: None,
            callosum_endpoint: None,
            cellrix_endpoint: None,
        }
    }
}
