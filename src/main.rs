
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
    // Config path flag: `--config <path>` wins over env/default (DNA 11 —
    // the cockpit stdio launcher injects the same config from any cwd).
    if let Some(cfg_path) = args.windows(2).find(|w| w[0] == "--config").map(|w| w[1].clone()) {
        std::env::set_var("ANAPHASE_CONFIG", cfg_path);
    }
    // Resource base: repo-relative paths (fixture-codex, …) must resolve
    // from the config's own directory, not the launcher cwd (TUI child).
    // Safe for the daemon too — its cwd is already the repo root.
    let cfg_abs = std::env::var("ANAPHASE_CONFIG")
        .map(|p| std::path::PathBuf::from(p))
        .unwrap_or_else(|_| std::path::PathBuf::from("config.toml"));
    if let Some(dir) = cfg_abs.parent().filter(|d| !d.as_os_str().is_empty()) {
        let _ = std::env::set_current_dir(dir);
    }
    // CI-144 stdio entry: accept both `--stdio` (native flag) and the
    // ecosystem launcher convention `--mode stdio` (Cellrix `--exec` appends
    // this pair for every stdio agent — one launch contract, every agent).
    let stdio_mode = args.iter().any(|a| a == "--stdio")
        || args.windows(2).any(|w| w[0] == "--mode" && w[1] == "stdio");

    if stdio_mode {
        run_stdio_mode().await?;
        return Ok(());
    }

    // O-5 (ADR-0023): demo-task source — CLI `--input` wins, then config
    // `smoke_input`, then this protocol default (no literal at the call site;
    // DNA principle 11). Demo task: exercises the six-stage pipeline with a
    // deterministic arithmetic job.
    const DEFAULT_SMOKE_INPUT: &str = "Calculate 2 to the power of 10";
    let cli_input = args
        .windows(2)
        .find(|w| w[0] == "--input")
        .map(|w| w[1].clone());

    tracing_subscriber::fmt::init();
    let config = config::load_config()?;

    let built = build_agent(&config).await;
    let mut agent = built.agent;
    let shared_events = built.events;

    // Candidate G-T2: shared snapshot projection (None = HTTP disabled).
    // The endpoint serves it; the loop refreshes it after each cycle.
    let mut shared_snapshot: Option<Arc<Mutex<Option<anaphase::run_cycle::AgentSnapshot>>>> = None;

    if config.anaphase.cap_http_enabled && !stdio_mode {
        use axum::{
            Router,
            middleware,
            routing::{get, post},
            extract::State,
            Json,
        };
        use std::sync::{Arc, Mutex};

        let shared: Arc<Mutex<Option<anaphase::run_cycle::AgentSnapshot>>> =
            Arc::new(Mutex::new(None));
        shared_snapshot = Some(shared.clone());

        // One-to-one binding (2026-09-07): Anaphase is the challenger. The
        // state lives here for the server's lifetime; the identity persists
        // to ~/.cellrix/anaphase-identity.json (0600) on confirm.
        let bind_state: Arc<Mutex<anaphase::bind::BindState>> =
            Arc::new(Mutex::new(anaphase::bind::BindState::default()));
        anaphase::bind::load(&config.anaphase, &bind_state);
        if bind_state.lock().unwrap().device.is_some() {
            println!("  bound: device {} (one-to-one, HMAC challenge-response)",
                bind_state.lock().unwrap().device.as_ref().unwrap().device_id);
        } else {
            println!("  bound: no — /v1/bind/start to bind your human (HITL)");
        }

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
            }))
            .route("/v1/trace", get({
                // Engram body half (2026-09-07): read-only, on-demand query
                // over the reasoning trace file. `trace_id` filters one round
                // (the join key the Tuck chain and the ledger share);
                // `limit` bounds the newest-window read. Reads only what a
                // query asks for — the file is append-only storage, never a
                // hot index (按需加载). Unconfigured -> empty, never 500.
                let trace_path = config.anaphase.reasoning_trace_path.clone();
                move |Query(params): Query<HashMap<String, String>>| async move {
                    let Some(path) = trace_path.as_deref() else {
                        return Json(serde_json::json!({
                            "configured": false, "count": 0, "entries": []
                        }));
                    };
                    let tid = params
                        .get("trace_id")
                        .map(|s| s.as_str())
                        .filter(|s| !s.is_empty());
                    // Endpoint protocol default: 20 entries per query.
                    let limit: usize = params
                        .get("limit")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(20);
                    match anaphase::trace::query_file(std::path::Path::new(path), tid, limit) {
                        Ok(entries) => Json(serde_json::json!({
                            "configured": true,
                            "count": entries.len(),
                            "entries": entries
                        })),
                        Err(e) => Json(serde_json::json!({
                            "configured": true, "count": 0, "entries": [],
                            "error": e.to_string()
                        })),
                    }
                }
            }))
            .route("/v1/sessions", get({
                // Session-management sidebar (Engram v2): one summary per
                // cognitive period, newest first. Reads only each file's
                // first/last row — on-demand, never a hot index. Unconfigured
                // or empty dir -> empty list, never 500 (按需加载).
                let events_dir = config.anaphase.session_events_path.clone();
                move |Query(params): Query<HashMap<String, String>>| async move {
                    let Some(dir) = events_dir.as_deref() else {
                        return Json(serde_json::json!({ "configured": false, "periods": [] }));
                    };
                    let limit: usize = params
                        .get("limit")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(50);
                    match anaphase::session_events::list_periods(
                        std::path::Path::new(dir),
                        limit,
                    ) {
                        Ok(periods) => Json(serde_json::json!({
                            "configured": true,
                            "periods": periods
                        })),
                        Err(e) => Json(serde_json::json!({
                            "configured": true, "periods": [], "error": e.to_string()
                        })),
                    }
                }
            }))
            .route("/v1/events", get({
                // One period's full event stream (Engram turn timeline):
                // the session-as-experience body (ADR-0026). `job_id` is the
                // derived run id — the same join key the Tuck audit chain
                // and the reasoning trace carry. Unknown id -> 404-style
                // empty list with a missing flag (honest, never 500).
                let events_dir = config.anaphase.session_events_path.clone();
                move |Query(params): Query<HashMap<String, String>>| async move {
                    let Some(dir) = events_dir.as_deref() else {
                        return Json(serde_json::json!({
                            "configured": false, "missing": true, "events": []
                        }));
                    };
                    let job_id = params
                        .get("job_id")
                        .map(|s| s.as_str())
                        .filter(|s| !s.is_empty())
                        .unwrap_or_default();
                    if job_id.is_empty() {
                        return Json(serde_json::json!({
                            "configured": true, "missing": true, "events": [],
                            "error": "job_id required"
                        }));
                    }
                    match anaphase::session_events::read_period(
                        std::path::Path::new(dir),
                        job_id,
                    ) {
                        Ok(events) => Json(serde_json::json!({
                            "configured": true, "missing": false, "events": events
                        })),
                        Err(e) => Json(serde_json::json!({
                            "configured": true, "missing": true, "events": [],
                            "error": e.to_string()
                        })),
                    }
                }
            }))
            .route("/v1/health", get({
                // Self-check (2026-09-07): Anaphase reports the physical
                // readiness of its own organs — config-derived, probed, never
                // guessed. The panel probes this instead of assuming.
                let acfg = config.anaphase.clone();
                move || async move { Json(anaphase::health::checks(&acfg)) }
            }))
            .route("/v1/bind/start", post({
                let acfg = config.anaphase.clone();
                let st = bind_state.clone();
                move || async move {
                    match anaphase::bind::start(&acfg, &st) {
                        Ok(v) => Json(v),
                        Err(e) => Json(serde_json::json!({ "error": e })),
                    }
                }
            }))
            .route("/v1/bind/confirm", post({
                let acfg = config.anaphase.clone();
                let st = bind_state.clone();
                move |Json(body): Json<serde_json::Value>| async move {
                    let code = body.get("pairing_code").and_then(|v| v.as_str()).unwrap_or("");
                    match anaphase::bind::confirm(&acfg, &st, code) {
                        Ok(v) => Json(v),
                        Err(e) => Json(serde_json::json!({ "error": e })),
                    }
                }
            }))
            .route("/v1/bind/status", get({
                let st = bind_state.clone();
                move || async move { Json(anaphase::bind::status(&st)) }
            }))
            .route("/v1/chat", post({
                // Partner-mode dialogue (2026-09-07): the panel's input box
                // lands here. Each request assembles a fresh Helix (build_agent
                // = same subconscious, same hand, same black box) and runs one
                // single-period cycle — no shared mutable state, no cross-
                // session bleed; conversation continuity is a future Memory
                // concern (L3 情景), not a v1 promise.
                // Fail-closed: Tuck down = refuse to reason (gate_ok), the
                // process stays alive to keep the panel honest.
                // Two transports, one contract: `Accept: text/event-stream`
                // yields live SSE deltas (typewriter chat, no timeout cliff);
                // the plain JSON path stays for curl / old clients.
                let cfg = config.clone();
                let shared = shared.clone();
                move |headers: axum::http::HeaderMap, Json(body): Json<serde_json::Value>| async move {
                    use axum::http::StatusCode;
                    use axum::response::IntoResponse;
                    let msg = body
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    if msg.is_empty() {
                        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "empty message" }))).into_response();
                    }
                    if let Err(e) = anaphase::health::gate_ok(&cfg.anaphase) {
                        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({
                            "error": "tuck unreachable", "detail": e.to_string()
                        }))).into_response();
                    }
                    let wants_sse = headers
                        .get(axum::http::header::ACCEPT)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("")
                        .contains("text/event-stream");
                    let mut built = build_agent(&cfg).await;
                    // Explicit continuation (ADR-0026): `job_id` names a
                    // previous experience; its last round is injected as
                    // true history so the new period continues the
                    // conversation instead of meeting a stranger.
                    if let Some(job) = body.get("job_id").and_then(|v| v.as_str()) {
                        if let Some(dir) = &cfg.anaphase.session_events_path {
                            let resume = anaphase::session_events::read_summary(
                                &std::path::PathBuf::from(dir),
                                job,
                                400,
                            );
                            if let Some(r) = resume {
                                built.agent.context.resume = Some(r);
                            }
                        }
                    }
                    if wants_sse {
                        // Stream deltas live, then a final line carries the
                        // full reply + done flag. The cycle itself is untouched
                        // — streaming is transport only (judgement still runs
                        // on the complete text).
                        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
                        built.agent.stream_tx = Some(tx);
                        let mut agent = built.agent;
                        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
                        let msg2 = msg.clone();
                        tokio::spawn(async move {
                            let res = agent.run_cycle(&msg2).await;
                            let reply = res.map(|_| agent.context.reasoning_output.clone());
                            *shared.lock().unwrap() = Some(agent.capture());
                            let _ = done_tx.send(reply);
                        });
                        let body = axum::body::Body::from_stream(futures_util::stream::unfold(
                            (rx, Some(done_rx), false),
                            |(mut rx, mut done, mut finished)| async move {
                                if finished {
                                    return None;
                                }
                                tokio::select! {
                                    Some(d) = rx.recv() => {
                                        let line = format!("data: {}\n\n", serde_json::json!({"delta": d}));
                                        Some((Ok::<_, std::convert::Infallible>(axum::body::Bytes::from(line)), (rx, done, false)))
                                    }
                                    r = async {
                                        match done.as_mut() {
                                            Some(d) => d.await,
                                            None => std::future::pending::<Result<Result<String, String>, _>>().await,
                                        }
                                    } => {
                                        match r {
                                            Ok(Ok(reply)) => {
                                                let line = format!("data: {}\n\n", serde_json::json!({"done": true, "reply": reply}));
                                                Some((Ok::<_, std::convert::Infallible>(axum::body::Bytes::from(line)), (rx, None, true)))
                                            }
                                            Ok(Err(e)) => {
                                                let line = format!("data: {}\n\n", serde_json::json!({"error": e}));
                                                Some((Ok::<_, std::convert::Infallible>(axum::body::Bytes::from(line)), (rx, None, true)))
                                            }
                                            Err(_) => None,
                                        }
                                    }
                                }
                            },
                        ));
                        return (
                            StatusCode::OK,
                            [
                                (axum::http::header::CONTENT_TYPE, "text/event-stream"),
                                (axum::http::header::CACHE_CONTROL, "no-cache"),
                            ],
                            body,
                        )
                            .into_response();
                    }
                    match built.agent.run_cycle(&msg).await {
                        Ok(out) => {
                            *shared.lock().unwrap() = Some(built.agent.capture());
                            (StatusCode::OK, Json(serde_json::json!({
                                "reply": built.agent.context.reasoning_output,
                                "done": out.done
                            })))
                        }
                        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e }))),
                    }
                    .into_response()
                }
            }))
            .layer(middleware::from_fn_with_state(
                bind_state.clone(),
                auth_mw,
            ));
        // Bind gate: once a human is bound, every endpoint except the bind
        // flow itself and /v1/health requires a signed Bearer (v1.<id>.<ts>.
        // <nonce>.<hmac>). Unbound = open — honest, never silently locked.
        use axum::response::IntoResponse;
        async fn auth_mw(
            State(st): State<Arc<Mutex<anaphase::bind::BindState>>>,
            req: axum::http::Request<axum::body::Body>,
            next: middleware::Next,
        ) -> axum::response::Response {
            let path = req.uri().path().to_string();
            let open = path.starts_with("/v1/bind/") || path == "/v1/health";
            if !open {
                let header = req
                    .headers()
                    .get(axum::http::header::AUTHORIZATION)
                    .and_then(|v| v.to_str().ok());
                if let Err(e) = anaphase::bind::verify_bearer(&st, header) {
                    return (axum::http::StatusCode::UNAUTHORIZED,
                            Json(serde_json::json!({ "error": e, "bound": true })))
                        .into_response();
                }
            }
            next.run(req).await
        }

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

    let user_input = cli_input
        .or_else(|| config.anaphase.smoke_input.clone())
        .unwrap_or_else(|| DEFAULT_SMOKE_INPUT.to_string());
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
        // Fail-closed gate (Tuck): refuse to reason while the audit/LLM
        // gateway is down — Tuck down = Helix stops thinking (SPOF
        // explicitly accepted). The process stays alive to keep showing
        // the panel and the honest ❌ state; only reasoning halts.
        if let Err(e) = anaphase::health::gate_ok(&config.anaphase) {
            eprintln!("\n⚠️  Tuck 不在岗，已停止工作：{e}");
            eprintln!("   请恢复 Tuck（如运行 `tuck` 网关）后重新运行本命令。");
            break;
        }
        let out = agent.run_cycle(&user_input).await?;
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
        format!("user: {}", &user_input),
        format!("assistant: {}", agent.context.reasoning_output),
        format!("reflection: {}", agent.context.reflection_notes),
    ];
    match session.dehydrate(&history) {
        Ok(d) => println!("[Dehydrate] 认知脱水完成（简报 {} 字，{} 条）", d.briefing.chars().count(), d.history_len),
        Err(e) => eprintln!("[Dehydrate] 写入失败：{}", e),
    }
    Ok(())
}

