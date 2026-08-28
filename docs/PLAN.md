# Anaphase-Helix PLAN — 当前阶段导航

> **DNA 方法论 v1.0** ｜ PLAN.md 是导航牌，不是历史档案（≤150 行）。完成记录进 GROWTH.md。

## 当前阶段：P11a — 认知工艺 CraftAdapter（单向 Anaphase → Mind）

**目标**：Anaphase 调用认知工艺（单向：Anaphase → Mind）。**调研优先**——先确认 Helix-Mind 侧认知工艺 RPC 现状，再决定 CraftAdapter 形态。CraftQuery 为**第二阶段显式调用**（暂缓，勿增实体）。

### 任务清单

| # | 任务 | 内容 | 入口 |
|---|---|---|---|
| T0 | **调研：Mind 侧认知工艺 RPC 现状** | 确认 Helix-Mind 是否已暴露/计划暴露认知工艺触发 RPC（当前 v4.1 冻结契约无 CraftQuery）；确认 P10b 间接触发（budget_tier+suggested_mode）是否已覆盖"Anaphase 调用认知工艺"第一层需求 | ADR-0001 P10c 决议 |
| T1 | CraftAdapter 决策 | 基于 T0 调研：若间接触发已覆盖 → 不建 CraftAdapter（勿增实体）；若需显式 → 定义 CraftQuery 形态（第二阶段） | T0 结论 |
| T2 | 集成测试 | "不确定时调用认知工艺" + "Mind 不可用时不 panic"（降级） | ADR-0001 |

### T0 调研要点（P11a 第一动作）

1. Helix-Mind `docs/spec/cognitive-craft.md`（ADR-0021）——认知工艺的 System 0 门控/工序编排是否暴露 gRPC
2. Helix-Mind proto v4.1 冻结契约——现有 RPC 清单（HelixQuery/HelixConsolidate/FederatedDAGShare/TriggerReincarnation/Remember）
3. P10b 已实现的间接触发链路：Anaphase 传 `budget_tier` + `suggested_mode` → Mind System 0 门控消费 → 是否满足"认知工艺调用"语义
4. 结论：CraftAdapter 是"必要"还是"过度设计"（裁决后定 T1）

### 技术前提

- P10c 已完成：生命周期实体化（wake_up/dehydrate）、任务 DAG（L0/L1 守卫）、生态手套感知（49 测试全绿）
- P10b 已打通 Anaphase → Mind 间接触发（budget_tier/suggested_mode 透传 + System 0 门控消费）
- Helix-Mind 认知工艺（ADR-0021）为 Mind 内部能力

### 风险与注意事项

- **勿增实体**：若间接触发已覆盖第一层需求，CraftAdapter 不建（等 Mind 侧暴露显式 RPC 再评估）
- **跨仓库依赖**：CraftQuery 若需 Mind 侧新增 RPC，涉及冻结契约变更（Mind 侧 ADR+Supersede），Anaphase 不单方面设计
- **脑手分离**：Anaphase 调用认知工艺 ≠ 职责合并；Mind 规划/认知、Anaphase 执行/编排边界不变

### 验收标准

- T0 调研结论记录（ADR 追加：CraftAdapter 必要性裁决）
- 若建 CraftAdapter：集成测试"不确定时调用"+"Mind 不可用不 panic"；cargo test 全绿
- 方法论闭环：ADR 追加 P11a 决议、GROWTH 记录、spec 同步

## 下一阶段预览：P11b — 认知工艺调用编排（单向 Mind → Anaphase）

- 认知工艺工序生成 ActionSuggestion，Anaphase 正常执行（OrchestrationAdapter）
- OrchestrationCore 接口抽象预留（P11c 不实现复用，勿增实体）

---

*Anaphase-Helix PLAN v1.5（P10c 完成 → P11a 认知工艺 CraftAdapter，调研优先，2026-08-28）*
