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
    shutdown_tx.send(()).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let err = adapter
        .execute_tool("numbers", "{}", "tt_job-004#0")
        .await
        .unwrap_err();
    assert!(!err.is_empty(), "transport error must propagate");
}
