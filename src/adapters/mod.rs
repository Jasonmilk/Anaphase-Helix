pub mod mind;
pub mod flowmodus;
pub mod tentacle;
// New: Declare HTTP reasoning adapter module
pub mod http_reasoning;

use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::sync::Arc;

// ---------- Memory Adapter ----------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub nodes: Vec<String>,
    pub impasse_level: u8,
    pub suggested_actions: Vec<String>,
}

/// Cognitive craft outcome (P10a, ADR-0031): the deterministic orchestration
/// synthesis from Mind's helix_craft. Injected into the Reasoning prompt as a
/// zero-token "think first" note.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CraftNote {
    /// Deterministic trace (craft#{job_id}), replayable.
    pub trace_id: String,
    /// Hegelian convergence synthesis (the orchestration result).
    pub synthesis: String,
    /// P10b fills value_grade; empty until then (honest).
    pub value_grade: String,
}

/// One due alarm handed to the waker (P10d, ADR-0032): the semantic action
/// Anaphase will execute (action → consolidate kind), plus the deterministic
/// claim handle used for the ack.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WakeupAlarm {
    pub job_id: String,
    pub action: String,
    pub due_at: String,
    pub mode: String,
    pub claim_id: String,
}

#[async_trait]
pub trait MemoryAdapter: Send + Sync {
    async fn query(&self, query: &str, include_recessive: bool) -> Result<QueryResult, String>;
    async fn remember(&self, content: &str) -> Result<(), String>;
    /// 状态驱动钩子（P10b T2）：Amygdala PreAssessment 输出复杂度（1=简单/2=中等/3=复杂），
    /// adapter 据此调整 `suggested_mode`。默认实现为空操作（Noop 等忽略）。
    fn set_complexity(&self, _level: u8) {}
    /// P10a (ADR-0031): cognitive craft trigger — deterministic orchestration
    /// before the LLM reasoning step (zero tokens). Default = unavailable
    /// (Noop and other adapters ignore; GrpcMindAdapter implements). Callers
    /// degrade silently on Err: craft is an enhancement, never a dependency.
    async fn craft(&self, _query: &str, _job_id: &str) -> Result<CraftNote, String> {
        Err("craft unavailable".to_string())
    }
    /// P10d (ADR-0032): wake-up check — list due alarms from Mind's agenda
    /// (ana_wakeup). Default = unavailable (Noop ignores; GrpcMindAdapter
    /// implements). Callers degrade silently on Err: wake-up is an
    /// enhancement, never a dependency.
    async fn wakeup(&self, _jitter_minutes: u32) -> Result<Vec<WakeupAlarm>, String> {
        Err("wakeup unavailable".to_string())
    }
    /// P10d (ADR-0032): acknowledge a claimed alarm (ana_wakeup_ack).
    /// Default = unavailable; GrpcMindAdapter implements.
    async fn wakeup_ack(&self, _claim_id: &str, _status: &str) -> Result<(), String> {
        Err("wakeup_ack unavailable".to_string())
    }
    /// P10d (ADR-0032): run a Mind metabolism action (helix_consolidate).
    /// Default = unavailable; GrpcMindAdapter implements.
    async fn consolidate(&self, _kind: &str) -> Result<(), String> {
        Err("consolidate unavailable".to_string())
    }
}

pub struct NoopMemoryAdapter;

#[async_trait]
impl MemoryAdapter for NoopMemoryAdapter {
    async fn query(&self, _query: &str, _include_recessive: bool) -> Result<QueryResult, String> {
        Ok(QueryResult { nodes: vec![], impasse_level: 0, suggested_actions: vec![] })
    }
    async fn remember(&self, _content: &str) -> Result<(), String> {
        Ok(())
    }
}

