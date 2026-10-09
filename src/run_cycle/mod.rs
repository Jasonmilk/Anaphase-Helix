//! The cognitive loop: one `AgentLoop`, one state machine, one cycle.
//!
//! Split out of a single 2094-line file (ADR-0042). This module keeps the loop's
//! identity — the state and condition vocabulary, the engine struct, its builder,
//! and the cycle driver. The per-state work lives in submodules so that a state
//! can be read without the other six.
//!
//! Nothing here was rewritten in the move. The public path is unchanged:
//! `crate::run_cycle::AgentLoop` resolves exactly as before, because a directory
//! module and a file module are the same path.
//!
//! STAGE 1 OF 2. `execute_current_state` is still one 863-line match over seven
//! `HelixState` arms. Stage 2 lifts each arm into its own module. Until then this
//! file is over budget on purpose and carries the surviving waiver, so that
//! "the file moved" cannot be read as "the debt was paid".

use crate::adapters::*;
use crate::config::{Mode, RunCycleConfig};
use crate::contract::{
    contains_tool_request, contradicts_evidence, derive_episode_id, parse_reasoning_output, Call,
    TtJob,
};
use crate::evidence::EvidenceRecord;
use crate::hitl::HITLApprover;
use crate::ledger::unix_secs_to_rfc3339;
use crate::session_events::convergence::{
    answer_lodes as cv_answer_lodes, answer_lode as cv_answer_lode,
    answer_sediment as cv_answer_sediment,
};
use crate::pipeline::Pipeline;
use crate::reflex::ReflexArc;
use crate::states::HelixState;
use std::sync::Arc;
use std::collections::{BTreeMap, HashMap};
use tracing::{debug, info, trace, warn};

/// State transition conditions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TransitionCondition {
    Success,
    Failure,
    NeedsTool,
    NoToolNeeded,
    Impass,
    ReflexBlocked,
    ReflexPassed,
}

/// One episode (experience) of the cognitive loop (ADR-0006): the boundary
/// that groups the turns of a single conversation — "this conversation is an
/// experience Helix lived". A grouping key only: Mind node ids remain the
/// unique identity, so episode ids need not be globally unique.
#[derive(Debug, Clone, PartialEq)]
pub struct Episode {
    /// `ep-` + 16 hex, derived deterministically from the first input
    /// (shared FNV-1a primitive — no UUID, DNA principle 11).
    pub id: String,
    /// First input of the experience (the anchor turn).
    pub first_input: String,
    /// Completed turn index within the episode (0 at begin, +1 per cycle).
    pub step: usize,
}

/// Episode closure payload (ADR-0006 D2): what this experience was — id,
/// turn count, and the first-input anchor. Written to L3 through the memory
/// adapter (no new RPC); the cognitive craft (Mind ADR-0021) consumes it
/// during recap. "Forget the conversation, keep the lesson."
#[derive(Debug, Clone, PartialEq)]
pub struct EpisodeDigest {
    pub episode_id: String,
    pub turns: usize,
    pub first_input: String,
}

/// One serializable projection of the agent's live state (candidate G-T2).
/// The HTTP snapshot endpoint reads this; the loop refreshes it after each
/// cycle. Projection only — the pipeline ledger and the episode remain the
/// single sources of truth.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct AgentSnapshot {
    pub mode: Mode,
    pub state: crate::states::HelixState,
    pub episode: Option<EpisodeView>,
    /// Live ledger entries, newest first (projection, capped).
    pub ledger: Vec<crate::ledger::LedgerRecord>,
    /// Ecosystem component lights (O-1): what the body has in hand.
    pub ecosystem: Vec<crate::gloves::GloveInfo>,
}

/// Episode shape for the snapshot (id / anchor / progress).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct EpisodeView {
    pub id: String,
    pub first_input: String,
    pub step: usize,
}

/// Outcome of one cognitive period (ADR-0016 D1): the single-cycle
/// primitive result. The caller owns the looping policy — how many periods
/// to run and when to stop is a caller decision, never an engine property.
#[derive(Debug, Clone, PartialEq)]
pub struct CycleOutcome {
    /// Period finished (state machine returned to Perception).
    pub done: bool,
    /// Finished via a Success transition.
    pub success: bool,
    /// Finished via an impasse condition.
    pub impasse: bool,
    /// THE LEDGER AS OF COMPLETION (2026-10-09, human ruling).
    ///
    /// WHY A SNAPSHOT AND NOT THE LIVE OBJECT: a criterion that reads `self.pipeline…ledger`
    /// measures the READING INSTANT, not the result — and the reading instant is not what any
    /// assertion is about. Measured: `run_cycle_pipeline` gave two different failure sets for the
    /// same command (9 red vs 3 red), every test passed alone, serial made no difference, and
    /// adding one `eprintln!` turned it green — the signature of a criterion depending on WHEN it
    /// reads. A value taken at the completion point cannot depend on that.
    ///
    /// This is the `ADR-0018 batch 4` move ("一类缺陷变成不可能状态") applied one layer up:
    /// the criterion's INPUT. Callers read here; the live internals are closed off (`pub(crate)`).
    pub ledger: Vec<crate::ledger::LedgerRecord>,
}

impl Default for CycleOutcome {
    fn default() -> Self {
        Self { done: false, success: false, impasse: false, ledger: Vec::new() }
    }
}

