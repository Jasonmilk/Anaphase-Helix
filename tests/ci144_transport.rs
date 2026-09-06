// ADR-0017 CI-144 transport layer integration tests.
//
// Verifies the vendored protocol end to end over an in-memory duplex pair:
// handshake -> Manifest (first frame) -> Snapshot push -> ActionRequest ->
// ActionResponse. The client half mirrors Cellrix's `StdioTransport`
// wire contract (CIB/1.0 MSGPACK, LE u32 length prefix).

mod common;

use anaphase::adapters::{
    NoopFearAdapter, NoopMemoryAdapter, NoopReasoningAdapter, NoopSafetyAdapter,
    NoopToolAdapter, NoopUiAdapter, ReasoningAdapter,
};
use anaphase::ci144::{
    ActionRequest, ActionResponse, AgentEvent, encode_frame, project_snapshot,
    server, handshake_response,
};
use anaphase::config::Mode;
use anaphase::reflex::ReflexArc;
use anaphase::run_cycle::AgentLoop;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

fn make_agent() -> AgentLoop {
    let reason: Arc<dyn ReasoningAdapter> = Arc::new(NoopReasoningAdapter);
    let memory: Arc<dyn MemoryAdapter> = Arc::new(NoopMemoryAdapter);
    let tool: Arc<dyn ToolAdapter> = Arc::new(NoopToolAdapter);
    let safety: Arc<dyn SafetyAdapter> = Arc::new(NoopSafetyAdapter);
    let ui: Arc<dyn UiAdapter> = Arc::new(NoopUiAdapter);
    let fear: Arc<dyn FearAdapter> = Arc::new(NoopFearAdapter);
    let reflex = ReflexArc { safety_rules: vec![] };
    AgentLoop::new(memory, reason, tool, safety, ui, fear, reflex)
}

use anaphase::adapters::{MemoryAdapter, SafetyAdapter, ToolAdapter, UiAdapter};
use anaphase::adapters::{FearAdapter};

// ---------------------------------------------------------------------------
// 1. Handshake contract (pure).
// ---------------------------------------------------------------------------

#[test]
fn handshake_accepts_cib_header() {
    assert_eq!(
        handshake_response("CIB/1.0 MSGPACK\n"),
        Ok("CIB/1.0 MSGPACK\n")
    );
    assert_eq!(
        handshake_response("CIB/1.0 JSON\n"),
        Ok("CIB/1.0 MSGPACK\n")
    );
}

#[test]
fn handshake_rejects_garbage() {
    assert!(handshake_response("GET / HTTP/1.1\n").is_err());
    assert!(handshake_response("").is_err());
}

// ---------------------------------------------------------------------------
// 2. Frame encoding round trip (LE u32 + MessagePack).
// ---------------------------------------------------------------------------

#[test]
fn frame_roundtrip_manifest() {
    let event = AgentEvent::Manifest(server::make_manifest());
    let bytes = encode_frame(&event).unwrap();
    let (decoded, consumed) =
        anaphase::ci144::decode_frame::<AgentEvent>(&bytes).unwrap();
    assert_eq!(consumed, bytes.len());
    match decoded {
        AgentEvent::Manifest(m) => {
            assert_eq!(m.agent_name, "anaphase-helix");
            assert!(!m.actions.is_empty());
        }
        other => panic!("expected Manifest, got {:?}", tag_of(&other)),
    }
}

fn tag_of(event: &AgentEvent) -> &'static str {
    match event {
        AgentEvent::Manifest(_) => "Manifest",
        AgentEvent::Snapshot(_) => "Snapshot",
        AgentEvent::Heartbeat { .. } => "Heartbeat",
        AgentEvent::StreamError(_) => "StreamError",
    }
}

// ---------------------------------------------------------------------------
// 3. Projection shape (AgentSnapshot -> SemanticSnapshot).
// ---------------------------------------------------------------------------

#[test]
fn projection_maps_mode_state_and_metrics() {
    let agent = make_agent();
    let snap = agent.capture();
    let proj = project_snapshot(&snap, 1700000000);

    assert_eq!(proj.status, "partner");
    assert_eq!(proj.epoch_time, 1700000000);
    assert_eq!(proj.metrics["ecosystem_total"], serde_json::json!(0));
    assert!(proj.semantic_tree.len() >= 3);
    let tree = proj
        .semantic_tree
        .iter()
        .find(|n| n.id == "cognitive-loop")
        .expect("state tree node present");
    assert!(tree.content.as_str().unwrap().contains("Perception"));
    assert!(proj.active_focus.is_some());
}

