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

pub mod convergence;
pub mod crystallize;
pub mod gc;
pub mod identity;
pub mod naming;
pub mod pins;
pub mod query;
pub mod refs;
pub mod types_and_stream;

pub use gc::{
    collect_garbage, plan as gc_plan, purge_content, read_vacancies, rewrite_atomically, replay_exists, replay_exists_ignoring_purge,
    replay_live, rfc3339_to_secs, vacancies, ContentPurgeReport, GcInput, GcOutcome, GcPlan, Object as GcObject,
    VacancyView,
    Vacancy as GcVacancy,
};
pub use identity::{
    allocate_period_id, ambiguous_error, is_job_id, is_period_id, not_found_error, resolve_one,
    resolve_period, try_allocate_period_id, PeriodRef, Resolved,
};
pub use convergence::{ConvergenceConfig, PeriodStatus, StatusSource};
pub use crystallize::{crystallize, CrystalSuggestion};
pub use naming::{freeze_name, rename_period};
pub use pins::{
    check_owner, declared_owners, orphan_pins, pin, release_orphan_owner, replay as replay_pins, unpin,
};
pub use query::{
    count_tombstoned, list_periods, read_period, read_summary, tombstone_period, PeriodSummary,
};
pub use refs::{
    check_retention_covers_grace, delete_ref, list_refs, read_ref, read_ref_log, release_stale_writer,
    retention_covers_grace, write_ref, RefEntry, RefLogEntry, WriterLock, REF_MOVE_GRACE_SECS,
    REF_MOVE_RETENTION_SECS,
};
pub use types_and_stream::{
    mode_wire_opt, repair_torn_tail, EventType, SessionEvent, SessionEventStream,
};

#[cfg(test)]
pub mod test_support;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod pins_tests;
mod query_tests;
mod gc_tests;
mod refs_tests;
