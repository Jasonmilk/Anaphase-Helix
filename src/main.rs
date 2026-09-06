
use anaphase::adapters::*;
use anaphase::adapters::flowmodus::{FlowModusAdapter, GrpcFlowModusAdapter};
// New: Add direct import for HttpReasoningAdapter
use anaphase::adapters::http_reasoning::HttpReasoningAdapter;
use anaphase::run_cycle::AgentLoop;
use anaphase::lifecycle::SessionNotes;
use anaphase::reflex::ReflexArc;
use anaphase::config;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    // CI-144 stdio entry: accept both `--stdio` (native flag) and the
    // ecosystem launcher convention `--mode stdio` (Cellrix `--exec` appends
    // this pair for every stdio agent — one launch contract, every agent).
    let stdio_mode = args.iter().any(|a| a == "--stdio")
        || args.windows(2).any(|w| w[0] == "--mode" && w[1] == "stdio");

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
    // O-1 (ADR-0016 D3): one physical probe at task start — "look at the
    // pocket before leaving the house". Fail-open: dark components degrade,
    // never block.
    agent.context.ecosystem = anaphase::gloves::probe_ecosystem(&config.anaphase).await;
    // candidate E (ADR-0005): run_cycle constants come from the config source
    // (DNA principle 11 / ADR-0002), overridable via config.toml.
    agent.run_config = config.anaphase.run_cycle.clone();
    // ADR-0006: the interaction mode is the semantic record carried through
    // the loop; physical Mind participation is decided by resolve_memory_adapter
    // (Noop vs gRPC) — Drive auto-achieves "no experience written" through
    // the Noop adapter without any runtime branch.
    agent.mode = config.anaphase.run_cycle.mode;

    // ADR-0007 D'-3: wire the deterministic execution channel at startup.
    // `tentacle_endpoint` configured -> the six-stage pipeline replaces the
    // legacy echo fallback; empty/failed -> fail-open (None, legacy path).
    let pipeline_config =
        anaphase::pipeline::PipelineConfig::from_codex("knowledge_base/fixture-codex.json")
            .map_err(|e| eprintln!("Warning: failed to load fixture-codex: {e}")) // warn + continue
            .ok();
    // O-2/O-3 (ADR-0019/0020/0021): the event ring is mode-agnostic — it is
    // created once (cap from the codex contract, DNA principle 11: no literal
    // fallback — a missing contract means no ring, fail-closed, never a
    // cap=0 ring that silently drops the black box) and shared by the agent
    // (cycle-level black box, stage 0) and the pipeline (stage 1..=6) when
    // wired. Drive mode without a pipeline still records its black box.
    let shared_events: Option<std::sync::Arc<std::sync::Mutex<anaphase::events::EventRing>>> =
        pipeline_config.as_ref().map(|p| {
            std::sync::Arc::new(std::sync::Mutex::new(anaphase::events::EventRing::new(
                p.events_cap,
            )))
        });
    if let Some(pcfg) = pipeline_config {
        if let Some(mut pipeline) =
            anaphase::pipeline::resolve_pipeline(config.anaphase.tentacle_endpoint.clone(), pcfg)
                .await
        {
            // ADR-0021: the pipeline shares the mode-agnostic ring (one
            // stream, one ?after cursor). Its own ring is replaced by the
            // shared one BEFORE the restore, so history lands in the single
            // ring. Restore is fail-open: missing/corrupt log -> fresh trail.
            let events_path = config
                .anaphase
                .events_log_path
                .clone()
                .unwrap_or_else(|| "events.jsonl".to_string());
            let cap = shared_events.as_ref().unwrap().lock().unwrap().cap();
            match std::fs::read_to_string(&events_path) {
                Ok(content) => match anaphase::events::EventRing::from_jsonl(&content, cap) {
                    Ok(restored) => {
                        let n = restored.events().len();
                        *pipeline.events.lock().unwrap() = restored;
                        eprintln!("Events trail restored from {} ({} events)", events_path, n);
                    }
                    Err(e) => eprintln!("Warning: event trail unreadable ({events_path}): {e} (fresh trail)"),
                },
                Err(_) => eprintln!("Events trail: no existing log at {} (fresh trail)", events_path),
            }
            pipeline.events = shared_events.clone().unwrap();
            agent = agent.with_pipeline(pipeline);
            eprintln!("Pipeline wired to Tentacle endpoint (deterministic execution channel active)");
        }
    }
    // ADR-0021: inject the shared ring regardless of pipeline assembly —
    // every cycle records begin/state/end (+ tool) events, Drive included.
    // No contract (codex missing) -> no ring (fail-closed), warned above.
    if let Some(ring) = shared_events.clone() {
        agent = agent.with_events(ring);
    }

    // Rails (ADR-0018): mount the external human knowledge rail — read-only
    // citation asset. Missing/invalid kb dir degrades to None (fail-open,
    // loop unaffected); a dangling link is an authoring error surfaced here.
    agent.rails_config = config.anaphase.rails.clone();
    if config.anaphase.rails.enabled {
        let rails_root = std::path::Path::new(&config.anaphase.rails.kb_dir);
        if rails_root.exists() {
            match anaphase::rails::build_index(rails_root) {
                Ok(index) => {
                    agent = agent.with_rails(index);
                    eprintln!("Rails mounted: {}", rails_root.display());
                }
                Err(e) => eprintln!("Warning: rail index failed: {e} (loop continues unmounted)"),
            }
        }
    }

    // Candidate G-T2: shared snapshot projection (None = HTTP disabled).
    // The endpoint serves it; the loop refreshes it after each cycle.
    let mut shared_snapshot: Option<Arc<Mutex<Option<anaphase::run_cycle::AgentSnapshot>>>> = None;

    if config.anaphase.cap_http_enabled && !stdio_mode {
        use axum::{Router, routing::get, Json};
        use std::sync::{Arc, Mutex};

        let shared: Arc<Mutex<Option<anaphase::run_cycle::AgentSnapshot>>> =
            Arc::new(Mutex::new(None));
        shared_snapshot = Some(shared.clone());

        use axum::extract::Query;
        use std::collections::HashMap;

        let app = Router::new()
            .route("/v1/agent/snapshot", get({
                let shared = shared.clone();
                move || async move {
                    match shared.lock().unwrap().clone() {
                        Some(snap) => Json(serde_json::json!({ "status": "Active", "snapshot": snap })),
                        None => Json(serde_json::json!({ "status": "booting" })),
                    }
                }
            }))
            .route("/v1/agent/events", get({
                let events = shared_events.clone();
                move |Query(params): Query<HashMap<String, String>>| async move {
                    let after: u64 = params
                        .get("after")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0);
                    match &events {
                        Some(ring) => {
                            let guard = ring.lock().unwrap();
                            let evs = guard.after(after);
                            let dropped = guard.dropped();
                            Json(serde_json::json!({
                                "events": evs,
                                "dropped": dropped,
                                "last_seq": guard.last_seq()
                            }))
                        }
                        None => Json(serde_json::json!({ "status": "no pipeline" })),
                    }
                }
            }));
        let addr = format!("0.0.0.0:{}", config.anaphase.cap_http_port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        println!("CAP HTTP server started: http://{}", addr);
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
    }

    println!("Anaphase-Helix v0.1.0 started successfully");
    println!("CAP HTTP endpoint: http://0.0.0.0:{}", config.anaphase.cap_http_port);

    // P10c T1：纪元开始 → 强制苏醒（跨纪元认知重载，读取上一纪元认知脱水简报）
    let session_path = config
        .anaphase
        .session_notes_path
        .clone()
        .unwrap_or_else(|| "session_notes.json".to_string());
    let session = SessionNotes::new(std::path::PathBuf::from(&session_path));
    match session.wake_up() {
        Ok(a) => {
            if a.has_history {
                println!("[WakeUp] 认知重载：上一纪元脱水简报 {} 字", a.briefing.chars().count());
            } else {
                println!("[WakeUp] 无历史纪元（首次唤醒）");
            }
        }
        Err(e) => eprintln!("[WakeUp] 读取失败（降级为无历史）：{}", e),
    }

    println!("Press Ctrl+C to shutdown the service");

    let user_input = "Calculate 2 to the power of 10";
    println!("User: {}", user_input);
    // O-1 (ADR-0016 D1): run_cycle is a single-period primitive — the caller
    // owns the looping policy. Cap from config (DNA principle 11: the 7 is a
    // conservative local-LLM context-budget default, not a protocol value).
    // O-3: the event trail is appended after every cycle (incremental, only
    // events after the flushed cursor) — a crash between cycles loses at most
    // the in-flight cycle, never the settled trail.
    let events_path = config
        .anaphase
        .events_log_path
        .clone()
        .unwrap_or_else(|| "events.jsonl".to_string());
    let mut flushed_seq = 0u64;
    for _ in 0..agent.run_config.cycle_cap {
        let out = agent.run_cycle(user_input).await?;
        if let Some(ring) = agent.events.as_ref() {
            let guard = ring.lock().unwrap();
            let fresh: Vec<anaphase::events::StageEvent> = guard.after(flushed_seq);
            if !fresh.is_empty() {
                flushed_seq = guard.last_seq();
                let mut buf = String::new();
                for e in &fresh {
                    buf.push_str(&serde_json::to_string(e).unwrap_or_default());
                    buf.push('\n');
                }
                use std::io::Write;
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&events_path)
                {
                    let _ = f.write_all(buf.as_bytes());
                }
            }
        }
        if out.done {
            break;
        }
    }
    println!("\nCognitive cycle completed successfully.");

    // Candidate G-T2: refresh the shared snapshot after the cycle, so the
    // endpoint serves the real mode / episode / ledger projection.
    if let Some(shared) = &shared_snapshot {
        *shared.lock().unwrap() = Some(agent.capture());
    }

    tokio::signal::ctrl_c().await?;
    println!("Shutting down...");

    // P10c T1：纪元结束 → 认知脱水（压缩当前纪元历史为简报，供下一纪元加载）
    let history = vec![
        format!("user: {}", user_input),
        format!("assistant: {}", agent.context.reasoning_output),
        format!("reflection: {}", agent.context.reflection_notes),
    ];
    match session.dehydrate(&history) {
        Ok(d) => println!("[Dehydrate] 认知脱水完成（简报 {} 字，{} 条）", d.briefing.chars().count(), d.history_len),
        Err(e) => eprintln!("[Dehydrate] 写入失败：{}", e),
    }
    Ok(())
}

