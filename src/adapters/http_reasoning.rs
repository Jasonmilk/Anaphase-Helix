use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::json;
use super::usage::{parse_usage, UpstreamMeta};
use super::ReasoningAdapter;

/// Identity rides the system channel: `[{system}, {user}]` — the vendor's
/// default identity is replaced, not argued with. No system = user-only
/// (honest degraded state, e.g. no gene lock configured).
fn messages_with_system(system: Option<&str>, prompt: &str) -> serde_json::Value {
    let mut msgs = Vec::new();
    if let Some(s) = system {
        if !s.trim().is_empty() {
            msgs.push(json!({"role": "system", "content": s}));
        }
    }
    msgs.push(json!({"role": "user", "content": prompt}));
    serde_json::Value::Array(msgs)
}

pub struct HttpReasoningAdapter {
    endpoint: String,
    model: String,
    api_key: Option<String>,
    route_tier: Option<String>,
    max_tokens: u32,
    /// L0 identity + L1 tool awareness — sent as the **system** message so
    /// it overrides the model vendor's default identity ("I am Agnes…").
    /// A user-role identity claim is context, not authority; a system-role
    /// identity claim is who the model IS for this session. This is the
    /// OpenAI-compatible channel contract, not a prompt trick.
    system_prompt: Option<String>,
    client: reqwest::Client,
    /// Upstream response metadata of the last round trip (ADR-0036 model +
    /// ADR-0038 usage): captured from the response, shared across the
    /// buffered and streaming paths, read by run_cycle via `last_meta()`.
    last_meta: std::sync::Arc<std::sync::Mutex<UpstreamMeta>>,
}

