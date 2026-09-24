use async_trait::async_trait;
use tonic::transport::Channel;
use super::ReasoningAdapter;

// -----------------------------------------------------------------------------
// Original HTTP version of FlowModus Adapter (preserved as required)
// -----------------------------------------------------------------------------
/// HTTP adapter for FlowModus — deterministic LLM scheduling engine.
/// This is the legacy HTTP implementation, kept for backward compatibility.
/// `endpoint` is retained for structural completeness (the gRPC adapter is
/// the current channel); the field has no reader by design.
#[allow(dead_code)]
pub struct FlowModusAdapter {
    endpoint: String,
}

impl FlowModusAdapter {
    /// Create a new HTTP FlowModus adapter with the given endpoint.
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
        }
    }
}

#[async_trait]
impl ReasoningAdapter for FlowModusAdapter {
    async fn reason(&self, _prompt: &str, _mode: &str, _trace_id: &str) -> Result<String, String> {
        // Legacy HTTP implementation placeholder
        Ok("HTTP FlowModus is deprecated, use gRPC instead".to_string())
    }
}

use super::usage::{UpstreamMeta, UsageSnapshot};

// gRPC adapter (corrected imports)
use crate::flowmodus_api::flow_modus_client::FlowModusClient;
use crate::flowmodus_api::ReasonRequest;

pub struct GrpcFlowModusAdapter {
    client: FlowModusClient<Channel>,
    /// The model to ask for, from `reasoning_model`. Empty = let FlowModus route.
    model: String,
    /// What the last round trip actually was (ADR-0036 + 0038). Same shape as the
    /// HTTP adapter, for the same reason: this is response metadata already on the
    /// wire, and it is the ONLY source for `assistant/usage` and for the routed
    /// model's name.
    last_meta: std::sync::Arc<std::sync::Mutex<UpstreamMeta>>,
}

impl GrpcFlowModusAdapter {
    pub async fn new(endpoint: &str, model: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // tonic 的 Channel::from_shared 需要带 scheme（http://）的地址；
        // 调用方剥离 grpc:// 后只剩 host:port，这里补上（0 硬编码：只有缺 scheme 才补）。
        let addr = if endpoint.contains("://") {
            endpoint.to_string()
        } else {
            format!("http://{endpoint}")
        };
        let channel = Channel::from_shared(addr)?
            .connect()
            .await?;
        let client = FlowModusClient::new(channel);
        Ok(Self {
            client,
            model: model.to_string(),
            last_meta: std::sync::Arc::new(std::sync::Mutex::new(UpstreamMeta::default())),
        })
    }
}

#[async_trait]
impl ReasoningAdapter for GrpcFlowModusAdapter {
    async fn reason(&self, prompt: &str, mode: &str, _trace_id: &str) -> Result<String, String> {
        let request = tonic::Request::new(ReasonRequest {
            prompt: prompt.to_string(),
            cognitive_mode: mode.to_string(),
            model: self.model.clone(),
            max_tokens: 2048,
        });
        let response = self
            .client
            .clone()
            .reason(request)
            .await
            .map_err(|e| e.to_string())?
            .into_inner();
        /* Record the round trip where the response is in hand.
         *
         * This adapter never overrode `last_meta()`, so the routed model and the
         * usage the upstream reported were dropped on the floor: `emit_usage` saw
         * `usage: None` and wrote no `assistant/usage` row, and a turn that had
         * SUCCEEDED still showed `model: null` in the 证轨 — the one thing a
         * reader needs to tell which layer actually answered. */
        {
            let mut m = self.last_meta.lock().unwrap();
            m.model = if response.model.is_empty() { None } else { Some(response.model.clone()) };
            m.usage = response.usage.map(|u| UsageSnapshot {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                cached_tokens: u.cached_tokens,
                reasoning_tokens: u.reasoning_tokens,
            });
        }
        Ok(response.content)
    }

    fn last_meta(&self) -> UpstreamMeta {
        self.last_meta.lock().unwrap().clone()
    }
}
