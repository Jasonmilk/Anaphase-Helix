use async_trait::async_trait;
use std::collections::HashMap;
use tonic::transport::Channel;
use crate::tentacle_api::tentacle_service_client::TentacleServiceClient;
use crate::tentacle_api::{ExecuteToolRequest, ExecuteToolResponse};
use super::ToolAdapter;

/// gRPC adapter for the Helix-Tentacle execution layer (Tentacle v1 protocol).
///
/// M1 (ADR-0003 decision 11): the deterministic pipeline holds this adapter
/// directly via `execute_tool`; the `ToolAdapter` trait impl is retained as a
/// compatibility shim for run_cycle. identity_labels / seen_entropy_bloom
/// semantics are defined in M1.5 (ADR-0004) — see `execute_tool` docs.
pub struct GrpcTentacleAdapter {
    client: TentacleServiceClient<Channel>,
}

impl GrpcTentacleAdapter {
    pub async fn new(endpoint: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let channel = Channel::from_shared(endpoint.to_string())?
            .connect()
            .await?;
        let client = TentacleServiceClient::new(channel);
        Ok(Self { client })
    }

    /// Execute a tool with a raw JSON params string (M1 pipeline entry point).
    ///
    /// `trace_id` is forwarded verbatim (Anaphase is the trace root, DNA
    /// principle 9). Returns the full protocol response so the pipeline can
    /// inspect `ok` / `data` / `error` without lossy reshaping.
    ///
    /// M1.5 (ADR-0004): identity_labels / seen_entropy_bloom semantics.
    /// - identity_labels: caller identity for Tuck audit + progressive
    ///   disclosure (e.g. {"tenant":"...", "channel":"...", "app":"..."}).
    ///   Plain labels only — never credentials (Tentacle does not pass secrets).
    /// - seen_entropy_bloom: caller-side "already-seen entropy" Bloom filter,
    ///   used to detect duplicate / replayed tool calls (Callosum replay
    ///   defense). Optional: empty = no replay guard on this call.
    pub async fn execute_tool(
        &self,
        tool: &str,
        params: &str,
        trace_id: &str,
    ) -> Result<ExecuteToolResponse, String> {
        self.execute_tool_with_labels(tool, params, trace_id, HashMap::new(), String::new())
            .await
    }

    /// Execute a tool with explicit identity labels and replay-guard bloom.
    pub async fn execute_tool_with_labels(
        &self,
        tool: &str,
        params: &str,
        trace_id: &str,
        identity_labels: HashMap<String, String>,
        seen_entropy_bloom: String,
    ) -> Result<ExecuteToolResponse, String> {
        let request = tonic::Request::new(ExecuteToolRequest {
            tool: tool.to_string(),
            params: params.to_string(),
            identity_labels,
            trace_id: trace_id.to_string(),
            seen_entropy_bloom,
        });
        let response = self
            .client
            .clone()
            .execute_tool(request)
            .await
            .map_err(|e| e.to_string())?
            .into_inner();
        Ok(response)
    }
}

#[async_trait]
impl ToolAdapter for GrpcTentacleAdapter {
    /// run_cycle compatibility shim: maps (command, args) onto ExecuteTool.
    /// Not consumed by M1; replaced by execute_tool wiring in M1.5.
    async fn execute(&self, command: &str, args: &[String]) -> Result<String, String> {
        let params = serde_json::to_string(args).unwrap_or_else(|_| "[]".to_string());
        let resp = self.execute_tool(command, &params, "").await?;
        if !resp.ok {
            return Err(format!("tool '{}' failed: {}", command, resp.error));
        }
        Ok(resp.data)
    }

    /// Perceive has no counterpart in the Tentacle v1 protocol (no Perceive RPC).
    /// Returns an explicit error instead of inventing a fake mapping
    /// (zero hardcoding, DNA principle 11).
    async fn perceive(&self, _query: &str) -> Result<String, String> {
        Err("perceive is not supported by the Tentacle v1 protocol".to_string())
    }
}
