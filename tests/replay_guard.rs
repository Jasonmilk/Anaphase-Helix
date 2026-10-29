//! Replay-guard fingerprint + bootstrap wiring tests — ADR-0007 (D'-1 / D'-3).
//!
//! D'-1: `seen_entropy_bloom` carries a real deterministic fingerprint
//! (`bl-` + FNV-1a over `{tool}#{params}`) instead of the `""` placeholder.
//! D'-3: `resolve_pipeline` wires the deterministic execution channel at
//! startup with fail-open semantics (empty endpoint / failed connect -> None).
//! 2026-10-29: that single `None` carried two meanings (factory state vs broken
//! environment) — `resolve_binding` now separates them (Unbound / Unreachable).
//! The assembly stays fail-open; only the vocabulary was split.

use std::collections::BTreeMap;

use anaphase::contract::derive_seen_bloom;
use anaphase::pipeline::{resolve_binding, resolve_pipeline, Pipeline, PipelineBinding, PipelineConfig};

mod common;
use common::{spawn_mock_tentacle, MockTentacle};

/// Replay-shaped pipeline input (same job_id, same calls, same labels).
fn replay_input() -> anaphase::pipeline::PipelineInput {
    anaphase::pipeline::PipelineInput {
        job_id: "tt_job-replay".into(),
        created_at: "2026-09-05T00:00:00Z".into(),
        llm_content: r#"{"calls":[{"tool":"numbers","args":{},"expect":"numbers"}]}"#.into(),
        identity_labels: BTreeMap::new(),
    }
}

/// Wire a pipeline to a fresh MockTentacle. The shutdown sender and join
/// handle are returned so the test keeps the server alive until it ends
/// (dropping the sender shuts the mock server down).
async fn wired_pipeline(
    mock: MockTentacle,
) -> (
    Pipeline,
    MockTentacle,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let (endpoint, _captured, shutdown_tx, handle) = spawn_mock_tentacle(mock.clone()).await;
    let config = PipelineConfig::from_codex("knowledge_base/fixture-codex.json").unwrap();
    (
        Pipeline::new(
            anaphase::adapters::tentacle::GrpcTentacleAdapter::new(&endpoint)
                .await
                .unwrap(),
            Box::new(anaphase::ledger::FakeClock(1000)),
            config,
        ),
        mock,
        shutdown_tx,
        handle,
    )
}

#[tokio::test]
async fn execute_calls_carries_real_bloom_fingerprint() {
    let mock = MockTentacle::new().with_tool("numbers", "{}");
    let (mut pipeline, mock, _shutdown_tx, _handle) = wired_pipeline(mock).await;
    let outcome = pipeline.run(replay_input()).await.unwrap();
    assert_eq!(outcome.job_id, "tt_job-replay");

    // The wire carried a real deterministic fingerprint, not the "" placeholder.
    let blooms = mock.captured_bloom.all();
    assert_eq!(blooms.len(), 1);
    assert_eq!(blooms[0], derive_seen_bloom("numbers", "{}"));
    assert!(blooms[0].starts_with("bl-"));
    assert_eq!(blooms[0].len(), 3 + 16);
}

#[tokio::test]
async fn bloom_fingerprint_is_replay_stable() {
    let mock = MockTentacle::new().with_tool("numbers", "{}");
    let (mut pipeline, mock, _shutdown_tx, _handle) = wired_pipeline(mock).await;
    pipeline.run(replay_input()).await.unwrap();
    pipeline.run(replay_input()).await.unwrap();
    // Same job twice -> byte-identical fingerprints (deterministic replay).
    let blooms = mock.captured_bloom.all();
    assert_eq!(blooms, vec![derive_seen_bloom("numbers", "{}"); 2]);
}

#[tokio::test]
async fn resolve_pipeline_fails_open_on_empty_or_bad_endpoint() {
    let config = PipelineConfig::from_codex("knowledge_base/fixture-codex.json").unwrap();
    // Empty endpoint -> None (legacy echo fallback keeps working).
    assert!(resolve_pipeline(None, config.clone()).await.is_none());
    assert!(resolve_pipeline(Some(String::new()), config.clone()).await.is_none());
    // Unreachable endpoint -> None, never a startup error (fail-open).
    assert!(resolve_pipeline(Some("http://127.0.0.1:1".into()), config).await.is_none());
}

#[tokio::test]
async fn resolve_pipeline_wires_mock_endpoint() {
    let mock = MockTentacle::new().with_tool("numbers", "{}");
    let (endpoint, _captured, _tx, _handle) = spawn_mock_tentacle(mock.clone()).await;
    let config = PipelineConfig::from_codex("knowledge_base/fixture-codex.json").unwrap();
    let pipeline = resolve_pipeline(Some(endpoint), config).await;
    assert!(pipeline.is_some(), "configured endpoint must wire the pipeline");
}

/// 三态必须**可区分**（2026-10-29 拆 `Option<Pipeline>` 的两义 None）。
///
/// 反样本就是旧形状本身：`None` 对「没配」与「配了连不上」给出同一个值，
/// 于是「这台机器出厂就没有执行通道」与「执行通道坏了」在后代代码里不可分。
/// 这里断言三件事：state 名不同、refusal 文案不同、且只有 Bound 有 pipeline。
#[tokio::test]
async fn resolve_binding_separates_unbound_from_unreachable() {
    let config = PipelineConfig::from_codex("knowledge_base/fixture-codex.json").unwrap();

    let unbound = resolve_binding(None, config.clone()).await;
    let empty = resolve_binding(Some(String::new()), config.clone()).await;
    let unreachable = resolve_binding(Some("http://127.0.0.1:1".into()), config).await;

    // 1) 三态名互不相同：出厂态 ≠ 环境故障。
    assert_eq!(unbound.state(), "unbound");
    assert_eq!(empty.state(), "unbound", "empty string is the factory state, not a fault");
    assert_eq!(unreachable.state(), "unreachable");
    assert_ne!(unbound.state(), unreachable.state(), "the two Nones must not collapse");

    // 2) 都不是 Bound（fail-open 装配保持不变：ADR-0007 D2 未被推翻）。
    assert!(!unbound.is_bound() && !empty.is_bound() && !unreachable.is_bound());


    // 3) 具名拒绝理由：都出声，但说的是不同的原因，且都不许是空串。
    let r_unbound = unbound.refusal().expect("unbound must name its refusal");
    let r_unreachable = unreachable.refusal().expect("unreachable must name its refusal");
    assert!(r_unbound.contains("unbound"), "{r_unbound}");
    assert!(r_unreachable.contains("unreachable"), "{r_unreachable}");
    assert!(r_unreachable.contains("127.0.0.1:1"), "the failed endpoint must survive: {r_unreachable}");
    assert_ne!(r_unbound, r_unreachable, "different causes need different words");
}

/// Bound 才有 pipeline，且拒绝理由是 `None`（不是空串 - 空串会被读成“已拒绝”）。
#[tokio::test]
async fn resolve_binding_bound_names_no_refusal() {
    let mock = MockTentacle::new().with_tool("numbers", "{}");
    let (endpoint, _captured, _tx, _handle) = spawn_mock_tentacle(mock).await;
    let config = PipelineConfig::from_codex("knowledge_base/fixture-codex.json").unwrap();
    let bound = resolve_binding(Some(endpoint), config).await;
    assert_eq!(bound.state(), "bound");
    assert!(bound.refusal().is_none(), "a bound pipeline has nothing to refuse");
    assert!(matches!(bound, PipelineBinding::Bound(_)));
}
