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
