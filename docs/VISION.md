# 🌌 Anaphase-Helix 愿景索引
> **版本**：v2.0
> **日期**：2026-08-28
> **性质**：本文件是 `SPEC.md`（叙事）与 `spec/`（规格）之间的桥梁。
> **用途**：所有架构决策应以本文件中提炼的原子原则为最终裁判。
> **所属方法论**：DNA 自生长方法论 v2.0
> **继承自**：Anaphase-Helix 架构蓝图 v11.0、Helix-Mind 知识本体 v4.1

## 卷首语
Helix-Mind 是灵魂，Anaphase-Helix 是身体。
没有身体，灵魂几乎无法工作。Mind 是"如何思考"的系统（认知工艺），Anaphase 是"如何行动"的系统——执行、编排、感知物理世界、承载生命周期。
**Anaphase-Helix 是 Helix 的数字外骨骼与自适应操作系统。** 它封装物理感知、工具编译、硬件遥测与安全边界。它是身体的感知器官——持续感知宿主系统与资源状态（生命体征），作为 `EnergyContext` 输入 Mind，让认知决策基于真实感知而非猜测。
> 完整故事请阅读 `SPEC.md`。

## 原子原则（10 条，冻结于 DNA.md）
以下原则是 Anaphase-Helix 所有设计决策的最高依据。与 `docs/DNA.md` 一致。

| # | 原则 | 一句话解释 | 对应规格 |
|---|---|---|---|
| 1 | **脑手分离，无锁分立** | Mind 只建议不执行，Anaphase 是身体；不持有长期记忆图谱 | `spec/contract.md` + `spec/position.md` |
| 2 | **零信任凭证隔离** | 明文凭证永不进内存，由 Tuck 物理隔离 | `spec/position.md` + `spec/ci-144.md` |
| 3 | **工作态归 Anaphase** | 当前纪元工作记忆 + 任务 DAG + **生命体征感知**（→ EnergyContext） | `spec/architecture.md` + `spec/lifecycle.md` |
| 4 | **HITL 人在回路审批** | 高风险动作必须人类确认，否则物理拦截 | `spec/architecture.md` |
| 5 | **工具审计 + 物理隔离** | 新工具必须审计后入库；任务物理目录隔离 | `spec/architecture.md` |
| 6 | **情感后置** | 情感分数直出不映射表情；最后计算不影响推理 | `spec/architecture.md` |
| 7 | **双重熔断** | Token 预算熔断（80/95）+ 任务接力熔断（5 次） | `spec/lifecycle.md` |
| 8 | **事件驱动，队列闲时调度** | 严禁轮询；四象限优先级排队 | `spec/lifecycle.md` + `spec/ci-144.md` |
| 9 | **trace 根生成** | 每个外部请求入口生成 W3C traceparent，Mind 只透传 | `spec/contract.md` |
| 10 | **生态兼容** | 优先 Helix 生态，兼容通用生态（body-agnostic） | `spec/position.md` + `spec/contract.md` |

## 分卷导航
| 卷名 | 路径 | 一句话描述 |
|---|---|---|
| 生态定位 | `spec/position.md` | Anaphase 在 Helix 生态中的角色与边界 |
| 核心原则 | `spec/principles.md` | 8 条不可妥协的公理（冻结源：DNA.md） |
| 核心架构 | `spec/architecture.md` | 模块化编排、Agent Loop、动态代谢、物理隔离 |
| 与 Mind 的契约 | `spec/contract.md` | gRPC 契约对齐（v4.1）与触发链路 |
| CI-144 通信协议 | `spec/ci-144.md` | Anaphase 的"血液语言"——双模通信 |
| 生命周期协议 | `spec/lifecycle.md` | 纪元代谢、强制苏醒、认知脱水、双重熔断 |

## 生态位置（组件视图）
| 组件 | 角色 | 与 Anaphase 的关系 |
|---|---|---|
| **Helix-Mind** | 海马体/记忆中枢（灵魂） | 通过 gRPC 驱动，Mind 只建议不执行 |
| **Anaphase-Helix** | 执行躯体/编排中枢（身体） | 本仓库 |
| **Helix-Tentacle** | 战术武器库（手脚） | 工具执行、无状态动作 |
| **Tuck** | 物理合规闸门（免疫系统） | 凭证隔离、安全审计、硬拦截（强制） |
| **Helix-Callosum** | 神经桥接器（胼胝体） | 上下文压缩、KV 缓存复用 |
| **Cellrix** | 语义投影终端（眼睛/皮肤） | 渲染、交互、可视化 |

**生态对齐版本**：Helix-Mind v4.1（最新，优先对齐）｜ Anaphase-Helix v1.0（rs）｜ CI-144 v2.0 Draft（独立演进）

## 快速导航
| 你想知道 | 去看 |
|---|---|
| Anaphase 的完整故事 | `SPEC.md` |
| 不可变宪法 | `DNA.md` |
| 当前阶段导航 | `PLAN.md` |
| 生长记录 | `GROWTH.md` |
| 加载协议 | `RNA.md` |
| 凋亡清单 | `DEPRECATE.md` |
| 架构决策记录 | `docs/decisions/` |
| 依赖降级链 | `docs/design/dependency-fallback.md` |

## 组件仓库索引
| 组件 | 仓库 | 当前状态 |
|---|---|---|
| Helix-Mind | https://github.com/Jasonmilk/Helix-Mind/tree/rs-dev | Rust，活跃开发 |
| Anaphase-Helix | https://github.com/Jasonmilk/Anaphase-Helix/tree/rs | Rust，活跃开发（本仓库） |
| Cellrix | https://github.com/Jasonmilk/Cellrix | Rust，活跃开发 |
| Helix-Tentacle | https://github.com/Jasonmilk/Helix-Tentacle | Rust，活跃开发 |
| Helix-Callosum | https://github.com/Jasonmilk/Helix-Callosum | Rust，待实现 |
| Tuck | https://github.com/Jasonmilk/Tuck/tree/Tuck-beta | Python beta，规划 Rust 重构 |
| FlowModus | https://github.com/Jasonmilk/FlowModus | 半成品，规划 Rust 重构 |
| CI-144 协议家族 | https://github.com/CommonIntents | 协议栈（INTENT-7/BIND-19/CAPABILITY-13） |

---
*《Anaphase-Helix VISION.md》v2.0 完。*
