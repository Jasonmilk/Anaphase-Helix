//! tt_job contract types and LLM-calls parsing (ADR-0003).
//!
//! Boundary (ADR-0003 decision 5): the LLM outputs ONLY the `calls` array;
//! the envelope (`job_id`, `created_at`) is assembled by the pipeline.
//! Runtime validation is serde strong typing (enum enforced at compile time);
//! the JSON schema file is a human-readable contract, not a runtime dependency.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Expected result shape of a tool call (lowercase in JSON: numbers/rate/text).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Expect {
    Numbers,
    Rate,
    Text,
}

/// One planned tool call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Call {
    pub tool: String,
    /// JSON object; BTreeMap for deterministic key order (ADR-0003).
    pub args: BTreeMap<String, serde_json::Value>,
    pub expect: Expect,
}

/// Full tt_job envelope, assembled by the pipeline (not by the LLM).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TtJob {
    pub job_id: String,
    pub created_at: String,
    pub calls: Vec<Call>,
}

/// Parse LLM chat content into a plan of calls.
///
/// Accepts either a bare `[...]` array or `{"calls": [...]}`. M1 semantics:
/// a parse failure is an error raised by the pipeline — nothing is recorded
/// and nothing is retried (single-pass philosophy, ADR-0003 decision 5).
pub fn parse_llm_calls(response: &str) -> Result<Vec<Call>, String> {
    let value: serde_json::Value =
        serde_json::from_str(response).map_err(|e| format!("invalid JSON from LLM: {e}"))?;
    let arr = value.get("calls").cloned().unwrap_or(value);
    let calls: Vec<Call> =
        serde_json::from_value(arr).map_err(|e| format!("calls schema mismatch: {e}"))?;
    Ok(calls)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_tt_job_is_byte_identical() {
        let job = TtJob {
            job_id: "tt_job-001".into(),
            created_at: "2026-09-03T00:00:00Z".into(),
            calls: vec![Call {
                tool: "numbers".into(),
                args: BTreeMap::new(),
                expect: Expect::Numbers,
            }],
        };
        let json = serde_json::to_string(&job).unwrap();
        let back: TtJob = serde_json::from_str(&json).unwrap();
        assert_eq!(serde_json::to_string(&back).unwrap(), json);
    }

    #[test]
    fn parse_llm_calls_accepts_wrapped_object() {
        let resp = r#"{"calls":[{"tool":"rate","args":{"numerator":10},"expect":"rate"}]}"#;
        let calls = parse_llm_calls(resp).unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool, "rate");
        assert_eq!(calls[0].expect, Expect::Rate);
    }

    #[test]
    fn parse_llm_calls_rejects_invalid_json() {
        let err = parse_llm_calls("not json at all").unwrap_err();
        assert!(err.contains("invalid JSON"), "got: {err}");
    }

    #[test]
    fn parse_llm_calls_rejects_unknown_expect_enum() {
        let resp = r#"{"calls":[{"tool":"x","args":{},"expect":"bogus"}]}"#;
        let err = parse_llm_calls(resp).unwrap_err();
        assert!(err.contains("schema mismatch"), "got: {err}");
    }
}
