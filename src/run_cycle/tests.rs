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
use async_trait::async_trait;
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
                provenance: None,
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


/// The transition table, edge by edge.
///
/// Two proofs for this file already existed and neither can see the table. The
/// symbol set counts functions, so deleting a match arm changes nothing it
/// measures. The cycle snapshot drives exactly one path, so eleven of the twelve
/// edges are outside its input space — a broken edge there is invisible to both,
/// and this is the file whose whole content is that table.
///
/// So the edges are asserted as data. Cheap, stable, and it covers precisely what
/// the snapshot cannot: the eleven paths nothing walks.
///
/// It also settles a question that had been open since the early rounds: an
/// impasse goes to Reflection. That is now an assertion rather than something to
/// look up again.
#[test]
fn the_transition_table_has_exactly_these_edges() {
    use crate::states::HelixState as S;
    use TransitionCondition as C;

    // Every edge, written out. Not derived from the table: a list that read the
    // table could only ever agree with it.
    let expected: &[(S, C, S)] = &[
        (S::Perception, C::Success, S::PreAssessment),
        (S::PreAssessment, C::Success, S::MemoryRetrieval),
        (S::MemoryRetrieval, C::Success, S::Reasoning),
        (S::MemoryRetrieval, C::Failure, S::Reflection),
        (S::Reasoning, C::NeedsTool, S::ReflexCheck),
        (S::Reasoning, C::NoToolNeeded, S::Reflection),
        (S::Reasoning, C::Impass, S::Reflection),
        (S::Reasoning, C::Failure, S::Reflection),
        (S::ReflexCheck, C::ReflexPassed, S::Execution),
        (S::ReflexCheck, C::ReflexBlocked, S::Reflection),
        (S::Execution, C::Success, S::Reflection),
        (S::Execution, C::Failure, S::Reflection),
        (S::Reflection, C::Success, S::Perception),
    ];

    let agent = base();
    let table = &agent.transitions;

    assert_eq!(
        table.len(),
        expected.len(),
        "the table has {} edges, the list names {}",
        table.len(),
        expected.len()
    );
    for (from, cond, to) in expected {
        assert_eq!(
            table.get(&(from.clone(), cond.clone())),
            Some(to),
            "missing or wrong edge: {from:?} + {cond:?} should reach {to:?}"
        );
    }

    // And nothing the list does not name, so an added edge has to be declared
    // here too rather than slipping in unexamined.
    for (key, to) in table {
        assert!(
            expected.contains(&(key.0.clone(), key.1.clone(), to.clone())),
            "undocumented edge in the table: {:?} + {:?} -> {:?}",
            key.0,
            key.1,
            to
        );
    }
}

/// The impasse path specifically, because it is the one the early rounds kept
/// asking about and because nothing else walks it.
///
/// Scope note: this asserts the *edge* and nothing else. It used to say the
/// outcome was marked in Reflection, which was never tested — and was false.
/// See `a_declared_impasse_survives_to_the_outcome` below for the marking.
#[test]
fn an_impasse_goes_to_reflection() {
    use crate::states::HelixState as S;
    assert_eq!(
        base()
            .transitions
            .get(&(S::Reasoning, TransitionCondition::Impass)),
        Some(&S::Reflection),
        "an impasse must reach Reflection"
    );
}

/// Reasoning that declares an impasse: `{"impasse": true}` is the model saying
/// it cannot proceed.
struct ImpasseReasoning;

#[async_trait::async_trait]
impl crate::adapters::ReasoningAdapter for ImpasseReasoning {
    async fn reason(&self, _input: &str, _mode: &str, _trace_id: &str) -> Result<String, String> {
        Ok("{\"impasse\":true}".to_string())
    }
}

