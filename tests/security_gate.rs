//! Security gate wiring tests — ADR-0008 (candidate D'-2).
//!
//! Verifies the pipeline wiring point with a mock gate: Pass / HardOverride
//! proceed; Reject / HitlRequired block the call, short-circuit with an Err,
//! and write a `blocked` ledger record.
//!
//! **B7 changed what `None` means.** It used to be "legacy behaviour" — i.e. no check at
//! all, which is how the production path ran ungated (K-044: `None` read as benign while
//! meaning "no door"). Now **absence is not approval**: with no gate configured, a
//! *high-risk* call is blocked and low-risk calls keep the legacy path. `PermissiveGate`
//! is the explicit, visible opt-out.
//!
//! The mock Tentacle is used only to observe whether a call reached the
//! wire (blocked calls must never reach Tentacle).

use std::collections::BTreeMap;
use std::sync::Arc;

use anaphase::adapters::tentacle::GrpcTentacleAdapter;
use anaphase::ledger::{FakeClock, LedgerRecord};
use anaphase::pipeline::{Pipeline, PipelineConfig, PipelineInput};
use anaphase::security::{GateCheck, GateVerdict, SecurityGate};

mod common;
use common::{spawn_mock_tentacle, MockTentacle};

/// Gate that always returns one verdict.
#[derive(Clone)]
struct FixedGate(GateVerdict);

#[async_trait::async_trait]
impl SecurityGate for FixedGate {
    async fn check(&self, _check: &GateCheck) -> GateVerdict {
        self.0.clone()
    }
}

fn input() -> PipelineInput {
    PipelineInput {
        job_id: "tt_job-gate".into(),
        created_at: "2026-09-06T00:00:00Z".into(),
        llm_content: r#"{"calls":[{"tool":"numbers","args":{},"expect":"numbers"}]}"#.into(),
        identity_labels: BTreeMap::new(),
    }
}

/// A call the local classifier marks high-risk. B7 uses that classifier deliberately:
/// it is the *weakest* available judgement (B4 records how weak), so it under-blocks
/// rather than over-blocks. If it ever stops calling `rm` high-risk, the B7 tests below
/// fail — which is the point of routing them through the real classifier.
///
/// `expect` stays inside the grammar's enum (`ok`); only `tool` is free-form, and `tool`
/// is what the classifier reads. (`expect` rejecting an unknown variant is what told me
/// this — the field that is an enum is not the field that carries the risk.)
fn high_risk_input() -> PipelineInput {
    PipelineInput {
        llm_content: r#"{"calls":[{"tool":"rm","args":{},"expect":"ok"}]}"#.into(),
        ..input()
    }
}

async fn wired(mock: MockTentacle) -> Pipeline {
    let (endpoint, _captured, shutdown_tx, handle) = spawn_mock_tentacle(mock).await;
    let config = PipelineConfig::from_codex("knowledge_base/fixture-codex.json").unwrap();
    let pipeline = Pipeline::new(
        GrpcTentacleAdapter::new(&endpoint).await.unwrap(),
        Box::new(FakeClock(1000)),
        config,
    );
    // Keep the server alive for the lifetime of the test.
    std::mem::forget((shutdown_tx, handle));
    pipeline
}

#[tokio::test]
async fn no_gate_is_legacy_compatible() {
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0,3.0,4.0]}"#);
    let pipeline = wired(mock).await;
    let mut pipeline = pipeline.with_security_gate(None);
    let outcome = pipeline.run(input()).await.unwrap();
    assert_eq!(outcome.job_id, "tt_job-gate");
    // Legacy path: a verdict, never a blocked record.
    assert!(pipeline.ledger.records().iter().all(|r| matches!(r, LedgerRecord::Verdict { .. })));
}

/// B7. **Absence of a gate is not approval.** With no door-keeper configured, a
/// high-risk call must be blocked — that is the whole difference between "we have an
/// adapter" and "the pipeline is gated".
///
/// Non-vacuity: the mock **does** serve `rm`, so a regression that lets high-risk calls
/// through fails here by succeeding, instead of erroring for some duller reason.
// guards: no-gate-blocks-high-risk
#[tokio::test]
async fn no_gate_blocks_a_high_risk_call() {
    let mock = MockTentacle::new().with_tool("rm", r#"{"ok":true}"#);
    let pipeline = wired(mock).await;
    let mut pipeline = pipeline.with_security_gate(None);
    let err = pipeline
        .run(high_risk_input())
        .await
        .expect_err("a high-risk call must not proceed when no gate is configured");
    assert!(err.contains("blocked by security gate"), "{err}");
    assert!(
        pipeline
            .ledger
            .records()
            .iter()
            .any(|r| matches!(r, LedgerRecord::Blocked { .. })),
        "the block must be recorded in the ledger, not merely returned as an Err"
    );
}

/// The other half of B7: the legacy promise is **narrowed, not revoked**. A low-risk
/// call still runs with no gate configured, so this is not a blanket fail-closed that
/// would break every existing deployment.
#[tokio::test]
async fn no_gate_still_allows_a_low_risk_call() {
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0]}"#);
    let pipeline = wired(mock).await;
    let mut pipeline = pipeline.with_security_gate(None);
    pipeline
        .run(input())
        .await
        .expect("low-risk calls keep the legacy path");
    assert!(
        pipeline
            .ledger
            .records()
            .iter()
            .all(|r| !matches!(r, LedgerRecord::Blocked { .. })),
        "a low-risk call must not be recorded as blocked"
    );
}

