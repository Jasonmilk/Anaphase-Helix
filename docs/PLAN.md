# Anaphase-Helix PLAN — 当前阶段导航

> **DNA 方法论 v1.0** ｜ PLAN.md 是导航牌，不是历史档案（≤150 行）。完成记录进 GROWTH.md。

## 当前阶段：候选 G 完成（G-T2..T6 + G-3 transport 修复 + G-4 bootstrap `up`）——驾驶舱真实渲染、一条命令起全栈；下一步 G-5 易用引导 UX

**候选 D' 目标（ADR-0007/0008/0009）**：M1.5 深化四项全部完成。

### 候选 D' 完成状态（本轮）

| 任务 | 内容 | 状态 |
|---|---|---|
| D'-1 | `contract::derive_seen_bloom`（bl- 前缀，fnv64 共享原语）替换 execute_calls 空串占位 | ✅ ADR-0007 |
| D'-3 | `pipeline::resolve_pipeline`（fail-open）+ main.rs tentacle_endpoint 接线 | ✅ ADR-0007 |
| D'-2 | Tuck 深度集成：`SecurityGate` 接线点 + ledger `blocked` 记录 + 真实 TuckSecurityGate 连通 | ✅ ADR-0008 |
| D'-4 | 真实场景插件（非 fixture，接入 MCP-Learner stable/ 工具）：`Expect::Ok` 结构判据 + live e2e（真实 Tentacle + 学习产物 3/3 全绿） | ✅ ADR-0009 |

**关键成果**：`seen_entropy_bloom` 从 `""` 占位升级为真实确定性指纹（`bl-` + FNV-1a(`{tool}#{params}`)）；配置 `tentacle_endpoint` 后启动即走六 stage 流水线（fail-open，未配置/失败保持 echo fallback）；**D'-2 管控闭环咽喉落地**——pipeline 执行路径可被 Tuck 闸门拦截（`src/security.rs` SecurityGate trait + `with_security_gate` + ledger `Blocked` 记录，Reject/HITL 阻塞 call 且不进 Tentacle；真实连通测试经 dev-only tuck-core 验证 Low→Pass 执行 / Catastrophic→Reject / Critical→HitlRequired）；**候选 G-T2（ADR-0010）**——`AgentLoop::capture()` 共享快照投影（mode/state/episode/ledger），`/v1/agent/snapshot` 输出真实状态（消除 `token_consumed: 1234` 硬编码），HTTP 端点不触碰 agent 内部（极致解耦）；126 passed + 6 live（#[ignore]，含 m1_e2e_live 3 + m1_5_d4_live 3）。

### M1.5 / 候选 E / 候选 F 剩余项（已消项）

- ~~Reasoning 输出结构化~~（E-T2 完成）
- ~~suggested_actions 结构化 + pipeline 完整 merge~~（E-T3..T5 完成）
- ~~Helix 无会话概念~~（候选 F 完成）
- ~~seen_entropy_bloom 空串占位~~（D'-1 完成）
- ~~main.rs 未消费 tentacle_endpoint~~（D'-3 完成）

### 下一阶段候选

- **候选 D' 剩余**：无（四项全部完成；真实 MCP 执行体升级属 ECOSYSTEM 第二优先级 #4）
- **候选 G：Anaphase 驾驶舱**（完成：G-T2 ✅ / G-T3..T5 ✅ / G-T6 ✅ / G-3 transport 契约修复 ✅ / G-4 bootstrap ✅）——Cellrix 白盒驾驶舱（模式栏 + 经历时间线 + Ledger 审查视图 + 生态状态板）；TUI 先行，Web 面板（G2）后续；**bootstrap `up`（ADR-0011）：一条命令起全栈（tentacle→anaphase→探测→可选 --cockpit）**；G-5 易用引导 UX（首跑向导）待启
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

*Anaphase-Helix PLAN v2.3（候选 G 完成 + G-4 bootstrap，2026-09-06）*
