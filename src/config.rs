use serde::{Deserialize, Serialize};

/// Interaction mode of the cognitive loop (ADR-0006): the same Helix with
/// different Mind participation — not three separate minds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Human drives Anaphase directly; Mind absent (Noop assembly, no
    /// experience written). Harness-style usage.
    Drive,
    /// Helix works with the human as a memory-bearing partner; every turn is
    /// written as an L3 experience (default — Helix's native state).
    Partner,
    /// Mind lives autonomously (sleep/metabolism/recap); Anaphase is its
    /// executor. Reverse drive lands with Mind P10a; enum reserved here.
    Survive,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Partner // Helix's native state: a memory-bearing partner
    }
}

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

    /// 纪元会话笔记路径（P10c T1 强制苏醒/认知脱水）：默认 "session_notes.json"
    pub session_notes_path: Option<String>,

    /// run_cycle state-machine constants (candidate E, ADR-0005).
    /// DNA principle 11 (ADR-0002): the five historical literals in
    /// agent_loop.rs now have a config source. Overridable via
    /// config.toml `[anaphase.run_cycle]`.
    #[serde(default)]
    pub run_cycle: RunCycleConfig,
}

/// run_cycle state-machine constants (candidate E, ADR-0005).
///
/// Previously hardcoded in `src/agent_loop.rs` (`0.7/0.3/0.2`,
/// `"left_brain"`, `p_death > 0.7`, `"echo"`, `0..7`) — this struct is now
/// their single source (DNA principle 11 / ADR-0002). Defaults are the
/// documented protocol values; `config.toml` may override each one.
#[derive(Debug, Clone, Deserialize)]
pub struct RunCycleConfig {
    /// PreAssessment default amygdala vector (heliotropism, pulse, vigilance).
    pub amygdala_default_vector: (f64, f64, f64),
    /// Reasoning mode label passed to the ReasoningAdapter.
    pub reasoning_mode: String,
    /// ReflexCheck soft-reflex block threshold (p_death above -> blocked).
    pub soft_reflex_threshold: f64,
    /// Legacy Execution placeholder command when no real tool is resolved.
    pub execution_placeholder: String,
    /// run_cycle loop cap (prevents infinite cognitive cycles).
    pub cycle_cap: usize,
    /// Interaction mode (ADR-0006): Drive (no Mind) / Partner (default,
    /// with Mind + episode lifecycle) / Survive (Mind autonomous, reserved).
    #[serde(default)]
    pub mode: Mode,
}

impl Default for RunCycleConfig {
    fn default() -> Self {
        Self {
            amygdala_default_vector: (0.7, 0.3, 0.2), // positive, calm, relaxed
            reasoning_mode: "left_brain".to_string(),
            soft_reflex_threshold: 0.7,
            execution_placeholder: "echo".to_string(),
            cycle_cap: 7,
            mode: Mode::Partner, // Helix's native state: memory-bearing partner
        }
    }
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

            session_notes_path: None,
            run_cycle: RunCycleConfig::default(),
        }
    }
}

/// Load configuration from config.toml or return defaults (Noop mode)
pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    match std::fs::read_to_string("config.toml") {
        Ok(content) => {
            let config: Config = toml::from_str(&content)?;
            Ok(apply_env_overrides(config))
        }
        Err(_) => {
            // config.toml not found, use Noop mode
            Ok(Config::default())
        }
    }
}

/// Environment overrides on top of config.toml (12-factor, candidate G-4
/// bootstrap: `up` spawns Anaphase with `ANAPHASE_TENTACLE_ENDPOINT` so the
/// deterministic execution channel is injected without touching config.toml
/// — no concurrent file writes, reversible, explicit).
///
/// Env vars win over the file: the launcher is the caller, the file is the
/// base state. Unknown/empty env values are ignored (fail-open).
fn apply_env_overrides(mut config: Config) -> Config {
    if let Ok(v) = std::env::var("ANAPHASE_TENTACLE_ENDPOINT") {
        if !v.is_empty() {
            config.anaphase.tentacle_endpoint = Some(v);
        }
    }
    if let Ok(v) = std::env::var("ANAPHASE_REASONING_ENDPOINT") {
        if !v.is_empty() {
            config.anaphase.reasoning_endpoint = Some(v);
        }
    }
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> Config {
        Config {
            anaphase: AnaphaseConfig {
                mind_endpoint: None,
                flowmodus_endpoint: None,
                tentacle_endpoint: None,
                tuck_endpoint: None,
                callosum_endpoint: None,
                cellrix_endpoint: None,
                cap_http_enabled: true,
                cap_http_port: 50061,
                reasoning_endpoint: None,
                reasoning_model: None,
                reasoning_api_key: None,
                reasoning_max_tokens: None,
                session_notes_path: None,
                run_cycle: RunCycleConfig {
                    amygdala_default_vector: (0.7, 0.3, 0.2),
                    reasoning_mode: "left_brain".into(),
                    soft_reflex_threshold: 0.7,
                    execution_placeholder: "echo".into(),
                    cycle_cap: 7,
                    mode: Mode::Partner,
                },
            },
        }
    }

    // Env vars are process-global: tests that touch them must be serialized
    // (Rust runs tests in parallel threads by default; a shared env would
    // race). One lock guards all env-mutating tests.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn env_override_wins_over_base() {
        let _g = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("ANAPHASE_TENTACLE_ENDPOINT", "http://127.0.0.1:50051"); }
        let c = apply_env_overrides(base_config());
        assert_eq!(c.anaphase.tentacle_endpoint.as_deref(), Some("http://127.0.0.1:50051"));
    }

    #[test]
    fn empty_env_value_is_ignored_fail_open() {
        let _g = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("ANAPHASE_TENTACLE_ENDPOINT", ""); }
        let c = apply_env_overrides(base_config());
        assert_eq!(c.anaphase.tentacle_endpoint, None);
    }

    #[test]
    fn reasoning_env_override_applies() {
        let _g = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("ANAPHASE_REASONING_ENDPOINT", "http://127.0.0.1:19999"); }
        let c = apply_env_overrides(base_config());
        assert_eq!(c.anaphase.reasoning_endpoint.as_deref(), Some("http://127.0.0.1:19999"));
    }
}
