use anaphase::{
    adapters::*,
    agent_loop::AgentLoop,
    config::load_config,
    helix_mind_api::helix_mind_client::HelixMindClient,
    reflex::ReflexArc,
};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = load_config().expect("Failed to load config.toml");

    // ------------------------------
    // Memory Adapter: gRPC or Noop
    // ------------------------------
    let memory: Arc<dyn MemoryAdapter> = if !config.anaphase.mind_endpoint.is_empty() {
        Arc::new(
            GrpcMindAdapter::new(&config.anaphase.mind_endpoint)
                .await
                .expect("Failed to connect to Helix-Mind gRPC"),
        )
    } else {
        Arc::new(NoopMemoryAdapter)
    };

    // Other adapters (Noop for now)
    let reason: Arc<dyn ReasoningAdapter> = Arc::new(NoopReasoningAdapter);
    let tool: Arc<dyn ToolAdapter> = Arc::new(NoopToolAdapter);
    let safety: Arc<dyn SafetyAdapter> = Arc::new(NoopSafetyAdapter);
    let ui: Arc<dyn UiAdapter> = Arc::new(NoopUiAdapter);
    let fear: Arc<dyn FearAdapter> = Arc::new(NoopFearAdapter);

    // Reflex arc
    let reflex = ReflexArc {
        safety_rules: vec!["rm -rf /".into(), "shutdown".into()],
    };

    // Agent loop
    let mut agent = AgentLoop::new(memory, reason, tool, safety, ui, fear, reflex);

    println!("Anaphase-Helix v0.1.0 started (Noop/gRPC hybrid mode)\n");

    let user_input = "Calculate 2 to the power of 10";
    println!("User: {}", user_input);

    let _ = agent.run_cycle(user_input).await;
}
