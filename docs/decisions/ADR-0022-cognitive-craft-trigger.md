# ADR-0022: 认知工艺触发接线验证 + Mind 适配器零硬编码收口

- **状态**: Accepted
- **日期**: 2026-09-06
- **决策范围**: Anaphase（GrpcMindAdapter / config / 伙伴模式触发链验证）
- **关联**: ADR-0001（P10a 记忆契约）、ADR-0002（DNA 原则 11 零硬编码）、ADR-0010（预算分级）、Helix-Mind 认知工艺（四拍/五工序）

## 1. 背景与问题

O 系列前序（O-1/O-2/O-3，ADR-0019/0020/0021）完成白盒四层后，剩余主线是
**认知工艺触发接线**（O-4）：伙伴模式下 run_cycle 必须真实触发 Helix-Mind 的
认知工艺（干活前检索自评 → 干活中风格对齐 → 交作业预期校准 → 收反馈差距评估），
suggested_actions 作为 Mind 的编排建议流入 Execution。

探查（物理事实优先）发现两件事：

1. **触发链代码早已存在但从未被验证**：`MemoryRetrieval → memory.query →
   GrpcMindAdapter.helix_query`（ADR-0001 时代落地，proto 契约含
   effective_mode / suggested_actions / impasse_level），suggested_actions
   注入 context 后被 Execution 消费（run_cycle.rs:490/493/605）。但一路
   fail-open 降级，真实 gRPC 链路从未跑通；`p11b_suggested_actions_flow_to_execution`
   已覆盖 adapter 层 + run_cycle 层闭环（mock Mind server 复用）。
2. **mind.rs 存在 12 处无来源字面量**：`token_budget: 1000`、
   `latency_limit_ms: 500`、`pulse: 0.3`、`vigilance: 0.2`、`familiarity: 0.5`、
   阈值 `0.8 / 4 / 60 / 10 / 40`、探针回退 `0.5`、`EXPLORE_KEYWORDS` 数组——
   违反 DNA 原则 11（每个字面量必须有 config/contract/派生来源）。

## 2. 决策

### D1: 触发链验证——复用既有 mock Mind，只补缺口
mock Mind server（`tests/mind_integration.rs`）与 run_cycle 闭环测试已存在。
O-4 **不重复建设**：只补唯一缺口——**驾驶模式不触发回归守卫**
（`drive_mode_never_contacts_mind`：Noop 装配下 Mind 零接触，计数不变）。
模式门是**装配本身**（Noop vs Grpc adapter），无运行时分支。

### D2: Mind 适配器零硬编码——MindConfig 单一来源
新增 `MindConfig`（`src/config.rs`，`[anaphase.mind]` 可覆盖）承载全部可调参数：
EnergyContext 数值（token_budget / latency_limit_ms / pulse / vigilance / familiarity）、
推导阈值（high_load / short_query / long_query / skilled_len / anchor_len）、
探针回退（probe_fallback）、探索语义关键词（explore_keywords）。
`GrpcMindAdapter::new(endpoint, config)` 签名变更（单一构造路径）。

### D3: 派生值不入 config（如无必要勿增实体）
- `heliotropism = 0.0` —— P10a 未实现的中性值（派生自"功能未实现"）
- `impasse_depth = 0` —— 循环起始 impasse（派生自"cycle 开始"）
- complexity clamp 0..=3 —— 认知模式状态范围（proto 契约）
以上保留字面量但**注释声明来源**，不膨胀 config 字段。

## 3. 验收结果（物理验证）

| 判据 | 结果 |
|---|---|
| 触发链真实连通（mock Mind gRPC） | ✅ 既有 normal_closed_loop / p11b 全绿 |
| 驾驶模式不触发 | ✅ T3 新增：Noop 装配下 mock 计数不变 |
| mind.rs 无字面量残留 | ✅ grep 仅剩 heliotropism=0.0（注释声明派生） |
| 全量测试 | ✅ 183 passed / 0 failed（182 + T3） |

## 4. 备选方案与拒绝理由

| 备选 | 拒绝理由 |
|---|---|
| 新建共享 mock Mind（common/mod.rs） | 重复建设——mind_integration.rs 已有完整 MockMind，已回滚（git checkout） |
| MindConfig 收 heliotropism/impasse_depth | 勿增实体——派生值有来源即合法，注释声明即可 |
| 保留 `new(endpoint)` + 新增 `with_config` | 双构造路径违背单一入口；改签名 + 8 处调用点更干净 |

## 5. 后果

**正面**：
- 伙伴模式认知工艺触发链有真实 wire-layer 验证，不再靠 fail-open 兜底；
- 驾驶模式"永不触发 Mind"成为可回归断言（物理事实优先）；
- mind.rs 全部可调参数单一来源（config），DNA 原则 11 在适配器层收口。

**负面/代价**：
- `GrpcMindAdapter::new` 签名变更波及 8 处调用点（一次性，已修）；
- MindConfig 12 字段——均为既有字面量的收口，非新增实体。

**风险与对策**：
- 真实 Mind 连通仍待 P10a（UDS 收尾）→ mock Mind 是确定性中间层，契约已锁定
  （proto reserved 15+，Append-Only Schema Evolution）。
