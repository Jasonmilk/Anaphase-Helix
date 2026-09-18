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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CycleOutcome {
    /// Period finished (state machine returned to Perception).
    pub done: bool,
    /// Finished via a Success transition.
    pub success: bool,
    /// Finished via an impasse condition.
    pub impasse: bool,
}

impl Default for CycleOutcome {
    fn default() -> Self {
        Self { done: false, success: false, impasse: false }
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
    pub pipeline: Option<Pipeline>,
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
fn remember_parents(context: &AgentContext) -> Vec<String> {
    context.memory_nodes.iter().map(|n| n.id.clone()).collect()
}

/// Context data flowing through the cognitive cycle
#[derive(Debug, Clone, Default)]
pub struct AgentContext {
    pub user_input: String,
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
                &crate::contract::derive_job_id(&self.context.user_input),
                0,
                phase,
                detail,
            );
        }
    }


    pub async fn run_cycle(&mut self, user_input: &str) -> Result<CycleOutcome, String> {
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
        for _ in 0..HelixState::ALL.len() {
            let condition = self.execute_current_state().await?;

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
                outcome.impasse =
                    undefined_transition || condition == TransitionCondition::Impass;
                // ...and an incomplete period is never a success. Without this,
                // `done = false` sat next to `success = true`: the machine failed
                // to proceed yet the outcome claimed it had succeeded, which is
                // the same contradiction the fallback fix removed, one field over.
                if !outcome.done {
                    outcome.success = false;
                }
                self.emit_cycle(
                    "end",
                    &format!(
                        "{{\"done\":{},\"success\":{},\"impasse\":{}}}",
                        outcome.done, outcome.success, outcome.impasse
                    ),
                );
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
                    let _ = ev.emit(
                        &ts,
                        crate::session_events::EventType::AssistantReply,
                        serde_json::json!({
                            "text": self.context.reasoning_output,
                            "chars": self.context.reasoning_output.chars().count(),
                            "model": model,
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
                        }),
                    );
                }
                break;
            }
        }
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
            HelixState::MemoryRetrieval => {
                info!("[MemoryRetrieval] Querying memory: {}", self.context.user_input);
                // Rails branch (ADR-0018): external human asset, read-only,
                // deterministic. When the query lands on a rail, verbatim
                // nodes are injected and the citation contract (rail_mode)
                // is set — Helix may only cite, never synthesize. Runs
                // before the mind memory query; the two knowledge lines are
                // disjoint (human asset vs Helix experience).
                if let Some(index) = self.rails.as_ref() {
                    let nav = crate::rails::navigate(
                        index,
                        &self.context.user_input,
                        self.rails_config.max_hits,
                    );
                    if !nav.hits.is_empty() {
                        let mut nodes: Vec<crate::rails::Node> = Vec::new();
                        let mut bytes = 0usize;
                        for id in &nav.hits {
                            if let Some(node) = index.nodes.get(id) {
                                if bytes + node.content.len() > self.rails_config.max_inject_bytes {
                                    break;
                                }
                                bytes += node.content.len();
                                nodes.push(node.clone());
                            }
                        }
                        if !nodes.is_empty() {
                            self.context.rail_nodes = nodes;
                            self.context.rail_mode = true;
                            info!(
                                "[MemoryRetrieval] rails hit ({} nodes, {} bytes)",
                                self.context.rail_nodes.len(),
                                bytes
                            );
                        }
                    }
                }
                match self.memory.query(&self.context.user_input, false).await {
                    Ok(result) => {
                        self.context.memory_nodes = result.nodes;
                        self.context.suggested_actions = result.suggested_actions;
                        // P10a (ADR-0031): on-demand cognitive craft —
                        // deterministic zero-token orchestration BEFORE the
                        // LLM reasoning step. Triggered by physical ability
                        // (GrpcMindAdapter implements craft; Noop returns
                        // Err and the loop skips — enhancement, never a
                        // dependency). Structured commands (!) skip thinking:
                        // the plan already exists. Degradation is silent:
                        // craft failure leaves craft_note = None.
                        let preview: Vec<String> = self
                            .context
                            .memory_nodes
                            .iter()
                            .take(2)
                            .map(|n| {
                                let cut: String = n.content.chars().take(100).collect();
                                cut
                            })
                            .collect();
                        info!(
                            "[MemoryRetrieval] {} memory node(s): {:?}",
                            self.context.memory_nodes.len(),
                            preview
                        );
                        if !self.context.structured {
                            let job_id = crate::contract::derive_job_id(&self.context.user_input);
                            match self.memory.craft(&self.context.user_input, &job_id).await {
                                Ok(note) => {
                                    let synth: String = note.synthesis.chars().take(120).collect();
                                    info!("[MemoryRetrieval] craft note (0 tokens): {}", synth);
                                    self.context.craft_note = Some(note);
                                }
                                Err(e) => {
                                    self.context.craft_note = None;
                                    trace!("[MemoryRetrieval] craft skipped: {}", e);
                                }
                            }
                        }
                        if result.impasse_level > 2 {
                            Ok(TransitionCondition::Failure)
                        } else {
                            Ok(TransitionCondition::Success)
                        }
                    }
                    Err(e) => {
                        warn!("Memory retrieval failed: {}", e);
                        Ok(TransitionCondition::Success)
                    }
                }
            }
            HelixState::Reasoning => {
                // O-1 (ADR-0016 D1): structured commands never reach the LLM —
                // the plan already exists, assemble the job and go.
                if self.context.structured {
                    if let Some(p) = self.pipeline.as_ref() {
                        let job_id = crate::contract::derive_job_id(&self.context.user_input);
                        // stage events (ADR-0019): stage1 = call parsing
                        // (structured triage happened in Perception — 0 tokens),
                        // stage2 = tt_job assembly (Reasoning tail).
                        p.emit_event(&job_id, 1, "begin", "structured");
                        p.emit_event(&job_id, 1, "end", &format!("calls={}", self.context.calls.len()));
                        let created_at = unix_secs_to_rfc3339(p.ledger.clock_now());
                        p.emit_event(&job_id, 2, "begin", "assemble");
                        self.context.job = Some(Pipeline::assemble_tt_job(
                            &job_id,
                            &created_at,
                            self.context.calls.clone(),
                        ));
                        p.emit_event(&job_id, 2, "end", "job=assembled");
                    }
                    return Ok(TransitionCondition::NeedsTool);
                }
                // O-1 (ADR-0016 D3): upgrade-to-LLM sensing point — look at the
                // pocket once before spending tokens (physical facts only).
                info!(
                    "[Reasoning] ecosystem: {:?}",
                    self.context.ecosystem.list()
                );
                // Rails citation contract (ADR-0018): when the query landed on
                // a rail, the answer is assembled verbatim from the injected
                // nodes — 0 tokens, no LLM, no synthesis possible by
                // construction. Helix selects an existing rail edge (a node);
                // it never generates one. The verifier (verify_reference) is
                // trivially satisfied because the answer IS the rail text.
                if self.context.rail_mode && !self.context.rail_nodes.is_empty() {
                    let answer = crate::rails::assemble_rail_answer(
                        &self.context.rail_nodes,
                        &self.rails_config.kb_dir,
                    );
                    self.context.reasoning_output = answer;
                    self.context.calls.clear();
                    info!("[Reasoning] rail citation answer (0 tokens, LLM bypassed)");
                    return Ok(TransitionCondition::NoToolNeeded);
                }
                info!("[Reasoning] Left-brain reasoning...");
                info!(
                    "[Reasoning] inject_chars={} memory_nodes={}",
                    self.memory_inject_chars,
                    self.context.memory_nodes.len()
                );
                // Time anchor (2026-09-09, 0 tokens): the message-arrival
                // instant and the current period instant, rendered in the
                // host's local time with a numeric offset — the human's
                // physical clock is the anchor (UTC would misorder days for
                // an Asia/Shanghai user). Helix answers with a temporal
                // anchor ("when did they say it", "when am I answering"),
                // so memories stay ordered by physical time.
                let anchor = format!(
                    "\n[time anchor — physical clock, 0 tokens]\nuser message at {}\nnow: {}",
                    crate::ledger::unix_secs_to_human_local(self.context.input_at),
                    crate::ledger::unix_secs_to_human_local(self.clock.now()),
                );
                // Identity (L0 gene lock + L1 tools) rides the reasoning
                // **system** channel — assembled once at build time, sent by
                // the adapter as the system message (authoritative identity).
                // The user channel carries only this round's cognition:
                // time anchor + memory + resume + craft + the input itself.
                let prompt = format!("{}

{}", anchor, self.context.user_input);
                // O-5 (ADR-0023): on-demand injection — the request carries
                // only what this round needs. Memory nodes (retrieved in
                // MemoryRetrieval, previously never consumed by the LLM) are
                // folded into the prompt up to the budget; 0 = stateless.
                let prompt = if self.memory_inject_chars == 0 {
                    prompt
                } else {
                    let inject = fold_memory_nodes(
                        &self.context.memory_nodes,
                        self.memory_inject_chars,
                    );
                    if inject.is_empty() {
                        prompt
                    } else {
                        format!("{}
\n[memory: Helix's past experiences — true history, answer from them]\n{}", prompt, inject)
                    }
                };
                // Explicit continuation (2026-09-07): resume a previous
                // experience as true history — the new period continues the
                // conversation instead of meeting a stranger.
                let prompt = match self.context.resume.as_ref() {
                    Some(r) => format!(
                        "{}
\n[previous episode — true history of this conversation's last round]\n{}",
                        prompt, r
                    ),
                    None => prompt,
                };
                // P10a (ADR-0031): fold the cognitive craft note (zero-token
                // deterministic orchestration from Mind) into the prompt —
                // think first, then spend tokens. None = no note (degraded or
                // unavailable); the note is bounded by construction (synthesis
                // is fixed-shape), so no extra budget knob is needed.
                let prompt = match self.context.craft_note.as_ref() {
                    Some(note) => format!("{}
\n[think-first (deterministic, 0 tokens)]\n{}", prompt, note.synthesis),
                    None => prompt,
                };
                // One derived trace id for this round: carried to the gateway
                // (x-tuck-trace -> Tuck chain), to the body trace, and to the
                // pipeline events — one join key across all three (ProveTrack).
                let trace_id = crate::contract::derive_job_id(&self.context.user_input);
                // Session event stream (ProveTrack turn timeline): open the
                // per-period stream and emit the period header — turn/start,
                // user/message, context/inject (summary only). `job_id` stays
                // the join key shared with the body trace and the Tuck audit
                // chain; `period_id` is what makes THIS run a distinct period
                // (K-006) — two runs of one input must not share a file.
                // `period_id` is what makes THIS run a distinct period (K-006):
                // two runs of one input must not share a file. The allocator
                // rejects a job id it cannot digest rather than repairing it
                // (B17'), and a rejection here means the identity layer has no
                // valid id to allocate — so the period stream stays unopened
                // rather than being opened under a wrong key.
                match crate::session_events::try_allocate_period_id(
                    &trace_id,
                    self.clock.now(),
                ) {
                    Ok(period_id) => {
                        self.session_events = self
                            .session_events_dir
                            .as_ref()
                            .and_then(|dir| {
                                crate::session_events::SessionEventStream::open(
                                    dir.clone(),
                                    &period_id,
                                    &trace_id,
                                    self.session_events_redact.clone(),
                                )
                                .ok()
                            });
                    }
                    Err(e) => {
                        warn!("[SessionEvents] no period stream this cycle: {}", e);
                        self.session_events = None;
                    }
                }
                let detail = self.memory_choice_detail();
                if let Some(ev) = self.session_events.as_mut() {
                    let ts = crate::ledger::unix_secs_to_rfc3339(self.clock.now());
                    // resume_from = machine-readable parent job id when this
                    // period continues a previous one (ProveTrack threading).
                    //
                    // It previously fell back to `context.resume` — the
                    // HUMAN-READABLE continuation summary — for callers that sent
                    // no job id. That wrote a paragraph into a parent-pointer slot:
                    // measured on the live stream, 12 of 139 periods carried prose
                    // (or an arbitrary caller-supplied string) as their `parent`,
                    // which orphaned their threads (83 apparent roots) and made list
                    // grouping look like a wrong MODEL when the INPUT was corrupt.
                    //
                    // The summary has its own carrier (`resume`, injected into the
                    // prompt) and does not belong in lineage. Absent job id =>
                    // absent parent; the reader enforces this too, because history
                    // is append-only and the old rows stay as they were written.
                    let resume_from = self.context.resume_job.as_deref();
                    let _ = ev.emit_period_start(
                        &ts,
                        &self.context.user_input,
                        self.context.memory_nodes.len(),
                        self.memory_inject_chars,
                        resume_from,
                        detail.as_ref(),
                    );
                }
                // Streaming when a delta sink is attached (SSE chat); the
                // buffered path otherwise — one contract, two transports.
                // ADR-0034: thinking shares the output token budget with the
                // answer; a reasoning model may spend the whole budget on
                // hidden reasoning and return empty content (DeepSeek-family
                // known behaviour — the mature-client answer is a large
                // budget plus a bounded direct-answer retry). Retry budget
                // comes from RunCycleConfig (0 = never retry).
                let retries = self.run_config.empty_reply_retries;
                let output: String;
                let mut attempt = 0u32;
                let mut effective_prompt = prompt;
                let thinking_sink = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
                loop {
                    let streamed = match &self.stream_tx {
                        Some(tx) => {
                            self.reason
                                .reason_stream(
                                    &effective_prompt,
                                    &self.run_config.reasoning_mode,
                                    &trace_id,
                                    tx.clone(),
                                    thinking_sink.as_ref(),
                                )
                                .await
                        }
                        None => {
                            self.reason
                                .reason(&effective_prompt, &self.run_config.reasoning_mode, &trace_id)
                                .await
                        }
                    };
                    match streamed {
                        Ok(o) => {
                            let empty = o.trim().is_empty();
                            // ADR-0038: meter this round trip BEFORE deciding
                            // whether to retry. A retried call was billed too,
                            // and the think/attempt rows further down sit
                            // outside this loop, so they only ever describe
                            // the surviving attempt.
                            usage::emit_usage(self);
                            if !empty || attempt >= retries {
                                output = o;
                                break;
                            }
                            // Budget starvation: the retry drops the reasoning
                            // demand so the answer gets the freed budget.
                            attempt += 1;
                            info!(
                                "[Reasoning] empty reply (thinking ate budget) — retry {}/{} with direct-answer directive",
                                attempt, retries
                            );
                            thinking_sink.lock().unwrap().clear();
                            effective_prompt = format!(
                                "{}\n\n[direct answer required — output your final answer directly, no reasoning]",
                                effective_prompt
                            );
                        }
                        Err(e) => {
                            warn!("Reasoning failed: {}", e);
                            self.context.reasoning_output.clear();
                            self.context.calls.clear();
                            return Ok(TransitionCondition::Impass);
                        }
                    }
                }
                // Private reasoning (ADR-0029): keep it on the context
                // and persist an `assistant/think` event (redacted on
                // write, display-only — criteria never sees it).
                self.context.reasoning_think =
                    thinking_sink.lock().unwrap().clone();
                        if let Some(ev) = self.session_events.as_mut() {
                            let ts = crate::ledger::unix_secs_to_rfc3339(self.clock.now());
                            let _ = ev.emit(
                                &ts,
                                crate::session_events::EventType::Think,
                                serde_json::json!({ "text": self.context.reasoning_think }),
                            );
                        }
                        // Body trace (ProveTrack join): record the round trip
                        // post-redaction/post-truncation. The trace id is the
                        // derived job id — the same key the pipeline events
                        // and the Tuck audit chain carry, so Cellrix joins
                        // body + chain by it. Timestamp from the injected
                        // clock (deterministic replay). Record failure is
                        // non-fatal: the cognitive loop must not die on a
                        // trace write.
                        if let Some(trace) = self.trace.as_ref() {
                            let ts = crate::ledger::unix_secs_to_rfc3339(self.clock.now());
                            let model = self.run_config.reasoning_mode.clone();
                            let _ = trace.record(&ts, &trace_id, &model, &effective_prompt, &output);
                        }
                        // Session event: the Reasoning attempt (redacted on
                        // write). The full prompt/response body stays in the
                        // reasoning trace; the event carries the output so a
                        // client can render the turn without opening trace.
                        if let Some(ev) = self.session_events.as_mut() {
                            let ts = crate::ledger::unix_secs_to_rfc3339(self.clock.now());
                            let _ = ev.emit(
                                &ts,
                                crate::session_events::EventType::Attempt,
                                serde_json::json!({
                                    "text": output,
                                    // ADR-0034: honest terminal — after the
                                    // bounded retry the reply may still be
                                    // empty (model refused/starved); the
                                    // client renders a clear hint instead of
                                    // pretending a blank line is an answer.
                                    "empty": output.trim().is_empty(),
                                }),
                            );
                        }
                        // candidate E (ADR-0005): structured output protocol
                        // replaces the legacy contains("tool_call") matching.
                        // parse_reasoning_output yields the calls plan + an
                        // explicit impasse flag (see docs/contracts/).
                        match parse_reasoning_output(&output) {
                            Ok(sig) => {
                                self.context.reasoning_output = output;
                                self.context.calls = sig.calls.clone();
                                if !sig.calls.is_empty() {
                                    // stage 2 (Reasoning tail): assemble the
                                    // deterministic tt_job envelope when a
                                    // pipeline is wired (job_id derived from
                                    // input, created_at from the injected clock).
                                    if let Some(p) = self.pipeline.as_ref() {
                                        let job_id = crate::contract::derive_job_id(&self.context.user_input);
                                        // stage events (ADR-0019): stage1 =
                                        // parse_llm_calls completed, stage2 =
                                        // tt_job assembly (Reasoning tail).
                                        p.emit_event(&job_id, 1, "begin", "llm");
                                        p.emit_event(&job_id, 1, "end", &format!("calls={}", sig.calls.len()));
                                        let created_at = unix_secs_to_rfc3339(p.ledger.clock_now());
                                        p.emit_event(&job_id, 2, "begin", "assemble");
                                        self.context.job = Some(Pipeline::assemble_tt_job(
                                            &job_id,
                                            &created_at,
                                            sig.calls.clone(),
                                        ));
                                        p.emit_event(&job_id, 2, "end", "job=assembled");
                                    }
                                    Ok(TransitionCondition::NeedsTool)
                                } else if sig.impasse {
                                    Ok(TransitionCondition::Impass)
                                } else {
                                    Ok(TransitionCondition::NoToolNeeded)
                                }
                            }
                            Err(e) => {
                                // P0-D-1 (2026-09-16): fail closed on contract
                                // violations. A plan-shaped reply that the
                                // schema rejects (e.g. malformed calls) must
                                // surface as an explicit failure — never leak
                                // the raw JSON as a reply. Free-form prose
                                // (invalid JSON) stays the legitimate
                                // no-plan path.
                                if e.starts_with("calls schema mismatch") {
                                    warn!("[Reasoning] Tool plan rejected (fail-closed, P0-D-1): {}", e);
                                    self.context.reasoning_output = output;
                                    return Ok(TransitionCondition::Failure);
                                }
                                // Unstructured conversational output: no plan.
                                warn!("[Reasoning] Unstructured output (no calls plan): {}", e);
                                self.context.reasoning_output = output;
                                Ok(TransitionCondition::NoToolNeeded)
                            }
                        }
            }
            HelixState::ReflexCheck => {
                info!("[ReflexCheck] Somatic reflex arc validation...");
                let action_str = self.context.suggested_actions.join(", ");
                
                // 1. Hard reflex: O(1) forbidden action check
                if !self.reflex.hard_reflex(&action_str) {
                    warn!("[ReflexCheck] Hard reflex blocked! Action forbidden: {}", action_str);
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
                            warn!("[ReflexCheck] Soft reflex blocked! p_death = {:.2}", p_death);
                            Ok(TransitionCondition::ReflexBlocked)
                        } else {
                            info!("[ReflexCheck] Passed, p_death = {:.2}", p_death);
                            Ok(TransitionCondition::ReflexPassed)
                        }
                    }
                    Err(e) => {
                        warn!("[ReflexCheck] Fear prediction failed, default allow: {}", e);
                        Ok(TransitionCondition::ReflexPassed)
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
                match self.hitl.check_approval(command, &[action_str.clone()]) {
                    Ok(true) => {
                        // 放行 → 工具审计（safety，原则 5 扩展点）
                        match self.safety.audit("execute", &action_str).await {
                            Ok(true) => {
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
                            Ok(false) => {
                                warn!("[Execution] Safety audit rejected");
                                Ok(TransitionCondition::Failure)
                            }
                            Err(e) => {
                                warn!("[Execution] Safety audit failed, fallback allow: {}", e);
                                Ok(TransitionCondition::Success)
                            }
                        }
                    }
                    Ok(false) => {
                        warn!("[Execution] HITL rejected: high-risk action blocked");
                        Ok(TransitionCondition::Failure)
                    }
                    Err(e) => {
                        warn!("[Execution] HITL unavailable, high-risk blocked (fail-closed): {}", e);
                        Ok(TransitionCondition::Failure)
                    }
                }
            }
            HelixState::Reflection => {
                info!("[Reflection] Memory consolidation...");
                // candidate E (ADR-0005): stages 5-6 — criteria check + verdict
                // ledger — when this cycle executed a structured plan.
                if !self.context.evidence.is_empty() {
                    if let Some(pipeline) = self.pipeline.as_mut() {
                        let job_id = self
                            .context
                            .job
                            .as_ref()
                            .map(|j| j.job_id.clone())
                            .unwrap_or_default();
                        // stage events (ADR-0019): stage5 = criteria checks,
                        // stage6 = verdict ledger write (Reflection tail).
                        pipeline.emit_event(&job_id, 5, "begin", "criteria");
                        let reports = Pipeline::check_results(&self.context.evidence, &pipeline.config.rules);
                        pipeline.emit_event(&job_id, 5, "end", &format!("reports={}", reports.len()));
                        let evidence_ids: Vec<String> = self
                            .context
                            .evidence
                            .iter()
                            .map(|r| r.evidence_id.clone())
                            .collect();
                        pipeline.emit_event(&job_id, 6, "begin", "ledger");
                        let verdict = pipeline.build_verdict(&job_id, evidence_ids, &reports, None);
                        let status = match &verdict {
                            crate::ledger::LedgerRecord::Verdict { status, .. } => {
                                format!("{status:?}")
                            }
                            crate::ledger::LedgerRecord::Blocked { .. } => "blocked".to_string(),
                        };
                        pipeline.ledger.append(verdict);
                        pipeline.emit_event(&job_id, 6, "end", &format!("verdict={status}"));
                        // Session events (ADR-0029): one check/status row per
                        // deterministic report (judge/gate/expect/actual/
                        // reason — no bare labels), then the derived verdict
                        // with its reason summary. END.success derives from
                        // this status below.
                        if let Some(ev) = self.session_events.as_mut() {
                            let ts = crate::ledger::unix_secs_to_rfc3339(self.clock.now());
                            for (i, rep) in reports.iter().enumerate() {
                                let _ = ev.emit(
                                    &ts,
                                    crate::session_events::EventType::Check,
                                    serde_json::json!({
                                        "check_id": format!("{job_id}#c{i}"),
                                        "check": rep.check,
                                        "passed": rep.passed,
                                        "judge": rep.judge,
                                        "gate": rep.gate,
                                        "expect": rep.expect,
                                        "evidence_id": rep.evidence_id,
                                        "actual": rep.detail,
                                        "reason": rep.detail,
                                    }),
                                );
                            }
                            let reason = if reports.is_empty() {
                                "no checks (no tool ran)".to_string()
                            } else {
                                let failed: Vec<&str> = reports
                                    .iter()
                                    .filter(|r| !r.passed)
                                    .map(|r| r.check.as_str())
                                    .collect();
                                if failed.is_empty() {
                                    "all checks passed".to_string()
                                } else {
                                    format!("failed: {}", failed.join(", "))
                                }
                            };
                            let _ = ev.emit(
                                &ts,
                                crate::session_events::EventType::Verdict,
                                serde_json::json!({
                                    "job_id": job_id,
                                    "status": status,
                                    "reason": reason,
                                    "checks": reports.len(),
                                }),
                            );
                        }
                        self.context.last_verdict = Some(status.clone());
                        info!("[Reflection] Ledger verdict written for job {}", job_id);
                    }
                    // Human-readable reply: replace the plan JSON with the tool
                    // result summary — the user asked a question, not for a
                    // call plan. The LLM's original output stays in the body
                    // trace (ProveTrack) for audit; this is the answer surface.
                    let lines: Vec<String> = self
                        .context
                        .evidence
                        .iter()
                        .map(|e| {
                            let body = if e.ok { &e.data } else { "execution failed" };
                            format!("{}: {}", e.tool, body)
                        })
                        .collect();
                    if !lines.is_empty() {
                        // Finalize (ADR-0036): tool evidence is NOT the
                        // deliverable — the deliverable is Helix's answer built
                        // from it. One cheap final call organizes the facts into
                        // a natural-language reply; on any failure we degrade to
                        // the raw evidence echo (never fabricate, never go
                        // silent).
                        //
                        // F1 (2026-09-17): the acceptance guard used to be
                        // "non-empty", which accepted a SECOND tool plan as the
                        // reply. The panel then rendered the human raw
                        // `{"calls":[...]}` JSON while the verdict still read
                        // Met — `answer.delivered` inspects the TOOL's return
                        // ("delivery confirmed at tool edge"), never the answer
                        // the human received. Measured on the live store: 9 of
                        // 50 periods shipped a raw plan, every one of them a
                        // tool turn. P0-D-1 (2026-09-16) already refuses this
                        // shape in Reasoning — "never leak the raw JSON as a
                        // reply" — and the same rule now holds here.
                        //
                        // F2: a follow-up plan normally asks for a REFINED
                        // query. Executing it here was rejected on evidence:
                        // `trace_id` is `{job_id}#{index}` with `index` local to
                        // the plan, and `record_evidence` appends without
                        // dedup, so a second plan under one job_id would collide
                        // on both evidence_id and trace_id — and one trace per
                        // period is a hard contract (ADR-0019/0026). Offsetting
                        // the index means changing the pipeline's signature, an
                        // architecture change that needs its own ADR. So the
                        // bounded follow-up is a RE-ASK instead: the results are
                        // already in hand, the directive forbids further tools,
                        // and a model that still answers with a plan degrades to
                        // the honest evidence echo rather than leaking JSON.
                        let mut rounds = 0u32;
                        let mut accepted: Option<String> = None;
                        // Set when the reply contradicted a tool's own scalar
                        // result; carried into the re-ask so the directive can
                        // state the authoritative value instead of only asking
                        // for prose.
                        let mut ground_truth: Option<String> = None;
                        loop {
                            let strict = rounds > 0;
                            let finalize_prompt = if strict {
                                let must_use = match &ground_truth {
                                    Some(v) => format!(
                                        "\nA tool returned this exact value: {v}. It is authoritative — reproduce it verbatim (its digits must appear unchanged) and do not recompute or reformat it."
                                    ),
                                    None => String::new(),
                                };
                                format!(
                                    "Original question: {}\nTool results:\n{}\n\nThe tool calls above have ALREADY been executed. You cannot call a tool again. Answer the user's question now, in natural language, using only these results. If they are insufficient, say plainly what is missing. No JSON, no tool calls, no internal format.{}",
                                    self.context.user_input,
                                    lines.join("\n"),
                                    must_use
                                )
                            } else {
                                format!(
                                    "Original question: {}\nTool results:\n{}\n\nAnswer the user's question directly in natural language, concise, no JSON, no internal format.",
                                    self.context.user_input,
                                    lines.join("\n")
                                )
                            };
                            let finalized = self
                                .reason
                                .reason(
                                    &finalize_prompt,
                                    &self.run_config.reasoning_mode,
                                    &self.context.job.as_ref().map(|j| j.job_id.clone()).unwrap_or_default(),
                                )
                                .await;
                            // ADR-0038: the finalize call is a second upstream
                            // round trip inside the same period — meter it too,
                            // or its cost vanishes from the period total.
                            usage::emit_usage(self);
                            let Ok(answer) = finalized else { break };
                            if answer.trim().is_empty() {
                                break;
                            }
                            // F1: a plan is not an answer — in ANY shape. The
                            // canonical check is kept (it also sees `impasse`),
                            // and `contains_tool_request` widens it to the
                            // shapes the model actually emits, e.g. prose
                            // followed by `Tool: {"tool": ...}`, which both
                            // parsers used to miss (measured live: the search it
                            // asked for never ran and the text became the reply).
                            let is_plan = contains_tool_request(answer.trim())
                                || matches!(
                                    parse_reasoning_output(answer.trim()),
                                    Ok(sig) if !sig.calls.is_empty()
                                );
                            // F2: the reply must not CONTRADICT the evidence it
                            // was built from. Measured live: `calc` returned
                            // "35184372088832", the reply said
                            // "35,184,372,088,32", and the verdict still read Met
                            // — because `answer.delivered` inspects the TOOL's
                            // return, never the answer the human received. A
                            // reply that misstates its own evidence is not a
                            // deliverable; the honest outcome is the evidence
                            // echo, which carries the value verbatim.
                            let contradiction = self.context.evidence.iter().find_map(|e| {
                                if e.ok {
                                    contradicts_evidence(&e.data, answer.trim())
                                } else {
                                    None
                                }
                            });
                            if is_plan || contradiction.is_some() {
                                if rounds < self.run_config.tool_followup_rounds {
                                    rounds += 1;
                                    if let Some(v) = &contradiction {
                                        warn!(
                                            "[Reflection] finalize reply contradicts its evidence (authoritative {} is absent) — re-asking (round {}/{})",
                                            v, rounds, self.run_config.tool_followup_rounds
                                        );
                                        ground_truth = contradiction.clone();
                                    } else {
                                        warn!(
                                            "[Reflection] finalize returned a tool plan, not an answer — re-asking (round {}/{})",
                                            rounds, self.run_config.tool_followup_rounds
                                        );
                                    }
                                    continue;
                                }
                                if let Some(v) = &contradiction {
                                    warn!(
                                        "[Reflection] finalize still contradicts its evidence (authoritative {} absent) after {} follow-up round(s) — degrading to the evidence echo",
                                        v, rounds
                                    );
                                } else {
                                    warn!(
                                        "[Reflection] finalize still returned a tool plan after {} follow-up round(s) — refusing to leak it (P0-D-1)",
                                        rounds
                                    );
                                }
                                break;
                            }
                            accepted = Some(answer);
                            break;
                        }
                        match accepted {
                            Some(answer) => {
                                self.context.reasoning_output = answer;
                                info!(
                                    "[Reflection] finalize ok: chars={}",
                                    self.context.reasoning_output.chars().count()
                                );
                            }
                            None => {
                                self.context.reasoning_output = lines.join("\n");
                                info!("[Reflection] finalize degraded to evidence echo");
                            }
                        }
                    } else {
                        info!("[Reflection] reply NOT replaced: evidence empty");
                    }
                }
                // L3 episodic note (P10): the EXPERIENCE, not the bookkeeping.
                // Previously only "Cycle completed. p_death: ..." was written —
                // the loop's ledger line, not the conversation — so nothing
                // retrievable about the human ever reached Mind. The L3
                // record is what happened this round: what the human said,
                // plus the minimal cognitive state that gives the note its
                // provenance. Retrieval (tokenized) then finds it verbatim.
                self.context.reflection_notes = format!(
                    "User said: {}\nCycle completed. p_death: {:.2}, impasse: {}",
                    self.context.user_input,
                    self.context.p_death,
                    self.context.memory_nodes.len()
                );
                // Write to L3 episodic memory. Within an active episode the
                // note carries the experience provenance `{episode_id}#{step}`
                // as a structured JSON field (ADR-0006 D1) — Mind's L3
                // `content: JSON` keeps structured records, no schema change.
                // Without an episode the note is written verbatim (legacy
                // behaviour, strictly backwards compatible).
                let _ = match &self.episode {
                    Some(ep) => {
                        let structured = serde_json::json!({
                            "episode": format!("{}#{}", ep.id, ep.step),
                            "note": self.context.reflection_notes,
                        })
                        .to_string();
                        let parents = remember_parents(&self.context);
                        self.memory.remember(&structured, &parents).await
                    }
                    None => {
                        let parents = remember_parents(&self.context);
                        self.memory.remember(&self.context.reflection_notes, &parents).await
                    }
                };
                Ok(TransitionCondition::Success)
            }
        }
    }

    /// Structured execution path (candidate E, ADR-0005): pipeline stage 3
    /// (gRPC execute) + stage 4 (evidence record). The HITL execution gate
    /// (DNA principle 4) and the tool audit gate (principle 5) still apply per
    /// planned call — low-risk tools pass through with zero extra delay.
    async fn execute_structured(&mut self) -> Result<TransitionCondition, String> {
        for c in &self.context.calls {
            match self.hitl.check_approval(&c.tool, &[]) {
                Ok(true) => {}
                _ => {
                    warn!("[Execution] HITL blocked tool: {}", c.tool);
                    return Ok(TransitionCondition::Failure);
                }
            }
            match self.safety.audit("execute", &c.tool).await {
                Ok(true) => {}
                _ => {
                    warn!("[Execution] Safety audit blocked tool: {}", c.tool);
                    return Ok(TransitionCondition::Failure);
                }
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

#[cfg(test)]
mod tests;
