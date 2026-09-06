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
use tracing::{info, warn};

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
}

/// Context data flowing through the cognitive cycle
#[derive(Debug, Clone, Default)]
pub struct AgentContext {
    pub user_input: String,
    pub amygdala_vector: (f64, f64, f64),  // (heliotropism, pulse, vigilance)
    pub memory_nodes: Vec<String>,
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
}

impl AgentLoop {
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
            hitl: HITLApprover::default(),
            tool_command: None,
            run_config: RunCycleConfig::default(),
            pipeline: None,
            mode: Mode::Partner,
            episode: None,
            events: None,
            rails: None,
            rails_config: crate::config::RailsConfig::default(),
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
    /// period (job_id-derived, deterministic). ts is the wall clock (audit
    /// truth for the black box); the ledger/stage replay contract stays with
    /// the injected FakeClock — two different determinism needs, two sources.
    fn emit_cycle(&self, phase: &str, detail: &str) {
        if let Some(ring) = self.events.as_ref() {
            let mut guard = ring.lock().unwrap();
            guard.emit(
                &chrono::Utc::now().to_rfc3339(),
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
                // 状态机驱动（P10b T2）：Amygdala 启发式复杂度评估 → memory.set_complexity，
                // 影响后续 query 的 suggested_mode。0=未知走兜底。
                self.memory.set_complexity(assess_complexity(&self.context.user_input));
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
                match self.reason.reason(&self.context.user_input, &self.run_config.reasoning_mode).await {
                    Ok(output) => {
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
                        info!("[Reflection] Ledger verdict written for job {}", job_id);
                    }
                }
                self.context.reflection_notes = format!(
                    "Cycle completed. p_death: {:.2}, impasse: {}",
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

/// Amygdala 启发式复杂度评估（P10b T2）：PreAssessment 状态输出 → suggested_mode 状态驱动。
/// 1=简单 / 2=中等 / 3=复杂。当前为 query 特征启发式（P10b 最小正确）；
/// 未来独立 `amygdala.rs` 时，此处可替换为多维评估（意图/情感/历史）。0 不返回（状态机必输出 1-3）。
fn assess_complexity(query: &str) -> u8 {
    let len = query.trim().chars().count();
    if len <= 10 {
        1
    } else if len < 40 {
        2
    } else {
        3
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
        async fn reason(&self, input: &str, mode: &str) -> Result<String, String> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(format!("{{\"calls\":[],\"impasse\":false}} // {}", input))
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
}
