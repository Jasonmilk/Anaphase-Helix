//! The MemoryRetrieval state arm (ADR-0042 pure move).
//!
//! **What this owns**: querying memory for this turn, folding the retrieved nodes into
//! the prompt under the injection budget, and recording which nodes were used so the
//! episode can name its parents.
//!
//! **What this does NOT own**: the transition table, the loop, and where to go next.
//!
//! **Changing X? Look here** if X is which memories are retrieved, how they are folded
//! into the prompt, or the injection budget. Look in `mod.rs` if X is which state
//! follows.
//!
//! This header is what makes the split reduce anything: a reader who cannot tell which
//! file to open reads all of them, and then the reduction is zero. Comments are free —
//! they do not count toward NCLOC.

use super::AgentLoop;
use super::TransitionCondition;
use tracing::{debug, info, trace, warn};

impl AgentLoop {
    /// One entry in the state machine: MemoryRetrieval.
    pub(super) async fn arm_memory_retrieval(&mut self) -> Result<TransitionCondition, String> {
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
}
