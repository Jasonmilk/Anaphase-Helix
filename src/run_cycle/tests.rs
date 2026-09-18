//! Tests for the cognitive loop.
//!
//! Split out of run_cycle.rs with the production code (ADR-0042). The paths
//! use `super::super::*` because this file is one level deeper than the code it
//! exercises; nothing else changed.

use super::super::*;
// The parent module glob-imports the adapter traits and types; a glob is not
// re-exported by `use super::*`, so the test file names it again. Same set, no
// new dependency.
use crate::adapters::*;
use super::*;
use std::sync::Arc;

    use crate::adapters::{
        NoopFearAdapter, NoopMemoryAdapter, NoopReasoningAdapter, NoopSafetyAdapter,
        NoopToolAdapter, NoopUiAdapter,
    };
    use crate::reflex::ReflexArc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Counting reasoning adapter (O-1): proves structured commands never
    /// reach the LLM — the counter must stay zero for `!tool` inputs.
    struct CountingReasoning(Arc<AtomicUsize>);

    #[async_trait::async_trait]
    impl crate::adapters::ReasoningAdapter for CountingReasoning {
        async fn reason(&self, input: &str, _mode: &str, _trace_id: &str) -> Result<String, String> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(format!("{{\"calls\":[],\"impasse\":false}} // {}", input))
        }
    }

    /// Prompt-spying reasoning adapter (O-5): records every prompt the LLM
    /// path receives, so injection (or its absence) is asserted verbatim.
    struct SpyReasoning(Arc<std::sync::Mutex<Vec<String>>>);

    #[async_trait::async_trait]
    impl crate::adapters::ReasoningAdapter for SpyReasoning {
        async fn reason(&self, input: &str, _mode: &str, _trace_id: &str) -> Result<String, String> {
            self.0.lock().unwrap().push(input.to_string());
            Ok("{\"calls\":[],\"impasse\":false}".to_string())
        }
    }

    /// Fixed-memory adapter (O-5): returns the same two nodes every round,
    /// so round-to-round injection size is measured in isolation.
    struct SpyMemory(Arc<Vec<MemoryNode>>);

    #[async_trait::async_trait]
    impl crate::adapters::MemoryAdapter for SpyMemory {
        async fn query(
            &self,
            _q: &str,
            _include_recessive: bool,
        ) -> Result<crate::adapters::QueryResult, String> {
            Ok(crate::adapters::QueryResult {
                nodes: (*self.0).clone(),
                impasse_level: 0,
                suggested_actions: vec![],
            })
        }
        async fn remember(&self, _c: &str, _p: &[String]) -> Result<String, String> {
            Ok(String::new())
        }
    }

    fn base() -> AgentLoop {
        AgentLoop::new(
            Arc::new(NoopMemoryAdapter),
            Arc::new(NoopReasoningAdapter),
            Arc::new(NoopToolAdapter),
            Arc::new(NoopSafetyAdapter),
            Arc::new(NoopUiAdapter),
            Arc::new(NoopFearAdapter),
            ReflexArc {
                safety_rules: vec![],
            },
        )
    }

    /// Empty-then-reply reasoning adapter (ADR-0034): first call returns an
    /// empty reply (thinking ate the token budget), the retry answers —
    /// proves the bounded direct-answer retry fires and lands the reply.
    struct EmptyThenReplyReasoning {
        calls: Arc<std::sync::Mutex<Vec<String>>>,
    }

    #[async_trait::async_trait]
    impl crate::adapters::ReasoningAdapter for EmptyThenReplyReasoning {
        async fn reason(&self, input: &str, _mode: &str, _trace_id: &str) -> Result<String, String> {
            let mut guard = self.calls.lock().unwrap();
            guard.push(input.to_string());
            let n = guard.len();
            drop(guard);
            if n == 1 {
                // Budget starvation: thinking consumed max_tokens, content empty.
                Ok("".to_string())
            } else {
                Ok("{\"calls\":[],\"impasse\":false}".to_string())
            }
        }
    }

    #[tokio::test]
    async fn empty_reply_retries_with_direct_answer_directive() {
        // ADR-0034: an empty reply (thinking ate the budget) must trigger one
        // bounded retry whose prompt carries the direct-answer directive, and
        // the cycle must end with the retried reply, not a blank line.
        let calls = Arc::new(std::sync::Mutex::new(vec![]));
        let mut agent = AgentLoop::new(
            Arc::new(NoopMemoryAdapter),
            Arc::new(EmptyThenReplyReasoning { calls: calls.clone() }),
            Arc::new(NoopToolAdapter),
            Arc::new(NoopSafetyAdapter),
            Arc::new(NoopUiAdapter),
            Arc::new(NoopFearAdapter),
            ReflexArc {
                safety_rules: vec![],
            },
        )
        .with_run_config(RunCycleConfig {
            empty_reply_retries: 1,
            ..RunCycleConfig::default()
        });
        let out = agent.run_cycle("hello").await.unwrap();
        assert!(out.done);
        let prompts = calls.lock().unwrap();
        assert_eq!(prompts.len(), 2, "one starved call + one direct-answer retry");
        assert!(
            prompts[1].contains("direct answer required"),
            "retry prompt must carry the direct-answer directive"
        );
        assert!(
            !prompts[1].contains("[think-first"),
            "retry must not re-trigger the craft note"
        );
    }

    #[tokio::test]
    async fn capture_reflects_mode_and_empty_ledger() {
        let agent = base().with_mode(Mode::Drive);
        let snap = agent.capture();
        assert_eq!(snap.mode, Mode::Drive);
        assert!(snap.episode.is_none());
        assert!(snap.ledger.is_empty());
    }

    #[tokio::test]
    async fn drive_mode_black_box_records_cycle_events_without_pipeline() {
        // ADR-0021: the ring is mode-agnostic — a Drive-mode agent with NO
        // pipeline (legacy echo assembly) still records its black box.
        let ring = std::sync::Arc::new(std::sync::Mutex::new(crate::events::EventRing::new(64)));
        let mut agent = base().with_mode(Mode::Drive).with_events(ring.clone());
        let out = agent.run_cycle("!tool numbers --n 3").await.unwrap();
        assert!(out.done);

        let guard = ring.lock().unwrap();
        let evs = guard.events();
        assert!(evs.len() >= 3, "begin + state(s) + end recorded");
        assert_eq!(evs[0].phase, "begin", "first event opens the cycle");
        assert_eq!(evs[0].stage, 0, "cycle-level events are stage 0");
        assert_eq!(evs[evs.len() - 1].phase, "end", "last event closes the cycle");
        assert!(
            evs.iter().all(|e| &e.trace_id == &evs[0].trace_id),
            "one deterministic trace per cycle"
        );
        assert!(
            evs.iter().any(|e| e.phase == "state"),
            "state transitions are recorded"
        );
    }

    #[tokio::test]
    async fn black_box_replays_byte_identical_under_fake_clock() {
        // ADR-0021 (fix): the black box REUSES the ledger Clock — under
        // FakeClock two identical runs emit byte-identical ts (极致复用 +
        // 确定性优先: one time source across cycle/stage/ledger).
        let run = |clock: u64| {
            let ring =
                std::sync::Arc::new(std::sync::Mutex::new(crate::events::EventRing::new(64)));
            let agent = base()
                .with_mode(Mode::Drive)
                .with_events(ring.clone())
                .with_clock(std::sync::Arc::new(crate::ledger::FakeClock(clock)));
            agent
        };
        let mut a = run(1000);
        let mut b = run(1000);
        let _ = a.run_cycle("!tool numbers --n 3").await.unwrap();
        let _ = b.run_cycle("!tool numbers --n 3").await.unwrap();
        let ea = a.events.as_ref().unwrap().lock().unwrap().to_jsonl();
        let eb = b.events.as_ref().unwrap().lock().unwrap().to_jsonl();
        assert_eq!(ea, eb, "same FakeClock -> byte-identical black box replay");
        assert!(
            ea.contains("1970-01-01T00:16:40Z"),
            "FakeClock(1000) ts lands in the JSONL (deterministic)"
        );
    }

    #[tokio::test]
    async fn capture_includes_active_episode() {
        let mut agent = base();
        agent.begin_episode("hello").await;
        let snap = agent.capture();
        let ep = snap.episode.unwrap();
        assert!(ep.id.starts_with("ep-"));
        assert_eq!(ep.first_input, "hello");
        assert_eq!(ep.step, 0);
    }

    #[tokio::test]
    async fn structured_command_bypasses_llm_entirely() {
        let calls_count = Arc::new(AtomicUsize::new(0));
        let reason = Arc::new(CountingReasoning(calls_count.clone()));
        let mut agent = AgentLoop::new(
            Arc::new(NoopMemoryAdapter),
            reason,
            Arc::new(NoopToolAdapter),
            Arc::new(NoopSafetyAdapter),
            Arc::new(NoopUiAdapter),
            Arc::new(NoopFearAdapter),
            ReflexArc { safety_rules: vec![] },
        );
        agent.context.user_input = "!date".to_string();
        agent.run_cycle("!date").await.unwrap();
        assert_eq!(
            calls_count.load(Ordering::Relaxed),
            0,
            "structured command must never call the LLM (0 tokens)"
        );
        assert_eq!(agent.context.calls.len(), 1);
        assert_eq!(agent.context.calls[0].tool, "date");
        assert!(agent.context.structured);
    }

    #[tokio::test]
    async fn free_text_still_reaches_llm() {
        let calls_count = Arc::new(AtomicUsize::new(0));
        let reason = Arc::new(CountingReasoning(calls_count.clone()));
        let mut agent = AgentLoop::new(
            Arc::new(NoopMemoryAdapter),
            reason,
            Arc::new(NoopToolAdapter),
            Arc::new(NoopSafetyAdapter),
            Arc::new(NoopUiAdapter),
            Arc::new(NoopFearAdapter),
            ReflexArc { safety_rules: vec![] },
        );
        agent.context.user_input = "帮我总结一下会议".to_string();
        agent.run_cycle("帮我总结一下会议").await.unwrap();
        assert_eq!(
            calls_count.load(Ordering::Relaxed),
            1,
            "free text still goes through the LLM (no triage regression)"
        );
    }

    #[tokio::test]
    async fn snapshot_carries_ecosystem_lights() {
        let agent = base();
        let snap = agent.capture();
        assert!(snap.ecosystem.is_empty(), "unprobed -> empty lights");
        let mut agent = base();
        agent.context.ecosystem
            .register("cellrix", crate::gloves::GloveTier::Native, crate::gloves::GloveStatus::Available);
        let snap = agent.capture();
        assert_eq!(snap.ecosystem.len(), 1);
        assert_eq!(snap.ecosystem[0].name, "cellrix");
        assert_eq!(snap.ecosystem[0].status, crate::gloves::GloveStatus::Available);
    }

    #[tokio::test]
    async fn single_period_reports_outcome() {
        // ADR-0016 D1: one period is an atomic walk that returns to
        // Perception; the outcome tells the caller whether to loop again.
        let mut agent = base();
        let out = agent.run_cycle("hello").await.unwrap();
        assert!(out.done, "one period always returns to Perception");
        assert!(out.success, "Noop adapters finish via Success");
        assert!(!out.impasse, "no impasse with Noop adapters");
    }

    #[tokio::test]
    async fn caller_loops_periods_and_step_advances_per_period() {
        // The caller owns the looping policy: two periods = two atomic walks.
        // Each period is one turn of the active episode (ADR-0006).
        let mut agent = base();
        agent.begin_episode("hello").await;
        let out1 = agent.run_cycle("hello").await.unwrap();
        let out2 = agent.run_cycle("world").await.unwrap();
        assert!(out1.done && out2.done);
        let ep = agent.episode.expect("episode active");
        assert_eq!(ep.step, 2, "one turn per period");
    }

    // ── O-5 (ADR-0023): on-demand cognitive injection ──────────────────

    fn mn(text: &str) -> MemoryNode {
        MemoryNode {
            content: text.to_string(),
            id: "n-test".to_string(),
            tier: "L3".to_string(),
            activation: 1.0,
            phase: "liquid".to_string(),
            recessive: false,
        }
    }

    #[test]
    fn fold_memory_nodes_empty_and_zero_budget() {
        assert_eq!(fold_memory_nodes(&[], 800), "");
        assert_eq!(fold_memory_nodes(&[mn("a")], 0), "");
    }

    #[test]
    fn fold_memory_nodes_within_budget_is_verbatim() {
        let nodes = vec![mn("alpha"), mn("beta")];
        let folded = fold_memory_nodes(&nodes, 100);
        assert!(folded.contains("alpha"));
        assert!(folded.contains("beta"));
        assert!(!folded.contains("[folded"));
    }

    #[test]
    fn fold_memory_nodes_over_budget_truncates_with_marker() {
        let long = "x".repeat(400);
        let folded = fold_memory_nodes(&[mn(&long)], 200);
        assert!(folded.contains("[folded"), "fold marker must appear");
        // the budget slice itself is exactly `budget` chars of node content
        let body: Vec<char> = folded.chars().collect();
        assert_eq!(body[..200].iter().filter(|c| **c == 'x').count(), 200);
    }

    #[tokio::test]
    async fn memory_nodes_are_injected_into_reasoning_prompt() {
        // O-5 core: MemoryRetrieval results finally reach the LLM (they were
        // retrieved but never consumed before — the broken link is fixed).
        let nodes = Arc::new(vec![
            mn("master taught: prefer deterministic tools over guessing"),
            mn("last time the gap was expectation vs actual feedback"),
        ]);
        let prompts = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let mut agent = base();
        agent.memory = Arc::new(SpyMemory(nodes));
        agent.reason = Arc::new(SpyReasoning(prompts.clone()));
        agent.run_cycle("help me with the tool plan").await.unwrap();
        let captured = prompts.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert!(
            captured[0].contains("[memory"),
            "memory section must be injected: {}",
            captured[0]
        );
        assert!(captured[0].contains("deterministic tools"));
    }

    #[tokio::test]
    async fn zero_budget_keeps_stateless_prompt() {
        // 0 = pure stateless (legacy behaviour): no [memory] section at all.
        let nodes = Arc::new(vec![mn("secret memory")]);
        let prompts = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let mut agent = base();
        agent.memory_inject_chars = 0;
        agent.memory = Arc::new(SpyMemory(nodes));
        agent.reason = Arc::new(SpyReasoning(prompts.clone()));
        agent.run_cycle("hello").await.unwrap();
        let captured = prompts.lock().unwrap();
        assert!(!captured[0].contains("[memory"), "budget 0 must not inject");
    }

    #[tokio::test]
    async fn fold_strips_bookkeeping_tail_keeps_experience() {
        // P10: L3 notes are "User said: …\nCycle completed. p_death: …". The
        // bookkeeping tail must not reach the LLM — only the experience line.
        let nodes = vec![
            mn("User said: 我叫Jason，请记住我的名字\nCycle completed. p_death: 0.00, impasse: 5"),
            mn("plain memory node"),
        ];
        let folded = fold_memory_nodes(&nodes, 400);
        assert!(folded.contains("我叫Jason，请记住我的名字"), "experience must survive");
        assert!(!folded.contains("Cycle completed"), "bookkeeping tail must be stripped");
        assert!(folded.contains("plain memory node"));
    }

    #[tokio::test]
    async fn twenty_five_rounds_context_stays_bounded() {
        // Memory-Efficient mode: injection size is budget-capped and does not
        // grow with round count — 25 rounds of the same task keep the LLM
        // context ~constant (O-5 acceptance).
        let nodes = Arc::new(vec![
            mn(&"n1 ".repeat(150)),
            mn(&"n2 ".repeat(150)),
        ]);
        let prompts = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let mut agent = base();
        agent.memory_inject_chars = 800;
        agent.memory = Arc::new(SpyMemory(nodes));
        agent.reason = Arc::new(SpyReasoning(prompts.clone()));
        for _ in 0..25 {
            agent.run_cycle("steady task").await.unwrap();
        }
        let captured = prompts.lock().unwrap();
        assert_eq!(captured.len(), 25, "one LLM call per round");
        for prompt in captured.iter() {
            let mem_start = prompt.find("[memory]").map(|i| i + 9).unwrap_or(prompt.len());
            let section = &prompt[mem_start..];
            assert!(
                section.chars().count() <= 800 + 64,
                "injected section must stay within budget: {}",
                section.chars().count()
            );
        }
    }

#[cfg(test)]
mod remember_parents_tests {
    use super::*;

    fn mem_node(id: &str) -> MemoryNode {
        MemoryNode {
            content: format!("content-{id}"),
            id: id.to_string(),
            tier: "L3".into(),
            activation: 0.5,
            phase: "liquid".into(),
            recessive: false,
        }
    }

    /// ADR-0043 T5b: the parent set is the memories this cycle reasoned over, in
    /// retrieval order. Order matters because Mind builds one edge per parent and
    /// the DAG check runs in that order.
    #[test]
    fn parents_are_the_retrieved_memory_ids_in_order() {
        let mut ctx = AgentContext::default();
        ctx.memory_nodes = vec![mem_node("n1"), mem_node("n2"), mem_node("n3")];
        assert_eq!(remember_parents(&ctx), vec!["n1", "n2", "n3"]);
    }

    /// No retrieved memories => no parents => no edges. This is the tolerant
    /// degradation path (ADR-0043 D6), and it is what a cold first cycle looks
    /// like, so it must be an empty slice rather than an error.
    #[test]
    fn no_retrieved_memories_yields_no_parents() {
        assert!(remember_parents(&AgentContext::default()).is_empty());
    }
}

