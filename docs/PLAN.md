# Anaphase-Helix PLAN — 当前阶段导航

> **DNA 方法论 v1.0** ｜ PLAN.md 是导航牌，不是历史档案（≤150 行）。完成记录进 GROWTH.md。

## 当前阶段：候选 D'（部分完成：D'-1 重放守卫指纹 + D'-3 启动接线）

**候选 D' 目标（ADR-0007）**：M1.5 深化——不阻塞项先行（D'-1 seen_entropy_bloom 重放守卫指纹 + D'-3 main.rs pipeline 接线）；D'-2（Tuck 深度集成）/ D'-4（真实场景插件）仍阻塞。

### 候选 D' 完成状态（本轮）

| 任务 | 内容 | 状态 |
|---|---|---|
| D'-1 | `contract::derive_seen_bloom`（bl- 前缀，fnv64 共享原语）替换 execute_calls 空串占位 | ✅ |
| D'-3 | `pipeline::resolve_pipeline`（fail-open）+ main.rs tentacle_endpoint 接线 | ✅ |
| D'-2 | Tuck 深度集成（HITL 审批通道 + identity_labels 审计闭环） | ⏸️ 阻塞（Tuck 侧接口） |
| D'-4 | 真实场景插件（非 fixture，接入 MCP-Learner stable/ 工具） | ⏸️ 阻塞（MCP-Learner 升级） |

**关键成果**：`seen_entropy_bloom` 从 `""` 占位升级为真实确定性指纹（`bl-` + FNV-1a(`{tool}#{params}`)）；配置 `tentacle_endpoint` 后启动即走六 stage 流水线（fail-open，未配置/失败保持 echo fallback）；110 passed + 3 live（#[ignore]）。

### M1.5 / 候选 E / 候选 F 剩余项（已消项）

- ~~Reasoning 输出结构化~~（E-T2 完成）
- ~~suggested_actions 结构化 + pipeline 完整 merge~~（E-T3..T5 完成）
- ~~Helix 无会话概念~~（候选 F 完成）
- ~~seen_entropy_bloom 空串占位~~（D'-1 完成）
- ~~main.rs 未消费 tentacle_endpoint~~（D'-3 完成）

### 下一阶段候选

- **候选 D' 剩余**：D'-2 Tuck 深度集成（Tuck 侧接口就绪后）/ D'-4 真实场景插件（MCP-Learner 升级后）
- **候选 G：Cellrix 门面 = 经历时间线**（界面层，独立仓库）——会话列表 = 经历时间线（episode 消化状态：待消化/已内化）、生活视图（睡眠/代谢）、模式状态栏；驾驶模式原型（纯 Anaphase）
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

*Anaphase-Helix PLAN v2.1（候选 D' 部分完成：D'-1 重放守卫指纹 + D'-3 启动接线，2026-09-05）*
