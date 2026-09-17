//! Crystallization (ADR-0029): UNMET periods are raw ore, rules are the product.
//!
//! `crystallize` scans the latest periods, folds each Unmet verdict's physical
//! outcome (tool/result) and its check rows into one rule suggestion. The
//! machine only suggests — a human reviews before any rule goes live; nothing is
//! auto-injected (fail-human, never fail-machine).

use std::fs;
use std::io::{self, Write};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::query::{list_periods, read_period};
use super::types_and_stream::EventType;


// ---- Crystallization (ADR-0029) ----
//
// UNMET periods are raw ore, rules are the product. `crystallize` scans
// the latest periods, folds each Unmet verdict's physical outcome
// (tool/result) and its check rows into one rule suggestion. The machine
// only suggests — a human reviews before any rule goes live; nothing is
// auto-injected (fail-human, never fail-machine).

/// One rule suggestion distilled from an Unmet period.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrystalSuggestion {
    /// The period that supplied the raw ore.
    pub job_id: String,
    pub tool: String,
    pub expect: String,
    /// Which deterministic checks failed (names).
    pub failed_checks: Vec<String>,
    /// The first failed check's reason (why it failed).
    pub symptom: String,
    /// Outcome fingerprints of the physical runs (join for the full body).
    pub outcome_shas: Vec<String>,
    /// 0-token, human-readable rule template (review, then inject).
    pub suggested_rule: String,
}

/// Scan the latest `limit` periods under `dir` and distill Unmet rounds
/// into rule suggestions, persisted under `{dir}/crystallized/`. Returns
/// the newly suggested rules (empty when nothing unmet).
pub fn crystallize(dir: &std::path::Path, limit: usize) -> io::Result<Vec<CrystalSuggestion>> {
    let mut suggestions = Vec::new();
    let periods = list_periods(dir, limit).unwrap_or_default();
    for p in periods {
        let Ok(events) = read_period(dir, &p.job_id) else {
            continue;
        };
        let mut verdict: Option<String> = None;
        let mut tool: Option<String> = None;
        let mut expect: Option<String> = None;
        let mut failed: Vec<String> = Vec::new();
        let mut symptom = String::new();
        let mut shas: Vec<String> = Vec::new();
        for ev in &events {
            match ev.event_type.as_str() {
                "tool/result" => {
                    tool = ev
                        .data
                        .get("tool")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    if let Some(s) = ev.data.get("outcome_sha").and_then(|v| v.as_str()) {
                        shas.push(s.to_string());
                    }
                }
                "tool/call" => {
                    expect = ev
                        .data
                        .get("expect")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
                "check/status" => {
                    let passed = ev.data.get("passed").and_then(|v| v.as_bool()).unwrap_or(false);
                    if !passed {
                        let check = ev
                            .data
                            .get("check")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?")
                            .to_string();
                        let reason = ev
                            .data
                            .get("reason")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        if symptom.is_empty() {
                            symptom = reason;
                        }
                        failed.push(check);
                    }
                }
                "verdict/status" => {
                    verdict = ev.data.get("status").and_then(|v| v.as_str()).map(|s| s.to_string());
                }
                _ => {}
            }
        }
        if verdict.as_deref() == Some("Unmet") && !failed.is_empty() {
            let tool = tool.unwrap_or_default();
            let expect = expect.unwrap_or_default();
            let rule = format!(
                "gate=hard judge=rule: when expect={} and {} then reject before execution",
                expect,
                failed.join("/")
            );
            suggestions.push(CrystalSuggestion {
                job_id: p.job_id.clone(),
                tool,
                expect,
                failed_checks: failed,
                symptom,
                outcome_shas: shas,
                suggested_rule: rule,
            });
        }
    }
    if !suggestions.is_empty() {
        let out_dir = dir.join("crystallized");
        fs::create_dir_all(&out_dir)?;
        for s in &suggestions {
            let path = out_dir.join(format!("rule-{}.json", s.job_id));
            if let Ok(json) = serde_json::to_string_pretty(s) {
                let mut f = fs::File::create(path)?;
                writeln!(f, "{json}")?;
            }
        }
    }
    Ok(suggestions)
}