/// Shared full assembly for both entry points (daemon + CI-144 stdio
/// cockpit). One assembly, two faces: memory (Mind gRPC, fail-open),
/// reasoning (LLM chain, fail-open Noop), deterministic pipeline wired to
/// Tentacle, shared event ring, rails, judge, mode. Extreme reuse: the
/// cockpit gets exactly the same Helix as the daemon — same subconscious,
/// same hand, same black box.
struct BuiltAgent {
    agent: AgentLoop,
    events: Option<std::sync::Arc<std::sync::Mutex<anaphase::events::EventRing>>>,
}

async fn build_agent(config: &config::Config) -> BuiltAgent {
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
        // O-5 (ADR-0023): cognitive-injection budget from config (protocol
        // default 800 lives in config.rs, not here).
        agent.memory_inject_chars = config.anaphase.memory_inject_chars;
        // L0 identity (gene lock) + L1 tool awareness (Tentacle manifests):
        // assembled once per build — on-demand, never guessed. Any piece
        // unavailable degrades to "absent from the block" (honest).
        agent.identity_block = build_identity_block(&config).await;
        // Reasoning body trace (Engram join): opt-in via
        // `reasoning_trace_path`. Max chars: config override or the
        // documented protocol default (4096, README Engram section).
        // Redaction: built-in credential shapes + config extra literals.
        agent.trace = config
            .anaphase
            .reasoning_trace_path
            .as_ref()
            .map(|p| {
                let max = config
                    .anaphase
                    .reasoning_trace_max_chars
                    .unwrap_or(4096);
                let extra = config
                    .anaphase
                    .reasoning_redact_patterns
                    .clone()
                    .unwrap_or_default();
                anaphase::trace::ReasoningTrace::open(
                    std::path::PathBuf::from(p),
                    max,
                    anaphase::trace::Redaction::new(extra),
                )
                .expect("reasoning trace path must be openable")
            });
        // Session event stream (Engram turn timeline, ADR-0023): opt-in via
        // `session_events_path`. The stream opens per period inside
        // run_cycle (the derived job id exists only then); this wires the
        // directory and the redactor (same credential shapes as trace).
        agent.session_events_dir = config
            .anaphase
            .session_events_path
            .as_ref()
            .map(|d| std::path::PathBuf::from(d));
        agent.session_events_redact = anaphase::trace::Redaction::new(
            config
                .anaphase
                .reasoning_redact_patterns
                .clone()
                .unwrap_or_default(),
        );
        // O-6 (ADR-0024): judge-point backend — explicit selection, rules by
        // default; small_llm needs endpoint+model, else degrades to rules
        // (fail-safe, surfaced as a startup warning).
        let (judge, judge_warn) = anaphase::judge::resolve_judge(
            config.anaphase.judge_backend,
            config.anaphase.judge_endpoint.as_deref(),
            config.anaphase.judge_model.as_deref(),
            config.anaphase.mind.skilled_len,
            config.anaphase.mind.anchor_len,
        );
        if let Some(w) = judge_warn {
            eprintln!("[Judge] warning: {}", w);
        }
        agent.judge = judge;
        // ADR-0006: the interaction mode is the semantic record carried through
        // the loop; physical Mind participation is decided by resolve_memory_adapter
        // (Noop vs gRPC) — Drive auto-achieves "no experience written" through
        // the Noop adapter without any runtime branch.
        agent.mode = config.anaphase.run_cycle.mode;

        // ADR-0007 D'-3: wire the deterministic execution channel at startup.
        // `tentacle_endpoint` configured -> the six-stage pipeline replaces the
        // legacy echo fallback; empty/failed -> fail-open (None, legacy path).
        // Codex path: env override (12-factor) > repo-relative default
        // (cwd = anaphase-helix). Zero-hardcoding: the default literal is
        // the documented repo layout, overridable for any working dir.
        let codex_path = std::env::var("HELIX_CODEX")
            .unwrap_or_else(|_| "knowledge_base/fixture-codex.json".to_string());
        let pipeline_config =
            anaphase::pipeline::PipelineConfig::from_codex(&codex_path)
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
    BuiltAgent { agent, events: shared_events }
}


