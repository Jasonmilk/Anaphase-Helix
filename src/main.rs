use anaphase::adapters::*;
use anaphase::adapters::mind::GrpcMindAdapter;
use anaphase::adapters::flowmodus::{FlowModusAdapter, GrpcFlowModusAdapter};
use anaphase::adapters::tentacle::GrpcTentacleAdapter;
use anaphase::agent_loop::AgentLoop;
use anaphase::reflex::ReflexArc;
use anaphase::config;
use std::sync::Arc;
// HTTP CAP server dependencies
use axum::{Router, routing::get, Json, response::IntoResponse};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging subsystem
    tracing_subscriber::fmt::init();

    // Load service configuration (file or defaults)
    let config = config::load_config()?;

    // ======================
    // Adapter Initialization
    // ======================

    // Memory adapter: Filter empty strings + graceful degradation (gRPC -> Noop on failure)
    let memory: Arc<dyn MemoryAdapter> = if let Some(endpoint) = &config.anaphase.mind_endpoint {
        if endpoint.is_empty() {
            Arc::new(NoopMemoryAdapter)
        } else {
            match GrpcMindAdapter::new(endpoint).await {
                Ok(adapter) => Arc::new(adapter),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to connect to Mind gRPC at {}: {}. Falling back to NoopMemoryAdapter.",
                        endpoint, e
                    );
                    Arc::new(NoopMemoryAdapter)
                }
            }
        }
    } else {
        Arc::new(NoopMemoryAdapter)
    };

    // Reasoning adapter: Filter empty strings + protocol auto-detection + Noop fallback
    let reason: Arc<dyn ReasoningAdapter> = if let Some(endpoint) = &config.anaphase.flowmodus_endpoint {
        if endpoint.is_empty() {
            Arc::new(NoopReasoningAdapter)
        } else {
            if endpoint.starts_with("grpc://") {
                Arc::new(GrpcFlowModusAdapter::new(&endpoint[7..]).await?)
            } else {
                Arc::new(FlowModusAdapter::new(endpoint))
            }
        }
    } else {
        Arc::new(NoopReasoningAdapter)
    };

    // Tool adapter: Filter empty strings + gRPC client + Noop fallback
    let tool: Arc<dyn ToolAdapter> = if let Some(endpoint) = &config.anaphase.tentacle_endpoint {
        if endpoint.is_empty() {
            Arc::new(NoopToolAdapter)
        } else {
            Arc::new(GrpcTentacleAdapter::new(endpoint).await?)
        }
    } else {
        Arc::new(NoopToolAdapter)
    };

    // Default fallback adapters
    let safety: Arc<dyn SafetyAdapter> = Arc::new(NoopSafetyAdapter);
    let ui: Arc<dyn UiAdapter> = Arc::new(NoopUiAdapter);
    let fear: Arc<dyn FearAdapter> = Arc::new(NoopFearAdapter);

    // Initialize safety reflex rules
    let reflex = ReflexArc {
        safety_rules: vec![
            "rm -rf /".to_string(),
            "shutdown".to_string()
        ],
    };

    // ======================
    // HTTP CAP Server (Configured, No Hardcoding)
    // ======================
    if config.anaphase.cap_http_enabled {
        let addr = format!("0.0.0.0:{}", config.anaphase.cap_http_port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        
        tokio::spawn(async move {
            let app = Router::new().route("/v1/agent/snapshot", get(cap_snapshot_handler));
            if axum::serve(listener, app).await.is_err() {
                eprintln!("Error: CAP HTTP server stopped unexpectedly");
            }
        });

        println!("CAP HTTP server started: http://{}", addr);
    }

    // ======================
    // Agent Loop Startup
    // ======================
    let mut agent = AgentLoop::new(
        memory, reason, tool, safety, ui, fear, reflex
    );

    println!("Anaphase-Helix v0.1.0 started successfully");
    
    // Test cognitive cycle
    let user_input = "Calculate 2 to the power of 10";
    println!("User: {}", user_input);
    agent.run_cycle(user_input).await?;
    println!("\nCognitive cycle completed successfully.");

    Ok(())
}

/// CAP snapshot API handler for remote debugging
async fn cap_snapshot_handler() -> impl IntoResponse {
    Json(json!({
        "status": "Active",
        "metrics": { "token_consumed": 1234 },
        "semantic_tree": [
            {
                "id": "1",
                "node_type": "state_tree",
                "label": "Cognitive Loop",
                "content": "Perception -> Reasoning -> Execution"
            }
        ]
    }))
}
