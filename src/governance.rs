//! B17 — governance visibility: which preconditions hold, and which this build
//! cannot see.
//!
//! The three are the Anaphase endpoint, `tuck_endpoint`, and Tuck's `audit_path`.
//! **Only `tuck_endpoint` has a source in this crate**; the other two live in
//! other organs' configs and are not wired here yet.
//!
//! A precondition with no source is **not met — it is unknown**, and reporting
//! unknown as met is the defect this family keeps producing (`unwrap_or("local")`,
//! `done` for an undefined transition, `ok: true` for an unconfigured endpoint,
//! `impasse = false` for a declared impasse, `ahead = 0` against a stale ref).
//!
//! So there are three states, not two:
//!
//! - `ungoverned`    — a **known** precondition is unmet (not configured, or unreachable)
//! - `indeterminate` — every known precondition is met, but some are **unknown**,
//!                     so the engine cannot honestly claim to be governed
//! - `governed`      — all preconditions known and met
//!
//! The middle state is the point. Folding it into either neighbour is the bug:
//! into `governed` it over-claims, into `ungoverned` it cries wolf about a
//! missing source rather than a missing guarantee.
//!
//! `governed` is therefore **not reachable in this build**, and that is the
//! honest answer rather than a bug to paper over.
//!
//! This is a module of its own rather than a corner of `health.rs` because the
//! line budget said so: appending it to a 198-line file is a 60% increase, and a
//! class that reaches files is capped at a tenth of the target precisely so that
//! "a fix" cannot quietly become "a feature". A feature gets a file.

use crate::config::AnaphaseConfig;
use serde_json::{json, Value};

/// B17's precondition count. Named because `preconditions_known` only means
/// something against it, and a bare `2` in a payload would be an unexplained
/// number of exactly the kind this ledger keeps correcting.
pub const PRECONDITIONS: usize = 3;

/// The preconditions this build can actually read.
///
/// A function rather than a literal inside `status`, because the set is a fact
/// about the contract and the tests assert against it. A literal would make
/// "which ones do we know" an implementation detail only `status` can see.
pub fn known_preconditions() -> &'static [&'static str] { &["tuck_endpoint"] }

/// B17's preconditions that have no config source in this crate yet.
///
/// **Their non-emptiness is what makes `governed` unreachable.** That is asserted,
/// not assumed — see `governed_is_dead_code_until_a_precondition_gains_a_source`.
/// The day one of these gains a source, that assertion goes red on purpose,
/// because `governed` stops being dead and whatever handles it needs tests.
pub fn unknown_preconditions() -> &'static [&'static str] {
    &["anaphase_endpoint", "tuck_audit_path"]
}

/// Which of the three hold, and which cannot be seen. See the module docs.
///
/// `probes` controls the reachability TCP probe: `false` reports configuration
/// only, which is what the startup path wants before the network can be assumed.
pub fn status(cfg: &AnaphaseConfig, probes: bool) -> Value {
    let mut unmet: Vec<&str> = Vec::new();
    let mut detail: Vec<String> = Vec::new();

    match cfg.tuck_endpoint.as_ref().filter(|s| !s.is_empty()) {
        None => {
            unmet.push("tuck_endpoint");
            detail.push("tuck_endpoint is not configured".to_string());
        }
        Some(ep) => {
            if probes {
                if let Err(e) = crate::health::tcp_reachable(ep) {
                    unmet.push("tuck_endpoint");
                    detail.push(format!("tuck_endpoint unreachable ({ep}): {e}"));
                }
            }
        }
    }

    // No config source in this crate yet. Listed so their absence is stated
    // rather than silently counted as satisfied.
    let unknown = unknown_preconditions();
    detail.push(format!(
        "{} of {} precondition(s) have no config source in this crate yet: {}",
        unknown.len(),
        PRECONDITIONS,
        unknown.join(", ")
    ));

    let state = if !unmet.is_empty() {
        "ungoverned"
    } else if !unknown.is_empty() {
        "indeterminate"
    } else {
        "governed"
    };

    json!({
        "state": state,
        "preconditions_total": PRECONDITIONS,
        "preconditions_known": PRECONDITIONS - unknown.len(),
        "unmet": unmet,
        "unknown": unknown,
        "detail": detail.join("; "),
    })
}

/// B17, for the startup path: a line to log when the engine is not governed.
///
/// `None` on "governed". Reports configuration only, with no reachability probe,
/// because this runs before the network can be assumed and must not block.
pub fn warning(cfg: &AnaphaseConfig) -> Option<String> {
    let gov = status(cfg, false);
    let state = gov["state"].as_str().unwrap_or("indeterminate");
    if state == "governed" {
        return None;
    }
    Some(format!(
        "governance: {state} — {} (preconditions known: {}/{})",
        gov["detail"].as_str().unwrap_or(""),
        gov["preconditions_known"].as_u64().unwrap_or(0),
        gov["preconditions_total"].as_u64().unwrap_or(0),
    ))
}