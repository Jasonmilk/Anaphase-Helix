//! P10a T3 集成测试：Helix-Mind gRPC 记忆契约闭环
//!
//! 覆盖（mock Mind server，不依赖真实 Helix-Mind 二进制）：
//! 1. 正常闭环：HelixQuery → QueryResult 节点返回
//! 2. trace 透传：请求 traceparent 为 W3C 格式，响应 echo 一致
//! 3. budget_tier 传递：EnergyContext.budget_tier 按查询特征推导
//! 4. Mind 离线 fail-open：连接失败 → resolve 回退 Noop（不 panic）；server 下线 → query 返回 Err

use anaphase::adapters::mind::GrpcMindAdapter;
use anaphase::adapters::{
    resolve_memory_adapter, FearAdapter, MemoryAdapter, NoopFearAdapter, NoopReasoningAdapter,
    NoopSafetyAdapter, NoopToolAdapter, NoopUiAdapter, ReasoningAdapter, SafetyAdapter,
    ToolAdapter, UiAdapter,
};
use anaphase::agent_loop::AgentLoop;
use anaphase::config::AnaphaseConfig;
use anaphase::helix_mind_api::helix_mind_server::{HelixMind, HelixMindServer};
use anaphase::helix_mind_api::{
    ActivationEntry, AdvancedQueryRequest, Edge, ForgetRequest, ForgetResponse,
    HelixConsolidateRequest, HelixConsolidateResult, HelixQueryRequest, HelixQueryResult, Node,
    QueryRequest, QueryResponse, ReloadGeneLockRequest, ReloadGeneLockResponse, RememberRequest,
    RememberResponse, SuggestedAction, SyncHumanViewRequest, SyncHumanViewResponse,
    TriggerReincarnationRequest, TriggerReincarnationResponse,
};
use anaphase::reflex::ReflexArc;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Request, Response, Status};

/// 捕获所有进入 mock 的 HelixQueryRequest（用于断言 traceparent / budget_tier）。
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<HelixQueryRequest>>>);

#[derive(Default)]
struct MockMind {
    captured: Captured,
    /// P11b：mock Mind 返回的 suggested_actions（模拟 Mind 认知工艺产出的动作建议）
    suggested_actions: Vec<SuggestedAction>,
}

fn mock_node(content: &str) -> Node {
    Node {
        id: "n1".into(),
        node_type: "L3".into(),
        content_json: content.into(),
        heat: 0.5,
        is_hypothetical: false,
        is_recessive: false,
        sensitivity: "private".into(),
        generation: 1,
        created_at: None,
        last_accessed_at: None,
        access_count: 0,
        initial_impact: 0.0,
        corrected_by: "".into(),
        notes: "".into(),
        derived_from: vec![],
    }
}

#[tonic::async_trait]
impl HelixMind for MockMind {
    async fn helix_query(
        &self,
        request: Request<HelixQueryRequest>,
    ) -> Result<Response<HelixQueryResult>, Status> {
        let req = request.into_inner();
        self.captured.0.lock().unwrap().push(req.clone());
        let result = HelixQueryResult {
            effective_mode: req.suggested_mode,
            mode_negotiation: "mock".into(),
            nodes: vec![mock_node("test-node-content")],
            edges: vec![],
            trace_id: "mock-trace".into(),
            latency_ms: 1,
            tokens_consumed: 10,
            is_partial: false,
            exhaustion_reason: "".into(),
            impasse_level: 0,
            stages_attempted: 1,
            suggested_actions: self.suggested_actions.clone(),
            activation_vector: vec![],
            // 响应 echo 请求的 traceparent（Mind 只透传不生成）
            traceparent: req.traceparent.clone(),
        };
        Ok(Response::new(result))
    }

    async fn remember(
        &self,
        _request: Request<RememberRequest>,
    ) -> Result<Response<RememberResponse>, Status> {
        Ok(Response::new(RememberResponse {
            node_id: "m1".into(),
        }))
    }

