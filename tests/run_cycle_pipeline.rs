// Candidate E (ADR-0005): run_cycle <-> deterministic pipeline full merge.
//
// The cognitive states now consume the six pipeline stages:
//   Reasoning  -> stage 1 (parse structured calls) + stage 2 (tt_job envelope)
//   Execution  -> stage 3 (gRPC execute) + stage 4 (evidence record)
//   Reflection -> stage 5 (criteria check) + stage 6 (verdict ledger)
//
// All tests run against the shared MockTentacle (deterministic; no real binary
// or node required). Backwards compatibility (legacy echo path without a
// pipeline) stays covered by integration_test.rs.
//
// Acceptance criteria (task book section 4.4):
//   1. one run_cycle loop walks the whole pipeline chain (verified here)
//   2. no contains("tool_call") string matching remains (see src/)
//   3. the five historical hardcodings are all config-sourced (run_config tests)
//   4. existing 78 tests stay green + these new tests are green

mod common;

use anaphase::adapters::*;
use anaphase::agent_loop::AgentLoop;
use anaphase::config::RunCycleConfig;
use anaphase::ledger::{FakeClock, LedgerRecord, VerdictStatus};
use anaphase::pipeline::{Pipeline, PipelineConfig};
use anaphase::reflex::ReflexArc;
use anaphase::states::HelixState;
use common::{spawn_mock_tentacle, MockTentacle, StructuredReasoning};
use std::sync::{Arc, Mutex};

/// Build a pipeline wired to a fresh MockTentacle with an injected clock.
async fn build_pipeline(mock: MockTentacle, clock_now: u64) -> Pipeline {
    let (endpoint, _captured, _tx, _handle) = spawn_mock_tentacle(mock).await;
    let tentacle = anaphase::adapters::tentacle::GrpcTentacleAdapter::new(&endpoint)
        .await
        .unwrap();
    let config = PipelineConfig::from_codex("knowledge_base/fixture-codex.json").unwrap();
    Pipeline::new(tentacle, Box::new(FakeClock(clock_now)), config)
}

/// Agent with default adapters + a structured reasoning stub.
fn base_agent(reason: Arc<dyn ReasoningAdapter>) -> AgentLoop {
    AgentLoop::new(
        Arc::new(NoopMemoryAdapter),
        reason,
        Arc::new(NoopToolAdapter),
        Arc::new(NoopSafetyAdapter),
        Arc::new(NoopUiAdapter),
        Arc::new(NoopFearAdapter),
        ReflexArc { safety_rules: vec![] },
    )
}

// ── full six-stage chain ───────────────────────────────────────────

#[tokio::test]
async fn run_cycle_full_chain_met() {
    let output = r#"{"calls":[{"tool":"numbers","args":{},"expect":"numbers"}],"impasse":false}"#;
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0,3.0,4.0]}"#);
    let mut agent = base_agent(Arc::new(StructuredReasoning { output: output.into() }))
        .with_pipeline(build_pipeline(mock, 1000).await);

    agent.run_cycle("calculate").await.unwrap();

    // stage 1-2: structured plan parsed, deterministic envelope assembled.
    assert_eq!(agent.context.calls.len(), 1);
    assert_eq!(agent.context.calls[0].tool, "numbers");
    let job = agent.context.job.as_ref().expect("envelope assembled");
    assert_eq!(job.calls.len(), 1);
    assert!(job.job_id.starts_with("run-"), "derived job id: {}", job.job_id);
    assert_eq!(job.created_at, "1970-01-01T00:16:40Z", "clock 1000 -> RFC3339");

    // stage 3-4: executed via the pipeline, evidence recorded.
    assert_eq!(agent.context.evidence.len(), 1);
    assert!(agent.context.evidence[0].ok);

    // stage 5-6: criteria MET verdict written to the ledger.
    let records = agent.pipeline.as_ref().unwrap().ledger.records();
    assert_eq!(records.len(), 1);
    match &records[0] {
        LedgerRecord::Verdict { status: VerdictStatus::Met, job_id, .. } => {
            assert_eq!(job_id, &job.job_id);
        }
        other => panic!("unexpected record: {other:?}"),
    }
    assert_eq!(agent.current_state, HelixState::Perception, "cycle returns home");
}

