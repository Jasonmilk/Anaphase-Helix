// Shared test helpers for M1 integration tests.
// Replicates the mind_integration.rs mock-server pattern for the Tentacle v1 protocol.
// English comments only (project convention).
//
// Each integration-test target compiles this module independently, so items
// unused by one target would otherwise warn as dead_code — allow them here.
#![allow(dead_code)]

use anaphase::tentacle_api::tentacle_service_server::{TentacleService, TentacleServiceServer};
use anaphase::tentacle_api::{
    ExecuteToolRequest, ExecuteToolResponse, ExecuteToolStreamResponse, GetManifestRequest,
    GetManifestResponse, ListManifestsRequest, ListManifestsResponse,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio_stream::wrappers::{ReceiverStream, TcpListenerStream};
use tonic::{Request, Response, Status};

/// Captures every ExecuteTool trace_id that reaches the mock (wire-layer assertion).
#[derive(Clone, Default)]
pub struct CapturedTraceIds(Arc<Mutex<Vec<String>>>);

impl CapturedTraceIds {
    pub fn all(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

/// Mock Tentacle service. ExecuteTool returns a preset JSON payload keyed by tool
/// name, or an explicit failure for tools registered via `with_failing_tool`.
#[derive(Clone, Default)]
pub struct MockTentacle {
    tool_data: Arc<Mutex<HashMap<String, String>>>,
    failing_tools: Arc<Mutex<HashMap<String, String>>>,
    pub captured_trace_ids: CapturedTraceIds,
}

impl MockTentacle {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a preset response payload (JSON string) for a tool name.
    pub fn with_tool(self, tool: &str, data: &str) -> Self {
        self.tool_data
            .lock()
            .unwrap()
            .insert(tool.to_string(), data.to_string());
        self
    }

    /// Register a tool that responds with `ok=false` and the given error message.
    pub fn with_failing_tool(self, tool: &str, error: &str) -> Self {
        self.failing_tools
            .lock()
            .unwrap()
            .insert(tool.to_string(), error.to_string());
        self
    }
}

#[tonic::async_trait]
impl TentacleService for MockTentacle {
    async fn execute_tool(
        &self,
        request: Request<ExecuteToolRequest>,
    ) -> Result<Response<ExecuteToolResponse>, Status> {
        let req = request.into_inner();
        self.captured_trace_ids.0.lock().unwrap().push(req.trace_id.clone());
        if let Some(err) = self.failing_tools.lock().unwrap().get(&req.tool).cloned() {
            return Ok(Response::new(ExecuteToolResponse {
                ok: false,
                data: String::new(),
                error: err,
                stop_reason: None,
                duration_ms: 1,
            }));
        }
        let data = self
            .tool_data
            .lock()
            .unwrap()
            .get(&req.tool)
            .cloned()
            .unwrap_or_else(|| "{}".to_string());
        Ok(Response::new(ExecuteToolResponse {
            ok: true,
            data,
            error: String::new(),
            stop_reason: None,
            duration_ms: 1,
        }))
    }

    async fn list_manifests(
        &self,
        _request: Request<ListManifestsRequest>,
    ) -> Result<Response<ListManifestsResponse>, Status> {
        Ok(Response::new(ListManifestsResponse { manifests: vec![] }))
    }

    async fn get_manifest(
        &self,
        _request: Request<GetManifestRequest>,
    ) -> Result<Response<GetManifestResponse>, Status> {
        Ok(Response::new(GetManifestResponse { manifest: None }))
    }

    type ExecuteToolStreamStream =
        ReceiverStream<Result<ExecuteToolStreamResponse, Status>>;

    async fn execute_tool_stream(
        &self,
        _request: Request<ExecuteToolRequest>,
    ) -> Result<Response<Self::ExecuteToolStreamStream>, Status> {
        // M1 does not exercise streaming; return an empty stream (never panics).
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        drop(tx);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

/// Spawn the mock Tentacle server on an ephemeral port.
/// Returns (endpoint, captured_trace_ids, shutdown_tx, join_handle)
/// following the mind_integration pattern.
pub async fn spawn_mock_tentacle(
    mock: MockTentacle,
) -> (
    String,
    CapturedTraceIds,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let captured = mock.captured_trace_ids.clone();
    let svc = TentacleServiceServer::new(mock);
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let handle = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(svc)
            .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
    });
    (format!("http://{}", addr), captured, shutdown_tx, handle)
}