    // 以下 RPC 不在 T3 覆盖范围，提供最小默认实现（不 panic）。
    async fn query(
        &self,
        _request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        Ok(Response::new(QueryResponse {
            nodes: vec![],
            edges: vec![],
            trace_id: "".into(),
            latency_ms: 0,
            is_partial: false,
            exhaustion_reason: "".into(),
        }))
    }
    async fn advanced_query(
        &self,
        _request: Request<AdvancedQueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        Ok(Response::new(QueryResponse {
            nodes: vec![],
            edges: vec![],
            trace_id: "".into(),
            latency_ms: 0,
            is_partial: false,
            exhaustion_reason: "".into(),
        }))
    }
    async fn forget(
        &self,
        _request: Request<ForgetRequest>,
    ) -> Result<Response<ForgetResponse>, Status> {
        Ok(Response::new(ForgetResponse { success: true }))
    }
    async fn helix_consolidate(
        &self,
        _request: Request<HelixConsolidateRequest>,
    ) -> Result<Response<HelixConsolidateResult>, Status> {
        Ok(Response::new(HelixConsolidateResult {
            success: true,
            message: "mock".into(),
        }))
    }
    async fn federated_dag_share(
        &self,
        _request: Request<anaphase::helix_mind_api::FederatedDagShareRequest>,
    ) -> Result<Response<anaphase::helix_mind_api::FederatedDagShareResponse>, Status> {
        Ok(Response::new(
            anaphase::helix_mind_api::FederatedDagShareResponse { cid: "".into() },
        ))
    }
    async fn trigger_reincarnation(
        &self,
        _request: Request<TriggerReincarnationRequest>,
    ) -> Result<Response<TriggerReincarnationResponse>, Status> {
        Ok(Response::new(TriggerReincarnationResponse {
            new_generation: 1,
        }))
    }
    async fn reload_gene_lock(
        &self,
        _request: Request<ReloadGeneLockRequest>,
    ) -> Result<Response<ReloadGeneLockResponse>, Status> {
        Ok(Response::new(ReloadGeneLockResponse {
            l0_hash: "".into(),
            lineage_name: "".into(),
            core_principles: vec![],
        }))
    }
    async fn sync_human_view(
        &self,
        _request: Request<SyncHumanViewRequest>,
    ) -> Result<Response<SyncHumanViewResponse>, Status> {
        Ok(Response::new(SyncHumanViewResponse {
            success: true,
            conflicts: vec![],
        }))
    }
}

/// 在随机端口启动 mock Mind server，返回 (endpoint, captured, shutdown_tx, handle)。
/// 停机使用 `serve_with_incoming_shutdown` + oneshot，确保已接受连接也优雅关闭。
async fn spawn_mock_mind() -> (
    String,
    Captured,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let captured = Captured::default();
    let svc = HelixMindServer::new(MockMind {
        captured: captured.clone(),
        suggested_actions: vec![],
    });
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let handle = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(svc)
            .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
    });
    (format!("http://{}", addr), captured, shutdown_tx, handle)
}

/// P11b：spawn 一个返回指定 suggested_actions 的 mock Mind（模拟 Mind 认知工艺产出的动作建议）
async fn spawn_mock_mind_with_actions(actions: Vec<SuggestedAction>) -> (
    String,
    Captured,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let captured = Captured::default();
    let svc = HelixMindServer::new(MockMind {
        captured: captured.clone(),
        suggested_actions: actions,
    });
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let handle = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(svc)
            .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
    });
    (format!("http://{}", addr), captured, shutdown_tx, handle)
}

#[tokio::test]
async fn normal_closed_loop_returns_nodes() {
    let (endpoint, _, _tx, _handle) = spawn_mock_mind().await;
    let adapter = GrpcMindAdapter::new(&endpoint).await.expect("adapter connect");
    let result = adapter
        .query("帮我查一下昨天的会议记录", false)
        .await
        .expect("query ok");
    assert_eq!(result.nodes, vec!["test-node-content".to_string()]);
}

