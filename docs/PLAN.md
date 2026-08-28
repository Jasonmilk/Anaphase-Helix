# Anaphase-Helix PLAN — 当前阶段导航

> **DNA 方法论 v1.0** ｜ PLAN.md 是导航牌，不是历史档案（≤150 行）。完成记录进 GROWTH.md。

## 当前阶段：P10c — 生命周期实体化 + 生态感知

**目标**：把 Anaphase 的"身体本能"实体化——跨纪元认知传承（强制苏醒/认知脱水）、任务 DAG 自主生长、生态手套可用性感知（Cellrix 原生优先）。范围以"认知工艺双向复用备忘录"审查裁决为终稿（候选纳入见下）。

> **⚠️ 范围裁决中**：用户已同步《Helix 认知工艺与编排能力双向复用升级备忘录 v1.0》（四阶段：P10c CraftAdapter → P11 编排 → P12 OrchestrationCore 接口 → P13 双向复用）。备忘录的"P10c"与本阶段定义存在**编号与范围冲突**，待审查裁决后定稿本阶段。

### 任务清单

| # | 任务 | 内容 | 入口 |
|---|---|---|---|
| T1 | 强制苏醒 / 认知脱水实体化 | `wake_up()` 读取 `session_notes.json`（跨纪元认知重载）；`dehydrate()` 压缩历史为简报供下纪元加载 | DNA 原则 3 |
| T2 | 任务 DAG 分支拓扑 | `task_dag.rs`：`dag_branch_create(parent, branch_name, intent)`，自主生长不越 L0/L1 边界 | DNA 原则 3 + 蓝图 |
| T3 | 生态手套可用性感知（渐进） | Cellrix（原生手套，优先）→ MCP 等通用手套：可用性状态 → 独立扩展位（勿增实体） | DNA 原则 3 + spec/position A.4 |

### 候选纳入（备忘录 v1.0，审查后裁决）

- **P10c'（备忘录）**：Anaphase 调用认知工艺（单向 Anaphase → Mind）：`CraftAdapter` + `CraftQuery` gRPC，集成测试覆盖"不确定时调用"+"Mind 不可用时不 panic"。
- **冲突点**：① 与 T1-T3 编号/范围并存或替换？② CraftQuery 需 Mind 侧暴露认知工艺 RPC（跨仓库依赖，Mind 是否已有）；③ "双向复用"终态与脑手分离铁律的边界。
- **Tentacle Rust 重构**：P10b 后自然启动（不阻塞本阶段），硬性对齐：凭证标签流转（Tuck 注入）/ 已见熵布隆过滤器（Callosum）/ 异步协程沙箱（ARM 端侧）/ 动态共识适配层 / 多传输层（gRPC/HTTP/MCP/STDIO）。

### 技术前提

- P10b 已完成：budget_tier 前置路由验证、suggested_mode 状态机驱动、HITL 审批通道（37 测试全绿）
- `states.rs` 7 状态机、`src/hitl.rs`、MemoryAdapter set_complexity 钩子已就位
- 蓝图 v11.0：强制苏醒/认知脱水/任务 DAG 概念已定义（未实体化）

### 风险与注意事项

- **跨纪元传承不破坏 L3 不可篡改**：`session_notes.json` 是工作态（Anaphase 私有），非记忆（Mind 所有）
- **任务 DAG 不越界**：自主生长绝对不越过 L0（基因锁）/L1（自画像），反向链接至 Mind 知识库
- **生态感知勿增实体**：先做可用性状态（独立扩展位），不做手套协议实现
- **CraftQuery 跨仓库依赖**：若裁决纳入，需先确认 Mind 侧认知工艺 RPC 暴露状态（避免 Anaphase 单方面设计）

### 验收标准

- `cargo test --workspace` 全绿（新增 wake_up/dehydrate/task_dag 测试）
- `wake_up()` 能读取上一纪元脱水简报；`dehydrate()` 产出可加载简报
- `dag_branch_create` 生成任务分支 DAG，不越过 L0/L1
- 生态手套可用性状态有扩展位（不实现协议）
- 方法论闭环：ADR 追加 P10c 决议、GROWTH 记录、spec 同步

## 下一阶段预览：P11 — 认知工艺编排（单向 Mind → Anaphase）

- 认知工艺工序可生成 ActionSuggestion，Anaphase 正常执行（备忘录 v1.0 待裁决）
- OrchestrationCore 接口抽象预留（P12 不实现复用，勿增实体）

---

*Anaphase-Helix PLAN v1.3（P10b 完成 → P10c 启动，范围待备忘录裁决，2026-08-28）*
