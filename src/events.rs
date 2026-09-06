//! Stage event ring (ADR-0019): an append-only, deterministic record of the
//! six-stage pipeline's progress — events are the *process*, the ledger is
//! the *fact*, evidence is the *support* (three disjoint layers, ADR-0016 D5).
//!
//! Semantics:
//! - append-only: events are never mutated or removed (dropped events are
//!   counted, never rewritten).
//! - deterministic: the timestamp comes from the caller (the injected Clock,
//!   same source as the ledger) and the seq is a monotonically increasing
//!   counter — replay is byte-identical under a FakeClock.
//! - pull, not push: consumers poll with `after(seq)` (incremental). The
//!   event ring is a record, never a control-flow backplane.
//! - naming follows the OTel GenAI signal spirit (event/exception/metric)
//!   without importing any dependency; stage/phase is our own vocabulary.

use serde::{Deserialize, Serialize};

/// One deterministic stage event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StageEvent {
    /// Monotonic sequence — the cursor for incremental pulls (`after(seq)`).
    pub seq: u64,
    /// RFC3339 timestamp from the injected clock (caller-supplied, so the
    /// ring itself stays clock-free and the caller stays deterministic).
    pub ts: String,
    /// Deterministic trace id: the derived job id (ADR-0003), no UUID.
    pub trace_id: String,
    /// Pipeline stage 1..=6 (ADR-0003 decision 9 mapping).
    pub stage: u8,
    /// "begin" | "end" | "verdict".
    pub phase: String,
    /// Short machine-readable payload (call count / tool / verdict status).
    pub detail: String,
}

/// Append-only event ring with a bounded capacity. When the cap is reached
/// new events are refused and counted in `dropped` — `after(seq)` never
/// returns a rewritten view, and the cursor semantics stay intact.
#[derive(Debug)]
pub struct EventRing {
    events: Vec<StageEvent>,
    seq: u64,
    cap: usize,
    dropped: u64,
}

impl EventRing {
    /// `cap` is a config value (PipelineConfig.events_cap, loaded from the
    /// fixture-codex contract — DNA principle 11, no literal here).
    pub fn new(cap: usize) -> Self {
        Self { events: Vec::new(), seq: 0, cap, dropped: 0 }
    }

    /// Append one event. Refused (counted) when the ring is full.
    pub fn emit(&mut self, ts: &str, trace_id: &str, stage: u8, phase: &str, detail: &str) {
        if self.events.len() >= self.cap {
            self.dropped += 1;
            return;
        }
        self.seq += 1;
        self.events.push(StageEvent {
            seq: self.seq,
            ts: ts.to_string(),
            trace_id: trace_id.to_string(),
            stage,
            phase: phase.to_string(),
            detail: detail.to_string(),
        });
    }

    /// All events in append order.
    pub fn events(&self) -> &[StageEvent] {
        &self.events
    }

    /// Incremental pull: everything after the given cursor (0 = all).
    pub fn after(&self, seq: u64) -> Vec<StageEvent> {
        self.events.iter().filter(|e| e.seq > seq).cloned().collect()
    }

    /// Number of events refused since creation (capacity saturation).
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Capacity (the codex-contract source; read-only, no mutation).
    pub fn cap(&self) -> usize {
        self.cap
    }

    /// Last sequence number (0 = empty) — the cursor a consumer would keep.
    pub fn last_seq(&self) -> u64 {
        self.seq
    }

    /// Deterministic JSONL dump (replay / audit trail).
    pub fn to_jsonl(&self) -> String {
        let mut out = String::new();
        for e in &self.events {
            out.push_str(&serde_json::to_string(e).unwrap_or_default());
            out.push('\n');
        }
        out
    }

