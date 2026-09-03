//! Append-only evidence store (ADR-0003).
//!
//! Both success and failure are recorded. Determinism clamps (ADR-0003):
//! - fixed-field struct, no HashMap, no endpoint/port fields
//! - `evidence_id` is derived from `{job_id}#{call_index}` (no randomness)

use crate::contract::Expect;
use serde::{Deserialize, Serialize};

/// A single tool execution record (success or failure).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceRecord {
    /// Derived as `{job_id}#{call_index}` (deterministic).
    pub evidence_id: String,
    pub job_id: String,
    pub call_index: u32,
    pub tool: String,
    /// Expected result shape — self-contained so evidence can be re-checked
    /// independently (M1.5 re-entry without re-deriving from calls).
    pub expect: Expect,
    pub ok: bool,
    /// Tool response data on success, or error message on failure.
    pub data: String,
    pub duration_ms: u64,
}

impl EvidenceRecord {
    pub fn new(
        job_id: &str,
        call_index: u32,
        tool: &str,
        expect: Expect,
        ok: bool,
        data: &str,
        duration_ms: u64,
    ) -> Self {
        Self {
            evidence_id: format!("{job_id}#{call_index}"),
            job_id: job_id.to_string(),
            call_index,
            tool: tool.to_string(),
            expect,
            ok,
            data: data.to_string(),
            duration_ms,
        }
    }
}

/// In-memory append-only store. File IO is the pipeline's job (stage 4);
/// this module owns the record model and lossless JSONL serialization.
#[derive(Debug, Default)]
pub struct EvidenceStore {
    records: Vec<EvidenceRecord>,
}

impl EvidenceStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a record. Once appended, records are never mutated or removed.
    pub fn append(&mut self, record: EvidenceRecord) {
        self.records.push(record);
    }

    pub fn records(&self) -> &[EvidenceRecord] {
        &self.records
    }

    /// Lossless JSONL serialization (one record per line).
    pub fn to_jsonl(&self) -> String {
        let mut out = String::new();
        for r in &self.records {
            out.push_str(&serde_json::to_string(r).expect("serialize evidence record"));
            out.push('\n');
        }
        out
    }

    /// Parse JSONL back into a store. Roundtrip must be byte-identical.
    pub fn from_jsonl(s: &str) -> Result<Self, String> {
        let mut store = Self::new();
        for (i, line) in s.lines().enumerate() {
            let record: EvidenceRecord = serde_json::from_str(line)
                .map_err(|e| format!("line {}: {e}", i + 1))?;
            store.records.push(record);
        }
        Ok(store)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_then_roundtrip_is_byte_identical() {
        let mut store = EvidenceStore::new();
        store.append(EvidenceRecord::new("tt_job-001", 0, "numbers", Expect::Numbers, true, r#"{"series":[1.0,2.0]}"#, 5));
        store.append(EvidenceRecord::new("tt_job-001", 1, "rate", Expect::Rate, false, "tool failed", 12));

        let jsonl = store.to_jsonl();
        let back = EvidenceStore::from_jsonl(&jsonl).unwrap();
        assert_eq!(back.to_jsonl(), jsonl, "roundtrip must be byte-identical");
        assert_eq!(back.records().len(), 2);
    }

    #[test]
    fn evidence_id_is_derived_not_random() {
        let r = EvidenceRecord::new("tt_job-007", 3, "text", Expect::Text, true, "{}", 1);
        assert_eq!(r.evidence_id, "tt_job-007#3");
    }

    #[test]
    fn failure_records_are_kept() {
        let mut store = EvidenceStore::new();
        store.append(EvidenceRecord::new("j", 0, "rate", Expect::Rate, false, "boom", 9));
        assert!(!store.records()[0].ok);
        assert_eq!(store.records()[0].data, "boom");
    }
}