/// 解析记忆适配器（DNA 铁律 6：所有依赖必须有降级策略，fail-open）。
/// - `mind_endpoint` 为空/未配置 → `NoopMemoryAdapter`（离线模式）
/// - 非空但连接失败 → 记录降级事件，回退 `NoopMemoryAdapter`（不 panic）
/// - 非空且连接成功 → `GrpcMindAdapter`
pub async fn resolve_memory_adapter(config: &crate::config::AnaphaseConfig) -> Arc<dyn MemoryAdapter> {
    match config.mind_endpoint.as_deref() {
        Some(ep) if !ep.is_empty() => {
            match mind::GrpcMindAdapter::new(ep, config.mind.clone()).await {
                Ok(adapter) => Arc::new(adapter),
                Err(e) => {
                    tracing::warn!(
                        "mind degraded at resolve: endpoint={} err={}; fallback Noop (fail-open)",
                        ep,
                        e
                    );
                    Arc::new(NoopMemoryAdapter)
                }
            }
        }
        _ => Arc::new(NoopMemoryAdapter),
    }
}

// ---------- Reasoning Adapter ----------
#[async_trait]
pub trait ReasoningAdapter: Send + Sync {
    /// Run one reasoning round trip. `trace_id` is the derived job id —
    /// carried to the gateway (x-tuck-trace) so the Tuck audit chain, the
    /// Anaphase body trace and the ledger share one join key (Engram).
    async fn reason(&self, prompt: &str, model: &str, trace_id: &str) -> Result<String, String>;

    /// Streaming variant: emits content deltas into `deltas` as they arrive
    /// and returns the full text (same contract as `reason`). Default = the
    /// buffered path (every adapter stays valid; only HTTP streams). The
    /// channel keeps the callback out of the async trait — no lifetime glue.
    async fn reason_stream(
        &self,
        prompt: &str,
        model: &str,
        trace_id: &str,
        deltas: tokio::sync::mpsc::UnboundedSender<String>,
    ) -> Result<String, String> {
        let out = self.reason(prompt, model, trace_id).await?;
        let _ = deltas.send(out.clone());
        Ok(out)
    }
}

pub struct NoopReasoningAdapter;

#[async_trait]
impl ReasoningAdapter for NoopReasoningAdapter {
    async fn reason(&self, _prompt: &str, _model: &str, _trace_id: &str) -> Result<String, String> {
        Ok("No reasoning available".to_string())
    }
}

// ---------- Tool Adapter ----------
#[async_trait]
pub trait ToolAdapter: Send + Sync {
    async fn execute(&self, command: &str, args: &[String]) -> Result<String, String>;
    async fn perceive(&self, query: &str) -> Result<String, String>;
}

pub struct NoopToolAdapter;

#[async_trait]
impl ToolAdapter for NoopToolAdapter {
    async fn execute(&self, _command: &str, _args: &[String]) -> Result<String, String> {
        Ok("Tool execution unavailable".to_string())
    }
    async fn perceive(&self, _query: &str) -> Result<String, String> {
        Ok("Perception unavailable".to_string())
    }
}

// ---------- Safety Adapter ----------
#[async_trait]
pub trait SafetyAdapter: Send + Sync {
    async fn audit(&self, action: &str, content: &str) -> Result<bool, String>;
}

pub struct NoopSafetyAdapter;

#[async_trait]
impl SafetyAdapter for NoopSafetyAdapter {
    async fn audit(&self, _action: &str, _content: &str) -> Result<bool, String> {
        Ok(true)
    }
}

// ---------- UI Adapter ----------
#[async_trait]
pub trait UiAdapter: Send + Sync {
    async fn render(&self, state: &str) -> Result<(), String>;
    async fn get_input(&self) -> Result<String, String>;
}

pub struct NoopUiAdapter;

#[async_trait]
impl UiAdapter for NoopUiAdapter {
    async fn render(&self, _state: &str) -> Result<(), String> {
        Ok(())
    }
    async fn get_input(&self) -> Result<String, String> {
        Ok("No UI available".to_string())
    }
}

// ---------- Fear Adapter ----------
#[async_trait]
pub trait FearAdapter: Send + Sync {
    async fn predict_death(&self, context: &str) -> Result<f64, String>;
}

pub struct NoopFearAdapter;

#[async_trait]
impl FearAdapter for NoopFearAdapter {
    async fn predict_death(&self, _context: &str) -> Result<f64, String> {
        Ok(0.0)
    }
}