    /// Restore from a JSONL dump (O-3): rebuilds events in append order and
    /// resumes `seq` after the highest restored sequence — the incremental
    /// cursor stays continuous across restarts. Malformed lines are refused
    /// (fail-closed: a corrupted trail must not silently truncate history).
    pub fn from_jsonl(s: &str, cap: usize) -> Result<Self, String> {
        let mut events = Vec::new();
        let mut seq = 0u64;
        for (i, line) in s.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let e: StageEvent = serde_json::from_str(line)
                .map_err(|e| format!("line {}: {e}", i + 1))?;
            seq = seq.max(e.seq);
            events.push(e);
        }
        if events.len() > cap {
            return Err(format!(
                "restored {} events exceed cap {cap} (history too long for this config)",
                events.len()
            ));
        }
        Ok(Self { events, seq, cap, dropped: 0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seq_is_monotonic_and_after_is_incremental() {
        let mut ring = EventRing::new(64);
        ring.emit("t1", "job-1", 1, "begin", "parse");
        ring.emit("t2", "job-1", 1, "end", "calls=2");
        ring.emit("t3", "job-1", 2, "begin", "assemble");
        assert_eq!(ring.last_seq(), 3);
        assert_eq!(ring.after(0).len(), 3, "after(0) is the full stream");
        assert_eq!(ring.after(1).len(), 2, "after(seq) is incremental");
        assert_eq!(ring.after(3).len(), 0, "after(last) is empty");
        assert_eq!(ring.after(1)[0].seq, 2);
        assert_eq!(ring.events()[0].trace_id, "job-1");
    }

    #[test]
    fn cap_refuses_and_counts_but_never_rewrites() {
        let mut ring = EventRing::new(2);
        ring.emit("t1", "a", 1, "begin", "");
        ring.emit("t2", "a", 1, "end", "");
        ring.emit("t3", "a", 2, "begin", "");
        assert_eq!(ring.events().len(), 2, "cap holds");
        assert_eq!(ring.dropped(), 1, "overflow counted");
        assert_eq!(ring.last_seq(), 2, "refused events do not advance the cursor");
        assert_eq!(ring.after(0).len(), 2, "stream stays consistent");
    }

    #[test]
    fn jsonl_is_deterministic() {
        let mut a = EventRing::new(8);
        let mut b = EventRing::new(8);
        for i in 0..3 {
            a.emit(&format!("t{i}"), "job-1", 3, "end", "ok");
            b.emit(&format!("t{i}"), "job-1", 3, "end", "ok");
        }
        assert_eq!(a.to_jsonl(), b.to_jsonl(), "same inputs -> byte-identical jsonl");
    }

    #[test]
    fn from_jsonl_restores_round_trip() {
        let mut ring = EventRing::new(32);
        for i in 0..3 {
            ring.emit(&format!("t{i}"), "job-1", 3, "end", "ok");
        }
        let dump = ring.to_jsonl();
        let restored = EventRing::from_jsonl(&dump, 32).expect("valid dump restores");
        assert_eq!(restored.to_jsonl(), dump, "round-trip byte-identical");
        assert_eq!(restored.last_seq(), ring.last_seq(), "seq resumed");
        assert_eq!(restored.after(0).len(), 3, "history replayed");
    }

    #[test]
    fn from_jsonl_keeps_cursor_continuous() {
        let mut ring = EventRing::new(32);
        ring.emit("t0", "job-1", 3, "begin", "");
        let restored = EventRing::from_jsonl(&ring.to_jsonl(), 32).unwrap();
        let mut resumed = restored;
        resumed.emit("t1", "job-1", 3, "end", "ok");
        assert_eq!(resumed.last_seq(), 2, "new event allocates the next seq after restored max");
        assert_eq!(resumed.events().len(), 2);
    }

    #[test]
    fn from_jsonl_refuses_corrupt_line() {
        let mut ring = EventRing::new(32);
        ring.emit("t0", "job-1", 3, "end", "ok");
        let mut bad = ring.to_jsonl();
        bad.push_str("{not-json}
");
        assert!(
            EventRing::from_jsonl(&bad, 32).is_err(),
            "corrupt line fails closed, never silently truncates"
        );
    }

    #[test]
    fn from_jsonl_enforces_cap() {
        let mut ring = EventRing::new(32);
        for i in 0..4 {
            ring.emit(&format!("t{i}"), "job-1", 3, "end", "ok");
        }
        let dump = ring.to_jsonl();
        assert!(
            EventRing::from_jsonl(&dump, 2).is_err(),
            "history longer than cap is refused (config mismatch surfaced)"
        );
    }
}
