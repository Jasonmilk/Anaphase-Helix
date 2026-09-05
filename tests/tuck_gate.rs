//! Real Tuck security gate connectivity — ADR-0008 (candidate D'-2).
//!
//! Proves the closed control loop Anaphase pipeline → TuckSecurityGate:
//! a Low-risk PFP passes and the call executes; a Catastrophic PFP is
//! rejected, the call never reaches Tentacle, and a `blocked` record lands
//! in the ledger. trace_id is derived deterministically via Uuid v5
//! (name-based, not random v4) so the gate request sequence is replayable.
//!
//! tuck-core is a dev-only dependency — the anaphase library itself stays
//! decoupled (ADR-0008 D2).

use std::collections::BTreeMap;
use std::sync::Arc;

use anaphase::adapters::tentacle::GrpcTentacleAdapter;
use anaphase::ledger::{FakeClock, LedgerRecord};
use anaphase::pipeline::{Pipeline, PipelineConfig, PipelineInput};
use anaphase::security::{GateCheck, GateVerdict, SecurityGate};

use tuck_core::anaphase_bridge::{
    GateDecision, SecurityGateRequest, TuckSecurityGate,
};
use tuck_core::credential::InMemoryCredentialStore;
use tuck_core::{Modality, OutputDest, OverrideFlag, ReplayEnable, RiskLevel, SecurityPolicy};
use uuid::Uuid;

mod common;
use common::{spawn_mock_tentacle, MockTentacle};

/// Build a 4-byte PFP header (CI-144 physical fact) from explicit parts.
fn pfp_bytes(risk: RiskLevel) -> [u8; 4] {
    let mut bytes = [0xCF, 0x14, 0, 0];
    bytes[2] = (Modality::Executive as u8) | ((risk as u8) << 2);
    bytes[3] = (OutputDest::External as u8)
        | ((OverrideFlag::Normal as u8) << 1)
        | ((ReplayEnable::Enabled as u8) << 2);
    bytes
}

/// Adapter that maps Anaphase's GateCheck onto Tuck's SecurityGateRequest.
///
/// Test-layer adapter (ADR-0008 D2: adapters live outside the library).
/// PFP risk is chosen by an explicit fixture table — a test-only source of
/// truth, not runtime hardcoding.
struct TuckGateAdapter {
    gate: tokio::sync::Mutex<TuckSecurityGate<InMemoryCredentialStore>>,
}

impl TuckGateAdapter {
    fn new(policy: SecurityPolicy) -> Self {
        Self {
            gate: tokio::sync::Mutex::new(TuckSecurityGate::new(
                policy,
                InMemoryCredentialStore::new(),
                "anaphase_test",
            )),
        }
    }

    /// Pure mapping from Tuck's gate decision to Anaphase's verdict
    /// (test-visible so the mapping layer is pinned without a full request).
    fn map_decision(decision: GateDecision) -> GateVerdict {
        match decision {
            GateDecision::Pass => GateVerdict::Pass,
            GateDecision::Reject => {
                GateVerdict::Reject("Security policy rejected this action".to_string())
            }
            GateDecision::HitlRequired => {
                GateVerdict::HitlRequired("human confirmation required (HITL gate)".to_string())
            }
            GateDecision::HardOverride => GateVerdict::HardOverride,
        }
    }

    fn risk_for(tool: &str) -> RiskLevel {
        match tool {
            // Fixture table: numbers is a read-only query -> Low.
            "numbers" => RiskLevel::Low,
            // danger_* marks catastrophic actions in this test fixture.
            _ if tool.starts_with("danger_") => RiskLevel::Catastrophic,
            _ => RiskLevel::Low,
        }
    }
}

#[async_trait::async_trait]
impl SecurityGate for TuckGateAdapter {
    async fn check(&self, check: &GateCheck) -> GateVerdict {
        // Deterministic trace: Uuid v5 over the same `{job_id}#{index}` string
        // the pipeline uses for Tentacle trace ids (ADR-0003).
        let trace = Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            format!("{}#{}", check.job_id, check.index).as_bytes(),
        );
        let request = SecurityGateRequest {
            trace_id: trace,
            source_id: "anaphase".to_string(),
            pfp: pfp_bytes(Self::risk_for(&check.tool)),
            sap: None,
            identity_label: check.identity_labels.get("identity").cloned(),
            action_type: "executive".to_string(),
            action_description: format!("{} {}", check.tool, check.args_json),
            injection_target: None,
        };
        let response = self.gate.lock().await.process(&request).await;
        Self::map_decision(response.decision)
    }
}

