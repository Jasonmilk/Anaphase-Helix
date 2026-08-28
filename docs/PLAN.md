# Anaphase-Helix PLAN — 当前阶段导航

> **DNA 方法论 v1.0** ｜ PLAN.md 是导航牌，不是历史档案（≤150 行）。完成记录进 GROWTH.md。

## 当前阶段：P10a — Mind 契约对齐 + 触发链路

**目标**：对齐 Helix-Mind v4.1 冻结契约（Anaphase 侧已漂移 3 个 Append-Only 字段），打通 Anaphase → Mind 的认知循环触发链路。

### 任务清单

| # | 任务 | 内容 | 入口 |
|---|---|---|---|
| T1 | proto 契约同步（Append-Only） | 与 Mind proto 逐字段对齐：`BudgetTier` enum + `EnergyContext.budget_tier=9` + `HelixQueryRequest.traceparent=7` + `reserved 8 to max` + `HelixQueryResult.activation_vector=13` + traceparent 回传 | ADR-0001 |
| T2 | mind.rs 补全 | 构造 `EnergyContext`（含 budget_tier，从状态推导）+ W3C 根 traceparent 生成透传 + 模式/自治从状态机推导（去硬编码） | ADR-0001 |
| T3 | 接线 + 测试 | `mind_endpoint` 接线 + 集成测试（正常闭环 + **Mind 离线降级闭环**） | ADR-0001 |

### T1 子步骤（proto 同步）

1. `proto/helix_mind.proto` 新增 `BudgetTier` enum（与 Mind 一致）：
   `AUGMENTABLE=0` / `ENDOGENOUS=1` / `EXOGENOUS_REQUIRED=2` / `VOID=3`
2. `EnergyContext` 追加 `BudgetTier budget_tier = 9;`（Append-Only，不动 1-8）
3. `HelixQueryRequest` 追加 `string traceparent = 7;` + `reserved 8 to max;`
4. `HelixQueryResult` 追加 `repeated ActivationEntry activation_vector = 13;` + traceparent 回传字段（对齐 Mind 字段号）
5. `ActivationEntry` message 定义（与 Mind proto 一致，字段号对齐）

### T1 契约对齐清单（字段号）

| 消息 | Mind（真相源） | Anaphase（目标） |
|---|---|---|
| `EnergyContext` | 1-8 现有 + `budget_tier=9` | 同 Mind |
| `HelixQueryRequest` | 1-6 现有 + `traceparent=7` + `reserved 8 to max` | 同 Mind |
| `HelixQueryResult` | 1-12 现有 + `activation_vector=13` + traceparent 回传 | 同 Mind |

### T2 子步骤（mind.rs 补全）

1. **EnergyContext 构造**：从 Anaphase 状态推导
   - `token_budget`：当前纪元预算（config 或状态）
   - `budget_tier`：由状态推导（用户层级/任务复杂度 → AUGMENTABLE/ENDOGENOUS/EXOGENOUS_REQUIRED/VOID）
   - `heliotropism/pulse/vigilance`：Amygdala 后置评估（可选，未启用则默认值）
   - `latency_limit_ms`：config
2. **traceparent 生成**：请求入口生成 W3C 根 `traceparent`（`00-<trace_id>-<span_id>-01`），经 `HelixQueryRequest` 透传
3. **去硬编码**：`suggested_mode` / `autonomy_level` 从 Anaphase 状态机推导，不写死 1
4. **降级钩子**：连接失败/超时 → 降级事件记录（含 trace_id），进入 Noop 路径

### T3 子步骤（接线 + 测试）

1. `config.toml`/`config.rs`：`mind_endpoint` 接线（默认空 = Noop 离线，与 DNA 降级一致）
2. 集成测试用例：
   - **正常闭环**：起 Mind 服务 → HelixQuery 返回节点 → Anaphase 收到记忆
   - **trace 透传**：断言请求中的 traceparent 与响应一致
   - **budget_tier 传递**：断言 Mind 收到非默认 budget_tier
   - **Mind 离线降级**：停 Mind → Anaphase 降级运行（无记忆直接推理，fail-open）+ 降级事件被记录

### 技术前提

- Helix-Mind v4.1 契约已冻结（真相源：`helix-mind/crates/helix-mind-api/proto/helix_mind.proto`）
- Anaphase 现有 proto 前 6 号字段与 Mind 对齐，仅缺 3 个追加字段
- `src/adapters/mind.rs` 当前 `energy_context: None` + 硬编码模式
- 降级链：`docs/design/dependency-fallback.md`（组件独立降级；Tuck 可配置硬依赖，启用后不可停摆）

### 风险与注意事项

- **字段号冲突**：追加必须与 Mind 完全一致，否则 gRPC 解码错乱（T1 验收重点）
- **UDS vs TCP**：Mind 默认 UDS + 远程 TCP（mTLS 预留）；P10a 测试用 TCP 即可（本地回环）
- **FTS/记忆不可用时**：降级为熟练模式直接推理，不阻塞（fail-open）
- **budget_tier 推导**：先做简单规则（任务复杂度 → tier），不做过度设计（勿增实体）

### 验收标准

- `cargo test --workspace` 全绿（含新增 mind 契约测试 ≥ 2 + **Mind 离线降级测试**）
- Anaphase proto 与 Mind proto 字段号完全对齐（无冲突）
- mind.rs 不再硬编码 `suggested_mode`/`autonomy_level`/`energy_context`
- traceparent 从请求入口生成并透传至 Mind
- **Mind 离线时 Anaphase 降级运行**（无记忆直接推理，fail-open），且降级事件被记录

## 下一阶段预览：P10b — 认知工艺触发链路

- System 0 门控经 `EnergyContext.budget_tier` 前置路由（ADR-0010）
- Anaphase 状态机推导认知模式 → Mind `suggested_mode`
- HITL 审批通道接入 Tentacle 契约

---

*Anaphase-Helix PLAN v1.1（P10a 细节补全，2026-08-28）*
