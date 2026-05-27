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
// STDIO mode dependencies
use std::io::{self, BufRead, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments for STDIO mode
    let args: Vec<String> = std::env::args().collect();
    let stdio_mode = args.iter().any(|a| a == "--stdio");

    // Start STDIO CAP protocol mode if enabled
    if stdio_mode {
        run_stdio_mode().await?;
        return Ok(());
    }

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
    // HTTP CAP Server (Only start in NON-STDIO mode)
    // ======================
    if !stdio_mode && config.anaphase.cap_http_enabled {
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
    // Agent Loop Startup (Async Task - NON BLOCKING)
    // ======================
    let mut agent = AgentLoop::new(
        memory, reason, tool, safety, ui, fear, reflex
    );

    // Spawn cognitive cycle as async task to avoid blocking main thread
    tokio::spawn(async move {
        let user_input = "Calculate 2 to the power of 10";
        println!("User: {}", user_input);
        if agent.run_cycle(user_input).await.is_ok() {
            println!("\nCognitive cycle completed successfully.");
        }
    });

    // ======================
    // Keep service running and wait for shutdown signal
    // ======================
    println!("Anaphase-Helix v0.1.0 started successfully");
    println!("CAP HTTP endpoint: http://0.0.0.0:{}", config.anaphase.cap_http_port);
    println!("Press Ctrl+C to shutdown the service");

    // Wait for Ctrl+C signal to terminate the program
    tokio::signal::ctrl_c().await?;
    println!("Shutting down...");

    Ok(())
}

/// STDIO mode: CAP protocol over standard input/output
async fn run_stdio_mode() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    // Print startup message to stderr (not captured by JSON parser)
    eprintln!("Anaphase STDIO CAP protocol mode active");

    // Read JSON lines from stdin
    for line in stdin.lock().lines() {
        let line = line?;
        let trimmed = line.trim();
        
        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Parse JSON command
        let cmd: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                // Return error response
                let error_resp = json!({
                    "status": "error",
                    "error": format!("Invalid JSON: {}", e)
                });
                writeln!(stdout, "{}", serde_json::to_string(&error_resp)?)?;
                stdout.flush()?;
                continue;
            }
        };

        // Get command type
        let cmd_type = cmd["type"].as_str().unwrap_or("");

        // Handle CAP protocol commands
        match cmd_type {
            // Handle connection and snapshot requests
            "connect" | "get_snapshot" => {
                let snapshot = json!({
                    "status": "Active",
                    "metrics": {
                        "token_consumed": 1234,
                        "active_tasks": 0,
                        "memory_nodes": 0
                    },
                    "semantic_tree": [
                        {
                            "id": "1",
                            "node_type": "state_tree",
                            "label": "Cognitive Loop",
                            "content": "Perception -> PreAssessment -> MemoryRetrieval -> Reasoning -> ReflexCheck -> Execution -> Reflection"
                        },
                        {
                            "id": "2",
                            "node_type": "text_panel",
                            "label": "Status",
                            "content": "Anaphase running in STDIO mode. Ready for commands."
                        }
                    ]
                });
                writeln!(stdout, "{}", serde_json::to_string(&snapshot)?)?;
                stdout.flush()?;
            }

            // Handle action commands
            "action" => {
                let action_id = cmd["action"].as_str().unwrap_or("");
                let params = &cmd["params"];
                let response = json!({
                    "status": "ok",
                    "action": action_id,
                    "params": params,
                    "message": format!("Action '{}' acknowledged", action_id)
                });
                writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
                stdout.flush()?;
            }

            // Handle exit command
            "exit" => {
                let response = json!({"status": "goodbye"});
                writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
                stdout.flush()?;
                break;
            }

            // Unknown command
            _ => {
                let error_resp = json!({
                    "status": "error",
                    "error": format!("Unknown command type: {}", cmd_type)
                });
                writeln!(stdout, "{}", serde_json::to_string(&error_resp)?)?;
                stdout.flush()?;
            }
        }
    }

    eprintln!("Anaphase STDIO mode exiting");
    Ok(())
}

/// API handler for CAP agent snapshot endpoint
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
