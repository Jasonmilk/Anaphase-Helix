//! Tests for B17 governance visibility.
//!
//! Split out rather than inline so the test code is not budgeted as production;
//! the checker excludes `*_tests.rs` (spec §3).

use crate::governance::{
    known_preconditions, status, unblock_for_governed, undefined_preconditions,
    unlocated_preconditions, warning, PRECONDITIONS,
};
use crate::config::AnaphaseConfig;

fn configured(tuck: Option<&str>) -> AnaphaseConfig {
    AnaphaseConfig { tuck_endpoint: tuck.map(|s| s.to_string()), ..Default::default() }
}

// The `live_endpoint` helper that used to sit here is gone with its last caller. It
// existed so "configured and reachable" was a real probe rather than a stub, which was
// the right instinct — but a real TCP connect is not a deterministic test input, and it
// made one test's verdict depend on machine load. The state-derivation tests now use
// `probes = false`, which is what they were actually about.

#[test]
fn a_missing_precondition_is_reported_as_ungoverned_not_as_ok() {
    let v = status(&configured(None), true);
    assert_eq!(v["state"], "ungoverned");
    assert!(
        v["unmet"].as_array().unwrap().iter().any(|x| x == "tuck_endpoint"),
        "the unmet precondition must be named, not merely counted: {v}"
    );
    let detail = v["detail"].as_str().unwrap();
    assert!(detail.contains("not configured"), "{detail}");
    assert!(
        detail.contains("defined outside this crate"),
        "and the preconditions this build cannot read must be stated: {detail}"
    );
    assert!(
        detail.contains("no definition anywhere"),
        "and B17's undefined name must be reported as a defect in its text, not as \
         an uncertainty here: {detail}"
    );
}

/// A precondition with no source is **unknown**, not met. Reporting unknown as
/// met is the defect; reporting unknown as unmet would cry wolf about a missing
/// source rather than a missing guarantee. Hence the third state, and hence
/// `governed` being unreachable while two preconditions have no source.
// guards: governance-states
#[test]
fn preconditions_that_are_not_readable_here_do_not_let_the_engine_claim_governed() {
    // `probes = false`: this test is about which CATEGORY a precondition falls into, not
    // about reaching it. It used a bound-but-not-accepting listener and a real TCP
    // connect, and a 10-second connect timeout is not a deterministic input — it passed
    // in the full suite and failed when run alone, i.e. the result depended on machine
    // load rather than on the code. A test whose verdict depends on timing is the same
    // defect as K-041's intermittent suite failure.
    let v = status(&configured(Some("tuck.invalid:1")), false);
    assert_eq!(
        v["state"], "indeterminate",
        "every known precondition is met, but the engine still cannot claim to be \
         governed: {v}"
    );
    assert!(v["unmet"].as_array().unwrap().is_empty(), "{v}");
    assert_eq!(
        v["preconditions_readable"].as_array().unwrap().len(),
        1,
        "only tuck_endpoint is readable here"
    );
    assert_eq!(
        v["preconditions_total"], PRECONDITIONS,
        "and the total counts only names with a definition"
    );
    assert!(
        v["preconditions_unlocated"].as_array().unwrap().contains(&serde_json::json!("tuck_audit_path")),
        "{v}"
    );
}

/// Configured but unreachable is ungoverned, and says which endpoint.
#[test]
fn an_unreachable_precondition_is_reported_with_its_endpoint() {
    // Port 1 on loopback: refused immediately, not a 10s timeout.
    let v = status(&configured(Some("127.0.0.1:1")), true);
    assert_eq!(v["state"], "ungoverned");
    let detail = v["detail"].as_str().unwrap();
    assert!(detail.contains("127.0.0.1:1"), "{detail}");
}

/// The startup line is the visible half of B17, and it must fire for both
/// non-governed states rather than only the one that happens to be loud.
#[test]
fn the_startup_warning_fires_for_every_non_governed_state() {
    let ungoverned = warning(&configured(None)).expect("must warn");
    assert!(ungoverned.contains("ungoverned"), "{ungoverned}");
    assert!(ungoverned.contains("known: 1/2"), "and give the count: {ungoverned}");

    let indeterminate = warning(&configured(Some("tuck.invalid:1"))).expect("must warn");
    assert!(indeterminate.contains("indeterminate"), "{indeterminate}");
}

