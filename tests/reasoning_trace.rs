//! Reasoning body trace (ProveTrack join): the round trip is appended
//! redacted + truncated, keyed by THIS PERIOD'S allocated id (`ADR-0048` D1, B18) —
//! joinable with the Tuck audit chain and the ledger in Cellrix's ProveTrack view.
//! It used to be keyed by the derived job id; that is an input digest, so two runs of
//! one input shared a key (measured 2026-10-09: 462 periods onto 56 digests, 219:1 worst).

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

    // ADR-0048 D1 (B18): the join key is the ALLOCATED period id, not the input digest.
    // Under the digest, two runs of one input shared a key — measured on the live store
    // (M0.5): 462 periods came from 56 digests, one of them covering 219 runs.
    assert!(
        anaphase::session_events::is_period_id(&entry.trace_id),
        "the join key must be an allocated period id: {}",
        entry.trace_id
    );
    assert_eq!(
        Some(&entry.trace_id),
        agent.context.period_id.as_ref(),
        "the body trace must key on THIS period's identity"
    );
    assert_ne!(
        entry.trace_id,
        anaphase::contract::derive_job_id(input),
        "the input digest is the OLD key and must not be used"
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
