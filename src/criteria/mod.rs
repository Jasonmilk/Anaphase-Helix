//! Deterministic criteria checkers (ADR-0003).
//!
//! Pure functions: no IO, no LLM, no global state. Rule/data separation:
//! thresholds come from `knowledge_base/fixture-codex.json` (via `RuleParams`),
//! data comes from tool returns. All output is serde-serializable for
//! byte-identical replay (acceptance criterion 2).

use crate::contract::Expect;
use serde::{Deserialize, Serialize};

/// Result of a single check. Fixed-field struct for deterministic JSON.
/// Provenance-carrying (ADR-0029): every judgement records who judged
/// (`judge`), how strict (`gate`), what contract (`expect`/`evidence_id`)
/// and why (`detail`) — no bare labels, the chain stays auditable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CheckReport {
    pub check: String,
    pub passed: bool,
    pub detail: String,
    /// Who judged: "rule" (deterministic criteria) or a model backend.
    pub judge: String,
    /// Hard = fail-closed (a failed check blocks the verdict); soft =
    /// bookkeeping only. 0-token rules are hard by default (生存伦理).
    pub gate: String,
    /// The expected contract this check validates (from the call's expect).
    pub expect: String,
    /// The evidence row this check read (join key `{job_id}#{index}`).
    pub evidence_id: String,
}

impl CheckReport {
    /// Fixed provenance defaults for pure checkers (patched by the call
    /// sites that know the expect name and evidence id).
    pub fn with_provenance(mut self, expect: &str, evidence_id: &str) -> Self {
        self.expect = expect.to_string();
        self.evidence_id = evidence_id.to_string();
        self
    }
}

/// Rule parameters sourced from `knowledge_base/fixture-codex.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleParams {
    pub low: f64,
    pub high: f64,
    pub min_len: usize,
    pub min_n: usize,
    pub tolerance: f64,
    pub min_ratio: f64,
}

// --- six pure checkers ---

/// Passes when `value` is within [low, high].
pub fn threshold(value: f64, low: f64, high: f64) -> CheckReport {
    let passed = value >= low && value <= high;
    CheckReport {
        check: "threshold".into(),
        passed,
        detail: format!("value={value:.4} in [{low:.4},{high:.4}]"),
        judge: "rule".into(),
        gate: "hard".into(),
        expect: String::new(),
        evidence_id: String::new(),
    }
}

/// Passes when the series has at least `min_len` elements.
pub fn sequence_length(items: &[f64], min_len: usize) -> CheckReport {
    let passed = items.len() >= min_len;
    CheckReport {
        check: "sequence_length".into(),
        passed,
        detail: format!("len={} >= {min_len}", items.len()),
        judge: "rule".into(),
        gate: "hard".into(),
        expect: String::new(),
        evidence_id: String::new(),
    }
}

/// Passes when the series has at least `min_n` valid (non-NaN) samples.
pub fn sample_size(items: &[f64], min_n: usize) -> CheckReport {
    let valid = items.iter().filter(|v| !v.is_nan() && !v.is_infinite()).count();
    let passed = valid >= min_n;
    CheckReport {
        check: "sample_size".into(),
        passed,
        detail: format!("valid={valid} >= {min_n}"),
        judge: "rule".into(),
        gate: "hard".into(),
        expect: String::new(),
        evidence_id: String::new(),
    }
}

/// Passes when two independent values corroborate within relative tolerance.
pub fn cross_check(a: f64, b: f64, tolerance: f64) -> CheckReport {
    let denom = a.abs().max(b.abs()).max(1.0);
    let passed = (a - b).abs() <= tolerance * denom;
    CheckReport {
        check: "cross_check".into(),
        passed,
        detail: format!("|{a:.4}-{b:.4}| <= {tolerance:.4}*{denom:.4}"),
        judge: "rule".into(),
        gate: "hard".into(),
        expect: String::new(),
        evidence_id: String::new(),
    }
}

/// Passes when two trends point in the same direction (same sign).
pub fn divergence(trend_a: f64, trend_b: f64) -> CheckReport {
    let passed = trend_a.signum() == trend_b.signum();
    CheckReport {
        check: "divergence".into(),
        passed,
        detail: format!("sign({trend_a:.4}) == sign({trend_b:.4})"),
        judge: "rule".into(),
        gate: "hard".into(),
        expect: String::new(),
        evidence_id: String::new(),
    }
}

