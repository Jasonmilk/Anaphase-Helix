use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use super::ReasoningAdapter;

/// HTTP adapter for FlowModus — deterministic LLM scheduling sidecar.
///
/// FlowModus listens on localhost:8080 (default) and exposes an
/// OpenAI-compatible `/v1/chat/completions` endpoint. It supports
/// three call modes via the `model` parameter:
/// - `"auto"` → full 5-layer pipeline (normalization → cost → filter → entropy)
/// - `"group:fast-lane"` → user-defined group routing
/// - `"deepseek-chat"` → direct manual routing
pub struct FlowModusAdapter {
    endpoint: String,
    client: reqwest::Client,
}

impl FlowModusAdapter {
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: String,
}

#[async_trait]
impl ReasoningAdapter for FlowModusAdapter {
    async fn reason(&self, prompt: &str, model: &str) -> Result<String, String> {
        // Map Anaphase cognitive modes to FlowModus routing modes
        let flowmodus_model = match model {
            "left_brain" | "cerebellum" => "auto",     // Auto: let FlowModus decide
            "right_brain" => "auto",                    // Auto with higher temperature
            other => other,                             // Pass through manual model names
        };

        let request_body = ChatRequest {
            model: flowmodus_model.to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens: 2048,
        };

        let url = format!("{}/v1/chat/completions", self.endpoint);

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| format!("FlowModus unreachable at {}: {}", url, e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("FlowModus returned {}: {}", status, body));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse FlowModus response: {}", e))?;

        chat_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| "FlowModus returned empty response".to_string())
    }
}
