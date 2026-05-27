use async_trait::async_trait;
use tonic::transport::Channel;
use crate::tentacle_api::tentacle_client::TentacleClient;
use crate::tentacle_api::{ExecuteRequest, PerceiveRequest};
use super::ToolAdapter;

/// gRPC adapter for Helix-Tentacle execution layer.
pub struct GrpcTentacleAdapter {
    client: TentacleClient<Channel>,
}

impl GrpcTentacleAdapter {
    pub async fn new(endpoint: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let channel = Channel::from_shared(endpoint.to_string())?
            .connect()
            .await?;
        let client = TentacleClient::new(channel);
        Ok(Self { client })
    }
}

#[async_trait]
impl ToolAdapter for GrpcTentacleAdapter {
    async fn execute(&self, command: &str, args: &[String]) -> Result<String, String> {
        let request = tonic::Request::new(ExecuteRequest {
            command: command.to_string(),
            args: args.to_vec(),
            channel: "native".to_string(),
            timeout_ms: 30000,
            max_memory_mb: 128,
        });
        let response = self.client.clone()
            .execute(request)
            .await
            .map_err(|e| e.to_string())?
            .into_inner();
        if response.status != 0 {
            Err(format!("Command failed (status {}): {}", response.status, response.stderr))
        } else {
            Ok(response.stdout)
        }
    }

    async fn perceive(&self, query: &str) -> Result<String, String> {
        let request = tonic::Request::new(PerceiveRequest {
            query: query.to_string(),
        });
        let response = self.client.clone()
            .perceive(request)
            .await
            .map_err(|e| e.to_string())?
            .into_inner();
        Ok(response.data)
    }
}
