use async_trait::async_trait;
use tonic::transport::Channel;
use helix_mind_api::helix_mind_client::HelixMindClient;
use helix_mind_api::{HelixQueryRequest, HelixQueryResult, RememberRequest, RememberResponse};
use super::{MemoryAdapter, QueryResult};

/// gRPC adapter for Helix-Mind memory service
pub struct GrpcMindAdapter {
    client: HelixMindClient<Channel>,
}

impl GrpcMindAdapter {
    /// Create a new GrpcMindAdapter and connect to the endpoint
    pub async fn new(endpoint: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let channel = Channel::from_shared(endpoint.to_string())?
            .connect()
            .await?;
        let client = HelixMindClient::new(channel);
        Ok(Self { client })
    }
}

#[async_trait]
impl MemoryAdapter for GrpcMindAdapter {
    async fn query(&self, query: &str, include_recessive: bool) -> Result<QueryResult, String> {
        let request = tonic::Request::new(HelixQueryRequest {
            query: query.to_string(),
            suggested_mode: 1, // Anchor
            energy_context: None,
            include_recessive,
            allow_imagination: false,
            autonomy_level: 1, // Open
        });

        let response = self
            .client
            .clone()
            .helix_query(request)
            .await
            .map_err(|e| e.to_string())?
            .into_inner();

        Ok(QueryResult {
            nodes: response.nodes.into_iter().map(|n| n.content_json).collect(),
            impasse_level: response.impasse_level as u8,
            suggested_actions: response
                .suggested_actions
                .into_iter()
                .map(|a| a.action_type)
                .collect(),
        })
    }

    async fn remember(&self, content: &str) -> Result<(), String> {
        let request = tonic::Request::new(RememberRequest {
            content: content.to_string(),
        });

        self.client
            .clone()
            .remember(request)
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}
