// M1-T8 end-to-end: deterministic pipeline closed loop (ADR-0003).
//
// Dual mock (LLM + Tentacle) with inlined fixtures. Two cases:
//   m1_e2e_met    - all criteria pass -> MET verdict recorded
//   m1_e2e_unmet  - a check fails      -> UNMET + retry_due recorded
// Acceptance criteria:
//   1. both cases green (double mock + inlined fixtures)
//   2. same input run twice -> ledger output byte-identical
//      (same job_id + same FakeClock)
//   3. fake clock -> reopen scan finds due UNMET records

mod common;

use anaphase::adapters::http_reasoning::HttpReasoningAdapter;
use anaphase::adapters::ReasoningAdapter;
use anaphase::config::AnaphaseConfig;
use anaphase::ledger::{FakeClock, LedgerRecord, VerdictStatus};
use anaphase::pipeline::{Pipeline, PipelineConfig, PipelineInput};
use common::{spawn_mock_tentacle, MockTentacle};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

/// Minimal OpenAI-compatible /chat/completions mock returning a fixed content.
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

/// Drive the LLM through HttpReasoningAdapter, then the full pipeline.
async fn run_closed_loop(
    llm_content: &str,
    job_id: &str,
    tentacle_mock: MockTentacle,
    clock_now: u64,
) -> (anaphase::pipeline::PipelineOutcome, Pipeline) {
    // LLM mock
    let (llm_endpoint, _llm_tx, _llm_handle) = spawn_mock_llm(llm_content.to_string()).await;
    let cfg = AnaphaseConfig {
        reasoning_endpoint: Some(llm_endpoint),
        reasoning_model: Some("mock".into()),
        reasoning_max_tokens: Some(16),
        ..AnaphaseConfig::default()
    };
    let llm = HttpReasoningAdapter::new(&cfg);
    let content = llm.reason("plan", "left_brain").await.unwrap();

    // Tentacle mock
    let (tent_endpoint, _captured, _tx, _handle) = spawn_mock_tentacle(tentacle_mock).await;
    let tentacle = anaphase::adapters::tentacle::GrpcTentacleAdapter::new(&tent_endpoint)
        .await
        .unwrap();

    let pipe_config = PipelineConfig::from_codex("knowledge_base/fixture-codex.json").unwrap();
    let mut pipeline = Pipeline::new(tentacle, Box::new(FakeClock(clock_now)), pipe_config);
    let outcome = pipeline
        .run(PipelineInput {
            job_id: job_id.to_string(),
            created_at: "2026-09-03T00:00:00Z".to_string(),
            llm_content: content,
        })
        .await
        .unwrap();
    (outcome, pipeline)
}

#[tokio::test]
async fn m1_e2e_met() {
    let llm_content = r#"{"calls":[
        {"tool":"numbers","args":{},"expect":"numbers"},
        {"tool":"rate","args":{},"expect":"rate"}
    ]}"#;
    let tentacle_mock = MockTentacle::new()
        .with_tool("numbers", r#"{"series":[1.0,2.0,3.0,4.0]}"#)
        .with_tool("rate", r#"{"numerator":10.0,"denominator":10.0}"#);

    let (outcome, pipeline) = run_closed_loop(llm_content, "tt_job-e2e-met", tentacle_mock, 1000).await;

    assert_eq!(outcome.verdict, VerdictStatus::Met, "all criteria pass");
    assert!(outcome.retry_due.is_none());
    assert_eq!(outcome.evidence_ids.len(), 2);
    assert!(outcome.check_reports.iter().all(|r| r.passed), "{:?}", outcome.check_reports);

    // Ledger holds a MET verdict; JSONL is well-formed.
    let jsonl = pipeline.ledger.to_jsonl();
    assert!(jsonl.contains("\"record_type\":\"verdict\""));
    assert!(jsonl.contains("\"status\":\"MET\""));
}

#[tokio::test]
async fn m1_e2e_unmet() {
    let llm_content = r#"{"calls":[
        {"tool":"numbers","args":{},"expect":"numbers"}
    ]}"#;
    // Short series -> sequence_length(min_len=3) fails.
    let tentacle_mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0]}"#);

    let (outcome, pipeline) = run_closed_loop(llm_content, "tt_job-e2e-unmet", tentacle_mock, 1000).await;

    assert_eq!(outcome.verdict, VerdictStatus::Unmet, "short series must fail");
    // base_delay_secs=3600 from codex; clock=1000 -> retry_due=4600
    assert_eq!(outcome.retry_due, Some(4600));
    assert!(outcome.check_reports.iter().any(|r| !r.passed));

    // Acceptance criterion 3: reopen scan (fake clock) finds the due UNMET.
    let due = pipeline.ledger.scan_due(4600);
    assert_eq!(due.len(), 1);
    match due[0] {
        LedgerRecord::Verdict { status: VerdictStatus::Unmet, job_id, .. } => {
            assert_eq!(job_id, "tt_job-e2e-unmet");
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn m1_e2e_deterministic_replay() {
    // Acceptance criterion 2: same input run twice with same job_id + FakeClock
    // must produce byte-identical ledger output.
    let llm_content = r#"{"calls":[
        {"tool":"rate","args":{},"expect":"rate"}
    ]}"#;
    let mk_tentacle = || {
        MockTentacle::new().with_tool("rate", r#"{"numerator":10.0,"denominator":10.0}"#)
    };

    let (_, p1) = run_closed_loop(llm_content, "tt_job-e2e-replay", mk_tentacle(), 777).await;
    let (_, p2) = run_closed_loop(llm_content, "tt_job-e2e-replay", mk_tentacle(), 777).await;

    assert_eq!(
        p1.ledger.to_jsonl(),
        p2.ledger.to_jsonl(),
        "same input + same clock must be byte-identical"
    );
}