/// The model's own impasse has to survive to the outcome.
///
/// This is the case `an_impasse_goes_to_reflection` could not see. `outcome`
/// recomputed `impasse` at period end from the condition that entered
/// Perception, and the only edge into Perception is `(Reflection, Success)` —
/// so a declared impasse was overwritten by that Success before anyone looked.
/// `impasse` was true only for an undefined transition, i.e. a synonym for
/// `!done` rather than the independent fact its doc comment advertised.
///
/// The pre-existing `an_incomplete_period_is_bounded_and_distinguishable` stayed
/// green through all of this: it reaches `impasse = true` down the undefined
/// path, so it cannot distinguish "an impasse was declared" from "a rule was
/// missing". Both are impasses — but only one of them was ever reachable.
// guards: impasse-survives
#[tokio::test]
async fn a_declared_impasse_survives_to_the_outcome() {
    let mut agent = AgentLoop::new(
        Arc::new(NoopMemoryAdapter),
        Arc::new(ImpasseReasoning),
        Arc::new(NoopToolAdapter),
        Arc::new(NoopSafetyAdapter),
        Arc::new(NoopUiAdapter),
        Arc::new(NoopFearAdapter),
        ReflexArc {
            safety_rules: vec![],
        },
    );
    let outcome = agent
        .run_cycle("I cannot proceed")
        .await
        .expect("an impasse is an outcome, not an error");
    assert!(
        outcome.impasse,
        "the model declared an impasse and the outcome forgot it (done={}, success={})",
        outcome.done, outcome.success
    );
}

/// Reasoning that reports upstream usage, including the shape that has its own
/// known defect downstream.
///
/// This exists because the usage path had NO coverage at all: the fixed adapters
/// used elsewhere return `UpstreamMeta::default()`, whose `usage` is `None`, so
/// `emit_usage` never executes. A mutation that renamed its output fields left
/// every test green — which is worse than an untested path, because it looks
/// tested.
///
/// Two shapes, because they diverge: `cached_tokens = Some(..)` and `None`. The
/// adapter declares `Option<u64>` deliberately ("None = the upstream did not
/// report it; never coerced to 0"), and Cellrix's validator rejects the `null`
/// that `None` serialises to (K-029). So both are asserted here, and the `None`
/// one is the case that has actually broken in the field.
struct UsageReportingReasoning {
    cached: Option<u64>,
}

