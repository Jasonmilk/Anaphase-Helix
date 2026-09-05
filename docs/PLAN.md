# Anaphase-Helix PLAN — 当前阶段导航

> **DNA 方法论 v1.0** ｜ PLAN.md 是导航牌，不是历史档案（≤150 行）。完成记录进 GROWTH.md。

## 当前阶段：候选 F（会话即经历 + 三模式参与度）— 完成

**候选 F 目标（ADR-0006）**：Helix 无会话概念——对话是 Helix 的经历（L3 情景），Mind 能"看到"会话（元认知）；驾驶/伙伴/生存三模式有效运行。不新建 crate / 协议 / RPC / L3 schema 字段，全部复用既有要素。

### 候选 F 完成状态

| 任务 | 内容 | 状态 |
|---|---|---|
| F-T1 | 探查：L3 结构 / 认知工艺 / INTENT-7 FINISH / Noop 装配（物理核验复用点） | ✅ ADR-0006 §1 |
| F-T2 | `contract::fnv64` 提取共用 + `derive_episode_id`（ep- 前缀，确定性回放） | ✅ |
| F-T3 | AgentLoop `episode: Option<Episode>` + begin/end 生命周期（自动收束旧经历、幂等） | ✅ |
| F-T4 | Reflection 写入带 `{id}#{step}` provenance（无 episode 原样，向后兼容） | ✅ |
| F-T5 | `config::Mode { Drive, Partner, Survive }`（默认 Partner）+ main.rs 装配接线 | ✅ |
| F-T6 | 测试 + 文档：episode_lifecycle 10 例 + ADR-0006 + 文档五件套 | ✅ |

**关键成果**：逐轮 L3 摄取升级为带经历边界的结构化经历（`{"episode":"ep-x#n","note":...}`）；经历收束 digest 经既有 remember 通道落 L3（语义对应 INTENT-7 FINISH）；三模式物理落地——Drive=Noop 装配（已有路径）、Partner=episode 生命周期、Survive=枚举占位（待 Mind P10a）；运行期零 if 分支（Noop 天然隔离）；105 passed + 3 live（#[ignore]）。

### M1.5 / 候选 E 剩余项（已消项）

- ~~Reasoning 输出结构化~~（E-T2 完成）
- ~~suggested_actions 结构化 + pipeline 完整 merge~~（E-T3..T5 完成）
- ~~Helix 无会话概念~~（候选 F 完成）

### 下一阶段候选

- **候选 D'：M1.5 深化**——seen_entropy_bloom 重放守卫（Callosum）/ Tuck 深度集成 / 真实场景插件（非 fixture）/ main.rs pipeline 接线（tentacle_endpoint 消费）
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

*Anaphase-Helix PLAN v2.0（候选 F 完成：会话即经历 + 三模式参与度，2026-09-05）*
