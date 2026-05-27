pub mod mind;
pub mod flowmodus;
pub mod tentacle;

use async_trait::async_trait;
use serde::{Serialize, Deserialize};

// ---------- Memory Adapter ----------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub nodes: Vec<String>,
    pub impasse_level: u8,
    pub suggested_actions: Vec<String>,
}

#[async_trait]
pub trait MemoryAdapter: Send + Sync {
    async fn query(&self, query: &str, include_recessive: bool) -> Result<QueryResult, String>;
    async fn remember(&self, content: &str) -> Result<(), String>;
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

// ---------- Reasoning Adapter ----------
#[async_trait]
pub trait ReasoningAdapter: Send + Sync {
    async fn reason(&self, prompt: &str, model: &str) -> Result<String, String>;
}

pub struct NoopReasoningAdapter;

#[async_trait]
impl ReasoningAdapter for NoopReasoningAdapter {
    async fn reason(&self, _prompt: &str, _model: &str) -> Result<String, String> {
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
