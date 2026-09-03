use anaphase::adapters::*;
use anaphase::agent_loop::AgentLoop;
use anaphase::reflex::ReflexArc;
use std::sync::Arc;

// ── M1.5-T6 recording adapter: captures the resolved Execution command ──

struct RecordingToolAdapter {
    pub last_command: std::sync::Mutex<Option<String>>,
}

impl RecordingToolAdapter {
    fn new() -> Self {
        Self { last_command: std::sync::Mutex::new(None) }
    }
}

#[async_trait::async_trait]
impl ToolAdapter for RecordingToolAdapter {
    async fn execute(&self, command: &str, _args: &[String]) -> Result<String, String> {
        *self.last_command.lock().unwrap() = Some(command.to_string());
        Ok(format!("executed: {command}"))
    }
    async fn perceive(&self, _query: &str) -> Result<String, String> {
        Ok("perceived".to_string())
    }
}

// ── Adapter Noop tests ─────────────────────────────────────────────

#[tokio::test]
async fn test_noop_memory_adapter() {
    let adapter = NoopMemoryAdapter;
    let result = adapter.query("test", false).await.unwrap();
    assert!(result.nodes.is_empty());
    assert_eq!(result.impasse_level, 0);
    assert!(adapter.remember("test").await.is_ok());
}

#[tokio::test]
async fn test_noop_reasoning_adapter() {
    let adapter = NoopReasoningAdapter;
    let result = adapter.reason("test", "left_brain").await.unwrap();
    assert_eq!(result, "No reasoning available");
}

#[tokio::test]
async fn test_noop_tool_adapter() {
    let adapter = NoopToolAdapter;
    assert_eq!(adapter.execute("echo", &[]).await.unwrap(), "Tool execution unavailable");
    assert_eq!(adapter.perceive("test").await.unwrap(), "Perception unavailable");
}

#[tokio::test]
async fn test_noop_safety_adapter() {
    let adapter = NoopSafetyAdapter;
    assert_eq!(adapter.audit("action", "content").await.unwrap(), true);
}

#[tokio::test]
async fn test_noop_ui_adapter() {
    let adapter = NoopUiAdapter;
    assert!(adapter.render("state").await.is_ok());
    assert_eq!(adapter.get_input().await.unwrap(), "No UI available");
}

#[tokio::test]
async fn test_noop_fear_adapter() {
    let adapter = NoopFearAdapter;
    assert_eq!(adapter.predict_death("test").await.unwrap(), 0.0);
}

// ── Reflex arc tests ────────────────────────────────────────────────

#[test]
fn test_hard_reflex_allows_safe_action() {
    let reflex = ReflexArc {
        safety_rules: vec!["rm -rf /".to_string(), "shutdown".to_string()],
    };
    assert!(reflex.hard_reflex("echo hello"));
}

#[test]
fn test_hard_reflex_blocks_dangerous_action() {
    let reflex = ReflexArc {
        safety_rules: vec!["rm -rf /".to_string(), "shutdown".to_string()],
    };
    assert!(!reflex.hard_reflex("execute rm -rf / now"));
}

#[test]
fn test_hard_reflex_blocks_exact_match() {
    let reflex = ReflexArc {
        safety_rules: vec!["shutdown".to_string()],
    };
    assert!(!reflex.hard_reflex("shutdown"));
}

#[test]
fn test_hard_reflex_empty_rules_allows_all() {
    let reflex = ReflexArc {
        safety_rules: vec![],
    };
    assert!(reflex.hard_reflex("anything"));
}

#[tokio::test]
async fn test_soft_reflex_with_noop_fear() {
    let reflex = ReflexArc {
        safety_rules: vec![],
    };
    let fear: Arc<dyn FearAdapter> = Arc::new(NoopFearAdapter);
    let p_death = reflex.soft_reflex(fear.as_ref(), "test").await.unwrap();
    assert_eq!(p_death, 0.0);
}

// ── Agent loop integration tests ────────────────────────────────────