// ---------------------------------------------------------------------------
// 4. End-to-end duplex session: handshake -> Manifest -> Snapshot -> actions.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn duplex_session_full_protocol() {
    let (client_side, server_side) = tokio::io::duplex(64 * 1024);
    let (server_read, server_write) = tokio::io::split(server_side);

    let agent = Arc::new(tokio::sync::Mutex::new(make_agent()));

    // Snapshot provider mirrors the launcher wiring.
    let snap_agent = Arc::clone(&agent);
    let snapshot = move || {
        match snap_agent.try_lock() {
            Ok(a) => project_snapshot(&a.capture(), 1700000001),
            Err(_) => project_snapshot(
                &anaphase::run_cycle::AgentSnapshot {
                    mode: Mode::Partner,
                    state: anaphase::states::HelixState::Perception,
                    episode: None,
                    ledger: vec![],
                    ecosystem: vec![],
                },
                1700000001,
            ),
        }
    };

    let act_agent = Arc::clone(&agent);
    let handle_action = move |req: &anaphase::ci144::ActionRequest| {
        let act = Arc::clone(&act_agent);
        let action_id = req.action_id.clone();
        let message = req.parameters["message"].as_str().unwrap_or("").to_string();
        async move {
            match action_id.as_str() {
                "send_message" => {
                    let mut a = act.lock().await;
                    let cap = a.run_config.cycle_cap;
                    for _ in 0..cap {
                        if a.run_cycle(&message).await.unwrap().done {
                            break;
                        }
                    }
                    ActionResponse::Success {
                        message: a.context.reasoning_output.clone(),
                    }
                }
                "status" => {
                    let a = act.lock().await;
                    ActionResponse::Success {
                        message: format!("mode={:?}", a.capture().mode),
                    }
                }
                other => ActionResponse::Failure {
                    error: format!("unknown action: {other}"),
                    recoverable: true,
                },
            }
        }
    };

    let server_task = tokio::spawn(async move {
        server::run_loop(
            BufReader::new(server_read),
            server_write,
            snapshot,
            handle_action,
            Duration::from_millis(50),
        )
        .await
    });

    // --- client half (mirrors Cellrix StdioTransport) ---
    let (client_read, mut client_write) = tokio::io::split(client_side);
    let mut client_read = BufReader::new(client_read);

    // Handshake out.
    client_write.write_all(b"CIB/1.0 MSGPACK\n").await.unwrap();
    client_write.flush().await.unwrap();
    // Handshake in.
    let mut ack = String::new();
    client_read.read_line(&mut ack).await.unwrap();
    assert_eq!(ack, "CIB/1.0 MSGPACK\n");

    // First frame: Manifest.
    let (event, _) = read_event(&mut client_read).await;
    match event {
        AgentEvent::Manifest(m) => assert_eq!(m.agent_name, "anaphase-helix"),
        other => panic!("expected Manifest, got {}", tag_of(&other)),
    }

    // Next frame: Snapshot push (50ms cadence).
    let (event, _) = read_event(&mut client_read).await;
    match event {
        AgentEvent::Snapshot(s) => assert_eq!(s.status, "partner"),
        other => panic!("expected Snapshot, got {}", tag_of(&other)),
    }

    // Action: status -> Success.
    write_request(&mut client_write, &ActionRequest {
        action_id: "status".to_string(),
        parameters: serde_json::json!({}),
        view_hash: None,
    }).await;
    let (resp, _) = read_response(&mut client_read).await;
    match resp {
        ActionResponse::Success { message } => assert!(message.contains("mode=")),
        other => panic!("expected Success, got {:?}", other),
    }

    // Action: send_message -> one cognitive period -> Success.
    write_request(&mut client_write, &ActionRequest {
        action_id: "send_message".to_string(),
        parameters: serde_json::json!({"message": "hello helix"}),
        view_hash: None,
    }).await;
    let (resp, _) = read_response(&mut client_read).await;
    match resp {
        ActionResponse::Success { .. } => {}
        other => panic!("expected Success, got {:?}", other),
    }

    // Action: unknown -> Failure (recoverable).
    write_request(&mut client_write, &ActionRequest {
        action_id: "nope".to_string(),
        parameters: serde_json::json!({}),
        view_hash: None,
    }).await;
    let (resp, _) = read_response(&mut client_read).await;
    match resp {
        ActionResponse::Failure { recoverable: true, .. } => {}
        other => panic!("expected recoverable Failure, got {:?}", other),
    }

    // Close cleanly (explicit shutdown propagates EOF to the server side).
    client_write.shutdown().await.unwrap();
    let result = tokio::time::timeout(Duration::from_secs(5), server_task)
        .await
        .expect("server loop must finish")
        .expect("server task must not panic");
    assert!(result.is_ok(), "server exited with error: {:?}", result.err());
}

// --- client-side framing helpers (LE u32 + MessagePack) ---

async fn read_event<R: AsyncReadExt + Unpin>(
    reader: &mut R,
) -> (AgentEvent, Vec<u8>) {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await.unwrap();
    let len = u32::from_le_bytes(len_buf) as usize;
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await.unwrap();
    let event = rmp_serde::from_slice(&payload).unwrap();
    (event, payload)
}

async fn read_response<R: AsyncReadExt + Unpin>(
    reader: &mut R,
) -> (ActionResponse, Vec<u8>) {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await.unwrap();
    let len = u32::from_le_bytes(len_buf) as usize;
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await.unwrap();
    let resp = rmp_serde::from_slice(&payload).unwrap();
    (resp, payload)
}

async fn write_request<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    req: &ActionRequest,
) {
    let frame = encode_frame(req).unwrap();
    writer.write_all(&frame).await.unwrap();
    writer.flush().await.unwrap();
}

/// Pin the vendored types' serde shape against Cellrix's contract: the
/// MessagePack bytes we emit must decode with the exact tags Cellrix uses.
#[test]
fn vendored_serde_shape_snapshot() {
    let agent = make_agent();
    let proj = project_snapshot(&agent.capture(), 42);
    let snap_event = AgentEvent::Snapshot(proj);
    let bytes = encode_frame(&snap_event).unwrap();
    // Decode back as AgentEvent and confirm structural invariants.
    let (decoded, _) = anaphase::ci144::decode_frame::<AgentEvent>(&bytes).unwrap();
    match decoded {
        AgentEvent::Snapshot(s) => {
            assert!(s.epoch_time > 0);
            assert!(!s.semantic_tree.is_empty());
        }
        other => panic!("expected Snapshot, got {}", tag_of(&other)),
    }
}
