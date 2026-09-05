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
    /// Active experience boundary (ADR-0006). None = no episode in progress
    /// (legacy turn-by-turn behavior, fully backwards compatible).
    pub episode: Option<Episode>,
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
    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
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
    pub async fn run_cycle(&mut self, user_input: &str) -> Result<(), String> {
        self.context.user_input = user_input.to_string();
        // Advance the experience turn index (ADR-0006): each completed cycle
        // is one turn within the active episode.
        if let Some(ep) = self.episode.as_mut() {
            ep.step += 1;
        }
        
        // Loop cap from config (DNA principle 11): prevents infinite cycles.
        for _ in 0..self.run_config.cycle_cap {
            let condition = self.execute_current_state().await?;
            
            if let Some(next_state) = self.transitions.get(&(self.current_state.clone(), condition.clone())) {
                info!("State transition: {:?} --{:?}--> {:?}", self.current_state, condition, next_state);
                self.current_state = next_state.clone();
            } else {
                warn!("No transition rule found: ({:?}, {:?}), returning to Perception", self.current_state, condition);
                self.current_state = HelixState::Perception;
            }
            
            // End cycle after returning to Perception from Reflection
            if self.current_state == HelixState::Perception && condition == TransitionCondition::Success {
                break;
            }
        }
        Ok(())
    }

    /// Execute logic for current state and return transition condition
    async fn execute_current_state(&mut self) -> Result<TransitionCondition, String> {
        match self.current_state {
            HelixState::Perception => {
                info!("[Perception] Received input: {}", self.context.user_input);
                // Basic intent parsing (extendable with NER later)
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
                                        let created_at = unix_secs_to_rfc3339(p.ledger.clock_now());
                                        self.context.job = Some(Pipeline::assemble_tt_job(
                                            &job_id,
                                            &created_at,
                                            sig.calls.clone(),
                                        ));
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
                                        Ok(TransitionCondition::Success)
                                    }
                                    Err(e) => {
                                        warn!("[Execution] Execution failed: {}", e);
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
                        let reports = Pipeline::check_results(&self.context.evidence, &pipeline.config.rules);
                        let evidence_ids: Vec<String> = self
                            .context
                            .evidence
                            .iter()
                            .map(|r| r.evidence_id.clone())
                            .collect();
                        let job_id = self
                            .context
                            .job
                            .as_ref()
                            .map(|j| j.job_id.clone())
                            .unwrap_or_default();
                        let verdict = pipeline.build_verdict(&job_id, evidence_ids, &reports, None);
                        pipeline.ledger.append(verdict);
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
