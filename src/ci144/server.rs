//! CI-144 agent-side event loop (ADR-0017 D3/D5).
//!
//! Sequence: handshake -> Manifest (first frame) -> Snapshot push loop
//! (configurable cadence) + ActionRequest handling. The clock is injected
//! (deterministic), the snapshot provider is a closure (projection only —
//! no second source of truth).

use super::{
    ActionRequest, ActionResponse, AgentEvent, CapabilityManifest, SemanticSnapshot,
    encode_frame, handshake_response,
};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::time::Duration;


/// Build the capability manifest for this agent (ADR-0017 D3).
pub fn make_manifest() -> CapabilityManifest {
    use super::{Action, LayoutHints, SecurityClass};
    CapabilityManifest {
        agent_name: "anaphase-helix".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        actions: vec![Action {
            id: "status".to_string(),
            label: "Query current cognitive status".to_string(),
            security_class: SecurityClass::Normal,
            lease_ms: None,
            parameters: serde_json::json!({}),
        }],
        layout_hints: Some(LayoutHints {
            preferred_panels: vec!["cockpit".to_string()],
            grid: None,
        }),
    }
}

/// Run the CI-144 agent loop over arbitrary byte streams (stdio or UDS
/// halves). `snapshot` is invoked per push tick and must return the current
/// projection; `handle_action` maps an incoming ActionRequest to a response
/// (protocol layer stays business-free — extreme decoupling, ADR-0017 D5).
///
/// Returns when the peer closes the stream or a fatal protocol error occurs.
pub async fn run_loop<R, W, F, H, Fut>(
    mut reader: R,
    mut writer: W,
    snapshot: F,
    handle_action: H,
    push_interval: Duration,
) -> Result<(), String>
where
    R: AsyncReadExt + AsyncBufReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
    F: Fn() -> SemanticSnapshot + Send + Sync + 'static,
    H: Fn(&ActionRequest) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ActionResponse> + Send,
{
    // 1. Handshake: read the client's CIB/1.0 header line, reply with our
    //    chosen wire format (ADR-0017 D2).
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .map_err(|e| format!("handshake read failed: {e}"))?;
    let response = handshake_response(&line)?;
    writer
        .write_all(response.as_bytes())
        .await
        .map_err(|e| format!("handshake write failed: {e}"))?;
    writer.flush().await.map_err(|e| format!("flush failed: {e}"))?;

    // 2. First frame must be the manifest.
    let manifest_frame = encode_frame(&AgentEvent::Manifest(make_manifest()))
        .map_err(|e| format!("manifest encode failed: {e}"))?;
    writer
        .write_all(&manifest_frame)
        .await
        .map_err(|e| format!("manifest write failed: {e}"))?;
    writer.flush().await.map_err(|e| format!("flush failed: {e}"))?;

    // 3+4. Single select loop: push snapshots on the tick, handle
    //    ActionRequests when a frame arrives. One task, no Send gymnastics,
    //    deterministic ordering (BIND-19 heartbeat interval constant lives
    //    at the caller; push cadence is the argument).
    let mut ticker = tokio::time::interval(push_interval);
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    loop {
        let mut len_buf = [0u8; 4];
        tokio::select! {
            biased;
            _ = ticker.tick() => {
                let snap = snapshot();
                let frame = match encode_frame(&AgentEvent::Snapshot(snap)) {
                    Ok(f) => f,
                    Err(e) => return Err(format!("snapshot encode failed: {e}")),
                };
                if writer.write_all(&frame).await.is_err()
                    || writer.flush().await.is_err()
                {
                    return Err("snapshot write failed".to_string());
                }
            }
            res = reader.read_exact(&mut len_buf) => {
                match res {
                    Ok(_) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                        return Ok(()); // peer closed cleanly
                    }
                    Err(e) => return Err(format!("frame read failed: {e}")),
                }
                let frame_len = u32::from_le_bytes(len_buf) as usize;
                buf.resize(frame_len, 0);
                reader.read_exact(&mut buf).await
                    .map_err(|e| format!("frame payload read failed: {e}"))?;
                let request: ActionRequest = match rmp_serde::from_slice(&buf) {
                    Ok(r) => r,
                    Err(e) => return Err(format!("action request decode failed: {e}")),
                };

                let response = handle_action(&request).await;
                let frame = match encode_frame(&response) {
                    Ok(f) => f,
                    Err(e) => return Err(format!("response encode failed: {e}")),
                };
                if writer.write_all(&frame).await.is_err()
                    || writer.flush().await.is_err()
                {
                    return Err("response write failed".to_string());
                }
            }
        }
    }
}

/// Convenience: run the loop over tokio stdin/stdout (the `--stdio` entry).
pub async fn run_stdio<F, H, Fut>(
    snapshot: F,
    handle_action: H,
    push_interval: Duration,
) -> Result<(), String>
where
    F: Fn() -> SemanticSnapshot + Send + Sync + 'static,
    H: Fn(&ActionRequest) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ActionResponse> + Send,
{
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let reader = BufReader::new(stdin);
    let writer = BufWriter::new(stdout);
    run_loop(reader, writer, snapshot, handle_action, push_interval).await
}
