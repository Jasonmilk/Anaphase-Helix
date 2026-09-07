//! Reasoning body trace (Engram join): the round trip is appended
//! redacted + truncated, keyed by the derived job id — joinable with the
//! Tuck audit chain and the ledger in Cellrix's Engram view.

use std::sync::Arc;

use anaphase::adapters::*;
use anaphase::reflex::ReflexArc;
use anaphase::run_cycle::AgentLoop;
use anaphase::trace::{ReasoningTrace, Redaction};

async fn build_agent() -> AgentLoop {
    let memory: Arc<dyn MemoryAdapter> = Arc::new(NoopMemoryAdapter);
    let reason: Arc<dyn ReasoningAdapter> = Arc::new(NoopReasoningAdapter);
    let tool: Arc<dyn ToolAdapter> = Arc::new(NoopToolAdapter);
    let safety: Arc<dyn SafetyAdapter> = Arc::new(NoopSafetyAdapter);
    let ui: Arc<dyn UiAdapter> = Arc::new(NoopUiAdapter);
    let fear: Arc<dyn FearAdapter> = Arc::new(NoopFearAdapter);
    let reflex = ReflexArc {
        safety_rules: vec!["rm -rf /".to_string()],
    };
    AgentLoop::new(memory, reason, tool, safety, ui, fear, reflex)
}

#[tokio::test]
async fn reasoning_round_trip_is_recorded_redacted() {
    let dir = std::env::temp_dir().join(format!("anaphase-trace-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("reasoning.jsonl");
    let _ = std::fs::remove_file(&path);

    let mut agent = build_agent().await;
    agent.trace = Some(
        ReasoningTrace::open(path.clone(), 4096, Redaction::default()).unwrap(),
    );

    let input = "what is 2+2 using my key sk-4056aabbccddeeff0011";
    let _ = agent.run_cycle(input).await;

    let content = std::fs::read_to_string(&path).expect("trace file must exist");
    let entry: anaphase::trace::ReasoningEntry =
        serde_json::from_str(content.trim()).unwrap();

    // The trace id is the derived job id — the join key with chain + ledger.
    assert_eq!(
        entry.trace_id,
        anaphase::contract::derive_job_id(input),
        "join key mismatch"
    );
    assert_eq!(entry.seq, 0);
    // Credentials never touch disk — the 2026-09-07 audit's lesson.
    assert!(
        !content.contains("sk-4056"),
        "credential leaked to trace: {content}"
    );
    assert!(content.contains("[REDACTED]"));
    // The model name is carried (which model produced this round).
    assert!(!entry.model.is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn trace_off_by_default() {
    let mut agent = build_agent().await;
    // Default: trace is None — nothing is recorded, nothing leaks.
    assert!(agent.trace.is_none());
    let _ = agent.run_cycle("hello").await;
}
