use async_trait::async_trait;
use super::ReasoningAdapter;

pub struct HttpReasoningAdapter {
    endpoint: String,
    model: String,
    api_key: Option<String>,
    max_tokens: u32,
    client: reqwest::Client,
}

impl HttpReasoningAdapter {
    pub fn new(config: &crate::config::AnaphaseConfig) -> Self {
        Self {
            endpoint: config.reasoning_endpoint.clone().unwrap_or_default(),
            model: config.reasoning_model.clone().unwrap_or_default(),
            api_key: config.reasoning_api_key.clone(),
            max_tokens: config.reasoning_max_tokens.unwrap_or(2048),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl ReasoningAdapter for HttpReasoningAdapter {
    async fn reason(&self, prompt: &str, _model: &str) -> Result<String, String> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": self.max_tokens
        });
        let url = format!("{}/chat/completions", self.endpoint.trim_end_matches('/'));
        let mut req = self.client.post(&url).json(&body);

        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let resp = req.send().await.map_err(|e| e.to_string())?;
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

        json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "No content in response".to_string())
    }
}
