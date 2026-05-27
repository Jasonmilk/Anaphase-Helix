use anaphase::adapters::*;
use anaphase::adapters::mind::GrpcMindAdapter;
use anaphase::adapters::flowmodus::{FlowModusAdapter, GrpcFlowModusAdapter};
use anaphase::adapters::tentacle::GrpcTentacleAdapter;
use anaphase::agent_loop::AgentLoop;
use anaphase::reflex::ReflexArc;
use anaphase::config;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = config::load_config()?;

    // Choose memory adapter: gRPC if endpoint is configured, Noop as fallback
    let memory: Arc<dyn MemoryAdapter> = if let Some(endpoint) = &config.anaphase.mind_endpoint {
        Arc::new(GrpcMindAdapter::new(endpoint).await?)
    } else {
        Arc::new(NoopMemoryAdapter)
    };

    // Dynamically inject FlowModus adapter based on endpoint protocol
    // - grpc:// prefix: use GrpcFlowModusAdapter
    // - http/other: use HTTP FlowModusAdapter
    // - no config: fall back to NoopReasoningAdapter
    let reason: Arc<dyn ReasoningAdapter> = if let Some(endpoint) = &config.anaphase.flowmodus_endpoint {
        if endpoint.starts_with("grpc://") {
            Arc::new(GrpcFlowModusAdapter::new(&endpoint[7..]).await?)
        } else {
            Arc::new(FlowModusAdapter::new(endpoint))
        }
    } else {
        Arc::new(NoopReasoningAdapter)
    };

    // Dynamically inject Tentacle adapter based on configuration
    // - gRPC adapter if endpoint is configured, Noop as fallback
    let tool: Arc<dyn ToolAdapter> = if let Some(endpoint) = &config.anaphase.tentacle_endpoint {
        Arc::new(GrpcTentacleAdapter::new(endpoint).await?)
    } else {
        Arc::new(NoopToolAdapter)
    };

    let safety: Arc<dyn SafetyAdapter> = Arc::new(NoopSafetyAdapter);
    let ui: Arc<dyn UiAdapter> = Arc::new(NoopUiAdapter);
    let fear: Arc<dyn FearAdapter> = Arc::new(NoopFearAdapter);

    let reflex = ReflexArc {
        safety_rules: vec!["rm -rf /".to_string(), "shutdown".to_string()],
    };

    let mut agent = AgentLoop::new(
        memory,
        reason,
        tool,
        safety,
        ui,
        fear,
        reflex,
    );

    println!("Anaphase-Helix v0.1.0 started");
    let user_input = "Calculate 2 to the power of 10";
    println!("User: {}", user_input);

    agent.run_cycle(user_input).await?;
    println!("\nCognitive cycle completed.");
    Ok(())
}