#[tokio::test]
async fn run_cycle_full_chain_unmet() {
    let output = r#"{"calls":[{"tool":"numbers","args":{},"expect":"numbers"}]}"#;
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0]}"#);
    let mut agent = base_agent(Arc::new(StructuredReasoning { output: output.into() }))
        .with_pipeline(build_pipeline(mock, 1000).await);

    agent.run_cycle("calculate").await.unwrap();

    let records = agent.pipeline.as_ref().unwrap().ledger.records();
    assert_eq!(records.len(), 1);
    match &records[0] {
        LedgerRecord::Verdict { status: VerdictStatus::Unmet, retry_due: Some(due), .. } => {
            assert_eq!(*due, 4600, "clock 1000 + base_delay 3600 from codex");
        }
        other => panic!("unexpected record: {other:?}"),
    }
    assert!(agent.context.evidence[0].ok, "execution itself succeeded");
}

#[tokio::test]
async fn run_cycle_unstructured_output_skips_pipeline() {
    let mut agent = base_agent(Arc::new(StructuredReasoning { output: "no plan here".into() }))
        .with_pipeline(build_pipeline(MockTentacle::new(), 1000).await);

    agent.run_cycle("hi").await.unwrap();

    assert!(agent.context.calls.is_empty());
    assert!(agent.context.evidence.is_empty());
    assert!(agent.context.job.is_none());
    assert!(
        agent.pipeline.as_ref().unwrap().ledger.records().is_empty(),
        "no plan -> no ledger record"
    );
}

#[tokio::test]
async fn run_cycle_deterministic_replay() {
    // Same input + same clock + same mock -> byte-identical ledger output.
    let output = r#"{"calls":[{"tool":"rate","args":{},"expect":"rate"}]}"#;
    let mk_tentacle =
        || MockTentacle::new().with_tool("rate", r#"{"numerator":10.0,"denominator":10.0}"#);

    let mut a = base_agent(Arc::new(StructuredReasoning { output: output.into() }))
        .with_pipeline(build_pipeline(mk_tentacle(), 777).await);
    a.run_cycle("compute ratio").await.unwrap();

    let mut b = base_agent(Arc::new(StructuredReasoning { output: output.into() }))
        .with_pipeline(build_pipeline(mk_tentacle(), 777).await);
    b.run_cycle("compute ratio").await.unwrap();

    assert_eq!(
        a.pipeline.as_ref().unwrap().ledger.to_jsonl(),
        b.pipeline.as_ref().unwrap().ledger.to_jsonl(),
        "same input + same clock + same mock -> byte-identical ledger"
    );
}

// ── E-T6: the five historical hardcodings are config-sourced ────────

#[tokio::test]
async fn run_config_cycle_cap_stops_early() {
    // cap=1: the loop returns after the first state, never reaching Perception.
    let mut agent = base_agent(Arc::new(NoopReasoningAdapter))
        .with_run_config(RunCycleConfig { cycle_cap: 1, ..RunCycleConfig::default() });
    agent.run_cycle("hello").await.unwrap();
    assert_ne!(
        agent.current_state,
        HelixState::Perception,
        "cap from config cuts the cycle short"
    );

    // default cap=7 lets a full cycle finish.
    let mut full = base_agent(Arc::new(NoopReasoningAdapter));
    full.run_cycle("hello").await.unwrap();
    assert_eq!(full.current_state, HelixState::Perception);
}

struct StubFear(pub f64);

#[async_trait::async_trait]
impl FearAdapter for StubFear {
    async fn predict_death(&self, _context: &str) -> Result<f64, String> {
        Ok(self.0)
    }
}

#[tokio::test]
async fn run_config_soft_reflex_threshold_blocks() {
    let output = r#"{"calls":[{"tool":"numbers","args":{},"expect":"numbers"}]}"#;

    // Default threshold 0.7: p_death 0.8 blocks before execution.
    let mut blocked = AgentLoop::new(
        Arc::new(NoopMemoryAdapter),
        Arc::new(StructuredReasoning { output: output.into() }),
        Arc::new(NoopToolAdapter),
        Arc::new(NoopSafetyAdapter),
        Arc::new(NoopUiAdapter),
        Arc::new(StubFear(0.8)),
        ReflexArc { safety_rules: vec![] },
    )
    .with_pipeline(build_pipeline(MockTentacle::new(), 1000).await);
    blocked.run_cycle("do it").await.unwrap();
    assert!(blocked.context.evidence.is_empty(), "blocked before execution");
    assert!(blocked.pipeline.as_ref().unwrap().ledger.records().is_empty());

    // Raised threshold 0.9: the same p_death passes and executes.
    let mut passed = AgentLoop::new(
        Arc::new(NoopMemoryAdapter),
        Arc::new(StructuredReasoning { output: output.into() }),
        Arc::new(NoopToolAdapter),
        Arc::new(NoopSafetyAdapter),
        Arc::new(NoopUiAdapter),
        Arc::new(StubFear(0.8)),
        ReflexArc { safety_rules: vec![] },
    )
    .with_run_config(RunCycleConfig { soft_reflex_threshold: 0.9, ..RunCycleConfig::default() })
    .with_pipeline(
        build_pipeline(MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0,3.0]}"#), 1000)
            .await,
    );
    passed.run_cycle("do it").await.unwrap();
    assert_eq!(passed.context.evidence.len(), 1, "threshold raised -> executes");
}