fn input(job_id: &str, tool: &str) -> PipelineInput {
    let llm_content = format!(
        r#"{{"calls":[{{"tool":"{tool}","args":{{}},"expect":"numbers"}}]}}"#
    );
    PipelineInput {
        job_id: job_id.into(),
        created_at: "2026-09-06T00:00:00Z".into(),
        llm_content,
        identity_labels: BTreeMap::new(),
    }
}

async fn wired(mock: MockTentacle, policy: SecurityPolicy) -> Pipeline {
    let (endpoint, _captured, shutdown_tx, handle) = spawn_mock_tentacle(mock).await;
    let config = PipelineConfig::from_codex("knowledge_base/fixture-codex.json").unwrap();
    let pipeline = Pipeline::new(
        GrpcTentacleAdapter::new(&endpoint).await.unwrap(),
        Box::new(FakeClock(1000)),
        config,
    )
    .with_security_gate(Some(Arc::new(TuckGateAdapter::new(policy))));
    // Keep the mock server alive for the lifetime of the test.
    std::mem::forget((shutdown_tx, handle));
    pipeline
}

#[tokio::test]
async fn tuck_low_risk_passes_and_executes() {
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0,3.0,4.0]}"#);
    let mut pipeline = wired(mock.clone(), SecurityPolicy::default()).await;

    let outcome = pipeline.run(input("tt_job-tuck-pass", "numbers")).await.unwrap();
    assert_eq!(outcome.verdict, anaphase::ledger::VerdictStatus::Met);

    // The call reached Tentacle.
    assert_eq!(mock.captured_trace_ids.all().len(), 1);
    // No blocked record — the whole ledger is a normal verdict.
    assert!(pipeline
        .ledger
        .records()
        .iter()
        .all(|r| matches!(r, LedgerRecord::Verdict { .. })));
}

#[tokio::test]
async fn tuck_catastrophic_risk_blocks_and_records_blocked() {
    let mock = MockTentacle::new().with_tool("danger_del", r#"{"series":[1.0]}"#);
    let mut pipeline = wired(mock.clone(), SecurityPolicy::default()).await;

    let err = pipeline.run(input("tt_job-tuck-reject", "danger_del")).await.unwrap_err();
    assert!(err.contains("blocked by security gate"), "err = {err}");

    // The call never reached Tentacle.
    assert!(mock.captured_trace_ids.all().is_empty());

    // Ledger carries exactly one blocked record with Tuck's reject reason.
    let blocked: Vec<_> = pipeline
        .ledger
        .records()
        .iter()
        .filter(|r| matches!(r, LedgerRecord::Blocked { .. }))
        .collect();
    assert_eq!(blocked.len(), 1);
}

#[tokio::test]
async fn tuck_critical_risk_escalates_to_hitl() {
    // Policy: critical -> NeedHumanConfirm (Tuck default). A critical PFP
    // must surface as HitlRequired on the Anaphase side, through the
    // adapter's verdict mapping.
    let mut gate = TuckSecurityGate::new(
        SecurityPolicy::default(),
        InMemoryCredentialStore::new(),
        "anaphase_test",
    );
    let request = SecurityGateRequest {
        trace_id: Uuid::new_v5(&Uuid::NAMESPACE_OID, b"tt_job-tuck-hitl#0"),
        source_id: "anaphase".to_string(),
        pfp: pfp_bytes(RiskLevel::Critical),
        sap: None,
        identity_label: None,
        action_type: "executive".to_string(),
        action_description: "critical probe".to_string(),
        injection_target: None,
    };
    let response = gate.process(&request).await;
    assert_eq!(response.decision, GateDecision::HitlRequired);

    // Mapping layer: GateDecision -> GateVerdict.
    let verdict = TuckGateAdapter::map_decision(GateDecision::HitlRequired);
    assert_eq!(verdict, GateVerdict::HitlRequired("human confirmation required (HITL gate)".into()));
}
