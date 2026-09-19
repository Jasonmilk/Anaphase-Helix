//! The Reasoning state arm (ADR-0042 pure move).
//!
//! **What this owns**: turning assembled context into a `ReasoningSignal` — either a
//! structured call plan, or a plain answer, or an impasse — including the bounded
//! direct-answer retry when thinking ate the token budget (ADR-0034) and the usage
//! metering for each round trip (ADR-0038).
//!
//! **What this does NOT own**: the transition table, the loop, and the decision of
//! where to go next. It returns a `TransitionCondition`; `mod.rs` decides.
//!
//! **Changing X? Look here** if X is the reasoning prompt, the retry policy, the
//! structured-plan assembly, or `p_last`/usage reporting. Look in `mod.rs` if X is
//! which state follows.
//!
//! That paragraph is the point of the split, not decoration: an 85% reduction in what
//! a change must read only materialises if a reader can tell which file to open. Nine
//! unnamed files are read in full, and then the reduction is zero. Comments do not
//! count toward NCLOC, so this costs nothing in the budget.

use super::{fold_memory_nodes, usage, AgentLoop, TransitionCondition};
use crate::contract::parse_reasoning_output;
use crate::ledger::unix_secs_to_rfc3339;
use crate::pipeline::Pipeline;
use tracing::{debug, info, trace, warn};

impl AgentLoop {
    /// One entry in the state machine: Reasoning.
    pub(super) async fn arm_reasoning(&mut self) -> Result<TransitionCondition, String> {
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
}
