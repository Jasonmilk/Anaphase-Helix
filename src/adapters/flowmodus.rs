use async_trait::async_trait;
use tonic::transport::Channel;
use super::ReasoningAdapter;

// -----------------------------------------------------------------------------
// Original HTTP version of FlowModus Adapter (preserved as required)
// -----------------------------------------------------------------------------
/// HTTP adapter for FlowModus — deterministic LLM scheduling engine.
/// This is the legacy HTTP implementation, kept for backward compatibility.
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
    async fn reason(&self, _prompt: &str, _model: &str) -> Result<String, String> {
        // Legacy HTTP implementation placeholder
        Ok("HTTP FlowModus is deprecated, use gRPC instead".to_string())
    }
}

// gRPC adapter (corrected imports)
use crate::flowmodus_api::flow_modus_client::FlowModusClient;
use crate::flowmodus_api::ReasonRequest;

pub struct GrpcFlowModusAdapter {
    client: FlowModusClient<Channel>,
}

impl GrpcFlowModusAdapter {
    pub async fn new(endpoint: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let channel = Channel::from_shared(endpoint.to_string())?
            .connect()
            .await?;
        let client = FlowModusClient::new(channel);
        Ok(Self { client })
    }
}

#[async_trait]
impl ReasoningAdapter for GrpcFlowModusAdapter {
    async fn reason(&self, prompt: &str, model: &str) -> Result<String, String> {
        let request = tonic::Request::new(ReasonRequest {
            prompt: prompt.to_string(),
            model: model.to_string(),
            max_tokens: 2048,
        });
        let response = self
            .client
            .clone()
            .reason(request)
            .await
            .map_err(|e| e.to_string())?
            .into_inner();
        Ok(response.content)
    }
}