/// Core cognitive loop engine for Anaphase
pub struct AgentLoop {
    pub memory: Arc<dyn MemoryAdapter>,
    pub reason: Arc<dyn ReasoningAdapter>,
    pub tool: Arc<dyn ToolAdapter>,
    pub safety: Arc<dyn SafetyAdapter>,
    pub ui: Arc<dyn UiAdapter>,
    pub fear: Arc<dyn FearAdapter>,
    pub reflex: ReflexArc,
    /// State transition table: (current state, condition) -> next state
    transitions: HashMap<(HelixState, TransitionCondition), HelixState>,
    /// Current execution state
    pub current_state: HelixState,
    /// Context carried through the cognitive cycle
    pub context: AgentContext,
    /// Optional streaming deltas sink (SSE chat): when set, the reasoning
    /// adapter emits content deltas here instead of buffering silently.
    pub stream_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::adapters::StreamDelta>>,
    /// HITL 人在回路审批通道（P10b T3，执行闸；默认 fail-closed）
    pub hitl: HITLApprover,
    /// THE JUDGE THE ENGINE ASKS AT ITS ENTRY (ADR-0048 §233): `Unconfigured` by DEFAULT, which is
    /// exactly today's behaviour (ungoverned passes, and `governance::warning` announces it), so no
    /// existing test changes meaning. It is INJECTED, never stored-and-refreshed: a remembered probe
    /// result expires in the fail-OPEN direction, which is the one direction §210 forbids.
    pub gate: crate::gate::Gate,
    /// M1.5-T6 (ADR-0004): optional real tool name resolved for Execution.
    /// When set (e.g. "numbers"), Execution calls this tool via the configured
    /// ToolAdapter (e.g. GrpcTentacleAdapter) instead of the `echo` placeholder.
    /// None keeps the legacy echo path — existing tests stay untouched.
    /// This is the first, lowest-risk step of the six-stage run_cycle re-wire
    /// (ADR-0003 decision 9 mapping table).
    pub tool_command: Option<String>,
    /// run_cycle state-machine constants (candidate E, ADR-0005). Source for
    /// the five historical literals (DNA principle 11 / ADR-0002) — see
    /// `crate::config::RunCycleConfig`.
    pub run_config: RunCycleConfig,
    /// M1 deterministic pipeline (candidate E, ADR-0005). When wired, the
    /// cognitive states consume its six stages — Reasoning parses + assembles
    /// (stages 1-2), Execution executes + records evidence (stages 3-4),
    /// Reflection checks criteria + writes the verdict ledger (stages 5-6).
    /// None keeps the legacy string/echo path (backwards compatible).
    /* THE FENCE (2026-10-09). This was `pub`, so integration tests (`tests/` is a SEPARATE crate)
     * read the live ledger mid-flight: `agent.pipeline.as_ref().unwrap().ledger.records()`. That
     * makes an assertion depend on WHEN it reads, not on what happened — measured: the same command
     * produced two different failure sets, every test passed alone, serial changed nothing, and one
     * extra `eprintln!` turned it green.
     * `pub(crate)` is the whole fix: callers outside the crate can no longer reach the live object,
     * so they must use `CycleOutcome.ledger` — the snapshot taken at the completion point. Not
     * "remember not to write that", but "it does not compile".
     * (ADR-0018 batch 4: 一类缺陷变成不可能状态 — applied one layer up, to the criterion's INPUT.) */
    pub pipeline: Option<Pipeline>,   /* ③ 暂时撤回：先验 ② 的行为修复 */
    /// Interaction mode (ADR-0006): Drive (no Mind) / Partner (default) /
    /// Survive (Mind autonomous, reserved for P10a). Physical participation
    /// is decided at assembly time (Noop vs gRPC memory adapter); this field
    /// is the semantic record and the config source (no hardcoding).
    pub mode: Mode,
    /// Mode-agnostic event ring (ADR-0021): every cycle emits begin/state/end
    /// events regardless of assembly — a Drive-mode black box even when no
    /// pipeline is wired. The pipeline (when present) shares this same ring
    /// and adds stage 1..=6 events; stage 0 is reserved for cycle-level
    /// events. None = ring not injected (legacy behavior, tests untouched).
    pub events: Option<std::sync::Arc<std::sync::Mutex<crate::events::EventRing>>>,
    /// Injected time source (ADR-0021, fixed): REUSES the ledger Clock trait
    /// — one time source across cycle + stage + ledger (极致复用, no second
    /// clock). Defaults to SystemClock; tests inject FakeClock for
    /// byte-identical replay of the black box.
    pub clock: std::sync::Arc<dyn crate::ledger::Clock>,
    /// Active experience boundary (ADR-0006). None = no episode in progress
    /// (legacy turn-by-turn behavior, fully backwards compatible).
    pub episode: Option<Episode>,
    /// External human-authored knowledge rails (ADR-0018). None = no rail
    /// mounted (fail-open, loop unaffected). Read-only citation asset —
    /// Helix may only select an existing edge, never synthesize one.
    pub rails: Option<crate::rails::Index>,
    /// Rails runtime budgets (retrieval hits / injected bytes). Source for
    /// the navigation literals (DNA principle 11 / ADR-0002).
    pub rails_config: crate::config::RailsConfig,
    /// Cognitive-injection budget (O-5, ADR-0023): memory nodes folded into
    /// the Reasoning prompt, capped at this many chars — "the request carries
    /// only what this round needs". 0 = no injection (pure stateless).
    /// Source: config `[anaphase] memory_inject_chars` (protocol default
    /// const below, ADR-0023); main overrides from config.
    pub memory_inject_chars: usize,
    /// ADR-0049 D6: 会话收敛的旋钮（状态/骨架/下钻）。**缺省 = 协议默认**，
    /// main 从 `[anaphase]` 覆盖。`enabled=false` = 回到旧行为。
    pub convergence: crate::session_events::ConvergenceConfig,
    /// O-6 (ADR-0024): judge-point backend — complexity assessment for the
    /// Amygdala -> suggested_mode chain. Rules by default (zero tokens);
    /// SmallLlm (3B-class) when configured. Always returns 1/2/3.
    pub judge: std::sync::Arc<dyn crate::judge::Judge>,
    /// Reasoning body trace (ProveTrack join, 2026-09-07): append-only JSONL of
    /// every reasoning round trip (prompt + response, redacted + truncated).
    /// The audit chain stores metadata; this stores the *body*, on the side
    /// that constructs the prompt. None = trace off (opt-in, config
    /// `reasoning_trace_path`). `trace_id` = the derived job id, joining
    /// body + chain + ledger in Cellrix's ProveTrack view.
    pub trace: Option<crate::trace::ReasoningTrace>,
    /// Session event stream directory (ProveTrack turn timeline, ADR-0023):
    /// one append-only JSONL per cognitive period, keyed by the derived
    /// job id. Opened at period start (the id exists only then); None =
    /// stream off. Non-fatal: a failed open degrades to no stream.
    pub session_events_dir: Option<std::path::PathBuf>,
    /// Redactor for the event stream (same credential shapes as trace).
    pub session_events_redact: crate::trace::Redaction,
    /// The open per-period stream (rebuilt each period).
    pub session_events: Option<crate::session_events::SessionEventStream>,
}

/// Fold memory nodes into a bounded injection string (O-5, ADR-0023).
///
/// Deterministic fold: nodes joined with a separator, then truncated to
/// `budget` chars with an explicit fold marker — the LLM request carries a
/// fixed-size cognitive slice regardless of how many rounds have passed
/// (25-round context grows ~zero). Budget 0 -> empty (no injection).
fn fold_memory_nodes(nodes: &[MemoryNode], budget: usize) -> String {
    if budget == 0 || nodes.is_empty() {
        return String::new();
    }
    const SEP: &str = "\n---\n";
    const FOLD_MARKER: &str = "\n...[folded: more memory available on demand]";
    let mut acc = String::new();
    let mut first = true;
    for n in nodes {
        if !first {
            acc.push_str(SEP);
        }
        first = false;
        // P10: L3 notes are "User said: …\nCycle completed. p_death: …". The
        // bookkeeping tail is provenance, not experience — inject only the
        // experience line so the LLM reads the memory, not the ledger.
        let experience = n.content.split("\nCycle").next().unwrap_or(&n.content);
        acc.push_str(experience);
    }
    if acc.chars().count() <= budget {
        return acc;
    }
    let mut out: String = acc.chars().take(budget).collect();
    out.push_str(FOLD_MARKER);
    out
}

/// ADR-0043 T5b: the parent set for anything this cycle writes — the memories it
/// actually retrieved and reasoned over.
///
/// `derived_from` means "built on", so the honest parents are the OBSERVATIONS
/// this cycle reasoned over, not the previous note. This also gives real fan-in
/// rather than a thin chain, and its size is already bounded by Mind's own
/// `max_nodes_per_query` — hence no new configuration knob.
impl AgentLoop {
    /// B22's bell: make a reflex block observable without the panel.
    ///
    /// Three things, deliberately separate from the ring buffer, which only the panel
    /// reads, and from the session-event stream, which may not be configured at all:
    ///
    ///   1. a `warn!` that NAMES the direction, so a log reader can tell a block from a
    ///      pass without knowing the code (P13);
    ///   2. an `eprintln!` — stderr goes to whatever is collecting output, panel or not;
    ///   3. `context.last_reflex_block`, so a caller and a test can read it without a
    ///      tracing subscriber or a port.
    ///
    /// H5 made this load-bearing. Until the reflex failed closed, a block was rare; now
    /// an unavailable fear model produces one, and a block nobody reports would have
    /// replaced "pretends to succeed" with "silently stuck".
    fn ring_reflex_block(&mut self, reason: &str) {
        warn!("[ReflexCheck] BLOCKED ({reason}) — fail-closed, not a pass");
        eprintln!("⚠️  reflex gate blocked the action: {reason}");
        self.context.last_reflex_block = Some(reason.to_string());
    }
}