#[tokio::test]
async fn trace_passthrough_is_w3c_and_echoed() {
    let (endpoint, captured, _tx, _handle) = spawn_mock_mind().await;
    let adapter = GrpcMindAdapter::new(&endpoint).await.unwrap();
    let _ = adapter.query("查询", false).await.unwrap();

    let reqs = captured.0.lock().unwrap();
    assert_eq!(reqs.len(), 1, "捕获到一次请求");
    let tp = &reqs[0].traceparent;
    let parts: Vec<&str> = tp.split('-').collect();
    assert_eq!(parts.len(), 4, "traceparent 4 段: {tp}");
    assert_eq!(parts[0], "00");
    assert_eq!(parts[1].len(), 32);
    assert_eq!(parts[2].len(), 16);
    assert_eq!(parts[3], "01");
    assert!(parts[1].chars().all(|c| c.is_ascii_hexdigit()));
}

#[tokio::test]
async fn budget_tier_propagates_with_query_complexity() {
    let (endpoint, captured, _tx, _handle) = spawn_mock_mind().await;
    let adapter = GrpcMindAdapter::new(&endpoint).await.unwrap();

    // 中等查询 → AUGMENTABLE (0)
    adapter.query("帮我查一下昨天的会议记录", false).await.unwrap();
    // 极简查询 → ENDOGENOUS (1)
    adapter.query("你好", false).await.unwrap();
    // 探索查询 → EXOGENOUS_REQUIRED (2)
    adapter.query("帮我研究一下这个课题", false).await.unwrap();

    let reqs = captured.0.lock().unwrap();
    assert_eq!(reqs.len(), 3);
    let ec0 = reqs[0].energy_context.as_ref().expect("energy_context 存在");
    assert_eq!(ec0.budget_tier, 0, "中等查询 → AUGMENTABLE");
    assert!(ec0.system_load >= 0.0 && ec0.system_load <= 1.0, "system_load 真实感知");
    let ec1 = reqs[1].energy_context.as_ref().expect("energy_context 存在");
    assert_eq!(ec1.budget_tier, 1, "极简查询 → ENDOGENOUS");
    let ec2 = reqs[2].energy_context.as_ref().expect("energy_context 存在");
    assert_eq!(ec2.budget_tier, 2, "探索查询 → EXOGENOUS_REQUIRED（System 0 门控前置路由链路）");
}

#[tokio::test]
async fn mind_offline_resolve_falls_back_to_noop() {
    // 不可达端点（127.0.0.1:1 立即 refused）：resolve 必须回退 Noop，绝不 panic
    let cfg = AnaphaseConfig {
        mind_endpoint: Some("http://127.0.0.1:1".into()),
        ..Default::default()
    };
    let adapter = resolve_memory_adapter(&cfg).await;
    // Noop 返回空结果（fail-open），不抛错
    let result = adapter.query("anything", false).await.expect("fail-open 不报错");
    assert!(result.nodes.is_empty(), "Noop 降级返回空节点");
}

#[tokio::test]
async fn mind_offline_empty_endpoint_is_noop_not_panic() {
    // mind_endpoint 为空串 → Noop（不尝试连接、不 panic）
    let cfg = AnaphaseConfig {
        mind_endpoint: Some(String::new()),
        ..Default::default()
    };
    let adapter = resolve_memory_adapter(&cfg).await;
    let result = adapter.query("anything", false).await.expect("空端点 fail-open");
    assert!(result.nodes.is_empty());
}

#[tokio::test]
async fn grpc_adapter_connect_failure_returns_err() {
    // GrpcMindAdapter 连不可达端口 → 返回 Err（而非 panic）
    let result = GrpcMindAdapter::new("http://127.0.0.1:1").await;
    assert!(result.is_err(), "连接失败必须返回 Err 而非 panic");
}

