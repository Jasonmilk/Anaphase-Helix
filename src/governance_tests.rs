//! Tests for B17 governance visibility.
//!
//! Split out rather than inline so the test code is not budgeted as production;
//! the checker excludes `*_tests.rs` (spec §3).

use crate::governance::{known_preconditions, status, unknown_preconditions, warning, PRECONDITIONS};
use crate::config::AnaphaseConfig;
use std::net::TcpListener;

fn configured(tuck: Option<&str>) -> AnaphaseConfig {
    AnaphaseConfig { tuck_endpoint: tuck.map(|s| s.to_string()), ..Default::default() }
}

/// A live local listener, so "configured and reachable" is a real probe and not
/// a stubbed answer.
fn live_endpoint() -> (TcpListener, String) {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = l.local_addr().unwrap().to_string();
    (l, addr)
}

/// B17's criterion: not configured must be **reported**, and not as health.
/// The old shape could not express this — `check_endpoint` answered
/// `ok: true, detail: "not configured"`, so an absent precondition and a
/// satisfied one produced the same verdict, which is the family this ledger
/// keeps recording.
// guards: governance-states
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
        detail.contains("no config source"),
        "and the preconditions this build cannot see must be stated: {detail}"
    );
}

/// A precondition with no source is **unknown**, not met. Reporting unknown as
/// met is the defect; reporting unknown as unmet would cry wolf about a missing
/// source rather than a missing guarantee. Hence the third state, and hence
/// `governed` being unreachable while two preconditions have no source.
#[test]
fn unknown_preconditions_do_not_let_the_engine_claim_governed() {
    let (_l, addr) = live_endpoint();
    let v = status(&configured(Some(&addr)), true);
    assert_eq!(
        v["state"], "indeterminate",
        "every known precondition is met, but the engine still cannot claim to be \
         governed: {v}"
    );
    assert!(v["unmet"].as_array().unwrap().is_empty(), "{v}");
    assert_eq!(v["preconditions_known"], 1);
    assert_eq!(v["preconditions_total"], PRECONDITIONS);
    assert!(
        v["unknown"].as_array().unwrap().contains(&serde_json::json!("tuck_audit_path")),
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
    assert!(ungoverned.contains("known: 1/3"), "and give the count: {ungoverned}");

    let (_l, addr) = live_endpoint();
    let indeterminate = warning(&configured(Some(&addr))).expect("must warn");
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
/// config source, `unknown_preconditions` shrinks, this goes red, and whoever
/// moved it is told to re-derive the states and test whatever now handles
/// `governed`. The failure message says that, because the assertion's job is to
/// force a decision, not merely to be red.
// guards: governed-is-dead,config-fields-classified
#[test]
fn governed_is_dead_code_until_a_precondition_gains_a_source() {
    let known = known_preconditions();
    let unknown = unknown_preconditions();

    assert_eq!(
        known.len() + unknown.len(),
        PRECONDITIONS,
        "the two lists must account for every precondition B17 names, or one is \
         being counted nowhere"
    );
    assert!(
        known.iter().all(|k| !unknown.contains(k)),
        "a precondition cannot both have and lack a config source: known={known:?} \
         unknown={unknown:?}"
    );
    assert!(
        !unknown.is_empty(),
        "a precondition gained a config source, so `governed` may now be REACHABLE. \
         This assertion exists to make that day loud. Before deleting it: re-derive \
         the three states, check whether anything handles `governed` — and if it \
         became reachable, that arm stops being dead code and needs a test and a \
         mutation of its own. Also re-check `unknown` handling in `status`."
    );

    // And the behaviour, not just the list: with every KNOWN precondition
    // satisfied, the verdict still must not be `governed`.
    let (_l, addr) = live_endpoint();
    let v = status(&configured(Some(&addr)), true);
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
    assert_eq!(v["preconditions_known"], known_preconditions().len());
    assert_eq!(
        v["unknown"].as_array().unwrap().len(),
        unknown_preconditions().len()
    );
    assert_eq!(v["preconditions_total"], PRECONDITIONS);
}

/// `unknown_preconditions()` cannot express "we do not know what we do not know".
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
            known_preconditions().contains(&name.as_str()) || unknown_preconditions().contains(&name.as_str());
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
    for name in known_preconditions() {
        assert!(
            found.contains(&name.to_string()),
            "`{name}` is declared a precondition this build can READ, but no such \
             Option field exists in config.rs, so the declaration has outlived its subject"
        );
    }
}