/// Passes when numerator/denominator is at least `min` (denominator must be > 0).
pub fn ratio_band(numerator: f64, denominator: f64, min: f64) -> CheckReport {
    if denominator <= 0.0 {
        return CheckReport {
            check: "ratio_band".into(),
            passed: false,
            detail: format!("denominator={denominator:.4} must be > 0"),
            judge: "rule".into(),
            gate: "hard".into(),
            expect: String::new(),
            evidence_id: String::new(),
        };
    }
    let ratio = numerator / denominator;
    let passed = ratio >= min;
    CheckReport {
        check: "ratio_band".into(),
        passed,
        detail: format!("ratio={ratio:.4} >= {min:.4}"),
        judge: "rule".into(),
        gate: "hard".into(),
        expect: String::new(),
        evidence_id: String::new(),
    }
}

/// Passes when the executor reported structured success (tool layer).
/// Zero thresholds: structural contract only (D'-4). The delivery question
/// ("did the answer reach the human") is a separate check — exec_ok must
/// not conflate tool success with answer delivery (2026-09-09: the mcp
/// wrapper echo check made flat-contract tools like calc fail every run).
pub fn exec_ok(ok_flag: bool) -> CheckReport {
    CheckReport {
        check: "exec_ok".into(),
        passed: ok_flag,
        detail: format!("ok={ok_flag}"),
        judge: "rule".into(),
        gate: "hard".into(),
        expect: String::new(),
        evidence_id: String::new(),
    }
}

/// Passes when the tool produced a business result (delivery layer, judged
/// at the tool boundary — the strongest fact the criteria layer can see).
/// Detects a non-empty result field across both known contracts:
/// flat `{"ok":true,"result":...}` (Tentacle tools: calc/numbers/rate/text)
/// and the mcp_proxy wrapper `{"ok":true,"data":{"params":...}}`. The SSE
/// delivery itself is a transport fact owned by the /v1/chat handler; this
/// check closes the loop at the tool edge without guessing beyond it.
pub fn answer_delivered(ok_flag: bool, data: &serde_json::Value) -> CheckReport {
    let has_result = [
        "result",
        "series",
        "numerator",
        "denominator",
        "trend_a",
        "trend_b",
    ]
    .iter()
    .any(|k| {
        data.get(*k)
            .map(|v| !v.is_null() && v != &serde_json::Value::String(String::new()))
            .unwrap_or(false)
    });
    let mcp_echo = data
        .get("data")
        .and_then(|d| d.get("params"))
        .map(|p| !p.is_null())
        .unwrap_or(false);
    let delivered = ok_flag && (has_result || mcp_echo);
    let detail = if delivered {
        "result produced, delivery confirmed at tool edge".to_string()
    } else {
        "no business result in tool return".to_string()
    };
    CheckReport {
        check: "answer.delivered".into(),
        passed: delivered,
        detail,
        judge: "rule".into(),
        gate: "hard".into(),
        expect: String::new(),
        evidence_id: String::new(),
    }
}

// --- expect -> criteria mapping (ADR-0003 decision 6) ---

