// M1-T0 acceptance: GrpcTentacleAdapter round-trips against the mock Tentacle
// server. Verifies proto wire-layer alignment between Anaphase and Tentacle v1
// (ADR-0003). Note: green here != verified connectivity with a real Tentacle;
// that is M1.5 scope.

mod common;

use anaphase::adapters::tentacle::GrpcTentacleAdapter;
use common::{spawn_mock_tentacle, MockTentacle};

#[tokio::test]
async fn execute_tool_roundtrip_returns_preset_data() {
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0,3.0]}"#);
    let (endpoint, _captured, _tx, _handle) = spawn_mock_tentacle(mock).await;

    let adapter = GrpcTentacleAdapter::new(&endpoint).await.unwrap();
    let resp = adapter
        .execute_tool("numbers", "{}", "tt_job-001#0")
        .await
        .unwrap();

    assert!(resp.ok, "expected ok=true");
    assert_eq!(resp.data, r#"{"series":[1.0,2.0,3.0]}"#);
    assert!(resp.error.is_empty());
    assert!(resp.duration_ms >= 1);
}

#[tokio::test]
async fn execute_tool_forwards_trace_id_verbatim() {
    let mock = MockTentacle::new().with_tool("rate", r#"{"numerator":10,"denominator":5}"#);
    let (endpoint, captured, _tx, _handle) = spawn_mock_tentacle(mock).await;

    let adapter = GrpcTentacleAdapter::new(&endpoint).await.unwrap();
    let trace_id = "tt_job-002#1";
    adapter.execute_tool("rate", "{}", trace_id).await.unwrap();

    assert_eq!(captured.all(), vec![trace_id.to_string()]);
}

// --- M1-T7: mock integration tests (ADR-0003) ---
// Branch coverage: success (T0), tool failure (ok=false), transport error (Err).

#[tokio::test]
async fn execute_tool_failure_branch_propagates_error() {
    let mock = MockTentacle::new().with_failing_tool("rate", "division by zero");
    let (endpoint, _captured, _tx, _handle) = spawn_mock_tentacle(mock).await;

    let adapter = GrpcTentacleAdapter::new(&endpoint).await.unwrap();
    let resp = adapter
        .execute_tool("rate", "{}", "tt_job-003#0")
        .await
        .unwrap();

    assert!(!resp.ok, "tool failure must surface as ok=false");
    assert_eq!(resp.error, "division by zero");
    assert!(resp.data.is_empty());
}

#[tokio::test]
async fn execute_tool_transport_error_returns_err() {
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0]}"#);
    let (endpoint, _captured, shutdown_tx, _handle) = spawn_mock_tentacle(mock).await;

    let adapter = GrpcTentacleAdapter::new(&endpoint).await.unwrap();
    // Bring the server down, then the next call must fail at the transport layer.
    // ★ 语义预期同步（K26，2026-10-09）：**适配器这一层不变** —— 仍然返回 Err。
    //   变的是**它上面那层**：Pipeline::execute_calls 现在会为这个失败留下一条具名的账本行
    //   （见本文件下面的 `execution_failure_leaves_a_named_ledger_row`）。
    //   即：此前"降级成 warn!（对测试不可见）"，现在"留下一行可从断言层看见的记录"。
    shutdown_tx.send(()).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let err = adapter
        .execute_tool("numbers", "{}", "tt_job-004#0")
        .await
        .unwrap_err();
    assert!(!err.is_empty(), "transport error must propagate");
}

/// ★ K26 判据 ③-a（2026-10-09）：执行期失败必须留下**【具名的行】**。
///
/// 为什么这条判据必须有：`ExecutionFailed` 落地之前，工具调用失败只返回 Err，
/// 被 `run_cycle` 降级成 `warn!` —— 而**测试二进制里没有日志订阅者** ⇒ 完全静音 ⇒
/// 看起来像"这个调用从未存在"（missing node，违背 `run_cycle/mod.rs:722`）。
/// ⇒ 故判据必须**从测试断言层可见**（不靠日志），且**断言行的内容**：
/// tool 对得上、`class` 是**稳定可 grep 的指纹**、`detail` 带原始错误文本。
/// 手法复用本文件既有的 `execute_tool_transport_error_returns_err`：把服务器弄下来。
#[tokio::test]
async fn execution_failure_leaves_a_named_ledger_row() {
    use anaphase::contract::Call;
    use anaphase::ledger::{FakeClock, LedgerRecord};
    use anaphase::pipeline::{Pipeline, PipelineConfig};

    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0]}"#);
    let (endpoint, _captured, shutdown_tx, _handle) = spawn_mock_tentacle(mock).await;
    let tentacle = GrpcTentacleAdapter::new(&endpoint).await.unwrap();
    let config = PipelineConfig::from_codex("knowledge_base/fixture-codex.json").unwrap();
    let mut pipeline = Pipeline::new(tentacle, Box::new(FakeClock(1000)), config);

    // 把服务器弄下来 —— 与 execute_tool_transport_error_returns_err 同一手法。
    shutdown_tx.send(()).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let job = Pipeline::assemble_tt_job(
        "tt_job-k26#0",
        "1970-01-01T00:16:40Z",
        vec![Call {
            tool: "numbers".to_string(),
            args: std::collections::BTreeMap::new(),
            expect: None,
        }],
    );
    let err = pipeline
        .execute_calls(&job, &std::collections::BTreeMap::new())
        .await
        .expect_err("a dead server must fail the call");
    assert!(!err.is_empty(), "the error must carry a reason");

    // ★ 断言【行的内容】，而不是"有没有日志"。
    let rows: Vec<_> = pipeline
        .ledger
        .records()
        .iter()
        .filter_map(|r| match r {
            LedgerRecord::ExecutionFailed { job_id, tool, index, class, detail, .. } => {
                Some((job_id.clone(), tool.clone(), *index, class.clone(), detail.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(rows.len(), 1, "expected exactly one ExecutionFailed row, got {rows:?}");
    let (job_id, tool, index, class, detail) = &rows[0];
    assert_eq!(job_id, "tt_job-k26#0", "the row must be attributable to the job");
    assert_eq!(tool, "numbers", "the row must name the tool");
    assert_eq!(*index, 0, "the row must name which call failed");
    assert_eq!(class, "transport", "class must be a stable, greppable fingerprint");
    assert!(!detail.is_empty(), "detail must carry the raw error text");
}