/// `status` is surfaced through the health payload, so it has to be there and it
/// has to be a separate key: folding it into `ok` would make every optional organ
/// look like a failure, which is why it is not folded in.
#[test]
fn the_health_payload_carries_governance_separately_from_ok() {
    let v = crate::health::checks(&configured(None));
    assert_eq!(v["ok"], true, "an unconfigured engine is healthy in the old sense");
    assert_eq!(
        v["governance"]["state"], "ungoverned",
        "and ungoverned in the new one; both must be readable at once"
    );
}

/// `governed` is **dead code today**, and this is the assertion that keeps that
/// fact from becoming silent.
///
/// Round 31's point, and it is the transition-table lesson a second time: 13 edges
/// were declared and the denominator was what is *reachable*; here 3 states are
/// declared and 2 are reachable. A state that can never occur is a `match` arm
/// nobody will ever see execute — the next person writes
/// `match { Governed => ..., _ => warn }` and that arm is dead forever, with
/// nothing to say so.
///
/// So unreachability is pinned rather than tolerated. When a precondition gains a
/// config source, `unlocated`/`undefined` shrink, this goes red, and whoever
/// moved it is told to re-derive the states and test whatever now handles
/// `governed`. The failure message says that, because the assertion's job is to
/// force a decision, not merely to be red.
// guards: governed-is-dead,config-fields-classified,undefined-names-stay-loud
#[test]
fn governed_is_dead_code_until_a_precondition_gains_a_source() {
    let known = known_preconditions();
    let unlocated = unlocated_preconditions();
    let undefined = undefined_preconditions();

    assert_eq!(
        known.len() + unlocated.len(),
        PRECONDITIONS,
        "the DEFINED preconditions must account for the total. `undefined` is not \
         part of it: a name with no definition is not a precondition, it is a defect \
         in B17's text, and counting it would make the total describe a list that \
         does not exist"
    );
    for (a, b, an, bn) in [
        (known, unlocated, "known", "unlocated"),
        (known, undefined, "known", "undefined"),
        (unlocated, undefined, "unlocated", "undefined"),
    ] {
        assert!(
            a.iter().all(|k| !b.contains(k)),
            "a name cannot be in both {an} and {bn}: {a:?} vs {b:?}"
        );
    }
    assert!(
        !(unlocated.is_empty() && undefined.is_empty()),
        "a precondition gained a config source, so `governed` may now be REACHABLE. \
         This assertion exists to make that day loud. Before deleting it: re-derive \
         the three states, check whether anything handles `governed` — and if it \
         became reachable, that arm stops being dead code and needs a test and a \
         mutation of its own. Also re-check `unknown` handling in `status`."
    );

    // And the behaviour, not just the list: with every KNOWN precondition
    // satisfied, the verdict still must not be `governed`.
    let v = status(&configured(Some("tuck.invalid:1")), false);
    assert_ne!(
        v["state"], "governed",
        "the third state is unreachable; if this fires, the list assertion above \
         was passed by a build that still cannot reach `governed`, which means the \
         two disagree: {v}"
    );
}

/// The counts in the payload must be derived from the same lists the assertion
/// above reads. Otherwise `preconditions_known: 1` could be right while the
/// unknown list stopped meaning anything.
#[test]
fn the_payload_counts_come_from_the_same_lists() {
    let v = status(&configured(None), false);
    assert_eq!(
        v["preconditions_readable"].as_array().unwrap().len(),
        known_preconditions().len()
    );
    assert_eq!(
        v["preconditions_unlocated"].as_array().unwrap().len(),
        unlocated_preconditions().len()
    );
    assert_eq!(
        v["undefined_names"].as_array().unwrap().len(),
        undefined_preconditions().len()
    );
    assert_eq!(v["preconditions_total"], PRECONDITIONS);
}

