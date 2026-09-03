//! Append-only JSONL ledger (ADR-0003).
//!
//! Verdicts are written once and never mutated. UNMET records carry
//! `retry_due` + `parent_id` so M1.5 can requeue them by lineage without a
//! breaking append-only format change (ADR-0003 decision 10). The clock is
//! injectable so reopen scanning is deterministically testable.

use crate::criteria::CheckReport;
use serde::{Deserialize, Serialize};

/// Verdict status. `Met` closes a job; `Unmet` schedules a retry (M1.5 consumes).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum VerdictStatus {
    Met,
    Unmet,
}

/// One ledger entry. Tagged by `record_type` in JSON (snake_case).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "record_type", rename_all = "snake_case")]
pub enum LedgerRecord {
    Verdict {
        status: VerdictStatus,
        job_id: String,
        evidence_ids: Vec<String>,
        check_reports: Vec<CheckReport>,
        /// Present on UNMET only: `now + retry_policy.base_delay_secs` (unix secs).
        retry_due: Option<u64>,
        /// Lineage link to the previous attempt; M1 writes it, M1.5 reads it
        /// to count attempts by chain length (ADR-0003 decision 10).
        parent_id: Option<String>,
    },
}

impl LedgerRecord {
    pub fn met(job_id: &str, evidence_ids: Vec<String>, check_reports: Vec<CheckReport>) -> Self {
        LedgerRecord::Verdict {
            status: VerdictStatus::Met,
            job_id: job_id.to_string(),
            evidence_ids,
            check_reports,
            retry_due: None,
            parent_id: None,
        }
    }

    pub fn unmet(
        job_id: &str,
        evidence_ids: Vec<String>,
        check_reports: Vec<CheckReport>,
        retry_due: u64,
        parent_id: Option<String>,
    ) -> Self {
        LedgerRecord::Verdict {
            status: VerdictStatus::Unmet,
            job_id: job_id.to_string(),
            evidence_ids,
            check_reports,
            retry_due: Some(retry_due),
            parent_id,
        }
    }
}

/// Injectable time source (unix seconds) for deterministic testing.
pub trait Clock: Send + Sync {
    fn now(&self) -> u64;
}

/// Wall-clock implementation (production).
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

/// Fixed-time implementation for deterministic tests.
pub struct FakeClock(pub u64);

impl Clock for FakeClock {
    fn now(&self) -> u64 {
        self.0
    }
}

/// Render unix seconds as an RFC3339 UTC timestamp — the tt_job envelope
/// `created_at` format (`docs/contracts/tt_job.schema.json`: date-time).
/// Deterministic: the same seconds always yield the same string, so replay
/// with an injected clock is byte-identical (candidate E, ADR-0005).
pub fn unix_secs_to_rfc3339(secs: u64) -> String {
    chrono::DateTime::from_timestamp(secs as i64, 0)
        .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string())
}

/// Append-only ledger. Records are never mutated or removed.
pub struct Ledger {
    records: Vec<LedgerRecord>,
    clock: Box<dyn Clock>,
}

impl Ledger {
    pub fn new(clock: Box<dyn Clock>) -> Self {
        Self { records: vec![], clock }
    }

    pub fn clock_now(&self) -> u64 {
        self.clock.now()
    }

    pub fn append(&mut self, record: LedgerRecord) {
        self.records.push(record);
    }

    pub fn records(&self) -> &[LedgerRecord] {
        &self.records
    }

    /// Scan for UNMET records whose retry_due has passed (reopen candidates).
    /// This is the M1 definition of "reopen": scanning only; consuming the
    /// queue is M1.5 scope (ADR-0003 decision 10).
    pub fn scan_due(&self, now: u64) -> Vec<&LedgerRecord> {
        self.records
            .iter()
            .filter(|r| matches!(r, LedgerRecord::Verdict { status: VerdictStatus::Unmet, retry_due: Some(due), .. } if *due <= now))
            .collect()
    }

    /// Lossless JSONL serialization.
    pub fn to_jsonl(&self) -> String {
        let mut out = String::new();
        for r in &self.records {
            out.push_str(&serde_json::to_string(r).expect("serialize ledger record"));
            out.push('\n');
        }
        out
    }

    /// Parse JSONL back into a ledger (clock must be re-supplied).
    pub fn from_jsonl(s: &str, clock: Box<dyn Clock>) -> Result<Self, String> {
        let mut ledger = Self::new(clock);
        for (i, line) in s.lines().enumerate() {
            let record: LedgerRecord = serde_json::from_str(line)
                .map_err(|e| format!("line {}: {e}", i + 1))?;
            ledger.records.push(record);
        }
        Ok(ledger)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(passed: bool) -> CheckReport {
        CheckReport { check: "threshold".into(), passed, detail: "x".into() }
    }

    #[test]
    fn met_record_has_no_retry_due() {
        let mut ledger = Ledger::new(Box::new(FakeClock(1000)));
        ledger.append(LedgerRecord::met("tt_job-001", vec!["tt_job-001#0".into()], vec![report(true)]));
        assert!(matches!(ledger.records()[0],
            LedgerRecord::Verdict { status: VerdictStatus::Met, retry_due: None, .. }));
    }

    #[test]
    fn unmet_record_carries_retry_due_and_parent() {
        let mut ledger = Ledger::new(Box::new(FakeClock(1000)));
        ledger.append(LedgerRecord::unmet("tt_job-001", vec![], vec![report(false)], 4600, Some("tt_job-001".into())));
        let r = &ledger.records()[0];
        match r {
            LedgerRecord::Verdict { status: VerdictStatus::Unmet, retry_due: Some(due), parent_id: Some(p), .. } => {
                assert_eq!(*due, 4600);
                assert_eq!(p, "tt_job-001");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn scan_due_finds_only_passed_unmet() {
        let mut ledger = Ledger::new(Box::new(FakeClock(1000)));
        ledger.append(LedgerRecord::met("a", vec![], vec![report(true)]));
        ledger.append(LedgerRecord::unmet("b", vec![], vec![report(false)], 900, None)); // due
        ledger.append(LedgerRecord::unmet("c", vec![], vec![report(false)], 2000, None)); // not yet

        let due = ledger.scan_due(1000);
        assert_eq!(due.len(), 1);
        match due[0] {
            LedgerRecord::Verdict { job_id, .. } => assert_eq!(job_id, "b"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn jsonl_roundtrip_is_byte_identical() {
        let mut ledger = Ledger::new(Box::new(FakeClock(1000)));
        ledger.append(LedgerRecord::met("a", vec!["a#0".into()], vec![report(true)]));
        ledger.append(LedgerRecord::unmet("b", vec![], vec![report(false)], 4600, Some("a".into())));

        let jsonl = ledger.to_jsonl();
        let back = Ledger::from_jsonl(&jsonl, Box::new(FakeClock(1000))).unwrap();
        assert_eq!(back.to_jsonl(), jsonl, "roundtrip must be byte-identical");
    }

    #[test]
    fn unix_secs_to_rfc3339_epoch_and_offsets() {
        assert_eq!(unix_secs_to_rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(unix_secs_to_rfc3339(1000), "1970-01-01T00:16:40Z");
        // A realistic moment (2026-09-03T00:00:00Z) — deterministic UTC rendering.
        assert_eq!(unix_secs_to_rfc3339(1788393600), "2026-09-03T00:00:00Z");
    }
}
