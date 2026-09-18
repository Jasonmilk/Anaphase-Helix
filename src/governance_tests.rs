//! Tests for B17 governance visibility.
//!
//! Split out rather than inline so the test code is not budgeted as production;
//! the checker excludes `*_tests.rs` (spec §3).

use crate::governance::{status, warning, PRECONDITIONS};
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