/// Run the criteria set mapped from `expect` against tool-return `data`.
/// Malformed data yields a failed report (deterministic, never panics).
pub fn run_for_expect(expect: &Expect, data: &serde_json::Value, params: &RuleParams) -> Vec<CheckReport> {
    let tag = expect.as_str();
    let attach = |mut rs: Vec<CheckReport>| {
        for r in rs.iter_mut() {
            r.expect = tag.to_string();
        }
        rs
    };
    match expect {
        Expect::Numbers => {
            let series = match data.get("series").and_then(|v| v.as_array()) {
                Some(s) => s.iter().filter_map(|v| v.as_f64()).collect::<Vec<f64>>(),
                None => Vec::new(),
            };
            let sum: f64 = series.iter().sum();
            attach(vec![
                sequence_length(&series, params.min_len),
                sample_size(&series, params.min_n),
                threshold(sum, params.low, params.high),
            ])
        }
        Expect::Rate => {
            let numerator = data.get("numerator").and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
            let denominator = data.get("denominator").and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
            let ratio = if denominator > 0.0 { numerator / denominator } else { f64::NAN };
            attach(vec![
                threshold(ratio, params.low, params.high),
                ratio_band(numerator, denominator, params.min_ratio),
                cross_check(numerator, denominator, params.tolerance),
            ])
        }
        Expect::Text => {
            let trend_a = data.get("trend_a").and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
            let trend_b = data.get("trend_b").and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
            attach(vec![divergence(trend_a, trend_b)])
        }
        Expect::Ok => {
            // D'-4: structured execution success — two checks, one per layer
            // (2026-09-09 split, ADR-0015 Engram review): exec_ok owns the
            // tool layer (did the tool run), answer.delivered owns the
            // delivery layer (did it produce a business result). Both
            // contracts pass: flat `{"ok":true,"result":...}` (Tentacle
            // calc/numbers/rate) and mcp_proxy `{"ok":true,"data":{...}}`.
            let ok_flag = data.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            attach(vec![exec_ok(ok_flag), answer_delivered(ok_flag, data)])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> RuleParams {
        RuleParams { low: 0.0, high: 100.0, min_len: 3, min_n: 2, tolerance: 0.1, min_ratio: 0.5 }
    }

    #[test]
    fn threshold_bounds() {
        assert!(threshold(50.0, 0.0, 100.0).passed);
        assert!(!threshold(150.0, 0.0, 100.0).passed);
    }

    #[test]
    fn sequence_and_sample() {
        let s = [1.0, 2.0, 3.0, f64::NAN];
        assert!(sequence_length(&s, 3).passed);
        assert!(!sequence_length(&s, 5).passed);
        assert!(sample_size(&s, 2).passed, "NaN filtered out");
        assert!(!sample_size(&s, 4).passed);
    }

    #[test]
    fn cross_check_within_tolerance() {
        assert!(cross_check(10.0, 9.5, 0.1).passed);
        assert!(!cross_check(10.0, 5.0, 0.1).passed);
    }

    #[test]
    fn divergence_same_direction() {
        assert!(divergence(2.0, 5.0).passed);
        assert!(!divergence(2.0, -5.0).passed);
    }

    #[test]
    fn ratio_band_guard() {
        assert!(ratio_band(10.0, 20.0, 0.5).passed);
        assert!(!ratio_band(10.0, 20.0, 0.6).passed);
        assert!(!ratio_band(10.0, 0.0, 0.5).passed, "denominator <= 0 fails");
    }

    #[test]
    fn numbers_mapping_full_pass() {
        let data = serde_json::json!({"series": [1.0, 2.0, 3.0, 4.0]});
        let reports = run_for_expect(&Expect::Numbers, &data, &params());
        assert!(reports.iter().all(|r| r.passed), "{reports:?}");
    }

    #[test]
    fn rate_mapping_malformed_never_panics() {
        let data = serde_json::json!({"numerator": "oops"});
        let reports = run_for_expect(&Expect::Rate, &data, &params());
        assert_eq!(reports.len(), 3);
        assert!(reports.iter().all(|r| !r.passed), "malformed data must fail closed");
    }

    #[test]
    fn same_input_twice_is_byte_identical() {
        let data = serde_json::json!({"series": [1.0, 2.0, 3.0]});
        let a = run_for_expect(&Expect::Numbers, &data, &params());
        let b = run_for_expect(&Expect::Numbers, &data, &params());
        assert_eq!(serde_json::to_string(&a).unwrap(), serde_json::to_string(&b).unwrap());
    }

    #[test]
    fn ok_mapping_full_pass() {
        // mcp_proxy wrapper contract: data.params echo present.
        let data = serde_json::json!({"ok": true, "data": {"tool": "t", "params": {"x": 1}}});
        let reports = run_for_expect(&Expect::Ok, &data, &params());
        assert_eq!(reports.len(), 2, "two checks: exec_ok + answer.delivered");
        assert!(reports.iter().all(|r| r.passed), "{reports:?}");
    }

    #[test]
    fn ok_mapping_flat_contract_passes() {
        // Tentacle flat contract: {"ok": true, "result": ...} — the calc
        // bug (2026-09-09): exec_ok must not require the mcp wrapper.
        let data = serde_json::json!({"ok": true, "result": "40353607"});
        let reports = run_for_expect(&Expect::Ok, &data, &params());
        assert_eq!(reports.len(), 2);
        assert!(reports.iter().all(|r| r.passed), "{reports:?}");
        assert_eq!(reports[0].check, "exec_ok");
        assert_eq!(reports[1].check, "answer.delivered");
    }

    #[test]
    fn ok_mapping_fails_closed_on_missing_result() {
        let data = serde_json::json!({"ok": true});
        let reports = run_for_expect(&Expect::Ok, &data, &params());
        assert!(reports[0].passed, "tool ran ok");
        assert!(!reports[1].passed, "no business result must fail delivery");
    }

    #[test]
    fn ok_mapping_fails_closed_on_ok_false() {
        let data = serde_json::json!({"ok": false, "data": {"tool": "t", "params": {}}});
        let reports = run_for_expect(&Expect::Ok, &data, &params());
        assert!(reports.iter().all(|r| !r.passed), "ok=false must fail all");
    }
}