#[tokio::test]
async fn test_full_cycle_noop_mode() {
    let memory: Arc<dyn MemoryAdapter> = Arc::new(NoopMemoryAdapter);
    let reason: Arc<dyn ReasoningAdapter> = Arc::new(NoopReasoningAdapter);
    let tool: Arc<dyn ToolAdapter> = Arc::new(NoopToolAdapter);
    let safety: Arc<dyn SafetyAdapter> = Arc::new(NoopSafetyAdapter);
    let ui: Arc<dyn UiAdapter> = Arc::new(NoopUiAdapter);
    let fear: Arc<dyn FearAdapter> = Arc::new(NoopFearAdapter);
    let reflex = ReflexArc {
        safety_rules: vec!["rm -rf /".to_string()],
    };

    let mut agent = AgentLoop::new(memory, reason, tool, safety, ui, fear, reflex);
    let result = agent.run_cycle("What is 2+2?").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_dangerous_action_is_blocked() {
    let memory: Arc<dyn MemoryAdapter> = Arc::new(NoopMemoryAdapter);
    let reason: Arc<dyn ReasoningAdapter> = Arc::new(NoopReasoningAdapter);
    let tool: Arc<dyn ToolAdapter> = Arc::new(NoopToolAdapter);
    let safety: Arc<dyn SafetyAdapter> = Arc::new(NoopSafetyAdapter);
    let ui: Arc<dyn UiAdapter> = Arc::new(NoopUiAdapter);
    let fear: Arc<dyn FearAdapter> = Arc::new(NoopFearAdapter);
    let reflex = ReflexArc {
        safety_rules: vec!["rm -rf /".to_string()],
    };

    let mut agent = AgentLoop::new(memory, reason, tool, safety, ui, fear, reflex);
    // Simulate a dangerous tool call by setting suggested_actions before ReflexCheck
    agent.context.suggested_actions = vec!["rm -rf /".to_string()];
    
    // Run the cycle — it should hit ReflexCheck and block
    let result = agent.run_cycle("Delete everything").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_state_transitions_from_perception_to_reflection() {
    use anaphase::states::HelixState;
    
    let memory: Arc<dyn MemoryAdapter> = Arc::new(NoopMemoryAdapter);
    let reason: Arc<dyn ReasoningAdapter> = Arc::new(NoopReasoningAdapter);
    let tool: Arc<dyn ToolAdapter> = Arc::new(NoopToolAdapter);
    let safety: Arc<dyn SafetyAdapter> = Arc::new(NoopSafetyAdapter);
    let ui: Arc<dyn UiAdapter> = Arc::new(NoopUiAdapter);
    let fear: Arc<dyn FearAdapter> = Arc::new(NoopFearAdapter);
    let reflex = ReflexArc { safety_rules: vec![] };

    let mut agent = AgentLoop::new(memory, reason, tool, safety, ui, fear, reflex);
    assert_eq!(agent.current_state, HelixState::Perception);
    
    agent.run_cycle("Hello").await.unwrap();
    
    // After a full cycle, should return to Perception
    assert_eq!(agent.current_state, HelixState::Perception);
}

// ── M1.5-T6: Execution resolves the configured real tool name ──────────

/// Reasoning stub that signals a tool call (triggers NeedsTool -> Execution).
/// Emits the candidate-E structured output protocol (ADR-0005) instead of the
/// legacy "tool_call: numbers" string marker.
struct TriggerToolReasoning;

#[async_trait::async_trait]
impl ReasoningAdapter for TriggerToolReasoning {
    async fn reason(&self, _query: &str, _mode: &str) -> Result<String, String> {
        Ok(r#"{"calls":[{"tool":"numbers","args":{},"expect":"numbers"}]}"#.to_string())
    }
}

#[tokio::test]
async fn test_execution_resolves_real_tool_command() {
    use anaphase::states::HelixState;

    let memory: Arc<dyn MemoryAdapter> = Arc::new(NoopMemoryAdapter);
    let reason: Arc<dyn ReasoningAdapter> = Arc::new(TriggerToolReasoning);
    let tool = Arc::new(RecordingToolAdapter::new());
    let tool_adapter: Arc<dyn ToolAdapter> = tool.clone();
    let safety: Arc<dyn SafetyAdapter> = Arc::new(NoopSafetyAdapter);
    let ui: Arc<dyn UiAdapter> = Arc::new(NoopUiAdapter);
    let fear: Arc<dyn FearAdapter> = Arc::new(NoopFearAdapter);
    let reflex = ReflexArc { safety_rules: vec![] };

    // Configured real tool name -> Execution must call it (not the echo placeholder).
    let mut agent = AgentLoop::new(memory, reason, tool_adapter, safety, ui, fear, reflex)
        .with_tool_command("numbers");

    agent.run_cycle("Hello").await.unwrap();

    // Cycle ends back at Perception; the real tool name was resolved.
    assert_eq!(agent.current_state, HelixState::Perception);
    assert_eq!(
        *tool.last_command.lock().unwrap(),
        Some("numbers".to_string()),
        "Execution must dispatch to the configured real tool name"
    );
}

#[tokio::test]
async fn test_execution_keeps_echo_placeholder_when_unset() {
    let memory: Arc<dyn MemoryAdapter> = Arc::new(NoopMemoryAdapter);
    let reason: Arc<dyn ReasoningAdapter> = Arc::new(TriggerToolReasoning);
    let tool = Arc::new(RecordingToolAdapter::new());
    let tool_adapter: Arc<dyn ToolAdapter> = tool.clone();
    let safety: Arc<dyn SafetyAdapter> = Arc::new(NoopSafetyAdapter);
    let ui: Arc<dyn UiAdapter> = Arc::new(NoopUiAdapter);
    let fear: Arc<dyn FearAdapter> = Arc::new(NoopFearAdapter);
    let reflex = ReflexArc { safety_rules: vec![] };

    // No tool_command -> legacy echo placeholder (backwards compatible).
    let mut agent = AgentLoop::new(memory, reason, tool_adapter, safety, ui, fear, reflex);

    agent.run_cycle("Hello").await.unwrap();
    assert_eq!(
        *tool.last_command.lock().unwrap(),
        Some("echo".to_string()),
        "without tool_command, Execution keeps the echo placeholder"
    );
}
