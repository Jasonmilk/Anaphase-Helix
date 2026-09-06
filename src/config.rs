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

    /// O-3: stage-event trail log path. `None` = default "events.jsonl"
    /// (trail persistence on, cross-restart replay; same pattern as
    /// session_notes_path). Set an explicit path to relocate.
    pub events_log_path: Option<String>,
    /// O-5 (ADR-0023): cognitive-injection budget — memory nodes folded into
    /// the Reasoning prompt, capped at this many chars. 0 = no injection
    /// (pure stateless). Protocol default 800 (ADR-0023).
    #[serde(default)]
    pub memory_inject_chars: usize,
    /// Demo/smoke input for the local run loop (O-5, ADR-0023): CLI `--input`
    /// wins, then this config, then the protocol-default demo task. This is
    /// the demo task source — no literal in main.rs.
    #[serde(default)]
    pub smoke_input: Option<String>,

    /// run_cycle state-machine constants (candidate E, ADR-0005).
    /// DNA principle 11 (ADR-0002): the five historical literals in
    /// run_cycle.rs now have a config source. Overridable via
    /// config.toml `[anaphase.run_cycle]`.
    #[serde(default)]
    pub run_cycle: RunCycleConfig,
    /// Partner-mode mind-craft parameters (ADR-0022 O-4).
    /// DNA principle 11 (ADR-0002): the adapter literals in
    /// src/adapters/mind.rs now have a config source. Overridable via
    /// config.toml `[anaphase.mind]`.
    #[serde(default)]
    pub mind: MindConfig,

    /// External human-authored knowledge rails (ADR-0018): read-only,
    /// version-frozen citation rails. Overridable via
    /// config.toml `[anaphase.rails]`.
    #[serde(default)]
    pub rails: RailsConfig,
}

/// Rails: external human knowledge rails (心智外铁轨, ADR-0018).
/// Read-only citation asset — Helix may only select an existing edge,
/// never synthesize one. All literals below carry documented defaults
/// (DNA principle 11 / ADR-0002): conservative local budgets, tunable
/// per deployment, never protocol values.
#[derive(Debug, Clone, Deserialize)]
pub struct RailsConfig {
    /// Master switch. When no kb directory exists the index is empty and
    /// the loop is unaffected (fail-open, zero cost).
    #[serde(default = "default_rails_enabled")]
    pub enabled: bool,
    /// Rail root directory (relative to the project root) — points at one
    /// concrete kb (`rails/<kb>/`, e.g. a statutes rail). Point it at a real
    /// kb to replace the bundled demo. Convention: one kb per directory.
    #[serde(default = "default_rails_kb_dir")]
    pub kb_dir: String,
    /// Entry-hit cap for one navigation (retrieval budget; default:
    /// conservative local-LLM context budget, same policy family as
    /// RunCycleConfig.cycle_cap).
    #[serde(default = "default_rails_max_hits")]
    pub max_hits: usize,
    /// Total verbatim bytes injected per cycle (context budget; default:
    /// conservative local-LLM window slice, tunable per model).
    #[serde(default = "default_rails_max_inject")]
    pub max_inject_bytes: usize,
}

fn default_rails_enabled() -> bool {
    true
}

fn default_rails_kb_dir() -> String {
    "knowledge_base/rails/demo".to_string()
}

fn default_rails_max_hits() -> usize {
    3
}

fn default_rails_max_inject() -> usize {
    4096
}

impl Default for RailsConfig {
    fn default() -> Self {
        Self {
            enabled: default_rails_enabled(),
            kb_dir: default_rails_kb_dir(),
            max_hits: default_rails_max_hits(),
            max_inject_bytes: default_rails_max_inject(),
        }
    }
}

/// run_cycle state-machine constants (candidate E, ADR-0005).
///
/// Previously hardcoded in `src/run_cycle.rs` (`0.7/0.3/0.2`,
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
    /// Max periods a caller loop may run per interaction (ADR-0016 D1:
    /// run_cycle is a single-period primitive; this bounds the caller's loop).
    /// Source: conservative local-LLM context-budget default — the original
    /// design feared infinite loops and context growth (local endpoints are
    /// resource-constrained); tune per model window, never a protocol value.
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
            events_log_path: None,
            memory_inject_chars: 800, // protocol default (ADR-0023)
            smoke_input: None,
            run_cycle: RunCycleConfig::default(),
            rails: RailsConfig::default(),
            mind: MindConfig::default(),
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
                events_log_path: None,
                memory_inject_chars: 800,
                smoke_input: None,
                run_cycle: RunCycleConfig {
                    amygdala_default_vector: (0.7, 0.3, 0.2),
                    reasoning_mode: "left_brain".into(),
                    soft_reflex_threshold: 0.7,
                    execution_placeholder: "echo".into(),
                    cycle_cap: 7,
                    mode: Mode::Partner,
                },
                rails: RailsConfig::default(),
                mind: MindConfig::default(),
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

/// Partner-mode mind-craft adapter parameters (ADR-0022 O-4).
///
/// DNA principle 11 (ADR-0002): every adapter literal now has a single
/// source — this config. Defaults are the protocol defaults defined in
/// ADR-0022 (was inline literals in `src/adapters/mind.rs`); overridable
/// via config.toml `[anaphase.mind]`.
///
/// Deliberately excluded (derived / protocol state, not tunable):
/// - heliotropism = 0.0 — P10a unimplemented neutral value (ADR-0022)
/// - impasse_depth = 0 — cycle-start impasse (derived from "cycle begins")
/// - complexity clamp 0..=3 — cognitive-mode state range (proto contract)
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct MindConfig {
    /// EnergyContext token budget for the cognitive cycle.
    pub token_budget: u64,
    /// EnergyContext latency budget (ms) for the retrieval.
    pub latency_limit_ms: u64,
    /// EnergyContext pulse (arousal, 0..1).
    pub pulse: f64,
    /// EnergyContext vigilance (0..1).
    pub vigilance: f64,
    /// EnergyContext familiarity (0..1).
    pub familiarity: f64,
    /// System load above which expensive exogenous search is suppressed.
    pub high_load: f64,
    /// Query char-count at or below which budget tier is Endogenous.
    pub short_query: usize,
    /// Query char-count at or above which budget tier is ExogenousRequired.
    pub long_query: usize,
    /// Query char-count at or below which mode falls back to Skilled.
    pub skilled_len: usize,
    /// Query char-count below which mode falls back to Anchor (>= skilled_len).
    pub anchor_len: usize,
    /// System-probe failure fallback (neutral load, 0..1).
    pub probe_fallback: f64,
    /// Explore-semantics keywords (hit -> exploratory query).
    pub explore_keywords: Vec<String>,
}

impl Default for MindConfig {
    fn default() -> Self {
        Self {
            token_budget: 1000,
            latency_limit_ms: 500,
            pulse: 0.3,
            vigilance: 0.2,
            familiarity: 0.5,
            high_load: 0.8,
            short_query: 4,
            long_query: 60,
            skilled_len: 10,
            anchor_len: 40,
            probe_fallback: 0.5,
            explore_keywords: vec![
                "探索".into(),
                "研究".into(),
                "发现".into(),
                "创意".into(),
                "头脑风暴".into(),
                "未知".into(),
                "可能性".into(),
                "explore".into(),
                "research".into(),
                "brainstorm".into(),
                "imagine".into(),
            ],
        }
    }
}