fn remember_parents(context: &AgentContext) -> Vec<String> {
    context.memory_nodes.iter().map(|n| n.id.clone()).collect()
}

/// Context data flowing through the cognitive cycle
#[derive(Debug, Clone, Default)]
pub struct AgentContext {
    pub user_input: String,
    /// THE DECLARATION, BESIDE THE EFFECTIVE VALUE (ADR-0048 §346.3 ③). `AgentLoop.mode` is what the cycle
    /// RUNS with (it must hold some value to run); this is what the config DECLARED — and it stays `None`
    /// when the config said nothing, so the event payload can carry an ABSENCE instead of a default posing as
    /// a choice. `#[derive(Default)]` gives `None`, which is exactly "undeclared".
    pub declared_mode: Option<Mode>,
    /// Why the reflex gate last blocked, if it did (B22). `None` means it did not.
    ///
    /// This exists so a block is observable WITHOUT the panel. Before it, a blocked
    /// reflex reached the ring buffer — which only the panel reads — and a session-event
    /// stream that may not be configured, i.e. a block could be reported to nobody. H5's
    /// ruling made that matter: `soft_reflex` failing now blocks, so a silent block would
    /// have swapped "pretends to succeed" for "silently stuck", which is the same defect
    /// pointing the other way.
    ///
    /// `Option`, not `String`: an absent block and an empty reason are different facts,
    /// and collapsing them is this ledger's most-recorded mistake.
    pub last_reflex_block: Option<String>,
    /// Physical-clock time anchor (2026-09-09): when the user message
    /// arrived (epoch secs from the single injected clock, ADR-0021).
    /// Injected into the prompt at zero tokens so Helix never loses the
    /// temporal anchor — memories without time misorder. Rendering reuses
    /// `ledger::unix_secs_to_rfc3339` (one time source, one format).
    pub input_at: u64,
    pub amygdala_vector: (f64, f64, f64),  // (heliotropism, pulse, vigilance)
    pub memory_nodes: Vec<MemoryNode>,
    /// Explicit continuation (2026-09-07): when the panel asks to resume a
    /// previous experience (`job_id`), the last round's summary is injected
    /// here as true history — the new period opens as a continuation, not a
    /// fresh stranger.
    pub resume: Option<String>,
    /// Continuation parent (2026-09-14): the machine-readable job id of the
    /// resumed period. ProveTrack threads periods on THIS (session list
    /// aggregation), while `resume` carries the human-readable summary for
    /// prompt injection. One continuation, two carriers.
    pub resume_job: Option<String>,
    /// THE LINEAGE FACT, kept apart from the history fact (ADR-0048 §251). `resume_job` names the
    /// SCOPE whose history is injected; THIS names the exact PERIOD this run continues, and it is
    /// `Some` only when the resolver actually found a period id. Measured before this split: the
    /// normalized period was stored in `resume_job` and, when resolution failed, the raw job id was
    /// written into `resume_from` — 4 of 61 periods recorded a job id there and 1 recorded prose, so
    /// the reader (which refuses non-ids) dropped their threads and the sidebar saw them as roots.
    /// Absent resolution means ABSENT parent: a wrong parent is worse than a missing one.
    pub resume_period: Option<String>,
    /// THIS PERIOD'S JOIN KEY (ADR-0048 D1): allocated at the cycle entry, unique per run, and the
    /// value the Tuck chain (`x-tuck-trace`), the body trace and the ledger all key on. It replaced
    /// the input digest — measured 2026-10-09, 462 periods came from 56 digests and one covered 219
    /// runs. `None` = the identity layer refused; the key is then ABSENT, never a colliding digest.
    pub period_id: Option<String>,
    pub reasoning_output: String,
    /// Private reasoning (thinking) accumulated from the streaming sink
    /// (ADR-0029). Persisted as `assistant/think` (redacted, display-only).
    pub reasoning_think: String,
    /// Last criteria verdict status of this period ("MET"/"UNMET"/"blocked"/
    /// None when no tool ran). END.success derives from it (铁律:
    /// success ≡ verdict ≠ Unmet), never from the transition alone.
    pub last_verdict: Option<String>,
    /// Legacy unstructured action suggestions (from MemoryAdapter.query).
    /// Retained for P11b compatibility; Execution prefers the structured plan.
    pub suggested_actions: Vec<String>,
    /// Structured tool-call plan parsed from Reasoning output (candidate E).
    /// Consumed by Execution as the deterministic execution path.
    pub calls: Vec<Call>,
    /// Assembled tt_job envelope (pipeline stage 2, Reasoning tail).
    pub job: Option<TtJob>,
    /// Evidence records produced by this cycle's Execution (stages 3-4).
    /// Consumed by Reflection for criteria checks and the verdict ledger.
    pub evidence: Vec<EvidenceRecord>,
    pub p_death: f64,
    pub reflection_notes: String,
    /// 生态组件点亮状态（O-1，ADR-0016 D3）：任务开始前探测一次的投影，
    /// 供执行通道选择与驾驶舱展示。默认空 = 未探测（fail-open，不阻塞）。
    pub ecosystem: crate::gloves::EcosystemGloves,
    /// Structured-command marker (O-1): set by Perception when the input
    /// starts with `!`; Reasoning skips the LLM for this cycle (0 tokens).
    pub structured: bool,
    /// Rails hits (ADR-0018): verbatim nodes injected when the query lands
    /// on an external human rail. Content is a human asset — Helix cites,
    /// never rewrites it.
    pub rail_nodes: Vec<crate::rails::Node>,
    /// Citation-contract marker (ADR-0018): when true, the answer for this
    /// cycle must be verbatim rail citations (or NO_RAIL_CONTENT) — no
    /// synthesized paraphrase.
    pub rail_mode: bool,
    /// P10a (ADR-0031): cognitive craft note from Mind's helix_craft —
    /// deterministic zero-token orchestration folded into the Reasoning
    /// prompt (think-first, then spend tokens). None = craft unavailable or
    /// degraded this cycle (enhancement, never a dependency).
    pub craft_note: Option<crate::adapters::CraftNote>,
}

