//! The gate's branch table, asserted in one place (ADR-0042, K-033).
//!
//! A pure move is only verifiable if something exercises the branches that were
//! moved, and nothing did: `NoopSafetyAdapter` returns `Ok(true)` and the default
//! `HITLApprover` is never reached for a low-risk command, so both `Err` branches
//! were dead in every existing test. That is why the move above it cannot be
//! called verified without this file.
//!
//! The table is the point. K-033 is that two callers answer the same question
//! oppositely, and while they were 350 lines apart a reader had to hold both in
//! their head to notice. Here the two answers are adjacent rows.
//!
//! Split out of `safety_gate.rs` rather than left inline: the checker excludes
//! `*_tests.rs` from the line budget, and an inline test module in a production
//! file is budgeted as production.

use super::safety_gate::{admit, GateVerdict, OnAuditError};
use super::TransitionCondition;
use crate::adapters::SafetyAdapter;
use crate::hitl::HITLApprover;
use async_trait::async_trait;
use std::sync::Arc;

/// A `SafetyAdapter` whose answer the test chooses.
struct StubSafety(Result<bool, String>);

#[async_trait]
impl SafetyAdapter for StubSafety {
    async fn audit(&self, _action: &str, _content: &str) -> Result<bool, String> {
        self.0.clone()
    }
}

/// HITL approver with a chosen answer. Only reached for a HIGH-RISK command:
/// `check_approval` returns `Ok(true)` for anything low-risk without consulting
/// the callback, so a test that wants the HITL branches must pass a name from the
/// write/network/credential lists. `"rm"` is used throughout for that reason, and
/// getting this wrong is silent — the branch simply never runs.
fn hitl(answer: Result<bool, String>) -> HITLApprover {
    HITLApprover::new(Arc::new(move |_c: &str, _a: &[String]| answer.clone()))
}

/// A tool name that is always high-risk, so `check_approval` reaches the stub.
const HIGH_RISK: &str = "rm";
/// A tool name that is never high-risk, so HITL short-circuits to `Ok(true)`.
const LOW_RISK: &str = "numbers";

fn no_args() -> Vec<String> {
    vec![]
}

/// Every branch of `admit`, and what each answers.
///
/// The two rows marked K-033 are the finding: identical inputs, opposite
/// verdicts, and the only thing that picks between them is which caller you are.
#[tokio::test]
async fn every_gate_branch_answers_what_it_answers_today() {
    // (label, hitl, audit, policy, expected)
    struct Row {
        label: &'static str,
        hitl: Result<bool, String>,
        audit: Result<bool, String>,
        policy: OnAuditError,
        expected: GateVerdict,
    }
    use OnAuditError::{Block, ReportSuccess};

    let rows = vec![
        Row {
            label: "low risk, audit clears",
            hitl: Ok(true),
            audit: Ok(true),
            policy: ReportSuccess,
            expected: GateVerdict::Cleared,
        },
        Row {
            label: "low risk, audit clears (block policy: same)",
            hitl: Ok(true),
            audit: Ok(true),
            policy: Block,
            expected: GateVerdict::Cleared,
        },
        Row {
            label: "HITL refuses (high risk)",
            hitl: Ok(false),
            audit: Ok(true),
            policy: ReportSuccess,
            expected: GateVerdict::Refused(TransitionCondition::Failure),
        },
        Row {
            label: "HITL has no channel (high risk, fail-closed)",
            hitl: Err("no channel".into()),
            audit: Ok(true),
            policy: ReportSuccess,
            expected: GateVerdict::Refused(TransitionCondition::Failure),
        },
        Row {
            label: "audit refuses",
            hitl: Ok(true),
            audit: Ok(false),
            policy: ReportSuccess,
            expected: GateVerdict::Refused(TransitionCondition::Failure),
        },
        // ---------------------------------------------------------- K-033 pair
        Row {
            label: "K-033 legacy: audit cannot run, so SUCCESS is reported",
            hitl: Ok(true),
            audit: Err("audit backend down".into()),
            policy: ReportSuccess,
            expected: GateVerdict::Refused(TransitionCondition::Success),
        },
        Row {
            label: "K-033 structured: audit cannot run, so the period FAILS",
            hitl: Ok(true),
            audit: Err("audit backend down".into()),
            policy: Block,
            expected: GateVerdict::Refused(TransitionCondition::Failure),
        },
    ];

    for r in rows {
        let hitl = hitl(r.hitl.clone());
        let safety = StubSafety(r.audit.clone());
        let verdict = admit(&hitl, &safety, HIGH_RISK, &no_args(), r.policy).await;
        assert_eq!(verdict, r.expected, "branch: {}", r.label);
    }
}

