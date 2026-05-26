use crate::adapters::*;
use crate::reflex::ReflexArc;
use crate::states::HelixState;
use std::sync::Arc;
use std::collections::HashMap;
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
}

/// Context data flowing through the cognitive cycle
#[derive(Debug, Clone, Default)]
pub struct AgentContext {
    pub user_input: String,
    pub amygdala_vector: (f64, f64, f64),  // (heliotropism, pulse, vigilance)
    pub memory_nodes: Vec<String>,
    pub reasoning_output: String,
    pub suggested_actions: Vec<String>,
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
        }
    }

    /// Run one full cognitive cycle
    pub async fn run_cycle(&mut self, user_input: &str) -> Result<(), String> {
        self.context.user_input = user_input.to_string();
        
        // Max 7 loops to prevent infinite cycles
        for _ in 0..7 {
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
                // 3D emotional vector calculation (minimal implementation)
                self.context.amygdala_vector = (0.7, 0.3, 0.2); // Default: positive, calm, relaxed
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
                match self.reason.reason(&self.context.user_input, "left_brain").await {
                    Ok(output) => {
                        // Fix: Check tool needed before moving the value
                        let tool_needed = output.contains("tool_call") || output.contains("python") || output.contains("cli");
                        let impasse_detected = output.contains("impasse") || output.contains("unknown");
                        
                        self.context.reasoning_output = output;
                        
                        if tool_needed {
                            Ok(TransitionCondition::NeedsTool)
                        } else if impasse_detected {
                            Ok(TransitionCondition::Impass)
                        } else {
                            Ok(TransitionCondition::NoToolNeeded)
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
                        if p_death > 0.7 {
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
                // Safety audit
                let action_str = self.context.suggested_actions.join(", ");
                match self.safety.audit("execute", &action_str).await {
                    Ok(true) => {
                        // Execute tool
                        match self.tool.execute("echo", &[action_str.clone()]).await {
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
            HelixState::Reflection => {
                info!("[Reflection] Memory consolidation...");
                self.context.reflection_notes = format!(
                    "Cycle completed. p_death: {:.2}, impasse: {}",
                    self.context.p_death,
                    self.context.memory_nodes.len()
                );
                // Write to L3 episodic memory
                let _ = self.memory.remember(&self.context.reflection_notes).await;
                Ok(TransitionCondition::Success)
            }
        }
    }
}
