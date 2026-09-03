# Anaphase-Helix PLAN — 当前阶段导航

> **DNA 方法论 v1.0** ｜ PLAN.md 是导航牌，不是历史档案（≤150 行）。完成记录进 GROWTH.md。

## 当前阶段：M1.5 生态合流（候选 D）— 核心完成

**M1.5 目标（ADR-0004）**：Tentacle `--transport grpc` + fixture 插件 + 真实连通 + 语义定义 + run_cycle 渐进接线。

### M1.5 完成状态

| 任务 | 内容 | 状态 |
|---|---|---|
| T1 | Tentacle `--transport grpc` 接线（--grpc-port 默认 50051） | ✅ Tentacle d902151 |
| T2 | fixture 插件 numbers/rate（manifest+js+SHA-256，参数化 MET/UNMET） | ✅ Tentacle d902151 |
| T3/T4 | 真实连通 e2e（pipeline → 真实 grpc → node 执行） | ✅ 1c1e723 |
| T5 | identity_labels / seen_entropy_bloom 语义定义 + 透传 | ✅ 6ac3a0e |
| T6 | run_cycle Execution 接回 step 1（tool_command + echo fallback） | ✅ 4d7e3ec |
| T7 | ADR-0004 + PLAN v1.8 + GROWTH + README + ECOSYSTEM | ✅ 本次 |

**关键成果**：M1 mock → M1.5 真实无缝切换（fixture 默认值 = M1 mock 形状 + 判据契约双约束）；真实链路 2 条 e2e 全绿（m1_5_live_met / m1_5_live_unmet）。

### M1.5 剩余项（下一里程碑）

- **Reasoning 输出结构化**：替换 `contains("tool_call")` 字符串匹配（六 stage 映射表 stage1 落点）
- **suggested_actions 结构化** + pipeline 完整 merge（stage2-6 落点）
- seen_entropy_bloom 重放守卫启用（Callosum 协作）

## 认知工艺双向复用轨道状态（备忘录重编号后）

| 阶段 | 状态 | 说明 |
|---|---|---|
| **P11a** CraftAdapter | ✅ 完成（裁决：不建） | 间接触发已覆盖，意志优先，勿增实体 |
| **P11b** 编排链路 | ✅ 完成（验证闭环） | OrchestrationAdapter 不建，链路已通 |
| **P11c** OrchestrationCore | ⏸️ 暂缓 | trait 归属待 Mind 认知工艺显式化后裁决 |
| **P11d** 双向复用 | ⏸️ 暂缓 | 依赖 P11c；模式同构+接口复用，职责不合并 |

## 下一阶段候选

- **候选 E：Reasoning 结构化 + pipeline 完整 merge**（M1.5 剩余项，优先）——替换 contains 匹配，suggested_actions 结构化，六 stage 完整落点
- **候选 D'：M1.5 深化**——seen_entropy_bloom 重放守卫（Callosum）/ Tuck 深度集成 / 真实场景插件（非 fixture）
- **候选 A：Tentacle Rust 重构**（P10b 后自然启动）——凭证标签流转（Tuck 注入）/ 异步协程沙箱（ARM 端侧）/ 动态共识适配层 / 多传输层扩展
- **候选 B：生态手套协议渐进**（P10c 预留扩展位）——Cellrix 原生手套协议接入
- **候选 C：保持 P11c/P11d 暂缓**，等 Mind 侧认知工艺显式化

---

*Anaphase-Helix PLAN v1.8（M1.5 生态合流核心完成，立候选 E，2026-09-03）*
