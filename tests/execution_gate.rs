// 执行点停（execution-point stop）——判据，先于行为翻转。
//
// 本文件只做一件事：把「执行点该停」写成可跑红的判据。它现在**红着**，
// 因为旧行为还在（calls 非空 + 无 pipeline ⇒ 走 legacy echo 并执行）。
//
// 口径（园丁 2026-10-28 边界）：
//   判据/类型准备（行为不变、可逆）—— 自治
//   行为翻转（改判定、改执行）    —— 授权②
// 本文件属前者。翻转后本文件必须转绿，届时**不许改判据**，只许让它变绿。
//
// 四个态（前三个已在 src/pipeline/mod.rs 的 PipelineBinding 里，第四个是运行期）：
//   Unbound      没有端点（出厂态）
//   Unreachable  配了连不上（环境故障）
//   Bound        绑上了
//   Dark         绑上了、但运行期探针说 tentacle 不在了（第四态，本文件 test 3 盯它）
//
// 调用者（§3.10）：`cargo test --test execution_gate`，随每次 push/PR 的全量测试跑。
// 现在红 ⇒ 它若进 CI 报表，须与 K32 的两条古红**分开**读（那两条与本条无关）。

mod common;

use anaphase::adapters::*;
use anaphase::config::RunCycleConfig;
use anaphase::gloves::{GloveStatus, GloveTier};
use anaphase::run_cycle::AgentLoop;
use anaphase::reflex::ReflexArc;
use common::{MockTentacle, StructuredReasoning};
use std::sync::{Arc, Mutex};

/// 记录「工具有没有被真的调过」。判据读的是它，不是日志。
struct RecordingTool {
    calls: Arc<Mutex<Vec<String>>>,
}

#[async_trait::async_trait]
impl ToolAdapter for RecordingTool {
    async fn execute(&self, command: &str, _args: &[String]) -> Result<String, String> {
        self.calls.lock().unwrap().push(command.to_string());
        Ok(format!("executed: {command}"))
    }
    async fn perceive(&self, _query: &str) -> Result<String, String> {
        Ok("perceived".to_string())
    }
}

fn plan_with_one_call() -> String {
    // 有意用结构化 plan：它有 calls，所以「做」这件事是真的被请求了。
    r#"{"calls":[{"tool":"numbers","args":{},"expect":"numbers"}]}"#.to_string()
}

fn agent_with(recorder: Arc<Mutex<Vec<String>>>, pipeline: Option<anaphase::pipeline::Pipeline>) -> AgentLoop {
    let a = AgentLoop::new(
        Arc::new(NoopMemoryAdapter),
        Arc::new(StructuredReasoning { output: plan_with_one_call() }),
        Arc::new(RecordingTool { calls: recorder }),
        Arc::new(NoopSafetyAdapter),
        Arc::new(NoopUiAdapter),
        Arc::new(NoopFearAdapter),
        ReflexArc { safety_rules: vec![] },
    )
    .with_run_config(RunCycleConfig {
        execution_placeholder: "echo".into(),
        ..RunCycleConfig::default()
    });
    match pipeline {
        Some(p) => a.with_pipeline(p),
        None => a,
    }
}

/// test 1 —— Unbound：没有 pipeline，但有 calls ⇒ 执行点必须停。
///
/// 现在红：`run_cycle/mod.rs` 的 Execution 分支在 `pipeline.is_none()` 时落到
/// legacy，`self.tool.execute(command, …)` 照样被调用（默认 command = echo）。
#[tokio::test]
async fn unbound_execution_point_does_not_execute() {
    let recorder = Arc::new(Mutex::new(Vec::new()));
    let mut agent = agent_with(recorder.clone(), None);

    let _ = agent.run_cycle("please run numbers").await;

    let calls = recorder.lock().unwrap().clone();
    assert!(
        calls.is_empty(),
        "执行点没有停：calls={calls:?}。未绑定 pipeline 时不得执行任何工具调用。"
    );
    assert!(
        agent.context.evidence.is_empty(),
        "有证据就说明真的执行过了：evidence={:?}",
        agent.context.evidence
    );
}

/// test 2 —— Unbound：`calls` 被识别到了，所以「不执行」不是「没请求」。
///
/// 这条守的是判据自己：如果 calls 本来就是空的，test 1 的绿就是假的
/// （什么都没请求，当然什么都没执行）。
#[tokio::test]
async fn unbound_still_parses_the_calls_so_silence_is_not_absence() {
    let recorder = Arc::new(Mutex::new(Vec::new()));
    let mut agent = agent_with(recorder, None);

    let _ = agent.run_cycle("please run numbers").await;

    assert!(
        !agent.context.calls.is_empty(),
        "结构化 plan 的 calls 没被解析出来 —— test 1 的「没执行」就会是假绿"
    );
}

/// test 3 —— 第四态 Dark：绑上了，但运行期探针说 tentacle 不在了。
///
/// 现在的代码只 `warn!("[Execution] tentacle dark at runtime; …")` 然后照样
/// `execute_structured()`——一条日志不是判据（K22 族：东西在那儿，等于没有）。
/// 现在红：结构化路径照样执行，evidence 非空。
#[tokio::test]
async fn dark_tentacle_at_runtime_is_not_a_silent_proceed() {
    let recorder = Arc::new(Mutex::new(Vec::new()));
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0]}"#);
    let (endpoint, _captured, tx, _handle) = common::spawn_mock_tentacle(mock).await;
    common::keep_mock_alive(tx);
    let tentacle = anaphase::adapters::tentacle::GrpcTentacleAdapter::new(&endpoint)
        .await
        .expect("tentacle adapter");
    let config = anaphase::pipeline::PipelineConfig::from_codex("knowledge_base/fixture-codex.json")
        .expect("fixture codex");
    let pipeline = anaphase::pipeline::Pipeline::new(
        tentacle,
        Box::new(anaphase::ledger::FakeClock(1000)),
        config,
    );
    let mut agent = agent_with(recorder, Some(pipeline));
    // 先登记再置态：set_status 对未登记的手套返回 Err（读到 Err 而不看它 = 假样本）
    agent
        .context
        .ecosystem
        .register("tentacle", GloveTier::Standard, GloveStatus::Unavailable);
    assert_eq!(
        agent.context.ecosystem.status("tentacle"),
        Some(GloveStatus::Unavailable),
        "探针状态没设上 ⇒ 这条判据测的就不是第四态"
    );

    let _ = agent.run_cycle("please run numbers").await;

    assert!(
        agent.context.evidence.is_empty(),
        "运行期变暗后仍执行了：evidence={:?}（第四态只出声不处置 ⇒ 等于没停）",
        agent.context.evidence
    );
}
