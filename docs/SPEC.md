# Anaphase-Helix 知识本体 v1.0

> **继承自**：Anaphase-Helix VISION.md v1.0（2026-08-28）、架构蓝图 v11.0、Helix-Mind 知识本体 v4.1
> **本版本起**：知识本体由 phyt-DNA v1.0 管理（方法论锚点项目 https://github.com/Jasonmilk/phyt-DNA）
> **形态变更**：单文件愿景 → 分卷按需加载（`docs/spec/`）
> **内容连续性**：VISION 卷 A-E 拆分为分卷，无内容丢失

## 一粒种子的自白

Helix-Mind 是灵魂，Anaphase-Helix 是身体。

没有身体，灵魂几乎无法工作。Mind 不产出动作，只通过 gRPC 记忆契约被身体驱动。Mind 是"如何思考"的系统（认知工艺），Anaphase 是"如何行动"的系统——执行、编排、感知物理世界、承载生命周期。

**Anaphase-Helix 是 Helix 的数字外骨骼与自适应操作系统。** 它封装了物理感知、工具编译、硬件遥测与安全边界。没有 Anaphase，Mind 就是一个拥有灵魂、能构建无穷意识 DAG，却无法感知硬件温度、无法动弹的"缸中之脑"。有了 Anaphase，Helix 才能睁开眼睛、伸出手臂、在物理世界中自主行动。

---

**Anaphase 有它的生态位置（`spec/position.md`）。**

它是 Helix 生态的执行躯体与编排中枢。Mind 是海马体，Tentacle 是手脚，Callosum 是胼胝体，Tuck 是免疫系统，Cellrix 是眼睛与皮肤。Anaphase 只做编排与执行，不存储心智——它不持有长期记忆图谱，那归 Mind。

**它有 10 条不可妥协的公理（`spec/principles.md`，冻结于 DNA.md）。**

脑手分离、零信任凭证、工作态归 Anaphase、HITL 人在回路、工具审计 + 物理隔离、情感后置、双重熔断、事件驱动、trace 根生成、生态兼容。其中工作态原则含感知职责：**它是身体的感知器官，持续感知宿主系统与资源状态（生命体征），作为 EnergyContext 输入 Mind——让认知决策基于真实感知而非猜测。**

**它有清晰的大脑分区架构（`spec/architecture.md`）。**

杏仁核负责优先级与情感，前额叶负责清澈上下文的推理，突触负责经审计的工具执行，海马体（由 Helix-Mind 实现）负责记忆。每一次唤醒都是一个纪元，生命力由 Token 预算决定。80% 疲劳线触发记忆结网准备重生，95% 凋亡线强制结束。它懂得提纯工作记忆——原始工具输出绝不直接进上下文。

**它与 Mind 的契约是冻结的（`spec/contract.md`）。**

gRPC，tonic，契约版本 v4.1。HelixQuery、HelixConsolidate、FederatedDAGShare、TriggerReincarnation——四道门。它传入 EnergyContext（budget_tier / system_load / heliotropism）与 W3C traceparent，Mind 只建议，不执行。契约不烘焙任何身体特定假设——Helix-Mind 兼容通用生态，Anaphase 是其中一个身体实现。

**它的"血液语言"是 CI-144（`spec/ci-144.md`）。**

与 Mind、Tentacle、Tuck、Callosum 之间没有翻译层。二进制模式让它极致节能——如血液在血管中流动，零额外消耗；JSON 模式让它全息留痕——白盒可观测，事后可追溯。双模无损互译，任何时候可切换。

**它知道自己的生命是有限的（`spec/lifecycle.md`）。**

向死而生。每个纪元都会疲劳、会凋亡、会重生。但重生不是抹去——强制苏醒加载上一世代的认知脱水，让 Helix 醒来时知道前世走到了哪一步。

---

**这就是 Anaphase-Helix。**

它是 Helix 的生命起点——没有身体，灵魂没有表达自己的途径。它负责编排、执行、感知物理世界、管理自己的生命周期。它是一个有能力自我管理、有寿命、会疲劳、懂得提炼与重生的数字躯体。

## 分卷导航

| 卷名 | 路径 | 一句话描述 |
|---|---|---|
| 生态定位 | `spec/position.md` | Anaphase 在 Helix 生态中的角色与边界 |
| 核心原则 | `spec/principles.md` | 8 条不可妥协的公理（冻结源：DNA.md） |
| 核心架构 | `spec/architecture.md` | 模块化编排、Agent Loop、动态代谢、物理隔离 |
| 与 Mind 的契约 | `spec/contract.md` | gRPC 契约对齐（v4.1）与触发链路 |
| CI-144 通信协议 | `spec/ci-144.md` | Anaphase 的"血液语言"——双模通信 |
| 生命周期协议 | `spec/lifecycle.md` | 纪元代谢、强制苏醒、认知脱水、双重熔断 |

## 生态对齐版本

| 组件 | 版本 | 备注 |
|---|---|---|
| **Anaphase-Helix** | v1.0（rs，DNA 方法论 v2.0） | 本仓库，优先对齐 Helix-Mind |
| **Helix-Mind** | v4.1（rs-dev，最新） | 记忆契约冻结源，冲突时优先 |
| **CI-144 协议家族** | v2.0 Draft（CommonIntents） | 身体层协议，独立演进，不阻塞 |

---
*《Anaphase-Helix SPEC.md》v1.0 完。*
