use anaphase::adapters::*;
use anaphase::agent_loop::AgentLoop;
use anaphase::reflex::ReflexArc;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create all adapters with Noop implementations (zero external dependencies)
    let memory: Arc<dyn MemoryAdapter> = Arc::new(NoopMemoryAdapter);
    let reason: Arc<dyn ReasoningAdapter> = Arc::new(NoopReasoningAdapter);
    let tool: Arc<dyn ToolAdapter> = Arc::new(NoopToolAdapter);
    let safety: Arc<dyn SafetyAdapter> = Arc::new(NoopSafetyAdapter);
    let ui: Arc<dyn UiAdapter> = Arc::new(NoopUiAdapter);
    let fear: Arc<dyn FearAdapter> = Arc::new(NoopFearAdapter);

    // Create somatic reflex arc (initial safety rules)
    let reflex = ReflexArc {
        safety_rules: vec!["rm -rf /".to_string(), "shutdown".to_string()],
    };

    // Create cognitive loop engine
    let mut agent = AgentLoop::new(
        memory,
        reason,
        tool,
        safety,
        ui,
        fear,
        reflex,
    );

    println!("Anaphase-Helix v0.1.0 started successfully (Noop mode)\n");

    // Simulate a user interaction
    let user_input = "Calculate 2 to the power of 10";
    println!("User: {}", user_input);

    match agent.run_cycle(user_input).await {
        Ok(()) => println!("\nCognitive cycle completed. Reflection notes: {}", agent.context.reflection_notes),
        Err(e) => eprintln!("Cognitive cycle error: {}", e),
    }
}