impl HttpReasoningAdapter {
    pub fn new(config: &crate::config::AnaphaseConfig, system_prompt: Option<String>) -> Self {
        Self {
            endpoint: config.reasoning_endpoint.clone().unwrap_or_default(),
            model: config.reasoning_model.clone().unwrap_or_default(),
            api_key: config.reasoning_api_key.clone(),
            route_tier: config.reasoning_route_tier.clone(),
            max_tokens: config.reasoning_max_tokens.unwrap_or(2048),
            system_prompt,
            last_meta: std::sync::Arc::new(std::sync::Mutex::new(UpstreamMeta::default())),
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

    /// Begin a round trip: the previous round trip's accounting has been
    /// spent, so drop it (ADR-0038 D12).
    ///
    /// Without this, an upstream that reports usage on one call and omits it
    /// on the next would have its EARLIER numbers read again — the period
    /// total would silently bill the same tokens twice, which is a fabricated
    /// fact, not a rounded one.
    ///
    /// The model is deliberately NOT cleared: it describes the route, not the
    /// call, and ADR-0036 reads it after the retry loop has settled.
    fn begin_round_trip(&self) {
        self.last_meta.lock().unwrap().usage = None;
    }

    /// Capture upstream response metadata (ADR-0038 D6/D7/D8): the routed
    /// model and the token accounting ride the same response, so they are
    /// read in one pass.
    ///
    /// The write is **presence-gated per field**: a field that is absent — or
    /// present but `null`, which some OpenAI-compatible gateways send — means
    /// "no update", never "clear". This is load-bearing for the streaming
    /// path, where only the final chunk carries `usage`: an unconditional
    /// write would wipe, on the way out, the very value just captured from it.
    fn capture_meta(&self, json: &serde_json::Value) {
        let mut meta = self.last_meta.lock().unwrap();
        if let Some(m) = json.get("model").and_then(|v| v.as_str()) {
            if !m.is_empty() {
                meta.model = Some(m.to_string());
            }
        }
        if let Some(u) = parse_usage(json.get("usage")) {
            meta.usage = Some(u);
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
            "messages": messages_with_system(self.system_prompt.as_deref(), prompt),
            "max_tokens": self.max_tokens,
            "stream": stream
        });
        let url = format!("{}/chat/completions", self.endpoint.trim_end_matches('/'));
        let mut req = self.client.post(&url).json(&body);
        // Carry the derived trace id to the gateway: the Tuck audit chain
        // records it as the trace_id, so chain + body trace + ledger share
        // one join key in Cellrix's ProveTrack view (missing header = "local").
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
    /// Upstream response metadata of the LAST round trip (ADR-0036 + 0038):
    /// the routed model and the token accounting. Read via the trait object
    /// by run_cycle (an inherent impl would never be reached).
    ///
    /// Contract: `usage` describes the round trip that just finished and
    /// nothing else — an adapter must not carry it across calls (D12). An
    /// absent report stays absent rather than replaying the previous call's
    /// numbers.
    fn last_meta(&self) -> UpstreamMeta {
        self.last_meta.lock().unwrap().clone()
    }

    async fn reason(&self, prompt: &str, _model: &str, trace_id: &str) -> Result<String, String> {
        self.begin_round_trip();
        let resp = self.post_chat(prompt, trace_id, false).await?;
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        self.capture_meta(&json);
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
        self.begin_round_trip();
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
            self.capture_meta(&json);
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
                            self.capture_meta(&v);
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
        let adapter = HttpReasoningAdapter::new(&cfg, None);
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
        let adapter = HttpReasoningAdapter::new(&cfg, None);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<crate::adapters::StreamDelta>();
        let full = adapter
            .reason_stream("hi", "fake", "t1", tx, &std::sync::Mutex::new(String::new()))
            .await
            .unwrap();
        assert_eq!(full, "plain");
        assert_eq!(rx.try_recv().unwrap().content, "plain");
    }

    // ---------- ADR-0038: upstream usage capture ----------

    fn adapter_for(addr: std::net::SocketAddr) -> HttpReasoningAdapter {
        let mut cfg = crate::config::AnaphaseConfig::default();
        cfg.reasoning_endpoint = Some(format!("http://{addr}"));
        cfg.reasoning_model = Some("fake".into());
        HttpReasoningAdapter::new(&cfg, None)
    }

    #[tokio::test]
    async fn stream_usage_survives_chunks_that_lack_it() {
        // Only the final chunk carries usage (measured: 1 of 41). The forty
        // that do not must not wipe the value captured from it.
        let gw = spawn_gateway(
            "data: {\"model\":\"m1\",\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\n\n\
             data: {\"model\":\"m1\",\"choices\":[{\"delta\":{\"content\":\"b\"}}]}\n\n\
             data: {\"model\":\"m1\",\"choices\":[{\"delta\":{}}],\"usage\":{\"prompt_tokens\":288,\"completion_tokens\":46,\"prompt_tokens_details\":{\"cached_tokens\":256}}}\n\n\
             data: [DONE]\n\n",
        )
        .await;
        let adapter = adapter_for(gw.addr);
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<crate::adapters::StreamDelta>();
        let full = adapter
            .reason_stream("hi", "fake", "t1", tx, &std::sync::Mutex::new(String::new()))
            .await
            .unwrap();
        assert_eq!(full, "ab");
        let meta = adapter.last_meta();
        assert_eq!(meta.model.as_deref(), Some("m1"));
        let u = meta.usage.expect("usage captured from the final chunk");
        assert_eq!(u.prompt_tokens, 288);
        assert_eq!(u.completion_tokens, 46);
        assert_eq!(u.cached_tokens, Some(256));
        assert_eq!(u.uncached_input(), Some(32));
        gw.handle.abort();
    }

    #[tokio::test]
    async fn stream_without_usage_reports_none() {
        let gw = spawn_gateway(
            "data: {\"model\":\"m1\",\"choices\":[{\"delta\":{\"content\":\"x\"}}]}\n\n\
             data: [DONE]\n\n",
        )
        .await;
        let adapter = adapter_for(gw.addr);
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<crate::adapters::StreamDelta>();
        let _ = adapter
            .reason_stream("hi", "fake", "t1", tx, &std::sync::Mutex::new(String::new()))
            .await
            .unwrap();
        let meta = adapter.last_meta();
        assert!(meta.usage.is_none(), "no upstream usage -> honest None, never zero");
        assert_eq!(meta.model.as_deref(), Some("m1"), "model is captured independently");
        gw.handle.abort();
    }

    #[tokio::test]
    async fn null_usage_never_clears_a_captured_value() {
        // Some OpenAI-compatible gateways fill absent fields with null; that
        // means "no update", never "clear".
        let gw = spawn_gateway(
            "data: {\"model\":\"m1\",\"choices\":[{\"delta\":{\"content\":\"a\"}}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":2}}\n\n\
             data: {\"model\":\"m1\",\"choices\":[{\"delta\":{\"content\":\"b\"}}],\"usage\":null}\n\n\
             data: [DONE]\n\n",
        )
        .await;
        let adapter = adapter_for(gw.addr);
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<crate::adapters::StreamDelta>();
        let _ = adapter
            .reason_stream("hi", "fake", "t1", tx, &std::sync::Mutex::new(String::new()))
            .await
            .unwrap();
        let u = adapter.last_meta().usage.expect("a null update must not clear it");
        assert_eq!(u.prompt_tokens, 10);
        gw.handle.abort();
    }

    #[tokio::test]
    async fn buffered_path_captures_usage() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else { break };
                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    let mut buf = [0u8; 4096];
                    let _ = sock.read(&mut buf).await;
                    let body = r#"{"model":"m2","choices":[{"message":{"content":"ok"}}],"usage":{"prompt_tokens":11,"completion_tokens":3}}"#;
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        body.len(), body
                    );
                    let _ = sock.write_all(resp.as_bytes()).await;
                });
            }
        });
        let adapter = adapter_for(addr);
        assert_eq!(adapter.reason("hi", "fake", "t1").await.unwrap(), "ok");
        let meta = adapter.last_meta();
        assert_eq!(meta.model.as_deref(), Some("m2"));
        assert_eq!(meta.usage.expect("buffered usage").prompt_tokens, 11);
    }

    #[tokio::test]
    async fn non_sse_fallback_captures_usage() {
        // Gateway ignores stream=true and answers plain JSON: the honest
        // fallback path must capture metadata too.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else { break };
                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    let mut buf = [0u8; 4096];
                    let _ = sock.read(&mut buf).await;
                    let body = r#"{"model":"m3","choices":[{"message":{"content":"plain"}}],"usage":{"prompt_tokens":7,"completion_tokens":1,"prompt_cache_hit_tokens":5}}"#;
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        body.len(), body
                    );
                    let _ = sock.write_all(resp.as_bytes()).await;
                });
            }
        });
        let adapter = adapter_for(addr);
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<crate::adapters::StreamDelta>();
        assert_eq!(
            adapter
                .reason_stream("hi", "fake", "t1", tx, &std::sync::Mutex::new(String::new()))
                .await
                .unwrap(),
            "plain"
        );
        let u = adapter.last_meta().usage.expect("fallback usage");
        assert_eq!(u.prompt_tokens, 7);
        assert_eq!(u.cached_tokens, Some(5));
    }

    /// ADR-0038 read-side: the accounting of a round trip belongs to that
    /// round trip alone. When the NEXT call reports nothing, the earlier
    /// numbers must not be read again — re-reading bills the same tokens
    /// twice and the period total silently doubles.
    #[tokio::test]
    async fn a_call_without_usage_does_not_replay_the_previous_call() {
        // Two consecutive round trips on one adapter: the first reports usage
        // in its final chunk, the second never mentions it.
        let bodies = vec![
            "data: {\"model\":\"m1\",\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\n\n\
             data: {\"model\":\"m1\",\"choices\":[{\"delta\":{}}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":2}}\n\n\
             data: [DONE]\n\n"
                .to_string(),
            "data: {\"model\":\"m1\",\"choices\":[{\"delta\":{\"content\":\"b\"}}]}\n\n\
             data: [DONE]\n\n"
                .to_string(),
        ];
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let mut served = 0usize;
            loop {
                let Ok((mut sock, _)) = listener.accept().await else { break };
                let body = bodies.get(served).cloned().unwrap_or_default();
                served += 1;
                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    let mut buf = [0u8; 4096];
                    let _ = sock.read(&mut buf).await;
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = sock.write_all(resp.as_bytes()).await;
                });
            }
        });
        let adapter = adapter_for(addr);
        let sink = std::sync::Mutex::new(String::new());

        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel::<crate::adapters::StreamDelta>();
        adapter
            .reason_stream("one", "fake", "t1", tx1, &sink)
            .await
            .unwrap();
        assert!(
            adapter.last_meta().usage.is_some(),
            "call 1 reported usage — it must be captured"
        );

        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel::<crate::adapters::StreamDelta>();
        adapter
            .reason_stream("two", "fake", "t2", tx2, &sink)
            .await
            .unwrap();
        assert_eq!(
            adapter.last_meta().usage,
            None,
            "call 2 reported no usage — replaying call 1's numbers would bill the same tokens twice"
        );
    }
}