async fn run_stdio_mode() -> Result<(), Box<dyn std::error::Error>> {
    // Load config and initialize agent components for real reasoning.
    let config = config::load_config()?;

    let reason: Arc<dyn ReasoningAdapter> = if let Some(endpoint) = &config.anaphase.reasoning_endpoint {
        if endpoint.is_empty() {
            Arc::new(NoopReasoningAdapter)
        } else {
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
    let agent = std::sync::Arc::new(tokio::sync::Mutex::new(AgentLoop::new(
        memory, reason, tool, safety, ui, fear, reflex,
    )));

    eprintln!("Anaphase CI-144 transport mode active (ADR-0017)");

    // Snapshot provider: project the live agent state on each push tick.
    let snap_agent = std::sync::Arc::clone(&agent);
    let snapshot = move || {
        let guard = snap_agent.try_lock();
        match guard {
            Ok(a) => {
                let snap = a.capture();
                let clock = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                anaphase::ci144::project_snapshot(&snap, clock)
            }
            Err(_) => anaphase::ci144::project_snapshot(
                &agent_offline_snapshot(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            ),
        }
    };

    // Action handler: protocol layer stays business-free; real actions
    // (send_message -> one cognitive period) live here, in the launcher.
    let act_agent = std::sync::Arc::clone(&agent);
    let handle_action = move |req: &anaphase::ci144::ActionRequest| {
        let act = std::sync::Arc::clone(&act_agent);
        // Clone owned inputs before the async block: the future must not
        // borrow the request reference (lifetime must outlive Fn).
        let action_id = req.action_id.clone();
        let message = req.parameters["message"].as_str().unwrap_or("").to_string();
        async move {
        use anaphase::ci144::ActionResponse;
        match action_id.as_str() {
            "send_message" => {
                let guard = act.try_lock();
                match guard {
                    Ok(mut a) => {
                        let cap = a.run_config.cycle_cap;
                        for _ in 0..cap {
                            match a.run_cycle(&message).await {
                                Ok(out) if out.done => break,
                                Ok(_) => continue,
                                Err(e) => {
                                    return ActionResponse::Failure {
                                        error: format!("cycle failed: {e}"),
                                        recoverable: true,
                                    };
                                }
                            }
                        }
                        let text = a.context.reasoning_output.clone();
                        ActionResponse::Success { message: text }
                    }
                    Err(_) => ActionResponse::Failure {
                        error: "agent busy".to_string(),
                        recoverable: true,
                    },
                }
            }
            "status" => {
                let guard = act.try_lock();
                match guard {
                    Ok(a) => {
                        let snap = a.capture();
                        ActionResponse::Success {
                            message: format!(
                                "mode={:?} state={:?} episode_step={} ledger={}",
                                snap.mode,
                                snap.state,
                                snap.episode.as_ref().map(|e| e.step).unwrap_or(0),
                                snap.ledger.len(),
                            ),
                        }
                    }
                    Err(_) => ActionResponse::Failure {
                        error: "agent busy".to_string(),
                        recoverable: true,
                    },
                }
            }
            other => ActionResponse::Failure {
                error: format!("unknown action: {other}"),
                recoverable: true,
            },
        }
        }
    };

    anaphase::ci144::server::run_stdio(
        snapshot,
        handle_action,
        anaphase::ci144::SNAPSHOT_PUSH_INTERVAL,
    )
    .await
    .map_err(|e| std::io::Error::other(e))?;
    eprintln!("Anaphase CI-144 mode exiting");
    Ok(())
}

/// Offline placeholder snapshot for the projection when the agent mutex is
/// momentarily contended (never blocks the push tick; physical fact: the
/// projection degrades to "unknown", it never lies).
fn agent_offline_snapshot() -> anaphase::run_cycle::AgentSnapshot {
    use anaphase::run_cycle::AgentSnapshot;
    AgentSnapshot {
        mode: crate::config::Mode::Partner,
        state: anaphase::states::HelixState::Perception,
        episode: None,
        ledger: vec![],
        ecosystem: vec![],
    }
}

