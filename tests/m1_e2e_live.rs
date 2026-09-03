// M1.5-T4 live end-to-end: real Tentacle gRPC server + real fixture plugins.
//
// Unlike m1_e2e.rs (dual mock), this test spawns the REAL `tentacle` binary with
// `--transport grpc --plugins-dir fixtures/` and drives the pipeline over the
// real wire. It is the "Tentacle zero-change -> real connectivity" proof called
// for by ADR-0003 decision 7 / PLAN v1.7 candidate D.
//
// Requirements (manual run, hence #[ignore]):
//   1. helix-tentacle binary built:  cargo build -p tentacle  (in ../helix-tentacle)
//   2. node on PATH (fixture executables are .js)
//   3. Run from anaphase-helix:
//        cargo test --test m1_e2e_live -- --ignored --nocapture
//   Override the binary path with TENTACLE_BIN if the default relative path
//   (../helix-tentacle/target/debug/tentacle) is not correct.
//
// Fixtures are parameterized (fixture-data-shapes.md): default 20-series -> MET,
// {"series":[1.0]} -> UNMET.

use anaphase::adapters::http_reasoning::HttpReasoningAdapter;
use anaphase::adapters::ReasoningAdapter;
use anaphase::config::AnaphaseConfig;
use anaphase::ledger::{FakeClock, VerdictStatus};
use anaphase::pipeline::{Pipeline, PipelineConfig, PipelineInput};
use std::process::{Child, Command, Stdio};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

/// Minimal OpenAI-compatible /chat/completions mock (same as m1_e2e.rs).
async fn spawn_mock_llm(
    content: String,
) -> (
    String,
    oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => break,
                accepted = listener.accept() => {
                    if let Ok((mut stream, _)) = accepted {
                        let mut buf = [0u8; 4096];
                        let _ = stream.read(&mut buf).await;
                        let body = format!(
                            r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
                            serde_json::to_string(&content).unwrap()
                        );
                        let resp = format!(
                            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        );
                        let _ = stream.write_all(resp.as_bytes()).await;
                    }
                }
            }
        }
    });
    (format!("http://{}", addr), shutdown_tx, handle)
}

/// Reserve a free TCP port, then hand it to the real tentacle binary.
fn free_port() -> u16 {
    // Blocking bind on a local listener just to read an available port.
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .expect("free port")
}

/// Spawn the real tentacle binary as a gRPC server over the fixtures dir.
fn spawn_real_tentacle(port: u16) -> Child {
    let bin = std::env::var("TENTACLE_BIN").unwrap_or_else(|_| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../helix-tentacle/target/debug/tentacle"
        )
        .to_string()
    });
    let fixtures = concat!(env!("CARGO_MANIFEST_DIR"), "/../helix-tentacle/fixtures");
    Command::new(&bin)
        .args([
            "--transport",
            "grpc",
            "--plugins-dir",
            fixtures,
            "--grpc-port",
            &port.to_string(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| {
            panic!(
                "spawn real tentacle failed ({e}) — build it first with \
                 `cargo build -p tentacle` in ../helix-tentacle, or set TENTACLE_BIN"
            )
        })
}

/// Poll until the gRPC endpoint accepts a client, then return the adapter.
async fn connect_with_retry(endpoint: &str) -> anaphase::adapters::tentacle::GrpcTentacleAdapter {
    for _ in 0..60 {
        if let Ok(a) = anaphase::adapters::tentacle::GrpcTentacleAdapter::new(endpoint).await {
            return a;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("tentacle gRPC server not ready at {endpoint}");
}

/// One full live closed loop: mock LLM -> real Tentacle -> pipeline verdict.
async fn run_live_loop(llm_content: &str, job_id: &str, clock_now: u64) -> anaphase::pipeline::PipelineOutcome {
    // LLM mock (unchanged from M1).
    let (llm_endpoint, _llm_tx, _llm_handle) = spawn_mock_llm(llm_content.to_string()).await;
    let cfg = AnaphaseConfig {
        reasoning_endpoint: Some(llm_endpoint),
        reasoning_model: Some("mock".into()),
        reasoning_max_tokens: Some(16),
        ..AnaphaseConfig::default()
    };
    let llm = HttpReasoningAdapter::new(&cfg);
    let content = llm.reason("plan", "left_brain").await.unwrap();

    // Real Tentacle.
    let port = free_port();
    let mut child = spawn_real_tentacle(port);
    let endpoint = format!("http://127.0.0.1:{}", port);
    let tentacle = connect_with_retry(&endpoint).await;

    let pipe_config = PipelineConfig::from_codex("knowledge_base/fixture-codex.json").unwrap();
    let mut pipeline = Pipeline::new(tentacle, Box::new(FakeClock(clock_now)), pipe_config);
    let outcome = pipeline
        .run(PipelineInput {
            job_id: job_id.to_string(),
            created_at: "2026-09-03T00:00:00Z".to_string(),
            llm_content: content,
            identity_labels: std::collections::BTreeMap::new(),
        })
        .await
        .unwrap();

    // Best-effort cleanup of the real server.
    let _ = child.kill();
    let _ = child.wait();
    outcome
}

#[tokio::test]
#[ignore = "requires real tentacle binary + node (manual integration)"]
async fn m1_5_live_met() {
    // Real fixtures with default args -> 20-series numbers + rate 10/20 -> MET.
    let llm_content = r#"{"calls":[
        {"tool":"numbers","args":{},"expect":"numbers"},
        {"tool":"rate","args":{},"expect":"rate"}
    ]}"#;
    let outcome = run_live_loop(llm_content, "tt_job-live-met", 1000).await;
    assert_eq!(outcome.verdict, VerdictStatus::Met, "real fixtures must pass");
    assert!(outcome.retry_due.is_none());
    assert!(outcome.check_reports.iter().all(|r| r.passed), "{:?}", outcome.check_reports);
}

#[tokio::test]
#[ignore = "requires real tentacle binary + node (manual integration)"]
async fn m1_5_live_unmet() {
    // Parameterized fixture: {"series":[1.0]} -> short series -> UNMET + retry_due.
    let llm_content = r#"{"calls":[
        {"tool":"numbers","args":{"series":[1.0]},"expect":"numbers"}
    ]}"#;
    let outcome = run_live_loop(llm_content, "tt_job-live-unmet", 1000).await;
    assert_eq!(outcome.verdict, VerdictStatus::Unmet, "short series must fail");
    assert_eq!(outcome.retry_due, Some(4600), "clock 1000 + base_delay 3600");
    assert!(outcome.check_reports.iter().any(|r| !r.passed));
}
