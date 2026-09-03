# Anaphase-Helix PLAN — 当前阶段导航

> **DNA 方法论 v1.0** ｜ PLAN.md 是导航牌，不是历史档案（≤150 行）。完成记录进 GROWTH.md。

## 当前阶段：M1 里程碑（确定性流水线）— T0-T8 完成，T9 收尾

**M1 目标（ADR-0003）**：Anaphase 独立完成可回放闭环——mock LLM 产出 tool_calls → 组装 tt_job → gRPC 调 mock Tentacle → evidence 落盘 → 纯函数 criteria 校验 → ledger 写 JSONL。全程确定性，Tentacle 字面零改动。

### M1 完成状态

| 任务 | 内容 | 状态 |
|---|---|---|
| T0 | vendor Tentacle v1 proto + MockTentacle + GrpcTentacleAdapter 重写 | ✅ 59b08de |
| T2-T5 | contract / evidence / criteria / ledger 四模块 | ✅ d5f732c |
| T6-T7 | 复用 HttpReasoningAdapter + mock 联调三分支 | ✅ e2a86c6 |
| T8 | pipeline 六 stage + m1_e2e 三用例（MET/UNMET/replay） | ✅ e2a86c6 |
| T9 | ADR-0003 + PLAN v1.7 + GROWTH 快照 | ✅ 本次 |

**验收三判据**：① m1_e2e_met + m1_e2e_unmet 双用例全绿 ✅ ② 同一输入跑两次 ledger 字节级一致 ✅ ③ 假时钟扫出 due 的 UNMET ✅

**哲学落档**：DNA 原则 11「零硬编码」新增（ADR-0002）；代码注释统一英文。

### 关键裁决（ADR-0003）

- **引擎归属**：run_cycle 与 tt_job 基因不兼容（四证据）→ 旁路建 pipeline；M1.5 接回 run_cycle 为必做项（六 stage ↔ 六状态映射表）
- **重入语义**：UNMET + retry_due + parent_id 谱系；循环 = 队列消费（M1 落账，M1.5 消费）
- **确定性钳制**：trace_id/evidence_id 派生、禁 HashMap、禁 endpoint

## 认知工艺双向复用轨道状态（备忘录重编号后）

| 阶段 | 状态 | 说明 |
|---|---|---|
| **P11a** CraftAdapter | ✅ 完成（裁决：不建） | 间接触发已覆盖，意志优先，勿增实体 |
| **P11b** 编排链路 | ✅ 完成（验证闭环） | OrchestrationAdapter 不建，链路已通 |
| **P11c** OrchestrationCore | ⏸️ 暂缓 | trait 归属待 Mind 认知工艺显式化后裁决 |
| **P11d** 双向复用 | ⏸️ 暂缓 | 依赖 P11c；模式同构+接口复用，职责不合并 |

## 下一阶段候选

- **候选 D：M1.5 生态合流**（新立，M1 完成后优先）——Tentacle `--transport grpc` + fixture 插件 + 真实连通测试；pipeline 接回 run_cycle（六 stage 映射表）；Tuck 深度集成；identity_labels/seen_entropy_bloom 语义定义
- **候选 A：Tentacle Rust 重构**（P10b 后自然启动）——凭证标签流转（Tuck 注入）/ 已见熵布隆过滤器（Callosum）/ 异步协程沙箱（ARM 端侧）/ 动态共识适配层 / 多传输层（gRPC/HTTP/MCP/STDIO）
- **候选 B：生态手套协议渐进**（P10c 预留扩展位）——Cellrix 原生手套协议接入（状态已注册，协议实现挂扩展位）
- **候选 C：保持 P11c/P11d 暂缓**，等 Mind 侧认知工艺显式化

---

*Anaphase-Helix PLAN v1.7（M1 确定性流水线 T0-T8 完成，立候选 D，2026-09-03）*