async fn run_stdio_mode() -> Result<(), Box<dyn std::error::Error>> {
    // Shared full assembly (same as the daemon): Mind gRPC memory (fail-open),
    // LLM reasoning, Tentacle pipeline, rails, judge, mode — the cockpit
    // talks to the exact same Helix as the HTTP daemon.
    let config = config::load_config()?;
    let built = build_agent(&config).await;
    let agent = std::sync::Arc::new(tokio::sync::Mutex::new(built.agent));

    eprintln!("Anaphase CI-144 transport mode active (ADR-0017): full stack assembled (mind+tentacle+pipeline+rails)");

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


/// Assemble the L0 + L1 identity block: gene_lock.md (immutable identity) and
/// Tentacle's registered tool manifests (tool awareness). Every part is
/// optional — a missing gene lock or unreachable Tentacle simply leaves that
/// section out, so the block never fabricates identity or capability.
async fn build_identity_block(config: &anaphase::config::Config) -> String {
    use anaphase::adapters::tentacle::GrpcTentacleAdapter;
    let mut parts: Vec<String> = Vec::new();

    if let Some(path) = config.anaphase.gene_lock_path.as_deref() {
        if let Ok(text) = std::fs::read_to_string(path) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                parts.push(format!(
                    "[identity — gene lock, immutable]\n{}",
                    trimmed
                ));
            }
        }
    }

    if let Some(ep) = config.anaphase.tentacle_endpoint.as_deref() {
        if !ep.is_empty() {
            if let Ok(mut adapter) = GrpcTentacleAdapter::new(ep)
                .await
                .map_err(|e| e.to_string())
            {
                if let Ok(tools) = adapter.list_tools().await {
                    if !tools.is_empty() {
                        let lines: Vec<String> = tools
                            .iter()
                            .map(|(name, desc)| format!("- {}: {}", name, desc))
                            .collect();
                        parts.push(format!(
                            "[tools available — use them before guessing; 0-token tools first, then few-token, then big LLM]\n{}\n\
When you need a tool, end your reply with ONLY: {{\"calls\":[{{\"tool\":\"NAME\",\"args\":{{...}},\"expect\":\"ok\"}}]}} — no markdown, no prose.",
                            lines.join("\n")
                        ));
                    }
                }
            }
        }
    }

    parts.join("\n\n")
}
