# Anaphase-Helix PLAN — 当前阶段导航

> **DNA 方法论 v1.0** ｜ PLAN.md 是导航牌，不是历史档案（≤150 行）。完成记录进 GROWTH.md。

## 当前阶段：候选 E（Reasoning 结构化 + run_cycle ↔ pipeline 完整 merge）— 完成

**候选 E 目标（ADR-0005）**：替换 contains 字符串匹配，suggested_actions 结构化，六 stage 完整落点 run_cycle。

### 候选 E 完成状态

| 任务 | 内容 | 状态 |
|---|---|---|
| E-T1 | 探查：reasoning 输出协议 / suggested_actions 消费点 / 状态机扩展点 | ✅ docs/design/candidate-e-recon.md |
| E-T2 | Reasoning 结构化：`parse_reasoning_output` 协议（calls+impasse），替换 contains 匹配 | ✅ |
| E-T3 | AgentContext 结构化：`calls` / `job` / `evidence` 字段（suggested_actions 保留） | ✅ |
| E-T4 | Execution 接回：`execute_structured` → `Pipeline::execute_calls` + `record_evidence` | ✅ |
| E-T5 | Reflection 判据消费：`check_results` + `build_verdict` + ledger.append | ✅ |
| E-T6 | 5 处硬编码 → `config::RunCycleConfig`（config 来源，agent_loop 零字面量） | ✅ |
| E-T7 | 测试 + 文档：run_cycle↔pipeline 集成测试 8 例 + ADR-0005 + 文档五件套 | ✅ |

**关键成果**：run_cycle 一次循环走通六 stage 全链路（Reasoning 结构化 calls → Execution 真实 gRPC → Reflection criteria → ledger）；确定性可回放（同输入同时钟字节级一致）；94 passed + 3 live（#[ignore]）。

### M1.5 剩余项（已消项）

- ~~Reasoning 输出结构化~~（E-T2 完成）
- ~~suggested_actions 结构化 + pipeline 完整 merge~~（E-T3..T5 完成）

### 下一阶段候选

- **候选 D'：M1.5 深化**——seen_entropy_bloom 重放守卫（Callosum）/ Tuck 深度集成 / 真实场景插件（非 fixture）/ main.rs pipeline 接线（tentacle_endpoint 消费）
- **候选 A：Tentacle Rust 重构**（P10b 后自然启动）——凭证标签流转（Tuck 注入）/ 异步协程沙箱（ARM 端侧）/ 动态共识适配层 / 多传输层扩展
- **候选 B：生态手套协议渐进**（P10c 预留扩展位）——Cellrix 原生手套协议接入
- **候选 C：保持 P11c/P11d 暂缓**，等 Mind 侧认知工艺显式化

## 认知工艺双向复用轨道状态（备忘录重编号后）

| 阶段 | 状态 | 说明 |
|---|---|---|
| **P11a** CraftAdapter | ✅ 完成（裁决：不建） | 间接触发已覆盖，意志优先，勿增实体 |
| **P11b** 编排链路 | ✅ 完成（验证闭环） | OrchestrationAdapter 不建，链路已通 |
| **P11c** OrchestrationCore | ⏸️ 暂缓 | trait 归属待 Mind 认知工艺显式化后裁决 |
| **P11d** 双向复用 | ⏸️ 暂缓 | 依赖 P11c；模式同构+接口复用，职责不合并 |

---

*Anaphase-Helix PLAN v1.9（候选 E 完成：Reasoning 结构化 + pipeline 完整 merge，2026-09-03）*
