//! M1 deterministic pipeline (ADR-0003).
//!
//! Single-pass, replayable pipeline with six independent stages — no giant
//! `run()` blob (ADR-0003 decision 11 / v2.3 Δ17):
//!
//! | stage | fn | kind |
//! |---|---|---|
//! | 1. parse LLM calls | `contract::parse_llm_calls` | pure |
//! | 2. assemble tt_job | `Pipeline::assemble_tt_job` | pure |
//! | 3. gRPC execute | `Pipeline::execute_calls` | IO (Tentacle) |
//! | 4. evidence record | `Pipeline::record_evidence` | in-memory (serialized by caller) |
//! | 5. criteria check | `Pipeline::check_results` | pure |
//! | 6. verdict ledger | `Pipeline::build_verdict` | in-memory (serialized by caller) |
//!
//! File IO (writing evidence.jsonl / ledger.jsonl) is the caller's job — the
//! pipeline owns the state machine + serialization, keeping it replayable.

use crate::contract::{derive_seen_bloom, parse_llm_calls, Call, TtJob};
use crate::criteria::{run_for_expect, CheckReport, RuleParams};
use crate::evidence::{EvidenceRecord, EvidenceStore};
use crate::ledger::{Clock, Ledger, LedgerRecord, SystemClock, VerdictStatus};
use crate::security::{GateCheck, GateVerdict, SecurityGate};
use crate::adapters::tentacle::GrpcTentacleAdapter;
use serde::Deserialize;
use std::sync::Arc;

/// Retry scheduling policy (from fixture-codex.json).
#[derive(Debug, Clone, Deserialize)]
pub struct RetryPolicy {
    pub base_delay_secs: u64,
}

/// Pipeline configuration, loaded from `knowledge_base/fixture-codex.json`.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub rules: RuleParams,
    pub retry_policy: RetryPolicy,
}

#[derive(Debug, Deserialize)]
struct Codex {
    rules: RuleParams,
    retry_policy: RetryPolicy,
}

impl PipelineConfig {
    /// Load rules + retry policy from the codex contract file (zero hardcoding).
    pub fn from_codex(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("read codex {path}: {e}"))?;
        let codex: Codex = serde_json::from_str(&content)
            .map_err(|e| format!("parse codex {path}: {e}"))?;
        Ok(Self {
            rules: codex.rules,
            retry_policy: codex.retry_policy,
        })
    }
}

/// Input to one pipeline run. `job_id` / `created_at` are caller-supplied
/// (envelope assembled by the pipeline, ADR-0003 decision 5). `identity_labels`
/// are caller-identity labels forwarded to Tentacle for audit / disclosure
/// (ADR-0004); BTreeMap keeps the input deterministic.
pub struct PipelineInput {
    pub job_id: String,
    pub created_at: String,
    pub llm_content: String,
    pub identity_labels: std::collections::BTreeMap<String, String>,
}

/// Result of one pipeline run.
#[derive(Debug, Clone)]
pub struct PipelineOutcome {
    pub job_id: String,
    pub evidence_ids: Vec<String>,
    pub check_reports: Vec<CheckReport>,
    pub verdict: VerdictStatus,
    pub retry_due: Option<u64>,
}

pub struct Pipeline {
    pub tentacle: GrpcTentacleAdapter,
    pub config: PipelineConfig,
    pub evidence: EvidenceStore,
    pub ledger: Ledger,
    /// Optional security gate (ADR-0008 D'-2). `None` = legacy behavior.
    pub security_gate: Option<Arc<dyn SecurityGate>>,
}

impl Pipeline {
    pub fn new(
        tentacle: GrpcTentacleAdapter,
        clock: Box<dyn Clock>,
        config: PipelineConfig,
    ) -> Self {
        // The clock lives in the ledger; the pipeline reads it via the ledger
        // so a single injectable time source drives the whole run.
        let ledger = Ledger::new(clock);
        Self { tentacle, config, evidence: EvidenceStore::new(), ledger, security_gate: None }
    }

    /// Inject a security gate (ADR-0008). `None` restores legacy behavior.
    pub fn with_security_gate(mut self, gate: Option<Arc<dyn SecurityGate>>) -> Self {
        self.security_gate = gate;
        self
    }

    // ---- stage 2: pure ----

    /// Assemble the tt_job envelope around the LLM-produced calls.
    pub fn assemble_tt_job(job_id: &str, created_at: &str, calls: Vec<Call>) -> TtJob {
        TtJob { job_id: job_id.to_string(), created_at: created_at.to_string(), calls }
    }

    // ---- stage 3: IO (gRPC) ----

