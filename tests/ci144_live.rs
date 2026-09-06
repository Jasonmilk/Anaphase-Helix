// ADR-0017 CI-144 transport: real-binary live probe (manual, #[ignore]).
//
// Spawns the actual `anaphase --stdio` binary and drives the full wire
// contract over its stdin/stdout: handshake -> Manifest -> Snapshot push ->
// ActionRequest (status / send_message / unknown) -> clean EOF.
//
// Run: cargo test --test ci144_live -- --ignored

use anaphase::ci144::{ActionRequest, AgentEvent, encode_frame};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::{Duration, timeout};

#[tokio::test]
#[ignore = "manual live probe: requires the real binary (cargo build first)"]
async fn live_stdio_ci144_roundtrip() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_anaphase"))
        .arg("--stdio")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn anaphase binary");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // 1. Handshake out + ack in.
    stdin.write_all(b"CIB/1.0 MSGPACK\n").await.unwrap();
    stdin.flush().await.unwrap();
    let mut ack = String::new();
    stdout.read_line(&mut ack).await.unwrap();
    assert_eq!(ack, "CIB/1.0 MSGPACK\n", "handshake ack mismatch");

    // 2. First frame: Manifest.
    let (event, _) = read_event(&mut stdout).await;
    match &event {
        AgentEvent::Manifest(m) => {
            assert_eq!(m.agent_name, "anaphase-helix");
            assert!(m.actions.iter().any(|a| a.id == "status"));
        }
        other => panic!("expected Manifest, got {:?}", other),
    }

    // 3. Snapshot push (1s default cadence; wait up to 3s).
    let (event, _) = timeout(Duration::from_secs(3), read_event(&mut stdout))
        .await
        .expect("snapshot push timeout");
    match event {
        AgentEvent::Snapshot(s) => {
            assert_eq!(s.status, "partner");
            assert!(!s.semantic_tree.is_empty());
        }
        other => panic!("expected Snapshot, got {:?}", other),
    }

    // 4. status action.
    write_request(&mut stdin, &ActionRequest {
        action_id: "status".to_string(),
        parameters: serde_json::json!({}),
        view_hash: None,
    }).await;
    let (resp, _) = read_response(&mut stdout).await;
    match resp {
        anaphase::ci144::ActionResponse::Success { message } => {
            assert!(message.contains("mode="), "status message: {message}");
        }
        other => panic!("expected status Success, got {:?}", other),
    }

    // 5. send_message -> one real cognitive period.
    write_request(&mut stdin, &ActionRequest {
        action_id: "send_message".to_string(),
        parameters: serde_json::json!({"message": "hello helix"}),
        view_hash: None,
    }).await;
    let (resp, _) = timeout(Duration::from_secs(5), read_response(&mut stdout))
        .await
        .expect("send_message response timeout");
    match resp {
        anaphase::ci144::ActionResponse::Success { message } => {
            assert!(!message.is_empty(), "send_message returned empty text");
        }
        other => panic!("expected send_message Success, got {:?}", other),
    }

    // 6. unknown action -> recoverable Failure.
    write_request(&mut stdin, &ActionRequest {
        action_id: "nope".to_string(),
        parameters: serde_json::json!({}),
        view_hash: None,
    }).await;
    let (resp, _) = read_response(&mut stdout).await;
    match resp {
        anaphase::ci144::ActionResponse::Failure { recoverable: true, .. } => {}
        other => panic!("expected recoverable Failure, got {:?}", other),
    }

    // 7. Clean EOF: dropping stdin closes the write half; the loop returns
    //    on UnexpectedEof (verified via the plain-pipe probe above).
    drop(stdin);
    let status = timeout(Duration::from_secs(5), child.wait())
        .await
        .expect("binary must exit after EOF")
        .expect("wait failed");
    assert!(status.success(), "binary exited with {status}");
}

async fn read_event<R: AsyncReadExt + Unpin>(reader: &mut R) -> (AgentEvent, Vec<u8>) {
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
) -> (anaphase::ci144::ActionResponse, Vec<u8>) {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await.unwrap();
    let len = u32::from_le_bytes(len_buf) as usize;
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await.unwrap();
    let resp = rmp_serde::from_slice(&payload).unwrap();
    (resp, payload)
}

async fn write_request<W: AsyncWriteExt + Unpin>(writer: &mut W, req: &ActionRequest) {
    let frame = encode_frame(req).unwrap();
    writer.write_all(&frame).await.unwrap();
    writer.flush().await.unwrap();
}
