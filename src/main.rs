use std::io::{self, BufRead, Write};
use serde_json::json;

use anaphase::adapters::*;
use anaphase::adapters::flowmodus::{FlowModusAdapter, GrpcFlowModusAdapter};
// New: Add direct import for HttpReasoningAdapter
use anaphase::adapters::http_reasoning::HttpReasoningAdapter;
use anaphase::agent_loop::AgentLoop;
use anaphase::reflex::ReflexArc;
use anaphase::config;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let stdio_mode = args.iter().any(|a| a == "--stdio");

    if stdio_mode {
        run_stdio_mode().await?;
        return Ok(());
    }

    tracing_subscriber::fmt::init();
    let config = config::load_config()?;

    // 记忆适配器：DNA 铁律 6 fail-open 降级（空→Noop；连接失败→warn+Noop；成功→GrpcMindAdapter）
    let memory: Arc<dyn MemoryAdapter> = resolve_memory_adapter(&config.anaphase).await;

    // Priority 1: Use HTTP LLM reasoning adapter first
    let reason: Arc<dyn ReasoningAdapter> = if let Some(endpoint) = &config.anaphase.reasoning_endpoint {
        if endpoint.is_empty() {
            Arc::new(NoopReasoningAdapter)
        } else {
            // Simplified type name
            Arc::new(HttpReasoningAdapter::new(&config.anaphase))
        }
    }
    // Priority 2: Fallback to original FlowModus
    else if let Some(endpoint) = &config.anaphase.flowmodus_endpoint {
        if endpoint.is_empty() {
            Arc::new(NoopReasoningAdapter)
        } else if endpoint.starts_with("grpc://") {
            match GrpcFlowModusAdapter::new(&endpoint[7..]).await {
                Ok(adapter) => Arc::new(adapter),
                Err(e) => {
                    eprintln!("Warning: Failed to connect to FlowModus at {}: {}. Falling back to Noop reasoning.", endpoint, e);
                    Arc::new(NoopReasoningAdapter)
                }
            }
        } else {
            Arc::new(FlowModusAdapter::new(endpoint))
        }
    } else {
        Arc::new(NoopReasoningAdapter)
    };

    let tool: Arc<dyn ToolAdapter> = Arc::new(NoopToolAdapter);
    let safety: Arc<dyn SafetyAdapter> = Arc::new(NoopSafetyAdapter);
    let ui: Arc<dyn UiAdapter> = Arc::new(NoopUiAdapter);
    let fear: Arc<dyn FearAdapter> = Arc::new(NoopFearAdapter);

    let reflex = ReflexArc {
        safety_rules: vec!["rm -rf /".to_string(), "shutdown".to_string()],
    };

    let mut agent = AgentLoop::new(memory, reason, tool, safety, ui, fear, reflex);

    if config.anaphase.cap_http_enabled && !stdio_mode {
        use axum::{Router, routing::get, Json, response::IntoResponse};

        async fn cap_snapshot() -> impl IntoResponse {
            Json(serde_json::json!({
                "status": "Active",
                "metrics": { "token_consumed": 1234 },
                "semantic_tree": [
                    {
                        "id": "1",
                        "node_type": "state_tree",
                        "label": "Cognitive Loop",
                        "content": "Perception -> PreAssessment -> MemoryRetrieval -> Reasoning -> ReflexCheck -> Execution -> Reflection"
                    }
                ]
            }))
        }

        let app = Router::new().route("/v1/agent/snapshot", get(cap_snapshot));
        let addr = format!("0.0.0.0:{}", config.anaphase.cap_http_port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        println!("CAP HTTP server started: http://{}", addr);
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
    }

    println!("Anaphase-Helix v0.1.0 started successfully");
    println!("CAP HTTP endpoint: http://0.0.0.0:{}", config.anaphase.cap_http_port);
    println!("Press Ctrl+C to shutdown the service");

    let user_input = "Calculate 2 to the power of 10";
    println!("User: {}", user_input);
    agent.run_cycle(user_input).await?;
    println!("\nCognitive cycle completed successfully.");

    tokio::signal::ctrl_c().await?;
    println!("Shutting down...");
    Ok(())
}

async fn run_stdio_mode() -> Result<(), Box<dyn std::error::Error>> {
    // Load config and initialize agent components for real reasoning
    let config = config::load_config()?;

    // Initialize reasoning adapter (follow config priority)
    let reason: Arc<dyn ReasoningAdapter> = if let Some(endpoint) = &config.anaphase.reasoning_endpoint {
        if endpoint.is_empty() {
            Arc::new(NoopReasoningAdapter)
        } else {
            // Simplified type name
            Arc::new(HttpReasoningAdapter::new(&config.anaphase))
        }
    } else {
        Arc::new(NoopReasoningAdapter)
    };

    let memory: Arc<dyn MemoryAdapter> = Arc::new(NoopMemoryAdapter);
    let tool: Arc<dyn ToolAdapter> = Arc::new(NoopToolAdapter);
    let safety: Arc<dyn SafetyAdapter> = Arc::new(NoopSafetyAdapter);
    let ui: Arc<dyn UiAdapter> = Arc::new(NoopUiAdapter);
    let fear: Arc<dyn FearAdapter> = Arc::new(NoopFearAdapter);
    let reflex = ReflexArc { safety_rules: vec![] };
    let mut agent = AgentLoop::new(memory, reason, tool, safety, ui, fear, reflex);

    // STDIO IO start
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    eprintln!("Anaphase STDIO CAP protocol mode active");

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let cmd: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let error_resp = json!({
                    "status": "error",
                    "error": format!("Invalid JSON: {}", e)
                });
                writeln!(stdout, "{}", serde_json::to_string(&error_resp)?)?;
                stdout.flush()?;
                continue;
            }
        };

        let cmd_type = cmd["type"].as_str().unwrap_or("");

        match cmd_type {
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
            "action" => {
                let action_id = cmd["action"].as_str().unwrap_or("");
                if action_id == "send_message" {
                    let message = cmd["params"]["message"].as_str().unwrap_or("");
                    // Call real agent reasoning cycle
                    match agent.run_cycle(message).await {
                        Ok(()) => {
                            let response_text = agent.context.reasoning_output.clone();
                            let resp = json!({
                                "status": "ok",
                                "type": "message_response",
                                "content": response_text
                            });
                            writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                        }
                        Err(e) => {
                            let resp = json!({
                                "status": "error",
                                "content": format!("Reasoning failed: {}", e)
                            });
                            writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                        }
                    }
                    stdout.flush()?;
                } else {
                    let response = json!({
                        "status": "ok",
                        "action": action_id,
                        "message": format!("Action '{}' acknowledged", action_id)
                    });
                    writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
                    stdout.flush()?;
                }
            }
            "exit" => {
                let response = json!({"status": "goodbye"});
                writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
                stdout.flush()?;
                break;
            }
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
