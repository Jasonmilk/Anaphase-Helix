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
