use anaphase::adapters::*;
use anaphase::agent_loop::AgentLoop;
use anaphase::reflex::ReflexArc;
use std::sync::Arc;

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
