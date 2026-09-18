//! A whole-cycle snapshot: one deterministic `run_cycle`, pinned by its output.
//!
//! Why this exists before the file is split: `run_cycle.rs` is the largest file
//! in the repo and the one about to be moved. A move touches every line,
//! including the lines no test asserts, so the suite passing is the weakest
//! possible evidence. This pins what the cycle actually produces.
//!
//! Feasibility was checked first, because `session_events` succeeded for a reason
//! that might not generalise. It does generalise here, and by more than expected:
//!
//!   * the clock is injected (`AgentLoop::with_clock`, `FakeClock`),
//!   * `rand::`, `SystemTime`, `Instant`, `Uuid`, `thread_rng` and `Utc::now` have
//!     zero occurrences in run_cycle.rs,
//!   * the state-transition `HashMap` is built at construction and never
//!     iterated for output,
//!   * all seven external dependencies are trait objects, and the suite already
//!     has no-op doubles for every one of them.
//!
//! So a whole cycle is reproducible, not just a slice of one. That is worth
//! stating because the assumption going in was the opposite.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anaphase::adapters::{
    MemoryAdapter, NoopFearAdapter, NoopMemoryAdapter, NoopReasoningAdapter, NoopSafetyAdapter,
    NoopToolAdapter, NoopUiAdapter, QueryResult, ReasoningAdapter, ToolAdapter, UiAdapter,
};
use anaphase::ledger::FakeClock;
use anaphase::reflex::ReflexArc;
use anaphase::run_cycle::AgentLoop;
use async_trait::async_trait;

const GOLDEN: &str = "tests/golden/run_cycle_summary.txt";

/// A fixed instant, so every timestamp in the output is a constant.
const FIXED_NOW: u64 = 1_700_000_000;

/// Reasoning that returns the same tool plan every time. No network, no model.
struct FixedPlanReasoning;

#[async_trait]
impl ReasoningAdapter for FixedPlanReasoning {
    async fn reason(&self, _prompt: &str, _model: &str, _trace_id: &str) -> Result<String, String> {
        Ok(r#"{"calls":[{"tool":"numbers","args":{"expression":"7**9"},"expect":"numbers"}]}"#
            .to_string())
    }
}

/// A tool that echoes a fixed result, so tool output is a constant too.
struct FixedTool;

#[async_trait]
impl ToolAdapter for FixedTool {
    async fn execute(&self, _command: &str, _args: &[String]) -> Result<String, String> {
        Ok("40353607".to_string())
    }
    async fn perceive(&self, _query: &str) -> Result<String, String> {
        Ok("40353607".to_string())
    }
}

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rc-snapshot-{}-{}", std::process::id(), tag));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

/// Build one agent whose every input is fixed.
fn fixed_agent(events_dir: &Path) -> AgentLoop {
    let memory: Arc<dyn MemoryAdapter> = Arc::new(NoopMemoryAdapter);
    let reason: Arc<dyn ReasoningAdapter> = Arc::new(FixedPlanReasoning);
    let tool: Arc<dyn ToolAdapter> = Arc::new(FixedTool);
    let safety: Arc<dyn anaphase::adapters::SafetyAdapter> = Arc::new(NoopSafetyAdapter);
    let ui: Arc<dyn UiAdapter> = Arc::new(NoopUiAdapter);
    let fear: Arc<dyn anaphase::adapters::FearAdapter> = Arc::new(NoopFearAdapter);
    let mut agent = AgentLoop::new(
        memory,
        reason,
        tool,
        safety,
        ui,
        fear,
        ReflexArc {
            safety_rules: vec![],
        },
    )
    .with_clock(Arc::new(FakeClock(FIXED_NOW)));
    agent.session_events_dir = Some(events_dir.to_path_buf());
    agent
}

/// A stable, readable rendering of what one cycle produced.
///
/// Deliberately NOT a dump of internal structures: a snapshot that prints
/// everything breaks on any field rename, which teaches people to regenerate it
/// without reading. These are the outputs a caller can observe.
async fn run_and_render(events_dir: &Path) -> String {
    let mut agent = fixed_agent(events_dir);
    let outcome = agent
        .run_cycle("用计算器算 7 的 9 次方")
        .await
        .expect("the fixed cycle must not fail");

    let mut out = String::new();
    out.push_str(&format!("done = {}\n", outcome.done));
    out.push_str(&format!("success = {}\n", outcome.success));
    out.push_str(&format!("impasse = {}\n", outcome.impasse));
    out.push_str(&format!("current_state = {:?}\n", agent.current_state));

    // The session-event stream, if it was recorded.
    let mut rows: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(events_dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("jsonl") {
                if let Ok(text) = fs::read_to_string(&p) {
                    for line in text.lines().filter(|l| !l.trim().is_empty()) {
                        // Drop the allocated identity: it is per-run by design
                        // (B15), so including it would make the snapshot differ
                        // on every run and tempt someone to derive it again.
                        let mut v: serde_json::Value = serde_json::from_str(line).unwrap();
                        if let Some(o) = v.as_object_mut() {
                            o.remove("period_id");
                        }
                        rows.push(serde_json::to_string(&v).unwrap());
                    }
                }
            }
        }
    }
    rows.sort();
    out.push_str(&format!("event_rows = {}\n", rows.len()));
    for r in &rows {
        out.push_str("  ");
        out.push_str(r);
        out.push('\n');
    }
    out
}

fn golden_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(GOLDEN)
}

#[tokio::test]
async fn a_whole_cycle_matches_the_snapshot() {
    let dir = scratch("fixed");
    let produced = run_and_render(&dir).await;
    let path = golden_path();
    if std::env::var("REGEN_SNAPSHOT").is_ok() {
        fs::write(&path, &produced).unwrap();
        eprintln!("regenerated {}", path.display());
        return;
    }
    let golden = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}\nFirst run, or a deliberate change: regenerate with \
             REGEN_SNAPSHOT=1 and say why in the commit message.",
            path.display()
        )
    });
    assert_eq!(
        produced, golden,
        "the cycle's observable output changed; a pure move must not change it"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// The snapshot must be able to fail, and it must be stable across two runs of
/// the same code — otherwise it pins nothing.
#[tokio::test]
async fn the_snapshot_is_stable_and_can_differ() {
    let a = run_and_render(&scratch("stable-a")).await;
    let b = run_and_render(&scratch("stable-b")).await;
    assert_eq!(a, b, "two runs of identical code produced different output, so the \
                      snapshot above pins nothing");
    assert_ne!(
        format!("{a}x"),
        a,
        "the comparison cannot distinguish two different strings"
    );
}
