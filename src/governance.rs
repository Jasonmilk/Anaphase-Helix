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

/// How many of B17's preconditions this repository can talk about at all.
///
/// **Corrected from 3 to 2.** B17 names three, but one (`anaphase_endpoint`) is
/// defined nowhere, so counting it made the total describe a list that does not
/// exist — and made `preconditions_known` a fraction of a fiction. The two counted
/// here are the ones with a definition: `tuck_endpoint` in this crate,
/// `tuck_audit_path` in Tuck. The third is reported by
/// [`undefined_preconditions`] as a defect in B17's own text.
pub const PRECONDITIONS: usize = 2;

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
pub fn unlocated_preconditions() -> &'static [&'static str] {
    &["tuck_audit_path"]
}

/// Names B17 lists as preconditions that **this repository cannot define at all**.
///
/// A fourth category on purpose, because folding it into "unknown" launders a
/// defect into an honest uncertainty.
///
/// `anaphase_endpoint` is named in B17 and appears nowhere else: no config field,
/// no reader, no writer. The nearest real thing is `cap_http_port`, a `u16` with a
/// default, which **cannot be unmet** — so if that is what B17 means, it is not a
/// precondition; if it means something else, that something does not exist. Either
/// way B17's own text is wrong, and nothing in this repository distinguishes the
/// two readings.
///
/// **Why it must not be reported as `unknown`.** In the output, "unknown" and
/// "undefined" would be indistinguishable, and the next reader of `indeterminate`
/// would take it for the system being candid about a precondition it cannot see —
/// when it may only be reporting a naming accident. A mechanism for detecting what
/// cannot be seen must not be blind to what it is locked by.
///
/// So it gets its own field, it does **not** count toward [`PRECONDITIONS`], and it
/// is listed here so that resolving it — define the name, or delete it from B17 —
/// is a deliberate act with an obvious place to happen.
pub fn undefined_preconditions() -> &'static [&'static str] {
    &["anaphase_endpoint"]
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
    let unlocated = unlocated_preconditions();
    let undefined = undefined_preconditions();
    if !unlocated.is_empty() {
        detail.push(format!(
            "{} precondition(s) are defined outside this crate: {}",
            unlocated.len(),
            unlocated.join(", ")
        ));
    }
    if !undefined.is_empty() {
        detail.push(format!(
            "{} name(s) B17 lists as preconditions have no definition anywhere: {} \
             — a defect in B17's own text, not an uncertainty here",
            undefined.len(),
            undefined.join(", ")
        ));
    }

    let state = if !unmet.is_empty() {
        "ungoverned"
    } else if !unlocated.is_empty() || !undefined.is_empty() {
        "indeterminate"
    } else {
        "governed"
    };

    json!({
        "state": state,
        "preconditions_total": PRECONDITIONS,
        "preconditions_readable": known_preconditions(),
        "preconditions_unlocated": unlocated,
        // Deliberately not called "unknown". A name with no definition is not an
        // unknown precondition; it is a name with no definition.
        "undefined_names": undefined,
        // Non-null exactly while `governed` is unreachable, and naming the action
        // that would make it reachable. A dead state with no unblock is dead on
        // purpose; this one is not.
        "unblock_governed": unblock_for_governed(),
        "unmet": unmet,
        "detail": detail.join("; "),
    })
}

/// What would make `governed` reachable, and where that work lives.
///
/// **Every unreachable state must carry its unblock, or it gets deleted.** Round
/// 35's point: `governed` is unreachable *today* because a cross-organ read has not
/// been written — that is a to-do, not an impossibility. Without a field saying so,
/// the next person sees a dead `Governed => …` arm, concludes it is dead code, and
/// removes it — and it is exactly the arm that must come alive on the day the read
/// lands. Collapsing "not yet" into "never" is the same defect as collapsing
/// "unknown" into "met".
///
/// `unblock` is `None` only when the state is reachable. A state that is unreachable
/// with no unblock is a state nobody intends to reach, and that should be said
/// outright rather than implied by a missing field.
pub fn unblock_for_governed() -> Option<&'static str> {
    if !unlocated_preconditions().is_empty() {
        return Some(
            "read Tuck's `audit_path` from Tuck's own config (a cross-organ read).              Until then the engine cannot establish whether the audit chain is              intact, which is what `governed` claims.",
        );
    }
    if !undefined_preconditions().is_empty() {
        return Some(
            "B17 names a precondition this repository cannot define. Either give it              a config field or delete it from B17 — see the B17 revision in              CI-144_决策索引.md.",
        );
    }
    None
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
        gov["preconditions_readable"].as_array().map(|a| a.len()).unwrap_or(0),
        gov["preconditions_total"].as_u64().unwrap_or(0),
    ))
}