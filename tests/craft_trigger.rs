//! Cognitive craft trigger tests — P10a (ADR-0031).
//!
//! On-demand zero-token orchestration BEFORE the LLM reasoning step:
//! GrpcMindAdapter implements craft (helix_craft), the loop triggers it in
//! MemoryRetrieval (non-structured inputs), and Reasoning folds the
//! synthesis into the prompt as a [think-first] note. Degradation is silent:
//! adapters without craft (Noop) leave craft_note = None and the loop runs
//! unchanged (backwards compatible).

use std::sync::{Arc, Mutex};

use anaphase::adapters::*;
use anaphase::run_cycle::AgentLoop;
use anaphase::reflex::ReflexArc;

/// Memory adapter with craft implemented (probe for the trigger path).
struct CraftMemory {
    calls: Mutex<Vec<String>>,
}

impl CraftMemory {
    fn new() -> Self {
        Self {
            calls: Mutex::new(vec![]),
        }
    }
    fn craft_calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl MemoryAdapter for CraftMemory {
    async fn query(
        &self,
        _query: &str,
        _include_recessive: bool,
    ) -> Result<QueryResult, String> {
        Ok(QueryResult {
            nodes: vec![],
            impasse_level: 0,
            suggested_actions: vec![],
        })
    }
    async fn remember(&self, _content: &str) -> Result<(), String> {
        Ok(())
    }
    async fn craft(&self, query: &str, job_id: &str) -> Result<CraftNote, String> {
        self.calls.lock().unwrap().push(format!("{}#{}", query, job_id));
        Ok(CraftNote {
            trace_id: format!("craft#{}", job_id),
            synthesis: "CONVERGED-SYNTHESIS".to_string(),
            value_grade: String::new(),
        })
    }
}

/// Reasoning adapter that records the prompt it received (deterministic probe).
struct RecordingReasoning {
    prompts: Mutex<Vec<String>>,
}

impl RecordingReasoning {
    fn new() -> Self {
        Self {
            prompts: Mutex::new(vec![]),
        }
    }
    fn last_prompt(&self) -> String {
        self.prompts.lock().unwrap().last().cloned().unwrap_or_default()
    }
}

#[async_trait::async_trait]
impl ReasoningAdapter for RecordingReasoning {
    async fn reason(&self, prompt: &str, _model: &str) -> Result<String, String> {
        self.prompts.lock().unwrap().push(prompt.to_string());
        Ok("no plan, no tool".to_string())
    }
}

fn make_agent(memory: Arc<dyn MemoryAdapter>) -> (AgentLoop, Arc<RecordingReasoning>) {
    let reason = Arc::new(RecordingReasoning::new());
    let agent = AgentLoop::new(
        memory,
        reason.clone(),
        Arc::new(NoopToolAdapter),
        Arc::new(NoopSafetyAdapter),
        Arc::new(NoopUiAdapter),
        Arc::new(NoopFearAdapter),
        ReflexArc {
            safety_rules: vec![],
        },
    );
    (agent, reason)
}

#[tokio::test]
async fn craft_note_triggered_and_folded_into_reasoning_prompt() {
    let memory = Arc::new(CraftMemory::new());
    let (mut agent, reason) = make_agent(memory.clone());

    agent.run_cycle("评估这个方案的架构风险").await.unwrap();

    // Craft triggered exactly once, with the deterministic job id derived
    // from the input (same shared FNV-1a primitive as the pipeline).
    let calls = memory.craft_calls();
    assert_eq!(calls.len(), 1, "craft fires once per cycle");
    let job_id = anaphase::contract::derive_job_id("评估这个方案的架构风险");
    assert_eq!(
        calls[0],
        format!("评估这个方案的架构风险#{}", job_id),
        "craft carries the deterministic job id (shared FNV-1a primitive)"
    );

    // The zero-token synthesis reached the Reasoning prompt.
    let prompt = reason.last_prompt();
    assert!(
        prompt.contains("[think-first (deterministic, 0 tokens)]"),
        "craft note marker must be folded in, got: {}",
        prompt
    );
    assert!(
        prompt.contains("CONVERGED-SYNTHESIS"),
        "synthesis must reach the LLM prompt"
    );
}

#[tokio::test]
async fn structured_command_skips_craft() {
    let memory = Arc::new(CraftMemory::new());
    let (mut agent, _reason) = make_agent(memory.clone());

    // Structured commands (!) already have a plan — no thinking needed.
    agent.run_cycle("!numbers 1 2 3").await.unwrap();
    assert_eq!(
        memory.craft_calls().len(),
        0,
        "structured commands must skip craft (plan already exists)"
    );
}

#[tokio::test]
async fn adapter_without_craft_degrades_silently() {
    // NoopMemoryAdapter does not implement craft → default Err → the loop
    // proceeds with craft_note = None. Backwards compatible.
    let (mut agent, reason) = make_agent(Arc::new(NoopMemoryAdapter));

    agent.run_cycle("普通对话").await.unwrap();
    let prompt = reason.last_prompt();
    assert!(
        !prompt.contains("[think-first"),
        "no craft note when adapter lacks craft"
    );
    assert_eq!(agent.context.craft_note, None);
}