/// A self-reporting set cannot express "we do not know what we do not know".
///
/// Its own list is the only source of truth for what is unknown, so a fourth
/// precondition added to the config tomorrow would appear in neither list and
/// `known + unknown == PRECONDITIONS` would still be green. That is the bound of
/// any self-reporting set: it can enumerate what it has been told, not what it has
/// not.
///
/// So the classification is checked against the **source** instead of against
/// itself. Every `Option` field in the config must be accounted for: either it is
/// a governance precondition, or it is named here as explicitly not one. A new
/// `Option` field therefore cannot arrive silently — it must be classified, and
/// classifying it is the moment someone decides whether governance depends on it.
///
/// P15: the label is an index, the criterion is the gate. This list is the label;
/// the scan in `every_option_config_field_is_classified` is the gate.
#[test]
fn every_option_config_field_is_classified() {
    // Fields whose absence cannot change whether the engine is governed. The list
    // is the claim; the scan below is what makes the claim have to be complete.
    const NOT_A_PRECONDITION: &[&str] = &[
        // Organ endpoints that B17 does not name. Whether they *should* be
        // preconditions is a B17 question, not this test's; what this test forbids
        // is adding one without deciding.
        "cellrix_endpoint", "flowmodus_endpoint", "mind_endpoint", "tentacle_endpoint",
        // Reasoning configuration: a model choice, not a guarantee.
        "reasoning_api_key", "reasoning_endpoint", "reasoning_max_tokens", "reasoning_model",
        "reasoning_redact_patterns", "reasoning_route_tier", "reasoning_trace_max_chars",
        "judge_endpoint", "judge_model",
        // Local paths: data placement, not governance authority.
        "bind_state_path", "events_log_path", "gene_lock_path", "reasoning_trace_path",
        "session_events_path", "session_notes_path",
        // Operator input for a smoke run.
        "smoke_input",
    ];

    let src = include_str!("config.rs");
    let mut found: Vec<String> = Vec::new();
    for line in src.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("pub ") {
            if let Some((name, ty)) = rest.split_once(':') {
                if ty.trim_start().starts_with("Option<") {
                    found.push(name.trim().to_string());
                }
            }
        }
    }
    found.sort();
    found.dedup();
    assert!(
        found.len() > 15,
        "the scan found only {} Option field(s); it has probably stopped matching \
         the config's shape, which would make this test pass by finding nothing: {found:?}",
        found.len()
    );

    let mut unclassified = Vec::new();
    for name in &found {
        let is_precondition =
            known_preconditions().contains(&name.as_str())
                || unlocated_preconditions().contains(&name.as_str())
                || undefined_preconditions().contains(&name.as_str());
        if !is_precondition && !NOT_A_PRECONDITION.contains(&name.as_str()) {
            unclassified.push(name.clone());
        }
    }
    assert!(
        unclassified.is_empty(),
        "new Option config field(s) {unclassified:?} are in neither the governance \
         preconditions nor the not-a-precondition list. Decide which: if its absence \
         can change whether the engine is governed, it belongs in one of the two \
         lists in `governance.rs` (and `PRECONDITIONS` must grow with it); if not, \
         add it to NOT_A_PRECONDITION here with the reason."
    );

    // Reverse check, for the KNOWN list only: a name declared as a precondition
    // this build can read must actually be readable.
    //
    // Deliberately not applied to the unknown list, and the first version DID apply
    // it and went red — which turned out to be a real distinction rather than a bug.
    // "No such field in this crate" is what makes a precondition *unknown*, so
    // asserting the opposite would contradict the definition. But that failure also
    // showed the unknown list holds two different things:
    //
    //   `tuck_audit_path`   — has a source, in ANOTHER organ (Tuck's config)
    //   `anaphase_endpoint` — has no source anywhere yet: not unreadable-here,
    //                         but undefined
    //
    // Both are ungovernable from this crate today, so both stay in the unknown
    // bucket. Naming the difference matters because the fix differs: one needs a
    // cross-organ read, the other needs the field to exist first.
    for name in known_preconditions().iter().chain(unlocated_preconditions()) {
        // `unlocated` names are exempt from the "must exist in config.rs" rule
        // only when they are declared as belonging to another organ. The one that
        // does is Tuck's, so it is checked against a stated list rather than
        // against this crate's config.
        if !known_preconditions().contains(name) {
            continue;
        }
        assert!(
            found.contains(&name.to_string()),
            "`{name}` is declared a precondition this build can READ, but no such \
             Option field exists in config.rs, so the declaration has outlived its subject"
        );
    }
}

