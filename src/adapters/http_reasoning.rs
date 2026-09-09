use async_trait::async_trait;
use futures_util::StreamExt;
use super::ReasoningAdapter;

pub struct HttpReasoningAdapter {
    endpoint: String,
    model: String,
    api_key: Option<String>,
    route_tier: Option<String>,
    max_tokens: u32,
    client: reqwest::Client,
}

impl HttpReasoningAdapter {
    pub fn new(config: &crate::config::AnaphaseConfig) -> Self {
        Self {
            endpoint: config.reasoning_endpoint.clone().unwrap_or_default(),
            model: config.reasoning_model.clone().unwrap_or_default(),
            api_key: config.reasoning_api_key.clone(),
            route_tier: config.reasoning_route_tier.clone(),
            max_tokens: config.reasoning_max_tokens.unwrap_or(2048),
            // No idle connection reuse: a gateway-closed keep-alive makes the
            // second call fail with EAGAIN (os error 35). Fresh connect per
            // call is deterministic — the cheap local-LLM path pays no TLS,
            // the remote path pays one handshake per call (correctness first).
            client: reqwest::Client::builder()
                .pool_max_idle_per_host(0)
                .build()
                .expect("reqwest client build"),
        }
    }

    /// Shared chat-completions request: streaming flag + trace header + auth.
    async fn post_chat(
        &self,
        prompt: &str,
        trace_id: &str,
        stream: bool,
    ) -> Result<reqwest::Response, String> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": self.max_tokens,
            "stream": stream
        });
        let url = format!("{}/chat/completions", self.endpoint.trim_end_matches('/'));
        let mut req = self.client.post(&url).json(&body);
        // Carry the derived trace id to the gateway: the Tuck audit chain
        // records it as the trace_id, so chain + body trace + ledger share
        // one join key in Cellrix's Engram view (missing header = "local").
        req = req.header("x-tuck-trace", trace_id);
        if let Some(ref tier) = self.route_tier {
            req = req.header("X-Route-Tier", tier);
        }
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        req.send().await.map_err(|e| e.to_string())
    }
}

#[async_trait]
impl ReasoningAdapter for HttpReasoningAdapter {
    async fn reason(&self, prompt: &str, _model: &str, trace_id: &str) -> Result<String, String> {
        let resp = self.post_chat(prompt, trace_id, false).await?;
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "No content in response".to_string())
    }

    async fn reason_stream(
        &self,
        prompt: &str,
        _model: &str,
        trace_id: &str,
        deltas: tokio::sync::mpsc::UnboundedSender<crate::adapters::StreamDelta>,
        thinking: &std::sync::Mutex<String>,
    ) -> Result<String, String> {
        let resp = self.post_chat(prompt, trace_id, true).await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("gateway {status}: {text}"));
        }
        let ct = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        // Some gateways ignore stream=true and answer plain JSON — honest
        // fallback: buffer, emit once.
        if !ct.contains("text/event-stream") {
            let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            let full = json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let think = json["choices"][0]["message"]["reasoning_content"]
                .as_str()
                .unwrap_or("")
                .to_string();
            if !think.is_empty() {
                *thinking.lock().unwrap() = think.clone();
            }
            let _ = deltas.send(crate::adapters::StreamDelta {
                content: full.clone(),
                thinking: think,
            });
            return Ok(full);
        }
        // SSE line protocol: `data: {json}` separated by blank lines;
        // `data: [DONE]` closes the stream. Thinking (reasoning_content) is
        // forwarded as a disclosure slice; the returned full text is content
        // only — judgement must never run on the model's private deliberation.
        let mut full = String::new();
        let mut stream = resp.bytes_stream();
        let mut line = String::new();
        'outer: while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| e.to_string())?;
            let text = String::from_utf8_lossy(&chunk);
            for ch in text.chars() {
                if ch == '\n' {
                    let trimmed = line.trim();
                    if let Some(payload) = trimmed.strip_prefix("data:") {
                        let payload = payload.trim();
                        if payload == "[DONE]" {
                            break 'outer;
                        }
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
                            let delta = &v["choices"][0]["delta"];
                            let content = delta["content"].as_str().unwrap_or("");
                            let think = delta["reasoning_content"].as_str().unwrap_or("");
                            if !content.is_empty() {
                                full.push_str(content);
                            }
                            if !content.is_empty() || !think.is_empty() {
                                if !think.is_empty() {
                                    thinking.lock().unwrap().push_str(think);
                                }
                                let _ = deltas.send(crate::adapters::StreamDelta {
                                    content: content.to_string(),
                                    thinking: think.to_string(),
                                });
                            }
                        }
                    }
                    line.clear();
                } else {
                    line.push(ch);
                }
            }
        }
        Ok(full)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic fake gateway: answers a fixed SSE chat-completions
    /// stream so the parser contract is pinned byte-for-byte.
    struct FakeSseGateway {
        addr: std::net::SocketAddr,
        handle: tokio::task::JoinHandle<()>,
    }

    async fn spawn_gateway(stream_lines: &'static str) -> FakeSseGateway {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else { break };
                let body = stream_lines.to_string();
                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    let mut buf = [0u8; 4096];
                    let _ = sock.read(&mut buf).await; // swallow request
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = sock.write_all(resp.as_bytes()).await;
                });
            }
        });
        FakeSseGateway { addr, handle }
    }

    #[tokio::test]
    async fn reason_stream_parses_sse_deltas_in_order() {
        let gw = spawn_gateway(
            "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"think-a\",\"content\":\"hel\"}}]}\n\n\
             data: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\n\
             data: {\"choices\":[{\"delta\":{\"content\":\" world\"}}]}\n\n\
             data: [DONE]\n\n",
        )
        .await;
        let mut cfg = crate::config::AnaphaseConfig::default();
        cfg.reasoning_endpoint = Some(format!("http://{}", gw.addr));
        cfg.reasoning_model = Some("fake".into());
        let adapter = HttpReasoningAdapter::new(&cfg);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<crate::adapters::StreamDelta>();
        let full = adapter
            .reason_stream("hi", "fake", "t1", tx, &std::sync::Mutex::new(String::new()))
            .await
            .unwrap();
        assert_eq!(full, "hello world");
        let mut deltas = vec![];
        while let Ok(d) = rx.try_recv() {
            deltas.push(d);
        }
        assert_eq!(deltas[0].thinking, "think-a");
        assert_eq!(deltas[0].content, "hel");
        assert_eq!(deltas[1].content, "lo");
        assert_eq!(deltas[2].content, " world");
        gw.handle.abort();
    }

    #[tokio::test]
    async fn reason_stream_falls_back_to_plain_json() {
        // gateway ignores stream=true -> plain JSON body, honest fallback.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else { break };
                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    let mut buf = [0u8; 4096];
                    let _ = sock.read(&mut buf).await;
                    let body = r#"{"choices":[{"message":{"content":"plain"}}]}"#;
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = sock.write_all(resp.as_bytes()).await;
                });
            }
        });
        let mut cfg = crate::config::AnaphaseConfig::default();
        cfg.reasoning_endpoint = Some(format!("http://{addr}"));
        cfg.reasoning_model = Some("fake".into());
        let adapter = HttpReasoningAdapter::new(&cfg);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<crate::adapters::StreamDelta>();
        let full = adapter
            .reason_stream("hi", "fake", "t1", tx, &std::sync::Mutex::new(String::new()))
            .await
            .unwrap();
        assert_eq!(full, "plain");
        assert_eq!(rx.try_recv().unwrap().content, "plain");
    }
}
