//! Judge points (ADR-0024, judge-points contract JP-1): complexity assessment
//! with an explicitly selected backend — deterministic rules (default) or a
//! small LLM (3B-class) when semantic quality ROI justifies it.
//!
//! Contract rules:
//! - `assess_complexity` ALWAYS returns 1/2/3. Every backend is total.
//! - SmallLlm failures (network / invalid label / timeout) fall back to the
//!   rules backend (fail-safe, determinism first).
//! - The rules backend is the default; SmallLlm is opt-in via config.
//! - Backend switching is explicit (config), never an auto router.

use std::sync::Arc;

use async_trait::async_trait;

/// Complexity tiers: 1 = simple / 2 = moderate / 3 = complex.
pub const TIER_SIMPLE: u8 = 1;
pub const TIER_MODERATE: u8 = 2;
pub const TIER_COMPLEX: u8 = 3;

/// Backend selector for the complexity judge point (config `[anaphase]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JudgeBackend {
    /// Length heuristic over config thresholds. Zero tokens, always succeeds.
    Rules,
    /// 3B-class classifier over an OpenAI-compatible endpoint (e.g. Tuck's
    /// local llama base URL). Falls back to Rules on any failure.
    SmallLlm,
}

impl Default for JudgeBackend {
    fn default() -> Self {
        JudgeBackend::Rules
    }
}

/// A judge backend: total, side-effect-free w.r.t. the caller, always 1/2/3.
#[async_trait]
pub trait Judge: Send + Sync {
    async fn assess_complexity(&self, query: &str) -> u8;
}

/// Rules backend: char-length heuristic over config-sourced thresholds
/// (zero-hardcoding: the thresholds come from `MindConfig`, never literals).
#[derive(Debug, Clone)]
pub struct RulesJudge {
    pub skilled_len: usize,
    pub anchor_len: usize,
}

#[async_trait]
impl Judge for RulesJudge {
    async fn assess_complexity(&self, query: &str) -> u8 {
        let len = query.trim().chars().count();
        if len <= self.skilled_len {
            TIER_SIMPLE
        } else if len < self.anchor_len {
            TIER_MODERATE
        } else {
            TIER_COMPLEX
        }
    }
}

/// Classifier prompt contract: the small LLM must answer with exactly one
/// bare label. Anything else is an invalid label -> fallback to Rules.
const CLASSIFY_PROMPT: &str = "Classify the request into exactly one label: \
simple (routine, short, low-ambiguity), moderate (needs some planning), \
complex (multi-step, open-ended, high-ambiguity). Reply with the bare label \
only, nothing else.";

/// Parse a classifier reply into a tier. Any non-label text is an error.
fn parse_tier_label(reply: &str) -> Result<u8, String> {
    let label = reply.trim().trim_matches(|c: char| c == '"' || c == '\'' || c == '`').to_lowercase();
    match label.as_str() {
        "simple" => Ok(TIER_SIMPLE),
        "moderate" => Ok(TIER_MODERATE),
        "complex" => Ok(TIER_COMPLEX),
        other => Err(format!("invalid complexity label: {}", other)),
    }
}

/// SmallLlm backend: OpenAI-compatible chat completion against a 3B-class
/// endpoint. Any failure (connect / HTTP / invalid label) falls back to the
/// bundled rules judge — the caller always gets 1/2/3.
pub struct SmallLlmJudge {
    endpoint: String,
    model: String,
    client: reqwest::Client,
    fallback: RulesJudge,
}

impl SmallLlmJudge {
    pub fn new(endpoint: String, model: String, fallback: RulesJudge) -> Self {
        Self {
            endpoint,
            model,
            client: reqwest::Client::new(),
            fallback,
        }
    }

    async fn classify(&self, query: &str) -> Result<u8, String> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": CLASSIFY_PROMPT},
                {"role": "user", "content": query}
            ],
            "max_tokens": 8,
            "temperature": 0.0
        });
        let url = format!("{}/v1/chat/completions", self.endpoint.trim_end_matches('/'));
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("judge request failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("judge endpoint status: {}", resp.status()));
        }
        let payload: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("judge response unreadable: {}", e))?;
        let reply = payload["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| "judge response missing content".to_string())?;
        parse_tier_label(reply)
    }
}