impl AgentLoop {
    /// SA-Core choice detail for the ProveTrack cockpit (ADR-0033): which layers
    /// were picked and the top-activation nodes — provenance only (id/tier/activation/
    /// phase), never node content. Empty when no memory was retrieved.
    fn memory_choice_detail(&self) -> Option<serde_json::Value> {
        if self.context.memory_nodes.is_empty() {
            return None;
        }
        let mut tiers: std::collections::BTreeMap<String, usize> = Default::default();
        for n in &self.context.memory_nodes {
            *tiers.entry(n.tier.clone()).or_insert(0) += 1;
        }
        let mut top: Vec<serde_json::Value> = self
            .context
            .memory_nodes
            .iter()
            .filter(|n| !n.recessive)
            .map(|n| {
                serde_json::json!({
                    "id": n.id,
                    "tier": n.tier,
                    "activation": (n.activation * 100.0).round() / 100.0,
                    "phase": n.phase,
                })
            })
            .collect();
        top.sort_by(|a, b| {
            b["activation"]
                .as_f64()
                .unwrap_or(0.0)
                .partial_cmp(&a["activation"].as_f64().unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        top.truncate(3);
        Some(serde_json::json!({ "tiers": tiers, "top": top }))
    }

    /// Create a new cognitive loop engine with Noop adapters as default
    pub fn new(
        memory: Arc<dyn MemoryAdapter>,
        reason: Arc<dyn ReasoningAdapter>,
        tool: Arc<dyn ToolAdapter>,
        safety: Arc<dyn SafetyAdapter>,
        ui: Arc<dyn UiAdapter>,
        fear: Arc<dyn FearAdapter>,
        reflex: ReflexArc,
    ) -> Self {
        // O-5 (ADR-0023): protocol default for the cognitive-injection budget.
        // Overridden by main from config `[anaphase] memory_inject_chars`.
        const DEFAULT_INJECT_CHARS: usize = 800;
        // Build declarative state transition table
        let mut transitions = HashMap::new();
        transitions.insert((HelixState::Perception, TransitionCondition::Success), HelixState::PreAssessment);
        transitions.insert((HelixState::PreAssessment, TransitionCondition::Success), HelixState::MemoryRetrieval);
        transitions.insert((HelixState::MemoryRetrieval, TransitionCondition::Success), HelixState::Reasoning);
        transitions.insert((HelixState::MemoryRetrieval, TransitionCondition::Failure), HelixState::Reflection);
        transitions.insert((HelixState::Reasoning, TransitionCondition::NeedsTool), HelixState::ReflexCheck);
        transitions.insert((HelixState::Reasoning, TransitionCondition::NoToolNeeded), HelixState::Reflection);
        transitions.insert((HelixState::Reasoning, TransitionCondition::Impass), HelixState::Reflection);
        // (Reasoning, Failure) is REACHABLE: a reply shaped like a plan whose
        // schema is rejected returns Failure (P0-D-1, fail-closed on contract
        // violations). Without this edge that path hit the undefined-transition
        // fallback — which before the fail-closed fix reported a completed period
        // and afterwards reports an incomplete one, and neither is right: the
        // machine knows exactly where a rejected plan should go.
        //
        // Found by `every_returned_condition_has_a_rule`, which scans this file's
        // own source. A manual audit had missed it by mis-attributing the return
        // site to the following function.
        transitions.insert((HelixState::Reasoning, TransitionCondition::Failure), HelixState::Reflection);
        transitions.insert((HelixState::ReflexCheck, TransitionCondition::ReflexPassed), HelixState::Execution);
        transitions.insert((HelixState::ReflexCheck, TransitionCondition::ReflexBlocked), HelixState::Reflection);
        transitions.insert((HelixState::Execution, TransitionCondition::Success), HelixState::Reflection);
        transitions.insert((HelixState::Execution, TransitionCondition::Failure), HelixState::Reflection);
        transitions.insert((HelixState::Reflection, TransitionCondition::Success), HelixState::Perception);

        Self {
            memory,
            reason,
            tool,
            safety,
            ui,
            fear,
            reflex,
            transitions,
            current_state: HelixState::Perception,
            context: AgentContext::default(),
            stream_tx: None,
            hitl: HITLApprover::default(),
            gate: crate::gate::Gate::default(),
            tool_command: None,
            run_config: RunCycleConfig::default(),
            pipeline: None,
            mode: Mode::Partner,
            episode: None,
            events: None,
            clock: std::sync::Arc::new(crate::ledger::SystemClock),
            rails: None,
            rails_config: crate::config::RailsConfig::default(),
            memory_inject_chars: DEFAULT_INJECT_CHARS,
            convergence: crate::session_events::ConvergenceConfig::default(),
            judge: std::sync::Arc::new(crate::judge::RulesJudge {
                // config source, not literals (DNA principle 11 / ADR-0002):
                // MindConfig protocol defaults feed the rules judge; main
                // overrides the whole judge from `[anaphase] judge_*`.
                skilled_len: crate::config::MindConfig::default().skilled_len,
                anchor_len: crate::config::MindConfig::default().anchor_len,
            }),
            // Trace is opt-in: main wires it from `reasoning_trace_path`.
            trace: None,
            session_events_dir: None,
            session_events_redact: crate::trace::Redaction::default(),
            session_events: None,
        }
    }

    /// Configure the real tool name resolved for Execution (M1.5-T6).
    /// When set together with a gRPC-capable ToolAdapter, Execution calls the
    /// real Tentacle tool instead of the `echo` placeholder.
    pub fn with_tool_command(mut self, command: impl Into<String>) -> Self {
        self.tool_command = Some(command.into());
        self
    }

    /// Override the run_cycle state-machine constants (candidate E, ADR-0005).
    pub fn with_run_config(mut self, config: RunCycleConfig) -> Self {
        self.run_config = config;
        self
    }

    /// Wire the M1 deterministic pipeline into the cognitive loop (candidate E).
    /// When set, Execution/Reflection consume the six pipeline stages; None
    /// keeps the legacy string/echo path (backwards compatible).
    pub fn with_pipeline(mut self, pipeline: Pipeline) -> Self {
        self.pipeline = Some(pipeline);
        self
    }

    /// Set the interaction mode (ADR-0006). Physical Mind participation is
    /// decided at assembly time (Noop vs gRPC memory adapter); this is the
    /// semantic record carried through the loop.
    /// Mount an external knowledge rail (ADR-0018). Read-only: the caller
    /// passes the deterministic index; no write path exists.
    pub fn with_rails(mut self, index: crate::rails::Index) -> Self {
        self.rails = Some(index);
        self
    }

    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// Inject the shared event ring (ADR-0021): the same ring the pipeline
    /// uses (when wired), so the `?after=` cursor spans cycle + stage events
    /// in one stream. Without a pipeline this is still the Drive-mode black box.
    pub fn with_events(mut self, ring: std::sync::Arc<std::sync::Mutex<crate::events::EventRing>>) -> Self {
        self.events = Some(ring);
        self
    }

    /// Inject a deterministic clock (ADR-0021): with FakeClock the black box
    /// is byte-identical replayable, same as the ledger/stage replay contract.
    pub fn with_clock(mut self, clock: std::sync::Arc<dyn crate::ledger::Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Project the agent's live state for the snapshot endpoint (candidate
    /// G-T2). Reads only pub fields — no second source of truth.
    pub fn capture(&self) -> AgentSnapshot {
        AgentSnapshot {
            mode: self.mode,
            state: self.current_state.clone(),
            episode: self.episode.as_ref().map(|e| EpisodeView {
                id: e.id.clone(),
                first_input: e.first_input.clone(),
                step: e.step,
            }),
            ecosystem: self.context.ecosystem.list(),
            ledger: self
                .pipeline
                .as_ref()
                .map(|p| p.ledger.records().to_vec())
                .unwrap_or_default(),
        }
    }

    /// Begin a new episode (experience boundary, ADR-0006 D1). An active
    /// episode is closed first (digest recorded), so no experience is ever
    /// silently dropped. Id is deterministic from the first input (shared
    /// FNV-1a primitive — replayable, no UUID).
    pub async fn begin_episode(&mut self, input: &str) -> String {
        if self.episode.is_some() {
            let _ = self.end_episode().await;
        }
        let id = derive_episode_id(input);
        self.episode = Some(Episode {
            id: id.clone(),
            first_input: input.to_string(),
            step: 0,
        });
        id
    }

    /// Close the active episode (ADR-0006 D2): write an EpisodeDigest to L3
    /// via the existing memory adapter (no new RPC — semantics match
    /// INTENT-7 FINISH) and clear the boundary. Idempotent: None when no
    /// episode is active.
    pub async fn end_episode(&mut self) -> Option<EpisodeDigest> {
        let ep = self.episode.take()?;
        let digest = EpisodeDigest {
            episode_id: ep.id.clone(),
            turns: ep.step + 1,
            first_input: ep.first_input.clone(),
        };
        let note = serde_json::json!({
            "digest": "episode_close",
            "episode": ep.id,
            "turns": digest.turns,
            "first_input": digest.first_input,
        })
        .to_string();
        let parents = remember_parents(&self.context);
        let _ = self.memory.remember(&note, &parents).await;
        Some(digest)
    }

    /// Run one full cognitive cycle
    /// Cycle-level event (ADR-0021): stage 0 = cycle level, one trace per
    /// period (job_id-derived, deterministic). ts comes from the SAME injected
    /// Clock as the ledger/stage events (极致复用 — one time source, no second
    /// clock): FakeClock makes the black box byte-identical replayable too.
    /// P10d (ADR-0032): check the agenda and execute due alarms. Each due
    /// alarm: if the action is in the configured whitelist → run the
    /// consolidate chain (action maps 1:1 to a helix_consolidate kind),
    /// then ack done. Unknown action → ack done with a warning (released,
    /// never deadlocked); failure → ack done with the error recorded
    /// (honest release; retry policy is future work — no over-engineering).
    async fn check_wakeup(&self) {
        let jitter = self.run_config.wakeup_jitter_minutes;
        match self.memory.wakeup(jitter).await {
            Ok(alarms) if !alarms.is_empty() => {
                info!("[Wakeup] {} due alarm(s)", alarms.len());
                for alarm in alarms {
                    let supported = self.run_config.wakeup_actions.contains(&alarm.action);
                    if !supported {
                        warn!(
                            "[Wakeup] unsupported action '{}' (job {}), acking done",
                            alarm.action, alarm.job_id
                        );
                        let _ = self.memory.wakeup_ack(&alarm.claim_id, "done").await;
                        continue;
                    }
                    match self.memory.consolidate(&alarm.action).await {
                        Ok(()) => {
                            info!("[Wakeup] action '{}' executed (job {})", alarm.action, alarm.job_id);
                            let _ = self.memory.wakeup_ack(&alarm.claim_id, "done").await;
                        }
                        Err(e) => {
                            warn!(
                                "[Wakeup] action '{}' failed (job {}): {} — releasing claim",
                                alarm.action, alarm.job_id, e
                            );
                            let _ = self.memory.wakeup_ack(&alarm.claim_id, "done").await;
                        }
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                debug!("[Wakeup] skipped (unavailable or failed): {}", e);
            }
        }
    }

    fn emit_cycle(&self, phase: &str, detail: &str) {
        if let Some(ring) = self.events.as_ref() {
            let mut guard = ring.lock().unwrap();
            guard.emit(
                &crate::ledger::unix_secs_to_rfc3339(self.clock.now()),
                self.context.period_id.as_deref().unwrap_or_default(),
                0,
                phase,
                detail,
            );
        }
    }


    /// ⚠️ SUPERSEDED BY §208: this entry is a SECOND NAME for one guard, and the sink into
    /// `run_cycle` itself (with a CACHED probe) is the prescribed shape — measured, `gate_ok` is
    /// a blocking TCP probe with a 10s budget, so sinking it WITHOUT a cache would move a
    /// 0.80 req/s cliff (0.5% of normal) into every period. Kept until that笔 lands.
    ///
    /// "Tuck down = Helix stops thinking" is declared fail-closed at the CLI and at the HTTP handler — measured, that was **2 of 4** loops that
    /// reach reasoning (`main.rs:621` CLI ✅, `main.rs:365` HTTP ✅, `main.rs:964` CI-144 ❌,
    /// `tests/ci144_transport.rs:152` ❌, `tests/run_cycle_pipeline.rs:157` ❌), and the ENGINE
    /// itself had no gate at all (`grep gate_ok src/run_cycle/` was empty). A declaration enforced
    /// in one place of several is not fail-closed; it is fail-closed *where someone remembered*.
    ///
    /// THREE LAYERS, THREE HOMES (§206):
    ///   CAN IT RUN   (is the gate satisfied?)  → HERE, at the engine entry, fail-closed;
    ///   HOW IT RUNS  (loop mechanics)          → a shared helper (k: 4 → 1);
    ///   WHEN IT STOPS(the policy)             → still the caller's (ADR-0016 D1, unchanged).
    /// Putting "when to stop" in the helper would make the extraction violate the very principle
    /// it cites, so the stop predicate stays with the caller.
    pub async fn run_cycle_gated(
        &mut self,
        user_input: &str,
        cfg: &crate::config::AnaphaseConfig,
    ) -> Result<CycleOutcome, String> {
        if let Err(e) = crate::health::gate_ok(cfg) {
            /* NAMED, not a sentence: "did not run" must never share a word with "ran out of
             * budget" — measured, reading the old sentence left P(truth = Tuck absent) = 0.30
             * under two names, and 1.00 with five (the naming removes the PRIOR, not just adds
             * bits). */
            return Err(format!("TUCK-GATE-REFUSED: {e}"));
        }
        self.run_cycle(user_input).await
    }

    pub async fn run_cycle(&mut self, user_input: &str) -> Result<CycleOutcome, String> {
        /* PERIOD IDENTITY AT THE CYCLE ENTRY (ADR-0048 D1/D2). It is allocated HERE — before the
         * gate check, before the first state transition — so EVERY cycle event, stage event and
         * cross-source join in this run carries one key. It replaced the input digest: measured
         * (M0.5, 2026-10-09), 462 periods came from 56 digests and one digest covered 219 runs, so
         * the Tuck chain, the ledger and the body trace merged 219 executions into one. Allocation
         * failure leaves the key EMPTY (never the digest): a fallback would make "missing" and
         * "colliding" the same value downstream. */
        self.context.period_id = crate::session_events::try_allocate_period_id(
            &crate::contract::derive_job_id(user_input),
            self.clock.now(),
        )
        .map_err(|e| warn!("[Period] no identity this cycle: {}", e))
        .ok();
        /* THE SINK (ADR-0048 §235): every path that reaches reasoning asks the judge HERE, so the
         * check cannot be forgotten by a caller — §206 measured that a declaration enforced at two
         * of four call sites is fail-closed only "where someone remembered". The cost model is the
         * breaker's (§210): O(1) and IO-free unless it is half-open; an open breaker refuses without
         * touching the downstream at all (measured <100ms).
         *
         * `Unconfigured` passes, which is today's announced-ungoverned policy, so no existing test
         * changes meaning. The refusal is NAMED; turning it into "skip and record" is the next node
         * (`gate.refusal_record`). */
        if let Err(reason) = self.gate.check() {
            /* A REFUSAL IS A DECLARED ROW, NOT A MISSING NODE (ADR-0048 §235 + §210): with the gate
             * silent, a refused period produced NO cycle event at all, so "the gate refused", "the
             * period produced nothing" and "it ran empty" looked alike. Emitting the refusal under its
             * own phase keeps the DAG readable — the panel can show WHY a period has no steps.
             * Whether one refusal ENDS the caller's loop stays the caller's policy (ADR-0016 D1):
             * that is `gate.refusal_skip`, a separate node. */
            self.emit_cycle(
                "refused",
                &format!(
                    "{{\"reason\":{}}}",
                    serde_json::to_string(&reason).unwrap_or_else(|_| "\"unprintable\"".to_string())
                ),
            );
            return Err(format!("TUCK-GATE-REFUSED: {reason}"));
        }
        self.context.user_input = user_input.to_string();
        // Time anchor (2026-09-09): the user-message arrival instant, read
        // from the single injected clock — physical fact, zero tokens.
        self.context.input_at = self.clock.now();
        // Black box (ADR-0021): a cycle begins regardless of assembly.
        self.emit_cycle(
            "begin",
            &format!(
                "{{\"input\":{},\"mode\":\"{:?}\"}}",
                serde_json::to_string(user_input).unwrap_or_default(),
                self.mode
            ),
        );
        // Advance the experience turn index (ADR-0006): each completed period
        // is one turn within the active episode.
        if let Some(ep) = self.episode.as_mut() {
            ep.step += 1;
        }

        // P10d (ADR-0032): wake-up check — look at Mind's agenda once per
        // interaction (no daemon yet; elastic window limits frequency).
        // Silent degradation: unavailable adapter or failure skips the
        // check entirely (enhancement, never a dependency).
        if self.run_config.wakeup_enabled {
            self.check_wakeup().await;
        }

        // One period (ADR-0016 D1): walk the 7-state DAG until we return to
        // Perception. The DAG is acyclic, so at most one pass per state —
        // the period-step cap is the enum length (derived, not a literal).
        // Looping policy (how many periods, when to stop) belongs to the
        // caller; this primitive is atomic and replayable.
        let mut outcome = CycleOutcome::default();
        // Set when the machine meets a (state, condition) pair with no rule. The
        // period then ends without having completed, and says so.
        let mut undefined_transition = false;
        // An impasse is a fact observed *during* the period, not a property of
        // its last step. It has to be recorded as it happens: by the time the
        // machine returns to Perception the condition in hand is the `Success`
        // from Reflection, and the impasse is already behind it.
        //
        // Recomputing `impasse` at period end from that last condition made it
        // a synonym for `!done` — true only for an undefined transition — while
        // its doc comment advertised "finished via an impasse condition". A
        // model answering `{"impasse": true}` was reported as `done, success`.
        let mut seen_impasse = false;
        for _ in 0..HelixState::ALL.len() {
            let condition = self.execute_current_state().await?;
            if condition == TransitionCondition::Impass {
                seen_impasse = true;
            }

            if let Some(next_state) = self.transitions.get(&(self.current_state.clone(), condition.clone())) {
                info!("State transition: {:?} --{:?}--> {:?}", self.current_state, condition, next_state);
                self.emit_cycle(
                    "state",
                    &format!(
                        "{{\"from\":\"{:?}\",\"condition\":\"{:?}\",\"to\":\"{:?}\"}}",
                        self.current_state, condition, next_state
                    ),
                );
                self.current_state = next_state.clone();
            } else {
                // FAIL-CLOSED (2026-09-18). This used to warn and return to
                // Perception, after which the period-end block below marked the
                // period `done` and derived success from the condition — so an
                // undefined (state, condition) pair was indistinguishable, to a
                // caller, from a period that completed normally. That is the same
                // defect as `unwrap_or("local")`: an absence reported as a
                // legitimate value.
                //
                // The table defines 12 of the 7x7 = 49 pairs. An undefined pair
                // means the machine's own description is incomplete or a state
                // returned a condition it never should, and both are conditions
                // to surface. The loop still stops — it must, or it would spin —
                // but it stops as an INCOMPLETE period, not a successful one.
                warn!(
                    "No transition rule for ({:?}, {:?}): period ends as incomplete",
                    self.current_state, condition
                );
                undefined_transition = true;
                self.emit_cycle(
                    "state",
                    &format!(
                        "{{\"from\":\"{:?}\",\"condition\":\"{:?}\",\"to\":null,\"undefined\":true}}",
                        self.current_state, condition
                    ),
                );
                self.current_state = HelixState::Perception;
            }

            // Period end: the state machine returned to Perception.
            if self.current_state == HelixState::Perception {
                // A period that ended because the machine had no rule did NOT
                // complete. Reporting `done` for it is what let an undefined
                // transition pass as a normal end.
                outcome.done = !undefined_transition;
                outcome.impasse = outcome.impasse || undefined_transition;
                // 铁律 (ADR-0029): END.success derives from the criteria
                // verdict when a tool ran — success ≡ (verdict ≠ Unmet).
                // Never from the transition alone; a tool round that failed
                // its checks is NOT a success even when the machine moved on.
                outcome.success = match &self.context.last_verdict {
                    // VerdictStatus Debug spelling ("Met"/"Unmet") — the
                    // same string the session event carries.
                    Some(v) => v == "Met",
                    None => condition == TransitionCondition::Success,
                };
                // Undefined wins: a period that ended because the machine had no
                // rule is an impasse regardless of the last condition, and the
                // assignment above must not overwrite it.
                //
                // `seen_impasse` carries the impasses the period actually passed
                // through; `undefined_transition` is the one it fell into. Both
                // are impasses, and neither may be dropped just because the last
                // step happened to be a Success.
                outcome.impasse = undefined_transition || seen_impasse;
                // ...and an incomplete period is never a success. Without this,
                // `done = false` sat next to `success = true`: the machine failed
                // to proceed yet the outcome claimed it had succeeded, which is
                // the same contradiction the fallback fix removed, one field over.
                if !outcome.done {
                    outcome.success = false;
                }
                // B22: the verdict escapes here, at the source, not at the
                // panel. `emit_cycle` below writes to the in-memory ring, which
                // is read only while a panel is open, and the session-event
                // stream exists only when `session_events_dir` is configured —
                // so an impasse reported through those two alone is an impasse
                // reported to nobody. This line goes to whatever log sink the
                // process has, panel or no panel.
                let verdict = PeriodVerdict::from_outcome(&outcome);
                if verdict.is_reportable() {
                    warn!(
                        "[Cycle] period ended without completing: reason={}, done={}, success={}, impasse={}, state={:?}, condition={:?}",
                        verdict.reason_str(),
                        verdict.done(),
                        verdict.success(),
                        verdict.impasse(),
                        self.current_state,
                        condition,
                    );
                }
                self.emit_cycle("end", &verdict.to_string());
                // Session event: the period ended (back to Perception).
                if let Some(ev) = self.session_events.as_mut() {
                    let ts = crate::ledger::unix_secs_to_rfc3339(self.clock.now());
                    // The physical model that served this period (ADR-0036):
                    // from the upstream response — the routed fact. Zero
                    // tokens; absent when the adapter never saw a response.
                    let model = self.reason.last_model();
                    // The deliverable closes the ProveTrack chain: user → think →
                    // attempt → tools → verdict → REPLY → end. Emitted even
                    // when empty (honest zero-length answer), so the chain
                    // never silently loses what Helix actually said.
                    // 两处出口共用同一判定（各写一遍 = 一物两名；改条件只改这里）。
                    let bypass_val = (self.context.rail_mode && !self.context.rail_nodes.is_empty())
                        .then(|| serde_json::json!("rail"))
                        .unwrap_or(serde_json::Value::Null);
                    let _ = ev.emit(
                        &ts,
                        crate::session_events::EventType::AssistantReply,
                        serde_json::json!({
                            "text": self.context.reasoning_output,
                            "chars": self.context.reasoning_output.chars().count(),
                            "model": model,
                            /* ★ 具名：为什么【没有】模型（第 13 条：缺失必须具名）。
                             * `model: null` 有两个完全不同的意思 —— "按设计绕过了 LLM" 与 "上游没报模型" ——
                             * 而读者无法区分（第 16 条同族：一槽两义）。
                             * 条件【与旁路自身的条件逐字相同】（reasoning.rs:94），不是新规则：
                             *   rail_mode && !rail_nodes.is_empty()  ⇒ 走的正是 rail 引用作答那条路。
                             * 实测（2026-10-09）：问「用 calc 算 1234×5678」得到 [rail citation] 且 model:null，
                             * 该轮只有 2 行事件（健康轮次 14 行）—— 静默的答非所问。 */
                            "bypass": bypass_val,
                        }),
                    );
                    let _ = ev.emit(
                        &ts,
                        crate::session_events::EventType::TurnEnd,
                        serde_json::json!({
                            "done": outcome.done,
                            "success": outcome.success,
                            "impasse": outcome.impasse,
                            "verdict": self.context.last_verdict,
                            "reply": self.context.reasoning_output,
                            "model": model,
                            "bypass": bypass_val,
                        }),
                    );
                }
                break;
            }
        }
        /* THE SNAPSHOT IS TAKEN AT THE COMPLETION POINT — after every mutation, before the caller
         * can observe anything. Reading a live object later (what callers used to do) made the
         * criterion depend on the reading instant. */
        outcome.ledger = self
            .pipeline
            .as_ref()
            .map(|p| p.ledger.records().to_vec())
            .unwrap_or_default();
        Ok(outcome)
    }

    /// Execute logic for current state and return transition condition
    async fn execute_current_state(&mut self) -> Result<TransitionCondition, String> {
        match self.current_state {
            HelixState::Perception => {
                info!("[Perception] Received input: {}", self.context.user_input);
                // O-1 (ADR-0016 D1): structured-command triage — `!tool` inputs
                // bypass the LLM entirely (0 tokens, deterministic call plan).
                if let Some(calls) = crate::contract::parse_structured_command(&self.context.user_input) {
                    self.context.calls = calls;
                    self.context.structured = true;
                    info!("[Perception] structured command detected; LLM bypassed");
                }
                Ok(TransitionCondition::Success)
            }
            HelixState::PreAssessment => {
                info!("[PreAssessment] Amygdala pre-assessment");
                // 3D emotional vector from config source (DNA principle 11).
                self.context.amygdala_vector = self.run_config.amygdala_default_vector;
                // 状态机驱动（P10b T2）：judge 后端评估复杂度 → memory.set_complexity，
                // 影响后续 query 的 suggested_mode。后端=config 选择（rules 默认 / small_llm）。
                let tier = self.judge.assess_complexity(&self.context.user_input).await;
                self.memory.set_complexity(tier);
                Ok(TransitionCondition::Success)
            }
            HelixState::MemoryRetrieval => self.arm_memory_retrieval().await,
            HelixState::Reasoning => self.arm_reasoning().await,
            HelixState::ReflexCheck => {
                info!("[ReflexCheck] Somatic reflex arc validation...");
                let action_str = self.context.suggested_actions.join(", ");
                
                // 1. Hard reflex: O(1) forbidden action check
                if !self.reflex.hard_reflex(&action_str) {
                    self.ring_reflex_block("hard rule: action matches the static deny-list");
                    return Ok(TransitionCondition::ReflexBlocked);
                }
                
                // 2. Soft reflex: fear prediction
                let context_str = format!(
                    "action: {}, vigilance: {}",
                    action_str,
                    self.context.amygdala_vector.2
                );
                match self.reflex.soft_reflex(self.fear.as_ref(), &context_str).await {
                    Ok(p_death) => {
                        self.context.p_death = p_death;
                        // Block threshold from config source (DNA principle 11).
                        if p_death > self.run_config.soft_reflex_threshold {
                            self.ring_reflex_block(&format!("soft threshold: p_death {p_death:.2}"));
                            Ok(TransitionCondition::ReflexBlocked)
                        } else {
                            info!("[ReflexCheck] Passed, p_death = {:.2}", p_death);
                            Ok(TransitionCondition::ReflexPassed)
                        }
                    }
                    Err(e) => {
                        // H5 (ruled 2026-09-18): **fail-closed.** The fear model being
                        // unavailable is not permission. This used to report
                        // `ReflexPassed` with a "default allow" warning, which made an
                        // unavailable safety check indistinguishable from a passed one.
                        //
                        // The message names its direction on purpose (P13): a fail-open
                        // and a fail-closed branch that both log at `warn!` have the same
                        // loudness and opposite meanings, so loudness alone cannot tell a
                        // reader which one fired.
                        self.ring_reflex_block(&format!("fear model unavailable: {e}"));
                        Ok(TransitionCondition::ReflexBlocked)
                    }
                }
            }
            HelixState::Execution => {
                info!("[Execution] Executing tool call...");
                // candidate E (ADR-0005): a structured plan with a wired
                // pipeline takes the deterministic path (stages 3-4). Without
                // either, the legacy string/echo path stays (backwards compat).
                if self.pipeline.is_some() && !self.context.calls.is_empty() {
                    // O-1: pipeline resolved at startup, but the physical probe
                    // says the tentacle is dark — log the degradation fact.
                    match self.context.ecosystem.status("tentacle") {
                        Some(crate::gloves::GloveStatus::Unavailable) => {
                            warn!("[Execution] tentacle dark at runtime; pipeline path may fail-open");
                        }
                        _ => {}
                    }
                    return self.execute_structured().await;
                }
                let action_str = self.context.suggested_actions.join(", ");
                // M1.5-T6: resolved real tool name; placeholder from config
                // source (DNA principle 11 / ADR-0005) when unset.
                let command = self.tool_command.as_deref()
                    .unwrap_or(self.run_config.execution_placeholder.as_str());
                // HITL 执行闸（P10b T3，DNA 原则 4）：低风险 → 放行；高风险 → 人类确认
                // 工具审计（safety，原则 5 扩展点）。两者都在 safety_gate 里，
                // 因为 structured 路径对同一情况的答案相反（K-033）。
                let actions = [action_str.clone()];
                let verdict = safety_gate::admit(
                    &self.hitl,
                    self.safety.as_ref(),
                    command,
                    &actions,
                    // K-033: this path reports success when the audit cannot run.
                    safety_gate::OnAuditError::ReportSuccess,
                )
                .await;
                match verdict {
                    safety_gate::GateVerdict::Refused(condition) => Ok(condition),
                    safety_gate::GateVerdict::Cleared => {
                        // Execute tool
                        match self.tool.execute(command, &[action_str.clone()]).await {
                            Ok(result) => {
                                info!("[Execution] Execution result: {}", result);
                                self.emit_cycle(
                                    "tool",
                                    &format!(
                                        "{{\"tool\":{},\"ok\":true,\"result\":{}}}",
                                        serde_json::to_string(command).unwrap_or_default(),
                                        serde_json::to_string(&result).unwrap_or_default()
                                    ),
                                );
                                Ok(TransitionCondition::Success)
                            }
                            Err(e) => {
                                warn!("[Execution] Execution failed: {}", e);
                                self.emit_cycle(
                                    "tool",
                                    &format!(
                                        "{{\"tool\":{},\"ok\":false,\"error\":{}}}",
                                        serde_json::to_string(command).unwrap_or_default(),
                                        serde_json::to_string(&e).unwrap_or_default()
                                    ),
                                );
                                Ok(TransitionCondition::Failure)
                            }
                        }
                    }
                }
            }
            HelixState::Reflection => self.arm_reflection().await,
        }
    }

    /// Structured execution path (candidate E, ADR-0005): pipeline stage 3
    /// (gRPC execute) + stage 4 (evidence record). The HITL execution gate
    /// (DNA principle 4) and the tool audit gate (principle 5) still apply per
    /// planned call — low-risk tools pass through with zero extra delay.
    /// ADR-0049 D4 — the D6 drill-down, answered locally from the append-only store.
    /// `None` means "not a reading instruction": the caller keeps its normal path.
    fn answer_read_instruction(&self) -> Option<String> {
        let call = self.context.calls.first()?;
        let dir = self.session_events_dir.as_ref()?;
        let cfg = &self.convergence;
        let now = self.clock.now();
        let arg = call.args.get("0").and_then(|v| v.as_str()).unwrap_or("");
        // 理由：`!reject <id> <理由…>` —— 位置参数 1..n 拼回一句人话。
        let reason = (1..call.args.len())
            .filter_map(|i| call.args.get(&i.to_string()).and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join(" ");
        let leaf = self.context.resume_period.as_deref().unwrap_or(arg);
        match call.tool.as_str() {
            "lodes" => Some(cv_answer_lodes(dir, Some(leaf), cfg, now)),
            "lode" => Some(cv_answer_lode(dir, arg, cfg, now)),
            "sediment" => Some(cv_answer_sediment(dir, leaf, cfg, now)),
            /* 一次**具名用户动作**的落点。不是模型判断 —— 3B 上"自己决定该吸收还是驳回"
             * 会通过简单测试、在真实使用里才崩，那是装饰性绿灯。 */
            "reject" => Some(crate::session_events::convergence::answer_reject(
                dir,
                arg,
                &reason,
                "human",
                // when 来自**注入的时钟**（不是 SystemTime::now）：侧车因此不参与字节级回放。
                &crate::ledger::unix_secs_to_rfc3339(now),
            )),
            "revoke" => Some(crate::session_events::convergence::answer_revoke(
                dir,
                arg,
                &reason,
                "human",
                &crate::ledger::unix_secs_to_rfc3339(now),
            )),
            "settle" => match crate::session_events::convergence::write_state(
                dir, arg, crate::session_events::PeriodStatus::Converged)
            {
                Ok(()) => Some(format!("已定型 {arg}（status=converged，显式）")),
                Err(e) => Some(format!("(定型失败：{e})")),
            },
            "body" => crate::session_events::convergence::retrieve_body(dir, arg).map(|b| {
                format!("[full round {arg} — read from its append-only event stream]\n{b}")
            }),
            _ => None,
        }
    }

    async fn execute_structured(&mut self) -> Result<TransitionCondition, String> {
        for c in &self.context.calls {
            /* THE ARGS MUST REACH THE JUDGEMENT (ADR-0048 §225 F3): this was `&[]` verbatim —
             * measured, so the human-confirmation request carried a tool name with an EMPTY
             * parameter column, and the risk predicate never saw the arguments. The signature
             * already took `&[String]`; it was simply passed nothing. */
            let args_json = serde_json::to_string(&c.args).unwrap_or_default();
            match safety_gate::admit(
                &self.hitl,
                self.safety.as_ref(),
                &c.tool,
                &[args_json],
                // K-033: this path blocks when the audit cannot run. The legacy
                // path above reports success for the same situation.
                safety_gate::OnAuditError::Block,
            )
            .await
            {
                safety_gate::GateVerdict::Cleared => {}
                safety_gate::GateVerdict::Refused(condition) => return Ok(condition),
            }
        }
        let job = match &self.context.job {
            Some(job) => job.clone(),
            None => {
                warn!("[Execution] Structured calls without assembled envelope");
                return Ok(TransitionCondition::Failure);
            }
        };
        // identity_labels: none for run_cycle — protocol default empty map
        // (ADR-0004 semantics; zero hardcoding, DNA principle 11).
        let labels: BTreeMap<String, String> = BTreeMap::new();
        let Some(pipeline) = self.pipeline.as_mut() else {
            warn!("[Execution] Structured path without wired pipeline");
            return Ok(TransitionCondition::Failure);
        };
        match pipeline.execute_calls(&job, &labels).await {
            Ok(records) => {
                // Session events: one tool/call + tool/result pair per
                // executed call (redacted on write). The evidence rows carry
                // tool, args shape, result and duration — the turn timeline
                // renders execution from this single source.
                if let Some(ev) = self.session_events.as_mut() {
                    let ts = crate::ledger::unix_secs_to_rfc3339(self.clock.now());
                    for r in &records {
                        let _ = ev.emit(
                            &ts,
                            crate::session_events::EventType::ToolCall,
                            serde_json::json!({
                                "tool": r.tool,
                                "index": r.call_index,
                                "expect": r.expect,
                            }),
                        );
                        let _ = ev.emit(
                            &ts,
                            crate::session_events::EventType::ToolResult,
                            serde_json::json!({
                                "tool": r.tool,
                                "index": r.call_index,
                                "ok": r.ok,
                                "duration_ms": r.duration_ms,
                                "outcome": r.data,
                                "outcome_sha": crate::trace::short_sha(&r.data),
                            }),
                        );
                    }
                }
                pipeline.record_evidence(records.clone());
                let ids: Vec<String> = records.iter().map(|r| r.evidence_id.clone()).collect();
                self.context.evidence = records;
                info!("[Execution] Pipeline executed {} call(s): {:?}", ids.len(), ids);
                Ok(TransitionCondition::Success)
            }
            Err(e) => {
                warn!("[Execution] Pipeline execution failed: {}", e);
                Ok(TransitionCondition::Failure)
            }
        }
    }
}


mod usage;
pub mod verdict;
#[cfg(test)]
mod verdict_tests;
mod reasoning;
mod reflection;
mod memory_retrieval;
mod safety_gate;
#[cfg(test)]
mod safety_gate_tests;

pub use verdict::{EndReason, PeriodVerdict};

#[cfg(test)]
mod tests;
