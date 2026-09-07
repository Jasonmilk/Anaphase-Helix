use crate::adapters::*;
use crate::config::{Mode, RunCycleConfig};
use crate::contract::{derive_episode_id, parse_reasoning_output, Call, TtJob};
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
    /// L0 identity + L1 tool awareness (gene_lock.md + Tentacle manifests),
    /// assembled once at build time, injected ahead of the user input every
    /// cycle. Empty = honest degraded state (no identity, no tools).
    pub identity_block: String,
    /// O-6 (ADR-0024): judge-point backend — complexity assessment for the
    /// Amygdala -> suggested_mode chain. Rules by default (zero tokens);
    /// SmallLlm (3B-class) when configured. Always returns 1/2/3.
    pub judge: std::sync::Arc<dyn crate::judge::Judge>,
    /// Reasoning body trace (Engram join, 2026-09-07): append-only JSONL of
    /// every reasoning round trip (prompt + response, redacted + truncated).
    /// The audit chain stores metadata; this stores the *body*, on the side
    /// that constructs the prompt. None = trace off (opt-in, config
    /// `reasoning_trace_path`). `trace_id` = the derived job id, joining
    /// body + chain + ledger in Cellrix's Engram view.
    pub trace: Option<crate::trace::ReasoningTrace>,
    /// Session event stream directory (Engram turn timeline, ADR-0023):
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

/// Context data flowing through the cognitive cycle
#[derive(Debug, Clone, Default)]
pub struct AgentContext {
    pub user_input: String,
    pub amygdala_vector: (f64, f64, f64),  // (heliotropism, pulse, vigilance)
    pub memory_nodes: Vec<MemoryNode>,
    /// Explicit continuation (2026-09-07): when the panel asks to resume a
    /// previous experience (`job_id`), the last round's summary is injected
    /// here as true history — the new period opens as a continuation, not a
    /// fresh stranger.
    pub resume: Option<String>,
    pub reasoning_output: String,
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
    /// SA-Core choice detail for the Engram cockpit (ADR-0033): which layers
    /// were picked and the top-heat nodes — provenance only (id/tier/heat/
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
                    "heat": (n.heat * 100.0).round() / 100.0,
                    "phase": n.phase,
                })
            })
            .collect();
        top.sort_by(|a, b| {
            b["heat"]
                .as_f64()
                .unwrap_or(0.0)
                .partial_cmp(&a["heat"].as_f64().unwrap_or(0.0))
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
        transitions.insert((HelixState::ReflexCheck, TransitionCondition::ReflexPassed), HelixState::Execution);
        transitions.insert((HelixState::ReflexCheck, TransitionCondition::ReflexBlocked), HelixState::Reflection);
        transitions.insert((HelixState::Execution, TransitionCondition::Success), HelixState::Reflection);
        transitions.insert((HelixState::Execution, TransitionCondition::Failure), HelixState::Reflection);
        transitions.insert((HelixState::Reflection, TransitionCondition::Success), HelixState::Perception);

        Self {
            identity_block: String::new(),
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
        let _ = self.memory.remember(&note).await;
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
                warn!("No transition rule found: ({:?}, {:?}), returning to Perception", self.current_state, condition);
                self.current_state = HelixState::Perception;
            }

            // Period end: the state machine returned to Perception.
            if self.current_state == HelixState::Perception {
                outcome.done = true;
                outcome.success = condition == TransitionCondition::Success;
                outcome.impasse = condition == TransitionCondition::Impass;
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
                    let _ = ev.emit(
                        &ts,
                        crate::session_events::EventType::TurnEnd,
                        serde_json::json!({
                            "done": outcome.done,
                            "success": outcome.success,
                            "impasse": outcome.impasse,
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
                // L0 (gene lock) + L1 (tool awareness): the immutable identity
                // and the on-demand tool list lead every cycle, so Helix knows
                // who it is and what it can do before it thinks.
                let prompt = if !self.identity_block.is_empty() {
                    format!("{}

{}", self.identity_block, self.context.user_input)
                } else {
                    self.context.user_input.clone()
                };
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
                // pipeline events — one join key across all three (Engram).
                let trace_id = crate::contract::derive_job_id(&self.context.user_input);
                // Session event stream (Engram turn timeline): open the
                // per-period stream and emit the period header — turn/start,
                // user/message, context/inject (summary only). The stream is
                // keyed by the same derived job id as the body trace and the
                // Tuck audit chain, so the client joins all three on it.
                self.session_events = self
                    .session_events_dir
                    .as_ref()
                    .and_then(|dir| {
                        crate::session_events::SessionEventStream::open(
                            dir.clone(),
                            &trace_id,
                            self.session_events_redact.clone(),
                        )
                        .ok()
                    });
                let detail = self.memory_choice_detail();
                if let Some(ev) = self.session_events.as_mut() {
                    let ts = crate::ledger::unix_secs_to_rfc3339(self.clock.now());
                    let _ = ev.emit_period_start(
                        &ts,
                        &self.context.user_input,
                        self.context.memory_nodes.len(),
                        self.memory_inject_chars,
                        self.context.resume.as_deref(),
                        detail.as_ref(),
                    );
                }
                // Streaming when a delta sink is attached (SSE chat); the
                // buffered path otherwise — one contract, two transports.
                let streamed = match &self.stream_tx {
                    Some(tx) => {
                        self.reason
                            .reason_stream(
                                &prompt,
                                &self.run_config.reasoning_mode,
                                &trace_id,
                                tx.clone(),
                            )
                            .await
                    }
                    None => {
                        self.reason
                            .reason(&prompt, &self.run_config.reasoning_mode, &trace_id)
                            .await
                    }
                };
                match streamed {
                    Ok(output) => {
                        // Body trace (Engram join): record the round trip
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
                            let _ = trace.record(&ts, &trace_id, &model, &prompt, &output);
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
                                serde_json::json!({ "text": output }),
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
                                // Unstructured conversational output: no plan.
                                warn!("[Reasoning] Unstructured output (no calls plan): {}", e);
                                self.context.reasoning_output = output;
                                Ok(TransitionCondition::NoToolNeeded)
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Reasoning failed: {}", e);
                        Ok(TransitionCondition::Impass)
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
                        // Session event: the criteria verdict (MET/UNMET/
                        // blocked) — the turn timeline's terminal judgement.
                        if let Some(ev) = self.session_events.as_mut() {
                            let ts = crate::ledger::unix_secs_to_rfc3339(self.clock.now());
                            let _ = ev.emit(
                                &ts,
                                crate::session_events::EventType::Verdict,
                                serde_json::json!({ "job_id": job_id, "status": status }),
                            );
                        }
                        info!("[Reflection] Ledger verdict written for job {}", job_id);
                    }
                    // Human-readable reply: replace the plan JSON with the tool
                    // result summary — the user asked a question, not for a
                    // call plan. The LLM's original output stays in the body
                    // trace (Engram) for audit; this is the answer surface.
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
                        self.context.reasoning_output = lines.join("
");
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
                        self.memory.remember(&structured).await
                    }
                    None => self.memory.remember(&self.context.reflection_notes).await,
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
                                "ok": r.ok,
                                "duration_ms": r.duration_ms,
                                "data": r.data,
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

#[cfg(test)]
mod tests {
    use super::*;
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
        async fn remember(&self, _c: &str) -> Result<(), String> {
            Ok(())
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
            heat: 1.0,
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

}