/// Reasoning stub that records the mode label it was called with.
struct ModeRecordingReasoning {
    pub seen_mode: Arc<Mutex<Option<String>>>,
}

#[async_trait::async_trait]
impl ReasoningAdapter for ModeRecordingReasoning {
    async fn reason(&self, _prompt: &str, mode: &str) -> Result<String, String> {
        *self.seen_mode.lock().unwrap() = Some(mode.to_string());
        Ok("no plan".to_string())
    }
}

#[tokio::test]
async fn run_config_amygdala_and_reasoning_mode() {
    let seen = Arc::new(Mutex::new(None));
    let mut agent = AgentLoop::new(
        Arc::new(NoopMemoryAdapter),
        Arc::new(ModeRecordingReasoning { seen_mode: seen.clone() }),
        Arc::new(NoopToolAdapter),
        Arc::new(NoopSafetyAdapter),
        Arc::new(NoopUiAdapter),
        Arc::new(NoopFearAdapter),
        ReflexArc { safety_rules: vec![] },
    )
    .with_run_config(RunCycleConfig {
        amygdala_default_vector: (0.5, 0.4, 0.3),
        reasoning_mode: "test_mode".into(),
        ..RunCycleConfig::default()
    });

    agent.run_cycle("hello").await.unwrap();
    assert_eq!(agent.context.amygdala_vector, (0.5, 0.4, 0.3), "vector from config");
    assert_eq!(
        *seen.lock().unwrap(),
        Some("test_mode".to_string()),
        "mode from config"
    );
}

/// Tool stub that records the command it was dispatched.
struct RecordingTool {
    pub last_command: Arc<Mutex<Option<String>>>,
}

#[async_trait::async_trait]
impl ToolAdapter for RecordingTool {
    async fn execute(&self, command: &str, _args: &[String]) -> Result<String, String> {
        *self.last_command.lock().unwrap() = Some(command.to_string());
        Ok(format!("executed: {command}"))
    }
    async fn perceive(&self, _query: &str) -> Result<String, String> {
        Ok("perceived".to_string())
    }
}

#[tokio::test]
async fn run_config_execution_placeholder() {
    // No pipeline, no tool_command: the placeholder comes from config.
    let recorded = Arc::new(Mutex::new(None));
    let mut agent = AgentLoop::new(
        Arc::new(NoopMemoryAdapter),
        Arc::new(StructuredReasoning {
            output: r#"{"calls":[{"tool":"numbers","args":{},"expect":"numbers"}]}"#.into(),
        }),
        Arc::new(RecordingTool { last_command: recorded.clone() }),
        Arc::new(NoopSafetyAdapter),
        Arc::new(NoopUiAdapter),
        Arc::new(NoopFearAdapter),
        ReflexArc { safety_rules: vec![] },
    )
    .with_run_config(RunCycleConfig {
        execution_placeholder: "custom".into(),
        ..RunCycleConfig::default()
    });

    agent.run_cycle("hello").await.unwrap();
    assert_eq!(
        *recorded.lock().unwrap(),
        Some("custom".to_string()),
        "placeholder from config"
    );
}
