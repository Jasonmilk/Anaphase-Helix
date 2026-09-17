//! Session event stream — the "session as experience" body.
//!
//! Split out of a single 1545-line file (ADR-0042). The public path is
//! unchanged: every item below is re-exported, so `session_events::read_period`
//! and friends resolve exactly as before.
//!
//! One cognitive period appends its structured events to one JSONL file named
//! after its ALLOCATED `period_id`:
//!
//!   {"type":"turn/start","period_id":"run-…-p…","job_id":"run-…","seq":0,…}
//!
//! `job_id` is the join key the body trace and the Tuck audit chain carry
//! (ProveTrack); `period_id` is what makes one run distinct from another
//! (K-006). Events carry summaries only — the full prompt/response bodies live
//! in the reasoning trace — and credentials are redacted on write, so the
//! stream never becomes a sensitive data lake.

pub mod crystallize;
pub mod identity;
pub mod naming;
pub mod query;
pub mod types_and_stream;

pub use identity::{
    allocate_period_id, ambiguous_error, is_job_id, is_period_id, not_found_error, resolve_one,
    resolve_period, try_allocate_period_id, PeriodRef, Resolved,
};
pub use crystallize::{crystallize, CrystalSuggestion};
pub use naming::{freeze_name, rename_period};
pub use query::{list_periods, read_period, read_summary, PeriodSummary};
pub use types_and_stream::{EventType, SessionEvent, SessionEventStream};

#[cfg(test)]
pub mod test_support;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod query_tests;
