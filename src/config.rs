use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub anaphase: AnaphaseConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnaphaseConfig {
    pub mind_endpoint: Option<String>,
    pub flowmodus_endpoint: Option<String>,
    pub tentacle_endpoint: Option<String>,
    pub tuck_endpoint: Option<String>,
    pub callosum_endpoint: Option<String>,
    pub cellrix_endpoint: Option<String>,
    /// Enable HTTP CAP server for remote debugging
    pub cap_http_enabled: bool,
    /// Listening port for HTTP CAP server
    pub cap_http_port: u16,

    // New: LLM HTTP reasoning fields
    pub reasoning_endpoint: Option<String>,
    pub reasoning_model: Option<String>,
    pub reasoning_api_key: Option<String>,
    pub reasoning_max_tokens: Option<u32>,
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
            cap_http_enabled: false,
            cap_http_port: 50061,

            reasoning_endpoint: None,
            reasoning_model: None,
            reasoning_api_key: None,
            reasoning_max_tokens: None,
        }
    }
}

/// Load configuration from config.toml or return defaults (Noop mode)
pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    match std::fs::read_to_string("config.toml") {
        Ok(content) => {
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        }
        Err(_) => {
            // config.toml not found, use Noop mode
            Ok(Config::default())
        }
    }
}
