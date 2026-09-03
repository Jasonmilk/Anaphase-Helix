# 候选 E 探查笔记（E-T1）— Reasoning 结构化 + run_cycle ↔ pipeline 完整 merge

> **日期**：2026-09-03 ｜ **依据**：PLAN v1.8 / ADR-0003 决策 9 映射表 / ADR-0004 决策 5
> **方法**：物理事实优先——逐文件读代码确认，不按任务书照搬（M1 历史教训）。

## 1. 现状（代码证据）

### 1.1 Reasoning 输出协议（`src/agent_loop.rs` Reasoning 状态）
- `reason.reason(&user_input, "left_brain")` 返回自然语言 `String`
- 决策靠字符串匹配：`output.contains("tool_call") || contains("python") || contains("cli")` → NeedsTool；`contains("impasse") || contains("unknown")` → Impass
- **结论**：与 M1 结构化 `calls[]` 基因不兼容（ADR-0003 决策 9 四点不兼容中的第 1 点仍在）

### 1.2 suggested_actions 消费点
- 唯一产出方：`MemoryAdapter::query()` → `QueryResult.suggested_actions: Vec<String>`（Mind 侧 `SuggestedAction` 扁平为字符串，`src/adapters/mind.rs:70`）
- 消费点：ReflexCheck（`join(", ")` 喂 hard_reflex / soft_reflex 上下文）、Execution（`join(", ")` 作 args）
- 测试依赖：`p11b_suggested_actions_flow_to_execution` 断言 `context.suggested_actions == ["web_search"]`；`test_dangerous_action_is_blocked` 预设 suggested_actions
- **结论**：字段保留（P11b 兼容），另增结构化字段供 Execution 消费（E-T3）

### 1.3 run_cycle 状态机可扩展点
- 7 状态：Perception → PreAssessment → MemoryRetrieval → Reasoning → ReflexCheck → Execution → Reflection；声明式 transition 表
- Execution 现状（M1.5-T6）：`tool_command` 解析 + echo 占位回退；HITL 闸 + safety 审计闸在位
- Reflection 现状：仅 memory consolidation，无判据/账本消费
- **结论**：`pipeline: Option<Pipeline>` 挂 AgentLoop（None 保持 legacy 向后兼容），Reasoning/Execution/Reflection 三状态按映射表消费六 stage

### 1.4 HttpReasoningAdapter 输出解析（`src/adapters/http_reasoning.rs`）
- 已确认：返回 `choices[0].message.content` 字符串；M1 e2e 已证明该字符串可为 `{"calls":[...]}` JSON（mock LLM 注入）
- **结论**：trait 签名不变，run_cycle 从输出解析（ADR-0003 决策 4 保持——Http/Noop/FlowModus 零改动）

## 2. 设计结论（落 ADR-0005）

| # | 决策 |
|---|---|
| 1 | Reasoning 输出协议：JSON `{"calls":[...],"impasse":bool}` 或裸数组；`contract::parse_reasoning_output` 解析，**删除全部 contains 匹配** |
| 2 | 六 stage 落点：Reasoning = stage1 parse + stage2 信封；Execution = stage3 execute_calls + stage4 record_evidence（HITL/审计闸保留）；Reflection = stage5 check_results + stage6 build_verdict + ledger.append |
| 3 | AgentLoop 增 `pipeline: Option<Pipeline>` + `with_pipeline()`；None 保持 legacy echo 路径（向后兼容，ADR-0004 决策 5 延续） |
| 4 | AgentContext 增 `calls/job/evidence`；suggested_actions 保留不动 |
| 5 | 信封确定性：`job_id = FNV-1a(user_input)`（无 UUID）；`created_at = clock → RFC3339`（chrono，ledger::unix_secs_to_rfc3339） |
| 6 | E-T6 五常量 → `config::RunCycleConfig`（Default 即文档化协议值；config.toml `[anaphase.run_cycle]` 可覆盖）；agent_loop.rs 零字面量 |
| 7 | 边界守约：不改 Mind/Tentacle/Cellrix/FlowModus；不引入 lodestone；不做守护进程/Auto 路由 |

## 3. 已知相邻债（本候选不处理，记入 ADR-0005）
- `src/adapters/mind.rs` `build_energy_context` 内 `token_budget/pulse/vigilance/latency_limit_ms/familiarity` 字面量（非 run_cycle 5 处，属 Mind 适配器域）
- `main.rs` 未接线 pipeline（`tentacle_endpoint` 消费待后续任务，与 M1.5 一致）
- config.toml 的 `[cognitive]` / `[immune]` 段当前未被 AnaphaseConfig 消费（历史遗留）