/// B7's escape hatch, and why it is a hatch rather than a default: installing
/// `PermissiveGate` is an **explicit, visible** decision to run without policy.
#[tokio::test]
async fn permissive_gate_is_the_explicit_opt_out() {
    let mock = MockTentacle::new().with_tool("rm", r#"{"ok":true}"#);
    let pipeline = wired(mock).await;
    let mut pipeline = pipeline
        .with_security_gate(Some(Arc::new(anaphase::security::PermissiveGate)));
    pipeline
        .run(high_risk_input())
        .await
        .expect("an explicitly installed PermissiveGate must permit");
}

#[tokio::test]
async fn pass_gate_proceeds_and_verdicts() {
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0,3.0,4.0]}"#);
    let pipeline = wired(mock).await;
    let mut pipeline = pipeline.with_security_gate(Some(Arc::new(FixedGate(GateVerdict::Pass))));
    let outcome = pipeline.run(input()).await.unwrap();
    assert_eq!(outcome.verdict, anaphase::ledger::VerdictStatus::Met);
    assert!(!pipeline.ledger.records().is_empty());
}

#[tokio::test]
async fn hard_override_proceeds() {
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0,3.0,4.0]}"#);
    let pipeline = wired(mock).await;
    let mut pipeline =
        pipeline.with_security_gate(Some(Arc::new(FixedGate(GateVerdict::HardOverride))));
    let outcome = pipeline.run(input()).await.unwrap();
    assert_eq!(outcome.verdict, anaphase::ledger::VerdictStatus::Met);
}

#[tokio::test]
async fn reject_gate_blocks_and_records_blocked() {
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0,3.0,4.0]}"#);
    let mut pipeline = wired(mock.clone()).await;
    pipeline = pipeline
        .with_security_gate(Some(Arc::new(FixedGate(GateVerdict::Reject(
            "policy: catastrophic".into(),
        )))));

    let err = pipeline.run(input()).await.unwrap_err();
    assert!(err.contains("blocked by security gate"), "err = {err}");

    // The call never reached the wire.
    assert!(mock.captured_trace_ids.all().is_empty());

    // The ledger carries exactly one `blocked` record.
    let blocked: Vec<_> = pipeline
        .ledger
        .records()
        .iter()
        .filter(|r| matches!(r, LedgerRecord::Blocked { .. }))
        .collect();
    assert_eq!(blocked.len(), 1, "expected one blocked record");
    match blocked[0] {
        LedgerRecord::Blocked { job_id, tool, index, reason, .. } => {
            assert_eq!(job_id, "tt_job-gate");
            assert_eq!(tool, "numbers");
            assert_eq!(*index, 0);
            assert_eq!(reason, "policy: catastrophic");
        }
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn hitl_required_blocks_and_records_blocked() {
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0,3.0,4.0]}"#);
    let mut pipeline = wired(mock.clone()).await;
    pipeline = pipeline.with_security_gate(Some(Arc::new(FixedGate(
        GateVerdict::HitlRequired("human confirmation required".into()),
    ))));

    let err = pipeline.run(input()).await.unwrap_err();
    assert!(err.contains("human confirmation required"), "err = {err}");
    assert!(mock.captured_trace_ids.all().is_empty());

    let blocked = pipeline
        .ledger
        .records()
        .iter()
        .filter(|r| matches!(r, LedgerRecord::Blocked { .. }))
        .count();
    assert_eq!(blocked, 1);
}

#[tokio::test]
async fn gate_check_carries_full_facts() {
    // The GateCheck handed to the gate must carry job/index/tool/args/labels
    // so the gate implementation can audit and decide on real facts.
    let (tx, rx) = tokio::sync::oneshot::channel();
    let gate = FactCapturingGate::new(tx);
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0,3.0,4.0]}"#);
    let pipeline = wired(mock).await;
    let mut pipeline = pipeline.with_security_gate(Some(Arc::new(gate)));

    let mut labels = BTreeMap::new();
    labels.insert("identity".to_string(), "anaphase_test".to_string());
    let mut input = input();
    input.identity_labels = labels;

    let _ = pipeline.run(input).await;
    let check = rx.await.unwrap();
    assert_eq!(check.job_id, "tt_job-gate");
    assert_eq!(check.index, 0);
    assert_eq!(check.tool, "numbers");
    assert_eq!(check.identity_labels.get("identity").map(|s| s.as_str()), Some("anaphase_test"));
}

/// Captures the last GateCheck it saw.
struct FactCapturingGate(std::sync::Mutex<Option<tokio::sync::oneshot::Sender<GateCheck>>>);

impl FactCapturingGate {
    fn new(tx: tokio::sync::oneshot::Sender<GateCheck>) -> Self {
        Self(std::sync::Mutex::new(Some(tx)))
    }
}

#[async_trait::async_trait]
impl SecurityGate for FactCapturingGate {
    async fn check(&self, check: &GateCheck) -> GateVerdict {
        if let Some(tx) = self.0.lock().unwrap().take() {
            let _ = tx.send(check.clone());
        }
        GateVerdict::Pass
    }
}
