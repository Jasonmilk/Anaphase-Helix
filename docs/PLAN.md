# Anaphase-Helix PLAN — 当前阶段导航

> **DNA 方法论 v1.0** ｜ PLAN.md 是导航牌，不是历史档案（≤150 行）。完成记录进 GROWTH.md。

## 当前阶段：P10b — 认知工艺触发链路

**目标**：打通 Anaphase → Mind 的认知工艺触发链路——`budget_tier` 前置路由（System 0 门控）真正被认知工艺消费；`suggested_mode` 由状态机驱动（非纯长度启发式）；HITL 审批通道接入 Tentacle 契约。

### 任务清单

| # | 任务 | 内容 | 入口 |
|---|---|---|---|
| T1 | System 0 门控经 budget_tier 前置路由验证 | 验证 `EnergyContext.budget_tier` 从 Anaphase 前置传入 Mind 后，被 Helix-Mind 认知工艺（System 0 门控/预算路由）消费；mock Mind 断言 budget_tier 值 + 集成测试扩展 | ADR-0010（Helix-Mind）+ ADR-0001 |
| T2 | 状态机驱动 suggested_mode | `suggested_mode` 从 query 长度启发式改为 `states.rs` `HelixState` 状态驱动（PreAssessment 输出影响模式选择），保留启发式兜底 | ADR-0001 |
| T3 | HITL 审批通道（Tentacle 契约） | `check_hitl_approval()` 实现：高风险动作（写操作/网络请求/凭证使用）挂起直至人类确认；接入 Tentacle 契约 | DNA 原则 4 |

### T1 子步骤（System 0 门控验证）

1. 确认 Helix-Mind 认知工艺（`helix-mind/docs/spec/cognitive-craft.md`）System 0 门控对 `budget_tier` 的消费点
2. 扩展 mock Mind：断言收到的 `budget_tier` 与 Anaphase 推导一致（极简→ENDOGENOUS / 默认→AUGMENTABLE / 探索→EXOGENOUS_REQUIRED）
3. 集成测试：不同查询特征 → 断言 Mind 侧收到对应 budget_tier

### T2 子步骤（状态机驱动）

1. `states.rs` 暴露当前 `HelixState`（如 PreAssessment 完成态）
2. `derive_suggested_mode` 接收状态输入：PreAssessment 判定复杂 → ANCHOR/IMAGINATION；简单 → SKILLED
3. 保留长度启发式作为无状态兜底（状态缺失时）
4. 单元测试覆盖状态驱动路径

### T3 子步骤（HITL 审批）

1. `check_hitl_approval(action, params)`：高风险动作 → 挂起 + 请求人类确认
2. 未经确认的高风险动作被物理拦截（不执行）
3. 接入 Tentacle 契约（工具执行前审计）
4. 单元/集成测试：高风险挂起、确认后放行、拒绝后拦截

### 技术前提

- P10a 已完成：proto 对齐 v4.1、EnergyContext 构造（budget_tier 推导）、mind.rs 补全、mock Mind 测试基座
- `states.rs` HelixState 状态机（7 状态）存在
- Helix-Mind 认知工艺 System 0 门控定义（`docs/spec/cognitive-craft.md`）
- DNA 原则 4（HITL 人在回路）为设计依据

### 风险与注意事项

- **System 0 门控消费点确认**：若 Mind 侧认知工艺尚未显式消费 budget_tier，T1 退化为"断言传递正确"（勿过度设计，不强迫 Mind 实现）
- **状态机驱动不破坏启发式**：状态缺失时必须兜底，避免 panic
- **HITL 不阻塞无风险路径**：仅高风险动作走审批，日常动作零额外延迟
- **生态手套感知**：P10b 仅预留扩展位（勿增实体），可用性状态感知推迟至 P10c+

### 验收标准

- `cargo test --workspace` 全绿（新增 budget_tier 路由 / 状态驱动 / HITL 测试）
- mock Mind 断言 budget_tier 按查询特征正确传递（System 0 门控链路验证）
- `suggested_mode` 由状态驱动，无状态时兜底启发式（不 panic）
- 高风险动作未经确认被物理拦截，确认后放行
- 方法论闭环：ADR 追加 P10b 决策、GROWTH 记录、spec 同步（decision→code→docs）

## 下一阶段预览：P10c — 生态手套感知 + 生命周期实体化

- 生态手套（MCP/宇树/Unity/鸿蒙）可用性感知（DNA 原则 3 渐进扩展，可用性状态 → 独立扩展位）
- 强制苏醒 / 认知脱水实体化（`wake_up()` / `dehydrate()`，跨纪元认知重载）
- 任务 DAG 分支拓扑（`task_dag.rs`，自主生长不越 L0/L1）

---

*Anaphase-Helix PLAN v1.2（P10b 阶段启动，P10a 完成，2026-08-28）*
