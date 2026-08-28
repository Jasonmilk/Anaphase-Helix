# Anaphase-Helix PLAN — 当前阶段导航

> **DNA 方法论 v1.0** ｜ PLAN.md 是导航牌，不是历史档案（≤150 行）。完成记录进 GROWTH.md。

## 当前阶段：P10a — Mind 契约对齐 + 触发链路

**目标**：对齐 Helix-Mind v4.1 冻结契约（Anaphase 侧已漂移 3 个 Append-Only 字段），打通 Anaphase → Mind 的认知循环触发链路。

### 任务清单

| # | 任务 | 内容 | 入口 |
|---|---|---|---|
| T1 | proto 契约同步（Append-Only） | `BudgetTier` enum + `EnergyContext.budget_tier=9` + `HelixQueryRequest.traceparent=7` + `reserved 8 to max` + `HelixQueryResult.activation_vector=13` + traceparent 回传 | ADR-0001 |
| T2 | mind.rs 补全 | 构造 `EnergyContext`（含 budget_tier，从状态推导）+ W3C 根 trace_id 生成透传 + 模式/自治从状态机推导（去硬编码） | ADR-0001 |
| T3 | 接线 + 测试 | `mind_endpoint` 接线 + 集成测试（起 Mind 服务连真实端点，验证 HelixQuery 闭环 + traceparent 透传） | ADR-0001 |

### 技术前提

- Helix-Mind v4.1 契约已冻结（真相源：`helix-mind/crates/helix-mind-api/proto/helix_mind.proto`）
- Anaphase 现有 proto 前 6 号字段与 Mind 对齐，仅缺 3 个追加字段
- `src/adapters/mind.rs` 当前 `energy_context: None` + 硬编码模式

### 验收标准

- `cargo test --workspace` 全绿（含新增 mind 契约测试 ≥ 2）
- Anaphase proto 与 Mind proto 字段号完全对齐（无冲突）
- mind.rs 不再硬编码 `suggested_mode`/`autonomy_level`/`energy_context`
- traceparent 从请求入口生成并透传至 Mind

## 下一阶段预览：P10b — 认知工艺触发链路

- System 0 门控经 `EnergyContext.budget_tier` 前置路由（ADR-0010）
- Anaphase 状态机推导认知模式 → Mind `suggested_mode`
- HITL 审批通道接入 Tentacle 契约

---

*Anaphase-Helix PLAN v1.0（初始化，当前 P10a）*
