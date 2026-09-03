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

/// Structured Reasoning output signal (candidate E, ADR-0005).
///
/// The Reasoning output protocol replaces the legacy `contains("tool_call")`
/// string matching with a structured JSON plan:
///
/// ```json
/// {"calls":[{"tool":"numbers","args":{},"expect":"numbers"}],"impasse":false}
/// ```
///
/// `calls` and `impasse` are both optional:
/// - a bare `[...]` array is treated as `calls`;
/// - a structured object without `calls` is an explicit no-plan
///   (`impasse: true` signals a dead end, `impasse: false` a plain answer);
/// - invalid JSON or a malformed `calls` shape is an error.
#[derive(Debug, Clone, PartialEq)]
pub struct ReasoningSignal {
    pub calls: Vec<Call>,
    pub impasse: bool,
}

/// Parse structured Reasoning output into a plan signal (candidate E).
pub fn parse_reasoning_output(response: &str) -> Result<ReasoningSignal, String> {
    let value: serde_json::Value =
        serde_json::from_str(response).map_err(|e| format!("invalid JSON from LLM: {e}"))?;
    let impasse = value.get("impasse").and_then(|v| v.as_bool()).unwrap_or(false);
    let calls: Vec<Call> = match value.get("calls") {
        Some(arr) => serde_json::from_value(arr.clone())
            .map_err(|e| format!("calls schema mismatch: {e}"))?,
        None if value.is_array() => serde_json::from_value(value)
            .map_err(|e| format!("calls schema mismatch: {e}"))?,
        None => vec![], // structured object without calls: explicit no-plan
    };
    Ok(ReasoningSignal { calls, impasse })
}

/// Deterministic job-id derivation for run_cycle envelope assembly
/// (candidate E, ADR-0005). FNV-1a 64-bit over the input, hex-encoded —
/// no UUID (DNA principle 11 / ADR-0003 decision 12): the same input always
/// yields the same id, keeping the cognitive chain replayable.
pub fn derive_job_id(input: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325; // FNV-1a offset basis
    for b in input.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3); // FNV-1a prime
    }
    format!("run-{hash:016x}")
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

    #[test]
    fn parse_reasoning_output_wrapped_with_impasse() {
        let resp = r#"{"calls":[{"tool":"rate","args":{},"expect":"rate"}],"impasse":true}"#;
        let sig = parse_reasoning_output(resp).unwrap();
        assert_eq!(sig.calls.len(), 1);
        assert_eq!(sig.calls[0].tool, "rate");
        assert!(sig.impasse);
    }

    #[test]
    fn parse_reasoning_output_accepts_bare_array() {
        let resp = r#"[{"tool":"numbers","args":{},"expect":"numbers"}]"#;
        let sig = parse_reasoning_output(resp).unwrap();
        assert_eq!(sig.calls.len(), 1);
        assert!(!sig.impasse, "bare array implies no impasse flag");
    }

    #[test]
    fn parse_reasoning_output_impasse_only_object() {
        let sig = parse_reasoning_output(r#"{"impasse":true}"#).unwrap();
        assert!(sig.calls.is_empty());
        assert!(sig.impasse);
    }

    #[test]
    fn parse_reasoning_output_no_plan_object() {
        let sig = parse_reasoning_output(r#"{"impasse":false}"#).unwrap();
        assert!(sig.calls.is_empty());
        assert!(!sig.impasse);
    }

    #[test]
    fn parse_reasoning_output_rejects_invalid_json() {
        let err = parse_reasoning_output("not json at all").unwrap_err();
        assert!(err.contains("invalid JSON"), "got: {err}");
    }

    #[test]
    fn parse_reasoning_output_rejects_calls_schema_mismatch() {
        let resp = r#"{"calls":"not-an-array"}"#;
        let err = parse_reasoning_output(resp).unwrap_err();
        assert!(err.contains("schema mismatch"), "got: {err}");
    }

    #[test]
    fn derive_job_id_is_deterministic_and_distinct() {
        // Golden vectors pinned from an independent FNV-1a 64-bit implementation
        // (same input -> same id; different input -> different id).
        assert_eq!(derive_job_id("hello"), "run-a430d84680aabd0b");
        assert_eq!(derive_job_id("hello"), derive_job_id("hello"));
        assert_ne!(derive_job_id("hello"), derive_job_id("world"));
        assert!(derive_job_id("hello").starts_with("run-"));
        assert_eq!(derive_job_id("hello").len(), 4 + 16);
    }
}