/// B17's undefined precondition is **still unresolved**, and this pin makes
/// resolving it loud.
///
/// Round 34's point, and it is the sharpest thing said about this module: because
/// `anaphase_endpoint` sat in the "unknown" bucket, `unknown` was non-empty, so
/// `governed` was unreachable — and that "unreachability" might have been a naming
/// accident rather than an architectural fact. **A mechanism for detecting what
/// cannot be seen must not be blind to what it is locked by.**
///
/// So the ghost is reported in its own field and excluded from the total, and this
/// test states the current truth: the name is unresolved. The day someone defines
/// it (a real config field) or deletes it from B17, this goes red, and whoever
/// moved it is told to re-derive the three states — which is exactly the
/// re-derivation K-040 asked for.
///
/// Note that K-040's mutation still holds either way: emptying `unlocated` and
/// `undefined` together still reddens the unreachability assertion. What changed is
/// *why* `governed` is unreachable — and the reason is now a fact rather than a
/// possible typo.
#[test]
fn b17s_undefined_precondition_is_still_unresolved() {
    let undefined = undefined_preconditions();
    assert_eq!(
        undefined,
        &["anaphase_endpoint"],
        "B17's undefined precondition has been defined or removed. Before updating \
         this test: re-derive the three states, check whether `governed` became \
         reachable, and if it did, note that whatever handles it stops being dead \
         code. If the name was DELETED from B17 rather than given a field, also \
         correct PRECONDITIONS and the B17 entry in the decision index."
    );

    // And the reason `governed` is unreachable must be a fact, not the ghost:
    // the cross-organ source is what actually keeps it out of reach.
    assert!(
        !unlocated_preconditions().is_empty(),
        "if `unlocated` ever empties while `undefined` is non-empty, then the only \
         thing blocking `governed` would be the ghost name — which would mean the \
         module is reporting a typo as architectural uncertainty"
    );

    // The ghost must not leak into the arithmetic.
    assert_eq!(
        known_preconditions().len() + unlocated_preconditions().len(),
        PRECONDITIONS
    );
    let v = status(&configured(None), false);
    assert_eq!(v["undefined_names"][0], "anaphase_endpoint");
    assert_eq!(
        v["preconditions_total"], PRECONDITIONS,
        "the undefined name must not be counted in the total"
    );
}

/// An unreachable state must carry the condition that would unblock it.
///
/// `governed` cannot be reached today, and the temptation for the next reader is to
/// see a `Governed => …` arm that never runs and delete it as dead code. But it is
/// not dead — it is *not yet*, because a cross-organ read has not been written. The
/// two are different and a missing field makes them look the same.
///
/// So: while `governed` is unreachable, `unblock_for_governed()` must be `Some`, and
/// it must name an action rather than a condition. When the precondition set empties
/// and `governed` becomes reachable, this goes red — and the right response is to
/// delete the unblock, not to invent one.
#[test]
fn an_unreachable_state_must_say_what_would_unblock_it() {
    let unreachable = governed_is_unreachable();
    let unblock = unblock_for_governed();
    assert_eq!(
        unreachable,
        unblock.is_some(),
        "reachability and the unblock field must agree: unreachable={unreachable}, \
         unblock={unblock:?}. A state that is unreachable with no unblock is a state \
         nobody intends to reach; a reachable state with one is a stale instruction."
    );
    let text = unblock.expect("governed is unreachable, so it must be unblockable");
    assert!(
        text.contains("read") || text.contains("give it") || text.contains("delete"),
        "the unblock must name an ACTION, not restate the condition: {text:?}"
    );
    // And it must reach the payload, or it is a private note.
    let v = status(&configured(None), false);
    assert!(
        v["unblock_governed"].is_string(),
        "the unblock must be queryable at /v1/health: {v}"
    );
}

fn governed_is_unreachable() -> bool {
    !unlocated_preconditions().is_empty() || !undefined_preconditions().is_empty()
}
