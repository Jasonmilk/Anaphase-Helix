//! The Reflection state arm (ADR-0042 pure move).
//!
//! **What this owns**: turning an executed cycle into a verdict — memory
//! consolidation, the criteria check, the verdict ledger, the user-facing answer and
//! the ProveTrack trace. It contains the bounded verdict-retry loop.
//!
//! **What this does NOT own**: the transition table, the loop, and where to go next.
//! It returns a `TransitionCondition`; `mod.rs` decides.
//!
//! **Changing X? Look here** if X is what Helix concludes, what it remembers, what the
//! ledger records, or how a verdict is retried. Look in `mod.rs` if X is which state
//! follows.
//!
//! This header is not decoration: a split only reduces what a change must read if the
//! reader can tell which file to open. Comments do not count toward NCLOC.

use super::{remember_parents, usage, AgentLoop, TransitionCondition};
use crate::contract::{contains_tool_request, contradicts_evidence, parse_reasoning_output};
use crate::pipeline::Pipeline;
use tracing::{debug, info, trace, warn};

impl AgentLoop {
    /// One entry in the state machine: Reflection.
    pub(super) async fn arm_reflection(&mut self) -> Result<TransitionCondition, String> {
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
