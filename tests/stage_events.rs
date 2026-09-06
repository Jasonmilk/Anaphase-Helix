// O-2 (ADR-0019): stage event ring — events are the *process*, the ledger is
// the *fact*, evidence is the *support*.
//
// One run_cycle with a wired pipeline must emit the six-stage event trail:
//   stage1 begin/end (parse calls)   stage2 begin/end (tt_job assembly)
//   stage3 begin + per-call end      stage4 begin/end (evidence record)
//   stage5 begin/end (criteria)      stage6 begin/end (verdict ledger)
// seq is monotonic; after(seq) is incremental; the same cycle under the same
// clock is byte-identical (deterministic replay).

mod common;

use anaphase::adapters::*;
use anaphase::events::StageEvent;
use anaphase::ledger::{FakeClock, VerdictStatus};
use anaphase::pipeline::{Pipeline, PipelineConfig};
use anaphase::reflex::ReflexArc;
use anaphase::run_cycle::AgentLoop;
use common::{spawn_mock_tentacle, MockTentacle, StructuredReasoning};
use std::sync::Arc;

async fn build_pipeline(mock: MockTentacle, clock_now: u64) -> Pipeline {
    let (endpoint, _captured, _tx, _handle) = spawn_mock_tentacle(mock).await;
    let tentacle = anaphase::adapters::tentacle::GrpcTentacleAdapter::new(&endpoint)
        .await
        .unwrap();
    let config = PipelineConfig::from_codex("knowledge_base/fixture-codex.json").unwrap();
    Pipeline::new(tentacle, Box::new(FakeClock(clock_now)), config)
}

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

#[tokio::test]
async fn run_cycle_emits_full_six_stage_trail() {
    let output = r#"{"calls":[{"tool":"numbers","args":{},"expect":"numbers"}],"impasse":false}"#;
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0,3.0,4.0]}"#);
    let mut agent = base_agent(Arc::new(StructuredReasoning { output: output.into() }))
        .with_pipeline(build_pipeline(mock, 1000).await);

    let outcome = agent.run_cycle("numbers").await.expect("cycle runs");
    assert!(outcome.done && outcome.success);

    let ring = agent.pipeline.as_ref().expect("pipeline wired").events.lock().unwrap();
    let events: Vec<StageEvent> = ring.events().to_vec();

    // Six stages, all present in order, phases begin/end (+ verdict on 6).
    for stage in 1u8..=6 {
        let begins: Vec<&StageEvent> = events.iter().filter(|e| e.stage == stage && e.phase == "begin").collect();
        let ends: Vec<&StageEvent> = events.iter().filter(|e| e.stage == stage && e.phase == "end").collect();
        assert_eq!(begins.len(), 1, "stage {stage} has one begin");
        assert!(!ends.is_empty(), "stage {stage} has at least one end");
    }
    // stage3 emits one end per executed call.
    let stage3_ends = events.iter().filter(|e| e.stage == 3 && e.phase == "end").count();
    assert_eq!(stage3_ends, 1, "one call -> one stage3 end");
    // stage6 end carries the verdict.
    let stage6_end = events.iter().find(|e| e.stage == 6 && e.phase == "end").expect("stage6 end");
    assert!(stage6_end.detail.contains("Met"), "verdict recorded: {}", stage6_end.detail);
    // trace_id is the derived job id — deterministic, no UUID.
    let trace_ids: std::collections::BTreeSet<&str> = events.iter().map(|e| e.trace_id.as_str()).collect();
    assert_eq!(trace_ids.len(), 1, "one job, one trace id across the trail");
    assert!(!trace_ids.iter().any(|t| t.is_empty()), "trace id is derived, never blank");
    // seq is strictly monotonic.
    for w in events.windows(2) {
        assert!(w[0].seq < w[1].seq, "seq monotonic");
    }
}

#[tokio::test]
async fn incremental_pull_after_seq() {
    let output = r#"{"calls":[{"tool":"numbers","args":{},"expect":"numbers"}],"impasse":false}"#;
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0,3.0]}"#);
    let mut agent = base_agent(Arc::new(StructuredReasoning { output: output.into() }))
        .with_pipeline(build_pipeline(mock, 2000).await);

    agent.run_cycle("numbers").await.expect("cycle runs");

    let ring = agent.pipeline.as_ref().expect("pipeline wired").events.lock().unwrap();
    let all = ring.after(0);
    let last = ring.last_seq();
    assert!(!all.is_empty(), "after(0) returns the stream");
    assert!(ring.after(last).is_empty(), "after(last) is empty — cursor semantics");
    assert_eq!(ring.after(last - 1).len(), 1, "after(last-1) returns exactly the last event");
    let seqs: Vec<u64> = all.iter().map(|e| e.seq).collect();
    assert_eq!(seqs, (1..=last).collect::<Vec<u64>>(), "seqs are 1..=last with no gaps");
}

#[tokio::test]
async fn deterministic_replay_same_clock_same_trail() {
    let output = r#"{"calls":[{"tool":"numbers","args":{},"expect":"numbers"}],"impasse":false}"#;
    let mock_a = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0]}"#);
    let mock_b = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0]}"#);
    let mut a = base_agent(Arc::new(StructuredReasoning { output: output.into() }))
        .with_pipeline(build_pipeline(mock_a, 3000).await);
    let mut b = base_agent(Arc::new(StructuredReasoning { output: output.into() }))
        .with_pipeline(build_pipeline(mock_b, 3000).await);
    a.run_cycle("numbers").await.expect("a runs");
    b.run_cycle("numbers").await.expect("b runs");
    let ea = a.pipeline.as_ref().unwrap().events.lock().unwrap().to_jsonl();
    let eb = b.pipeline.as_ref().unwrap().events.lock().unwrap().to_jsonl();
    assert_eq!(ea, eb, "same input + same clock -> byte-identical event trail");
}

#[tokio::test]
async fn verdict_status_matches_ledger() {
    // A failing fixture (cross_check misses) drives UNMET; the stage6 event
    // must carry the same status the ledger recorded.
    let output = r#"{"calls":[{"tool":"rate","args":{},"expect":"rate"}],"impasse":false}"#;
    let mock = MockTentacle::new().with_tool("rate", r#"{"rates":[0.1]}"#);
    let mut agent = base_agent(Arc::new(StructuredReasoning { output: output.into() }))
        .with_pipeline(build_pipeline(mock, 4000).await);
    agent.run_cycle("rate").await.expect("cycle runs");

    let events = agent.pipeline.as_ref().unwrap().events.lock().unwrap().events().to_vec();
    let stage6_end = events.iter().find(|e| e.stage == 6 && e.phase == "end").expect("stage6 end");
    assert!(stage6_end.detail.contains("Unmet"), "verdict matches: {}", stage6_end.detail);

    let verdicts = agent.pipeline.as_ref().unwrap().ledger.records();
    let last = verdicts.last().expect("ledger has a verdict");
    match last {
        anaphase::ledger::LedgerRecord::Verdict { status: VerdictStatus::Unmet, .. } => {}
        other => panic!("ledger last record should be UNMET, got {other:?}"),
    }
}