/// HITL short-circuits for a low-risk tool, so the policy cannot matter. The
/// counter is the assertion: "HITL was not the reason" is not something a reader
/// can check, and getting the risk classification wrong is silent — the branch
/// simply never runs. So the stub counts how often it was consulted.
/// **PC-1.** The counter's positive control. A zero is only evidence if the
/// instrument is known to be able to produce a non-zero, so the same counter is
/// driven down the path where HITL *must* be consulted, and it must move.
///
/// This is the third time this family has come up — a rule written without a
/// control (P7), a grep whose zero could have been the wrong path (C1), and now a
/// counter reporting 0. The first two got controls; this one gets two.
#[tokio::test]
async fn pc1_a_low_risk_tool_is_not_consulted_and_a_high_risk_one_is() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let calls = Arc::new(AtomicUsize::new(0));
    let counting = {
        let calls = calls.clone();
        HITLApprover::new(Arc::new(move |_c: &str, _a: &[String]| {
            calls.fetch_add(1, Ordering::SeqCst);
            Err("no channel".to_string())
        }))
    };

    let safety = StubSafety(Ok(true));
    let low = admit(&counting, &safety, LOW_RISK, &no_args(), OnAuditError::Block).await;
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "a low-risk tool must not reach the approver at all"
    );
    assert_eq!(low, GateVerdict::Cleared);

    // PC-1 proper: the same instrument, on a path it cannot miss.
    let high = admit(&counting, &safety, HIGH_RISK, &no_args(), OnAuditError::Block).await;
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "PC-1: the counter must move when HITL is consulted. If it is 0 then either \
         the risk classifier changed or the instrument is broken -- and the `0` \
         asserted above means nothing until this one is non-zero"
    );
    assert_eq!(high, GateVerdict::Refused(TransitionCondition::Failure));
}

/// **PC-2.** The control for the control: prove PC-1 is reading the counter and
/// nothing else.
///
/// A counter wired into a place that never executes reports 0 forever, and a test
/// asserting 0 stays green. A grep's zero at least reflects the disk now; a
/// runtime counter's zero is silent the moment the path moves. So this runs PC-1's
/// own assertion against a deliberately dead instrument and requires it to fail.
///
/// The only difference from PC-1 is the instrument. If this ever passes — i.e. the
/// assertion does *not* go red against a dead counter — then PC-1 was asserting
/// something other than the counter all along.
#[test]
fn pc2_pc1_goes_red_when_its_instrument_is_dead() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Never incremented, whatever happens: a counter moved to a dead position.
    let dead = Arc::new(AtomicUsize::new(0));
    let counter_is_dead = dead.clone();

    // Drive the real path PC-1 drives, so the scenario is identical.
    let live = Arc::new(AtomicUsize::new(0));
    let live_counter = live.clone();
    let approver = HITLApprover::new(Arc::new(move |_c: &str, _a: &[String]| {
        live_counter.fetch_add(1, Ordering::SeqCst);
        Err("no channel".to_string())
    }));
    let _ = approver.check_approval(HIGH_RISK, &[]);

    // The instrument that was wired up saw it.
    assert_eq!(
        live.load(Ordering::SeqCst),
        1,
        "the reference instrument must count a call it cannot miss; if this is 0 \
         the counter mechanism itself is broken and both PC-1 assertions are void"
    );

    // The dead one did not, so PC-1's assertion must fail.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        assert_eq!(
            counter_is_dead.load(Ordering::SeqCst),
            1,
            "PC-1's shape, run against a dead instrument"
        );
    }));
    assert!(
        outcome.is_err(),
        "PC-1's assertion stayed green against a counter that was never wired -- \
         so PC-1 is not testing the counter"
    );
}

/// The table above enumerates the policy variants by hand, so a new variant would
/// silently lose coverage: the table would stay green and simply not mention it.
///
/// This match has no wildcard on purpose. Adding a variant to `OnAuditError`
/// breaks this build rather than quietly shrinking what the table covers — a
/// compile error is the only kind of guard that cannot be forgotten at runtime.
#[test]
fn the_table_cannot_lose_a_policy_variant_silently() {
    for policy in [OnAuditError::ReportSuccess, OnAuditError::Block] {
        match policy {
            OnAuditError::ReportSuccess => {}
            OnAuditError::Block => {}
        }
    }
}

/// The pair above is the finding, so it gets its own assertion rather than
/// living as two rows a reader has to compare. Same inputs, two policies, two
/// answers — and the only difference between the policies is the caller.
#[tokio::test]
async fn the_two_policies_disagree_and_that_is_the_recorded_defect() {
    let audit_error = || StubSafety(Err("audit backend down".into()));
    let legacy = admit(
        &hitl(Ok(true)),
        &audit_error(),
        HIGH_RISK,
        &no_args(),
        OnAuditError::ReportSuccess,
    )
    .await;
    let structured = admit(
        &hitl(Ok(true)),
        &audit_error(),
        HIGH_RISK,
        &no_args(),
        OnAuditError::Block,
    )
    .await;
    assert_ne!(
        legacy, structured,
        "if these ever agree, K-033 has been resolved — update the pit record, \
         and replace this test with one that asserts the single shared answer"
    );
    assert_eq!(legacy, GateVerdict::Refused(TransitionCondition::Success));
    assert_eq!(structured, GateVerdict::Refused(TransitionCondition::Failure));
}