#[tokio::test]
async fn suggested_mode_state_driven_overrides_length() {
    // P10b T2：set_complexity 状态驱动 suggested_mode（无视 query 长度），兜底保留
    let (endpoint, captured, _tx, _handle) = spawn_mock_mind().await;
    let adapter = GrpcMindAdapter::new(&endpoint).await.unwrap();

    // 状态=复杂(3)：短 query 也走 IMAGINATION(2)
    adapter.set_complexity(3);
    adapter.query("你好", false).await.unwrap();
    // 状态=简单(1)：长 query 也走 SKILLED(0)
    adapter.set_complexity(1);
    let long = "请深入分析这个复杂系统的架构与多维度权衡并给出完整方案建议与风险边界以及所有潜在的未知变量和未来可能的演进方向与备选路径";
    adapter.query(long, false).await.unwrap();

    let reqs = captured.0.lock().unwrap();
    assert_eq!(reqs.len(), 2);
    assert_eq!(reqs[0].suggested_mode, 2, "状态复杂 → IMAGINATION");
    assert_eq!(reqs[1].suggested_mode, 0, "状态简单 → SKILLED");
}

#[tokio::test]
async fn mind_offline_query_returns_err_after_server_down() {
    // server 上线 → adapter 连接成功 → 优雅停机（shutdown）→ query 必须返回 Err（fail-open 钩子），不 panic
    let (endpoint, _captured, shutdown_tx, handle) = spawn_mock_mind().await;
    let adapter = GrpcMindAdapter::new(&endpoint).await.unwrap();
    let _ = shutdown_tx.send(()); // 优雅停机：关闭 listener 与所有已接受连接
    let _ = handle.await;

    let result = adapter.query("查询", false).await;
    assert!(result.is_err(), "server 下线后 query 必须返回 Err（非 panic）");
}

/// P11b 验证闭环：mock Mind 返回 suggested_actions → 断言流转到 Execution。
/// 验证 Anaphase 侧单向编排链路（Mind 产出由真实 Mind 侧独立验证；mock 验证不阻塞）。
#[tokio::test]
async fn p11b_suggested_actions_flow_to_execution() {
    let actions = vec![SuggestedAction {
        action_type: "web_search".into(),
        parameters: "{}".into(),
        reason: "P11b mock：模拟 Mind 认知工艺产出动作建议".into(),
    }];
    let (endpoint, _captured, _tx, _handle) = spawn_mock_mind_with_actions(actions).await;
    let adapter = Arc::new(GrpcMindAdapter::new(&endpoint).await.unwrap());

    // 1) adapter 层：Mind 返回的 suggested_actions 被消费
    let result = adapter.query("帮我查一下最新研究", false).await.unwrap();
    assert_eq!(result.suggested_actions, vec!["web_search"], "suggested_actions 应从 Mind 响应消费");

    // 2) agent_loop 全流程：suggested_actions 注入 context → 流转到 Execution（HITL 闸就位）
    let reason: Arc<dyn ReasoningAdapter> = Arc::new(NoopReasoningAdapter);
    let tool: Arc<dyn ToolAdapter> = Arc::new(NoopToolAdapter);
    let safety: Arc<dyn SafetyAdapter> = Arc::new(NoopSafetyAdapter);
    let ui: Arc<dyn UiAdapter> = Arc::new(NoopUiAdapter);
    let fear: Arc<dyn FearAdapter> = Arc::new(NoopFearAdapter);
    let reflex = ReflexArc { safety_rules: vec![] };
    let mut agent = AgentLoop::new(adapter.clone(), reason, tool, safety, ui, fear, reflex);
    agent.run_cycle("帮我查一下最新研究").await.unwrap();
    assert_eq!(
        agent.context.suggested_actions,
        vec!["web_search"],
        "suggested_actions 应流转到 Execution（MemoryRetrieval → context）"
    );
}