#[async_trait]
impl Judge for SmallLlmJudge {
    async fn assess_complexity(&self, query: &str) -> u8 {
        match self.classify(query).await {
            Ok(tier) => tier,
            Err(_) => self.fallback.assess_complexity(query).await,
        }
    }
}

/// Build the judge backend from config. SmallLlm without an endpoint/model is
/// a config error -> degrade to Rules (fail-safe, documented in ADR-0024).
pub fn resolve_judge(
    backend: JudgeBackend,
    endpoint: Option<&str>,
    model: Option<&str>,
    skilled_len: usize,
    anchor_len: usize,
) -> (Arc<dyn Judge>, Option<String>) {
    let fallback = RulesJudge { skilled_len, anchor_len };
    match backend {
        JudgeBackend::Rules => (Arc::new(fallback), None),
        JudgeBackend::SmallLlm => match (endpoint, model) {
            (Some(ep), Some(m)) => (
                Arc::new(SmallLlmJudge::new(ep.to_string(), m.to_string(), fallback)),
                None,
            ),
            _ => (
                Arc::new(fallback),
                Some("judge_backend=small_llm requires judge_endpoint and judge_model; degraded to rules".to_string()),
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(skilled: usize, anchor: usize) -> RulesJudge {
        RulesJudge { skilled_len: skilled, anchor_len: anchor }
    }

    #[tokio::test]
    async fn rules_judge_uses_config_thresholds_not_literals() {
        let j = rules(5, 8);
        assert_eq!(j.assess_complexity("hi").await, TIER_SIMPLE);
        assert_eq!(j.assess_complexity("ab cd e").await, TIER_MODERATE);
        assert_eq!(j.assess_complexity("a b c d e f g h i j").await, TIER_COMPLEX);
    }

    #[test]
    fn parse_tier_label_accepts_bare_labels() {
        assert_eq!(parse_tier_label("simple"), Ok(TIER_SIMPLE));
        assert_eq!(parse_tier_label("  moderate  "), Ok(TIER_MODERATE));
        assert_eq!(parse_tier_label("\"complex\""), Ok(TIER_COMPLEX));
    }

    #[test]
    fn parse_tier_label_rejects_free_text() {
        assert!(parse_tier_label("I think this is complex because...").is_err());
        assert!(parse_tier_label("").is_err());
    }

    #[tokio::test]
    async fn small_llm_falls_back_to_rules_on_unreachable_endpoint() {
        // 127.0.0.1:1 refuses connections instantly -> classify errors ->
        // fallback returns the rules verdict (fail-safe, determinism first).
        let j = SmallLlmJudge::new("http://127.0.0.1:1".into(), "mock-3b".into(), rules(10, 40));
        let tier = j.assess_complexity("help me plan a research strategy").await;
        // 32 chars < anchor 40 -> the fallback rules verdict is moderate
        assert_eq!(tier, TIER_MODERATE);
    }

    #[tokio::test]
    async fn small_llm_maps_valid_classifier_reply() {
        // Local mock OpenAI-compatible endpoint answering a bare label: the
        // success path (label -> tier) must work end to end over HTTP.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("mock accept");
            let mut buf = [0u8; 4096];
            let _ = std::io::Read::read(&mut stream, &mut buf);
            let body = r#"{"choices":[{"message":{"content":"complex"}}]}"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = std::io::Write::write_all(&mut stream, resp.as_bytes());
        });
        let j = SmallLlmJudge::new(format!("http://{}", addr), "mock-3b".into(), rules(10, 40));
        let tier = j.assess_complexity("any query").await;
        assert_eq!(tier, TIER_COMPLEX);
    }

    #[tokio::test]
    async fn resolve_judge_degrades_to_rules_when_endpoint_missing() {
        let (judge, warn) = resolve_judge(JudgeBackend::SmallLlm, None, None, 10, 40);
        assert!(warn.is_some(), "config error must be surfaced");
        assert_eq!(judge.assess_complexity("hi").await, TIER_SIMPLE);
    }
}
