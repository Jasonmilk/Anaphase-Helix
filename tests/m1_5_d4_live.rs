// D'-4 live end-to-end: real Tentacle gRPC + REAL learned plugins (MCP-Learner).
//
// MCP-Learner learns a mock MCP Server -> post_learn review -> stable/
// manifests (mock-filesystem.*.manifest.json + mcp_proxy.js). This test
// spawns the REAL `tentacle` binary pointed at that learned plugin dir and
// drives the pipeline over the real wire with the `ok` expect shape
// (structured execution success, ADR-0009 D'-4).
//
// Requirements (manual run, hence #[ignore]):
//   1. helix-tentacle binary built:  cargo build -p tentacle  (in ../helix-tentacle)
//   2. node on PATH (mcp_proxy.js is a node script)
//   3. learned plugin dir exists (default: /tmp/d4-learn/stable):
//        cd ../Helix-MCP-Learner && ./target/release/mcp-learner learn \
//          --command python3 --args tests/mock_mcp_server.py \
//          --name mock-filesystem --output /tmp/d4-learn
//   4. Run from anaphase-helix:
//        cargo test --test m1_5_d4_live -- --ignored --nocapture
//   Override plugin dir with TENTACLE_PLUGINS_DIR, binary with TENTACLE_BIN.

mod common;

use anaphase::adapters::*;
use anaphase::agent_loop::AgentLoop;
use anaphase::ledger::{FakeClock, LedgerRecord, VerdictStatus};
use anaphase::pipeline::{Pipeline, PipelineConfig, PipelineInput};
use anaphase::reflex::ReflexArc;
use common::StructuredReasoning;
use std::collections::BTreeMap;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

/// Reserve a free TCP port, then hand it to the real tentacle binary.
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .map(|a| a.local_addr().unwrap().port())
        .expect("free port")
}

fn spawn_real_tentacle(port: u16) -> Child {
    let bin = std::env::var("TENTACLE_BIN").unwrap_or_else(|_| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../helix-tentacle/target/debug/tentacle"
        )
        .to_string()
    });
    let plugins = std::env::var("TENTACLE_PLUGINS_DIR")
        .unwrap_or_else(|_| "/tmp/d4-learn/stable".to_string());
    Command::new(&bin)
        .args([
            "--transport",
            "grpc",
            "--plugins-dir",
            &plugins,
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

/// One full D'-4 live loop: structured reasoning -> real Tentacle -> real
/// learned plugin -> pipeline verdict.
async fn run_d4_loop(llm_content: &str, job_id: &str, clock_now: u64) -> anaphase::pipeline::PipelineOutcome {
    let port = free_port();
    let mut child = spawn_real_tentacle(port);
    let endpoint = format!("http://127.0.0.1:{}", port);
    let tentacle = connect_with_retry(&endpoint).await;

    let pipe_config = PipelineConfig::from_codex("knowledge_base/fixture-codex.json").unwrap();
    let mut pipeline = Pipeline::new(tentacle, Box::new(FakeClock(clock_now)), pipe_config);
    let outcome = pipeline
        .run(PipelineInput {
            job_id: job_id.to_string(),
            created_at: "2026-09-06T00:00:00Z".to_string(),
            llm_content: llm_content.to_string(),
            identity_labels: BTreeMap::new(),
        })
        .await
        .unwrap();

    let _ = child.kill();
    let _ = child.wait();
    outcome
}

#[tokio::test]
#[ignore = "requires real tentacle binary + node + learned plugins (manual integration)"]
async fn m1_5_d4_live_plugin_met() {
    // Real learned plugin (mock-filesystem.list_files) -> structured ok -> MET.
    let llm_content = r#"{"calls":[
        {"tool":"mock-filesystem.list_files","args":{"directory":"/tmp"},"expect":"ok"}
    ]}"#;
    let outcome = run_d4_loop(llm_content, "tt_job-d4-live-met", 1000).await;
    assert_eq!(outcome.verdict, VerdictStatus::Met, "learned plugin must pass ok criteria");
    assert!(outcome.retry_due.is_none());
    assert!(outcome.check_reports.iter().all(|r| r.passed), "{:?}", outcome.check_reports);
}

#[tokio::test]
#[ignore = "requires real tentacle binary + node + learned plugins (manual integration)"]
async fn m1_5_d4_live_unknown_tool_errors() {
    // Unknown tool name -> gRPC transport error -> pipeline Err (single-pass
    // philosophy, ADR-0003 decision 5: execution errors raise, nothing is
    // recorded, nothing is retried — this is NOT an UNMET verdict).
    let llm_content = r#"{"calls":[
        {"tool":"mock-filesystem.does_not_exist","args":{},"expect":"ok"}
    ]}"#;
    let port = free_port();
    let mut child = spawn_real_tentacle(port);
    let endpoint = format!("http://127.0.0.1:{}", port);
    let tentacle = connect_with_retry(&endpoint).await;

    let pipe_config = PipelineConfig::from_codex("knowledge_base/fixture-codex.json").unwrap();
    let mut pipeline = Pipeline::new(tentacle, Box::new(FakeClock(1000)), pipe_config);
    let err = pipeline
        .run(PipelineInput {
            job_id: "tt_job-d4-live-unknown".to_string(),
            created_at: "2026-09-06T00:00:00Z".to_string(),
            llm_content: llm_content.to_string(),
            identity_labels: BTreeMap::new(),
        })
        .await
        .unwrap_err();

    assert!(err.contains("not found"), "expected transport not-found, got: {err}");
    let _ = child.kill();
    let _ = child.wait();
}

#[tokio::test]
#[ignore = "requires real tentacle binary + node + learned plugins (manual integration)"]
async fn m1_5_d4_live_run_cycle_real_plugin() {
    // run_cycle drives the FULL six-stage chain over the real tentacle binary
    // + the real learned plugin dir. Structured reasoning stub emits the calls
    // protocol directly; Execution hits the real gRPC wire; Reflection writes
    // the verdict ledger. Real learned plugin -> MET.
    let port = free_port();
    let mut child = spawn_real_tentacle(port);
    let endpoint = format!("http://127.0.0.1:{}", port);
    let tentacle = connect_with_retry(&endpoint).await;

    let pipe_config = PipelineConfig::from_codex("knowledge_base/fixture-codex.json").unwrap();
    let pipeline = Pipeline::new(tentacle, Box::new(FakeClock(1000)), pipe_config);

    let mut agent = AgentLoop::new(
        Arc::new(NoopMemoryAdapter),
        Arc::new(StructuredReasoning {
            output: r#"{"calls":[{"tool":"mock-filesystem.list_files","args":{"directory":"/tmp"},"expect":"ok"}]}"#.into(),
        }),
        Arc::new(NoopToolAdapter),
        Arc::new(NoopSafetyAdapter),
        Arc::new(NoopUiAdapter),
        Arc::new(NoopFearAdapter),
        ReflexArc { safety_rules: vec![] },
    )
    .with_pipeline(pipeline);

    agent.run_cycle("calculate").await.unwrap();

    let records = agent.pipeline.as_ref().unwrap().ledger.records();
    assert_eq!(records.len(), 1, "one verdict for the real plugin chain");
    match &records[0] {
        LedgerRecord::Verdict { status: VerdictStatus::Met, .. } => {}
        other => panic!("real run_cycle plugin chain must be MET, got: {other:?}"),
    }
    let _ = child.kill();
    let _ = child.wait();
}