    /// Execute every call against Tentacle, returning one evidence record each.
    /// trace_id is derived as `{job_id}#{index}` (deterministic, ADR-0003).
    /// identity_labels are forwarded per job (ADR-0004).
    pub async fn execute_calls(
        &mut self,
        job: &TtJob,
        identity_labels: &std::collections::BTreeMap<String, String>,
    ) -> Result<Vec<EvidenceRecord>, String> {
        let mut records = Vec::with_capacity(job.calls.len());
        for (i, call) in job.calls.iter().enumerate() {
            let params = serde_json::to_string(&call.args)
                .map_err(|e| format!("serialize args: {e}"))?;
            let trace_id = format!("{}#{i}", job.job_id);

            // Security gate before execution (ADR-0008 D'-2). Reject/HITL
            // block the call and write a `blocked` ledger record — the
            // action never reaches Tentacle.
            if let Some(gate) = &self.security_gate {
                let verdict = gate
                    .check(&GateCheck {
                        job_id: job.job_id.clone(),
                        index: i as u32,
                        tool: call.tool.clone(),
                        args_json: params.clone(),
                        identity_labels: identity_labels.clone(),
                    })
                    .await;
                if !verdict.permits() {
                    let reason = match &verdict {
                        GateVerdict::Reject(r) | GateVerdict::HitlRequired(r) => r.clone(),
                        _ => "blocked by security gate".to_string(),
                    };
                    self.ledger.append(LedgerRecord::blocked(
                        &job.job_id,
                        &call.tool,
                        i as u32,
                        &reason,
                        identity_labels.get("identity").map(|s| s.as_str()),
                    ));
                    return Err(format!("blocked by security gate: {reason}"));
                }
            }

            let started = std::time::Instant::now();
            let labels: std::collections::HashMap<String, String> =
                identity_labels.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            let resp = self
                .tentacle
                .execute_tool_with_labels(
                    &call.tool,
                    &params,
                    &trace_id,
                    labels,
                    derive_seen_bloom(&call.tool, &params), // ADR-0007 D'-1: real entropy fingerprint, not the "" placeholder
                )
                .await?;
            let duration_ms = started.elapsed().as_millis() as u64;
            let record = if resp.ok {
                EvidenceRecord::new(
                    &job.job_id,
                    i as u32,
                    &call.tool,
                    call.expect.clone(),
                    true,
                    &resp.data,
                    duration_ms,
                )
            } else {
                EvidenceRecord::new(
                    &job.job_id,
                    i as u32,
                    &call.tool,
                    call.expect.clone(),
                    false,
                    &resp.error,
                    duration_ms,
                )
            };
            records.push(record);
        }
        Ok(records)
    }

    // ---- stage 4: record evidence (in-memory) ----

    /// Append executed records into the evidence store.
    pub fn record_evidence(&mut self, records: Vec<EvidenceRecord>) {
        for r in records {
            self.evidence.append(r);
        }
    }

    // ---- stage 5: pure ----

    /// Check all evidence records against the criteria set mapped by `expect`.
    /// Failed executions (ok=false) yield a failed report (fail-closed).
    pub fn check_results(records: &[EvidenceRecord], rules: &RuleParams) -> Vec<CheckReport> {
        let mut reports = Vec::new();
        for r in records {
            let data: serde_json::Value = if r.ok {
                serde_json::from_str(&r.data).unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            };
            let mut rs = run_for_expect(&r.expect, &data, rules);
            if !r.ok {
                // The execution itself failed: fail every mapped check.
                for rep in rs.iter_mut() {
                    rep.passed = false;
                    rep.detail = format!("{} (execution failed: {})", rep.detail, r.data);
                }
            }
            reports.append(&mut rs);
        }
        reports
    }

    // ---- stage 6: build verdict (in-memory) ----

    /// Build the ledger verdict: MET when every check passes, otherwise UNMET
    /// with `retry_due = now + base_delay` and `parent_id` lineage (M1 writes,
    /// M1.5 consumes).
    pub fn build_verdict(
        &self,
        job_id: &str,
        evidence_ids: Vec<String>,
        reports: &[CheckReport],
        parent_id: Option<String>,
    ) -> LedgerRecord {
        let all_passed = reports.iter().all(|r| r.passed);
        if all_passed {
            LedgerRecord::met(job_id, evidence_ids, reports.to_vec())
        } else {
            let retry_due = self.ledger.clock_now() + self.config.retry_policy.base_delay_secs;
            LedgerRecord::unmet(job_id, evidence_ids, reports.to_vec(), retry_due, parent_id)
        }
    }

    // ---- orchestration: composes the six stages (not a giant blob) ----

    pub async fn run(&mut self, input: PipelineInput) -> Result<PipelineOutcome, String> {
        // stage 1 (pure)
        let calls = parse_llm_calls(&input.llm_content)?;
        // stage 2 (pure)
        let job = Self::assemble_tt_job(&input.job_id, &input.created_at, calls);
        // stage 3 (IO)
        let records = self.execute_calls(&job, &input.identity_labels).await?;
        // stage 4 (in-memory)
        let evidence_ids: Vec<String> = records.iter().map(|r| r.evidence_id.clone()).collect();
        self.record_evidence(records);
        // stage 5 (pure)
        let reports = Self::check_results(self.evidence.records(), &self.config.rules);
        // stage 6 (in-memory)
        let verdict = self.build_verdict(&job.job_id, evidence_ids.clone(), &reports, None);
        let (verdict_status, retry_due) = match &verdict {
            LedgerRecord::Verdict { status, retry_due, .. } => (status.clone(), *retry_due),
            // Blocked records are appended inside execute_calls and short-circuit
            // with an Err before stage 6 — unreachable here by construction.
            LedgerRecord::Blocked { .. } => unreachable!("blocked records only arise from the security gate path"),
        };
        self.ledger.append(verdict);

        Ok(PipelineOutcome {
            job_id: job.job_id,
            evidence_ids: evidence_ids.clone(),
            check_reports: reports,
            verdict: verdict_status,
            retry_due,
        })
    }
}

/// Bootstrap assembly of the deterministic execution channel (ADR-0007 D'-3):
/// fail-open — an empty endpoint or a failed connection yields None (legacy
/// echo fallback keeps working), never a startup error. Mirrors the
/// `resolve_memory_adapter` pattern (DNA principle 6 fail-open).
pub async fn resolve_pipeline(
    endpoint: Option<String>,
    config: PipelineConfig,
) -> Option<Pipeline> {
    let endpoint = match endpoint {
        Some(e) if !e.is_empty() => e,
        _ => return None,
    };
    match GrpcTentacleAdapter::new(&endpoint).await {
        Ok(tentacle) => Some(Pipeline::new(tentacle, Box::new(SystemClock), config)),
        Err(e) => {
            eprintln!("Warning: failed to connect to Tentacle at {endpoint}: {e}. Falling back to legacy execution.");
            None
        }
    }
}