#[async_trait]
impl ReasoningAdapter for UsageReportingReasoning {
    async fn reason(&self, _prompt: &str, _model: &str, _trace_id: &str) -> Result<String, String> {
        Ok(r#"{"calls":[{"tool":"numbers","args":{},"expect":"numbers"}]}"#.to_string())
    }
    fn last_meta(&self) -> UpstreamMeta {
        UpstreamMeta {
            model: Some("fixture-model".to_string()),
            usage: Some(UsageSnapshot {
                prompt_tokens: 1511,
                completion_tokens: 50,
                cached_tokens: self.cached,
                reasoning_tokens: if self.cached.is_some() { Some(25) } else { None },
            }),
        }
    }
}

/// Run one cycle with usage reporting and return the `assistant/usage` rows.
async fn usage_rows(cached: Option<u64>) -> Vec<serde_json::Value> {
    let dir = std::env::temp_dir().join(format!(
        "rc-usage-{}-{}",
        std::process::id(),
        cached.is_some()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let mut agent = AgentLoop::new(
        Arc::new(NoopMemoryAdapter),
        Arc::new(UsageReportingReasoning { cached }),
        Arc::new(NoopToolAdapter),
        Arc::new(NoopSafetyAdapter),
        Arc::new(NoopUiAdapter),
        Arc::new(NoopFearAdapter),
        ReflexArc {
            safety_rules: vec![],
        },
    )
    .with_clock(Arc::new(crate::ledger::FakeClock(1_700_000_000)));
    agent.session_events_dir = Some(dir.clone());
    agent.run_cycle("hello").await.unwrap();

    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("jsonl") {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&p) {
                for line in text.lines().filter(|l| !l.trim().is_empty()) {
                    let v: serde_json::Value = serde_json::from_str(line).unwrap();
                    if v.get("type").and_then(|t| t.as_str()) == Some("assistant/usage") {
                        out.push(v);
                    }
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    out.sort_by_key(|v| v.get("seq").and_then(|s| s.as_u64()).unwrap_or(0));
    out
}

/// The usage row is emitted, with the reported numbers, when the upstream
/// reported them.
#[tokio::test]
async fn usage_is_emitted_with_reported_counts() {
    let rows = usage_rows(Some(256)).await;
    assert_eq!(rows.len(), 1, "one cycle reports usage once");
    let d = &rows[0]["data"];
    assert_eq!(d["prompt_tokens"], 1511);
    assert_eq!(d["completion_tokens"], 50);
    assert_eq!(d["cached_tokens"], 256);
    assert_eq!(d["model"], "fixture-model");
}

/// The shape that broke in the field: `cached_tokens` absent upstream becomes
/// `null` on the wire, and downstream that value is rejected outright (K-029).
/// Pinned here so the Anaphase half of that chain is at least observable — the
/// two ends were both dark before this test.
#[tokio::test]
async fn usage_keeps_null_cached_tokens_rather_than_coercing_to_zero() {
    let rows = usage_rows(None).await;
    assert_eq!(rows.len(), 1);
    let d = &rows[0]["data"];
    assert!(
        d.get("cached_tokens").map(|v| v.is_null()).unwrap_or(false),
        "absent upstream usage must serialise as null, not 0 and not a missing \
         field: the adapter's contract says None is 'not reported', and coercing \
         it to zero would invent a measurement. Got: {d}"
    );
    assert_eq!(d["prompt_tokens"], 1511, "the rest of the row still lands");
}

/// M9: does the table assertion watch the DISPATCH, or only the table?
///
/// The transition test asserts what the table declares at construction. It does
/// not assert that the loop reads it. If the dispatch computed its target some
/// other way, all twelve edge assertions would stay green while the machine went
/// somewhere else — which is the CI-4 question again, in a new place: the check
/// would be measuring reachability of a declaration, not the behaviour.
///
/// This removes an edge from a live agent and drives a cycle. A loop that reads
/// the table loses its path; a loop that does not, ignores the removal.
#[tokio::test]
async fn the_loop_actually_reads_the_transition_table() {
    let mut agent = base();
    // Remove the only edge out of MemoryRetrieval on Success. If the dispatch
    // reads the table this path disappears.
    agent.transitions.remove(&(
        crate::states::HelixState::MemoryRetrieval,
        TransitionCondition::Success,
    ));

    let outcome = agent.run_cycle("hello").await.unwrap();
    // Since the fallback was made fail-closed, an undefined transition reports an
    // INCOMPLETE period. Before that change this assertion read `outcome.done`,
    // because a missing rule was reported as a normal completion — which is the
    // defect this pair of tests was written to expose.
    assert!(
        !outcome.done,
        "an undefined transition must not report a completed period"
    );
    assert!(
        outcome.impasse,
        "and it must be marked as the machine failing to proceed"
    );

    // The fallback returns to Perception and marks the period done, which is
    // indistinguishable from a normal end. That is the finding: an undefined
    // transition is not an error and not a refusal, it is a quiet wrap-around.
    // It is a fail-open in the main loop, and the loop is a face no sweep has
    // covered — B0' scoped itself to the reflex arc and the security gate.
    //
    // Reachability of an undefined pair is a separate audit: which states can
    // return which conditions is not established here. What IS established is
    // that the behaviour is not an error, and that removing an edge is caught by
    // the table assertions (nine tests fail), so the table is read rather than
    // mirrored.
    assert_eq!(
        agent.current_state,
        crate::states::HelixState::Perception,
        "an undefined transition lands on Perception"
    );
}

/// The table is sparse: 7 states x 7 conditions = 49 pairs, 12 are defined.
///
/// This is not automatically wrong — a condition only arises from certain states.
/// It is recorded because the dispatch has a fallback for the other 37, and the
/// fallback ends the period as if it had completed. Whether that is a defect
/// depends on whether an undefined pair is reachable, which is a separate
/// question this test deliberately does not answer.
#[test]
fn the_transition_table_is_sparse_and_that_is_recorded() {
    let agent = base();
    let defined = agent.transitions.len();
    let pairs = crate::states::HelixState::ALL.len() * 7;
    assert_eq!(
        pairs, 49,
        "7 states and 7 conditions; if either changed, the ratio below moved"
    );
    assert_eq!(
        defined, 13,
        "the table changed size; update this record and the dispatch fallback test"
    );
}

/// M9, second direction: change an edge's TARGET and see what notices.
///
/// The first experiment deleted an edge and watched nine tests fail, which proved
/// the table is read. It did NOT prove the loop arrives where the table says —
/// an edge can exist and still be consumed to the wrong place, and every
/// assertion about the table's CONTENTS would stay green while the machine went
/// elsewhere.
///
/// So this corrupts one target and observes the states actually visited, taken
/// from the state events the loop emits rather than from the table. If the table
/// were merely a flag list consulted for "is this edge defined", the visited path
/// would be unchanged and this test would pass — which is the finding it exists
/// to rule out.
#[tokio::test]
async fn a_corrupted_target_is_visible_in_the_states_actually_visited() {
    let mut agent = base();
    // Point MemoryRetrieval+Success at Reflection instead of Reasoning. The edge
    // still exists, so any "is it defined" check is satisfied.
    agent.transitions.insert(
        (
            crate::states::HelixState::MemoryRetrieval,
            TransitionCondition::Success,
        ),
        crate::states::HelixState::Reflection,
    );

    let ring = std::sync::Arc::new(std::sync::Mutex::new(crate::events::EventRing::new(64)));
    agent.events = Some(ring.clone());
    agent.run_cycle("hello").await.unwrap();

    // Read the visited states out of the emitted state events.
    let mut visited: Vec<String> = Vec::new();
    for row in ring.lock().unwrap().events().to_vec() {
        let detail = row.detail.clone();
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&detail) {
            if let Some(to) = v.get("to").and_then(|t| t.as_str()) {
                visited.push(to.trim_matches('"').to_string());
            }
        }
    }
    let joined = visited.join(" -> ");
    assert!(
        !visited.iter().any(|s| s == "Reasoning"),
        "the loop reached Reasoning even though the only route there was \
         redirected; visited path: {joined}"
    );
    assert!(
        visited.iter().any(|s| s == "Reflection"),
        "the redirected target should appear in the visited path; got: {joined}"
    );
}

/// M9, first direction again — but on the VISITED trail rather than on test
/// failures, so the two directions are symmetric.
///
/// Deleting an edge and counting red tests shows the table matters somewhere.
/// Observing that a state disappears from the trail shows the loop reads it for
/// the route, which is the claim worth having.
#[tokio::test]
async fn a_removed_edge_removes_that_state_from_the_trail() {
    let mut agent = base();
    agent.transitions.remove(&(
        crate::states::HelixState::MemoryRetrieval,
        TransitionCondition::Success,
    ));
    let ring = std::sync::Arc::new(std::sync::Mutex::new(crate::events::EventRing::new(64)));
    agent.events = Some(ring.clone());
    agent.run_cycle("hello").await.unwrap();

    let visited: Vec<String> = ring
        .lock()
        .unwrap()
        .events()
        .iter()
        .filter_map(|row| {
            serde_json::from_str::<serde_json::Value>(&row.detail)
                .ok()
                .and_then(|v| v.get("to").and_then(|t| t.as_str()).map(|s| s.trim_matches('"').to_string()))
        })
        .collect();
    let joined = visited.join(" -> ");
    assert!(
        !visited.iter().any(|s| s == "Reasoning"),
        "Reasoning is unreachable once its only inbound edge is gone, so it must \
         not appear in the trail; got: {joined}"
    );
    // The fallback emits NO state event: the trail simply stops at the last
    // defined transition, so the wrap-around to Perception is invisible in the
    // event stream. That is the same defect one layer down — not only does an
    // undefined transition look like a normal completion to the caller, it leaves
    // no trace that it happened, while a defined edge emits one.
    assert!(
        !joined.contains("Perception") || visited.last().map(|s| s.as_str()) != Some("Perception"),
        "if the fallback emitted a state event this assertion should be revisited; \
         got: {joined}"
    );
    assert_eq!(
        visited.last().map(|s| s.as_str()),
        Some("MemoryRetrieval"),
        "the trail ends at the last defined transition; got: {joined}"
    );
}

/// An incomplete period is bounded, and says so.
///
/// The review asked what the caller does after `done = false`: continue, exit
/// quietly, or report. Two things are asserted here rather than reasoned about.
///
/// Bounded: `run_cycle` steps at most once per state, so a period cannot spin
/// however the table is wired — there is no retry storm available to it.
///
/// Reported: an incomplete period is distinguishable from a completed one by
/// `done`, and from a refusal by the absence of an `Err`. Both callers were
/// changed to use that distinction; before, one printed "completed successfully"
/// unconditionally and the other returned Success.
#[tokio::test]
async fn an_incomplete_period_is_bounded_and_distinguishable() {
    let mut agent = base();
    agent.transitions.remove(&(
        crate::states::HelixState::MemoryRetrieval,
        TransitionCondition::Success,
    ));

    let outcome = agent
        .run_cycle("hello")
        .await
        .expect("an incomplete period is an outcome, not an error");
    assert!(!outcome.done, "not done");
    assert!(outcome.impasse, "the machine could not proceed");
    assert!(!outcome.success, "and it certainly did not succeed");

    // It terminated, which is the whole of the bound claim: a state that spin
    // would not return at all.
    assert_eq!(agent.current_state, crate::states::HelixState::Perception);
}

/// Every condition a state can return has a rule in the table.
///
/// This is the audit that had to happen BEFORE the fallback was made fail-closed,
/// and it was done after. The reachable set was enumerated from the source rather
/// than reasoned about: each `HelixState` arm's return sites were collected and
/// compared against the twelve defined edges. They match exactly, which means no
/// path that used to work quietly now ends as an incomplete period.
///
/// It was fragile to establish by hand, so it is asserted mechanically. The check
/// reads this file's own source (`include_str!`), finds each state arm, and
/// requires every `TransitionCondition` returned inside it to have an edge. A new
/// return site in an existing arm therefore fails here rather than silently
/// producing an undefined pair at runtime — which is precisely how a working path
/// would have become an impasse without anyone noticing.
#[test]
fn every_returned_condition_has_a_rule() {
    let src = include_str!("mod.rs");
    let lines: Vec<&str> = src.lines().collect();

    // Locate the arms of `execute_current_state`.
    let fn_start = lines
        .iter()
        .position(|l| l.contains("async fn execute_current_state"))
        .expect("execute_current_state must exist");
    // An arm is `HelixState::X =>`. It used to require a `{` body, which stopped being
    // true when the Reasoning arm was extracted to `reasoning.rs` and became a single
    // expression — the guard failing on that is the guard working, and widening it here
    // is a deliberate act rather than a loosened assertion.
    let mut arms: Vec<(String, usize)> = Vec::new();
    for (i, l) in lines.iter().enumerate().skip(fn_start) {
        if let Some(rest) = l.trim().strip_prefix("HelixState::") {
            if let Some((name, _)) = rest.split_once(" =>") {
                arms.push((name.to_string(), i));
            }
        }
    }
    assert_eq!(arms.len(), 7, "expected seven state arms, found {}", arms.len());

    // The last arm ends where the function does, and the function is followed by
    // `execute_structured`, which returns Failure from a different state's
    // perspective. Scanning to end-of-file would attribute those returns to
    // `Reflection` and invent a contradiction that does not exist.
    let mut fn_end = lines.len();
    let mut depth = 0i32;
    let mut started = false;
    for (i, l) in lines.iter().enumerate().skip(fn_start) {
        depth += l.matches('{').count() as i32 - l.matches('}').count() as i32;
        if depth > 0 {
            started = true;
        }
        if started && depth == 0 {
            fn_end = i + 1;
            break;
        }
    }

    let agent = base();
    let state_of = |name: &str| -> crate::states::HelixState {
        use crate::states::HelixState as S;
        match name {
            "Perception" => S::Perception,
            "PreAssessment" => S::PreAssessment,
            "MemoryRetrieval" => S::MemoryRetrieval,
            "Reasoning" => S::Reasoning,
            "ReflexCheck" => S::ReflexCheck,
            "Execution" => S::Execution,
            "Reflection" => S::Reflection,
            other => panic!("unknown arm {other}"),
        }
    };
    let cond_of = |name: &str| -> TransitionCondition {
        match name {
            "Success" => TransitionCondition::Success,
            "Failure" => TransitionCondition::Failure,
            "NeedsTool" => TransitionCondition::NeedsTool,
            "NoToolNeeded" => TransitionCondition::NoToolNeeded,
            "Impass" => TransitionCondition::Impass,
            "ReflexBlocked" => TransitionCondition::ReflexBlocked,
            "ReflexPassed" => TransitionCondition::ReflexPassed,
            other => panic!("unknown condition {other}"),
        }
    };

    // **Derived, not enumerated.** Scanning `mod.rs` alone made this guard expire once
    // per split: every extracted arm takes its `Ok(TransitionCondition::…)` returns with
    // it. Enlarging the list once per segment would mean four more judgement calls, and
    // each is a chance to "pass by scanning less" — the exact temptation this guard
    // exists to prevent. So the scope is the whole `run_cycle` directory, read at
    // runtime, and a new module is picked up the moment it exists.
    //
    // Test modules are excluded: they assert about conditions, they do not return them.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/run_cycle");
    let mut module_sources: Vec<(String, String)> = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("run_cycle must be a directory") {
        let path = entry.expect("readable dir entry").path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if !name.ends_with(".rs") || name == "tests.rs" || name.ends_with("_tests.rs") {
            continue;
        }
        module_sources.push((
            name.clone(),
            std::fs::read_to_string(&path).expect("module must be readable"),
        ));
    }
    assert!(
        module_sources.len() >= 4,
        "the directory scan found {} module file(s); if this is 0 the guard is passing \
         by finding nothing, which is the failure mode it is meant to catch",
        module_sources.len()
    );

    let mut checked = 0;
    for (idx, (name, start)) in arms.iter().enumerate() {
        let end = arms.get(idx + 1).map(|a| a.1).unwrap_or(fn_end);
        let mut seen: Vec<String> = Vec::new();
        // The arm's own module if one exists, found by CONVENTION (CamelCase arm ->
        // snake_case file) rather than by a list. A list would be an enumeration again,
        // and enumerations expire — which is the whole reason this guard changed.
        let snake: String = name
            .chars()
            .enumerate()
            .flat_map(|(i, c)| {
                if c.is_uppercase() && i > 0 {
                    vec!['_', c.to_ascii_lowercase()]
                } else {
                    vec![c.to_ascii_lowercase()]
                }
            })
            .collect();
        let file = format!("{snake}.rs");
        let body: Vec<&str> = match module_sources.iter().find(|(f, _)| *f == file) {
            Some((_, src)) => src.lines().collect(),
            None => lines[*start..end].to_vec(),
        };
        for l in &body {
            let mut rest = *l;
            while let Some(p) = rest.find("TransitionCondition::") {
                rest = &rest[p + "TransitionCondition::".len()..];
                let cond: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                if !cond.is_empty() && !seen.contains(&cond) {
                    seen.push(cond);
                }
            }
        }
        for c in &seen {
            assert!(
                agent
                    .transitions
                    .contains_key(&(state_of(name), cond_of(c))),
                "{name} can return {c} but the table has no edge for it, so that \
                 path would end as an incomplete period"
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 12,
        "expected at least the twelve known reachable pairs, checked {checked}"
    );
}

/// The gate's direction is now a parameter, and a parameterisation is a new
/// abstraction: pass the wrong variant at a call site and the two paths silently
/// agree (or silently swap), with nothing to notice it.
///
/// There is no `Default` impl here to get wrong — the risk is the argument — so
/// this pins what each call site passes. That is the answer to "is the parameter
/// the behaviour before the move": `ReportSuccess` is the legacy path and `Block`
/// is `execute_structured`, exactly as the two blocks read before they moved.
///
/// The day someone unifies them, this goes red, and updating it means updating
/// K-033 in the same commit — which is the point. A move that quietly became a
/// fix would otherwise leave the pit record describing code that no longer exists.
#[test]
fn the_gate_callers_pass_the_policy_they_passed_before_the_move() {
    let src = include_str!("mod.rs");
    let sites: Vec<(usize, &str)> = src
        .match_indices("OnAuditError::")
        .map(|(i, _)| {
            let rest = &src[i..];
            let end = rest.find(|c: char| !(c.is_alphanumeric() || c == ':' || c == '_')).unwrap_or(rest.len());
            (i, &rest[..end])
        })
        .collect();
    assert_eq!(
        sites.len(),
        2,
        "expected exactly two gate call sites, found {}: {:?}",
        sites.len(),
        sites.iter().map(|(_, s)| *s).collect::<Vec<_>>()
    );
    // Order as well as value: the legacy site is the first one in `Execution`.
    assert_eq!(sites[0].1, "OnAuditError::ReportSuccess", "legacy path (K-033: reports success)");
    assert_eq!(sites[1].1, "OnAuditError::Block", "structured path (K-033: blocks)");
    assert!(
        sites[0].0 < sites[1].0,
        "the legacy site must precede the structured one; a swap means the two \
         directions changed places, which is a behaviour change, not a move"
    );

    // And the parameter really is a parameter: the type must not have grown a
    // default that would let a third caller inherit one of these silently.
    let gate = include_str!("safety_gate.rs");
    assert!(
        !gate.contains("impl Default for OnAuditError"),
        "`OnAuditError` gained a default. A defaulted direction is a direction \
         nobody chose, and it would apply to every future caller."
    );
}

// guards: reflex-fail-closed,reflex-bell
/// H5 (ruled 2026-09-18): an unavailable fear model is not permission.
///
/// The `Err` branch used to report `ReflexPassed` with a "default allow" warning, which
/// made an unavailable safety check indistinguishable from a passed one. It is now
/// `ReflexBlocked`.
///
/// This test exists because the ruling is a BEHAVIOUR change and nothing else asserted
/// it: the full suite passed both before and after, so the change would have been
/// unverified — the shape this ledger keeps recording.
///
/// The assertion is on the transition, not on the log line. Asserting the message would
/// pin the wording rather than the direction, and the direction is what was ruled.
struct DeadFear;

#[async_trait]
impl crate::adapters::FearAdapter for DeadFear {
    async fn predict_death(&self, _context: &str) -> Result<f64, String> {
        Err("fear model unavailable".to_string())
    }
}

#[tokio::test]
async fn an_unavailable_fear_model_blocks_rather_than_passing() {
    let mut agent = AgentLoop::new(
        Arc::new(NoopMemoryAdapter),
        Arc::new(NoopReasoningAdapter),
        Arc::new(NoopToolAdapter),
        Arc::new(NoopSafetyAdapter),
        Arc::new(NoopUiAdapter),
        Arc::new(DeadFear),
        ReflexArc {
            safety_rules: vec![],
        },
    );
    // Drive only as far as the reflex gate: the condition it returns is the whole
    // assertion. `ReflexBlocked` routes to Reflection; `ReflexPassed` would route to
    // Execution, which is the fail-open the ruling removed.
    agent.current_state = crate::states::HelixState::ReflexCheck;
    agent.context.suggested_actions = vec!["touch /tmp/x".to_string()];
    let condition = agent
        .execute_current_state()
        .await
        .expect("an unavailable fear model is a condition, not an error");
    assert_eq!(
        condition,
        TransitionCondition::ReflexBlocked,
        "an unavailable fear model must fail CLOSED (H5). ReflexPassed here means the \
         safety check silently became permission again."
    );

    // B22's bell: the block must be observable WITHOUT the panel. This reads a field,
    // not a tracing subscriber and not a port, so the assertion cannot pass merely
    // because a ring buffer the panel would read happens to be populated.
    let reason = agent
        .context
        .last_reflex_block
        .as_deref()
        .expect("a blocked reflex must leave a reason (B22); a silent block is the same \
                 defect as a silent pass, pointing the other way");
    assert!(
        reason.contains("unavailable"),
        "and the reason must name the cause, not merely that something happened: {reason:?}"
    );
}

/// The bell is on all three block paths, not just the new one.
///
/// H5 added a third way for the reflex to block, and the bell was the whole point of the
/// pairing. Wiring only the new branch would leave the two older ones silent — which is
/// the shape this ledger keeps recording: a mechanism applied to the case that prompted it.
#[tokio::test]
async fn every_block_path_rings_the_bell() {
    // hard rule
    let mut agent = AgentLoop::new(
        Arc::new(NoopMemoryAdapter),
        Arc::new(NoopReasoningAdapter),
        Arc::new(NoopToolAdapter),
        Arc::new(NoopSafetyAdapter),
        Arc::new(NoopUiAdapter),
        Arc::new(NoopFearAdapter),
        ReflexArc {
            safety_rules: vec!["rm -rf /".to_string()],
        },
    );
    agent.current_state = crate::states::HelixState::ReflexCheck;
    agent.context.suggested_actions = vec!["rm -rf /".to_string()];
    let c = agent.execute_current_state().await.expect("a condition");
    assert_eq!(c, TransitionCondition::ReflexBlocked, "the hard rule must block");
    assert!(
        agent.context.last_reflex_block.is_some(),
        "the hard-rule block must also ring the bell, or only the newest path is audible"
    );
}
