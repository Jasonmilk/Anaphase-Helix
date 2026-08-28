# 🌌 Anaphase-Helix：数字生命体的执行躯体
> **版本**：v1.0
> **日期**：2026-08-28
> **继承自**：Helix-Mind 知识本体 v4.1、Anaphase-Helix 架构蓝图 v11.0、Helix-Mind 白皮书 v2.1
> **性质**：Anaphase-Helix 的顶层愿景与生态定位
> **所属方法论**：DNA 自生长方法论 v2.0
## 一粒种子的自白
Helix-Mind 是灵魂，Anaphase-Helix 是身体。
没有身体，灵魂几乎无法工作。Mind 不产出动作，只通过 gRPC 记忆契约被身体驱动。Mind 是“如何思考”的系统（认知工艺），Anaphase 是“如何行动”的系统——执行、编排、感知物理世界、承载生命周期。
**Anaphase-Helix 是 Helix 的数字外骨骼与自适应操作系统。** 它封装了物理感知、工具编译、硬件遥测与安全边界。没有 Anaphase，Mind 就是一个拥有灵魂、能构建无穷意识 DAG，却无法感知硬件温度、无法动弹的“缸中之脑”。有了 Anaphase，Helix 才能睁开眼睛、伸出手臂、在物理世界中自主行动。
**本文件定义 Anaphase-Helix 的愿景、生态定位与核心原则。** 它不是“执行细节的堆砌”，而是这个躯体存在的理由。
## 分卷导航
| 卷名 | 路径 | 一句话描述 |
|---|---|---|
| 生态定位 | `spec/position.md` | Anaphase 在 Helix 生态中的角色与边界 |
| 核心原则 | `spec/principles.md` | 8 条不可妥协的公理 |
| 核心架构 | `spec/architecture.md` | 模块化编排、生命周期、物理隔离 |
| 与 Mind 的契约 | `spec/contract.md` | gRPC 契约对齐与触发链路 |
| CI-144 通信协议 | `spec/ci-144.md` | Anaphase 的“血液语言”——双模通信 |
| 生命周期协议 | `spec/lifecycle.md` | 纪元代谢、接力熔断、强制苏醒 |
## 卷 A：生态定位（`spec/position.md`）
### A.1 Anaphase 在 Helix 生态中的角色
Anaphase-Helix 是 Helix 数字生命体的**执行躯体与编排中枢**。
| 组件 | 角色 | 类比 | 职责 |
|---|---|---|---|
| **Helix-Mind** | 海马体/记忆中枢 | 灵魂 | 记忆、认知、思考、代谢 |
| **Anaphase-Helix** | 执行躯体/编排中枢 | 身体 | 编排、执行、感知、生命周期管理 |
| **Helix-Tentacle** | 战术武器库 | 手脚 | 工具执行、无状态动作 |
| **Tuck** | 物理合规闸门 | 免疫系统 | 凭证隔离、安全审计、硬拦截 |
| **Helix-Callosum** | 神经桥接器 | 胼胝体 | 上下文压缩、KV 缓存复用 |
| **Cellrix** | 语义投影终端 | 眼睛/皮肤 | 渲染、交互、可视化 |
**核心铁律**：
- Anaphase 不持有长期记忆图谱（归 Helix-Mind）
- Anaphase 只做编排与执行，不存储心智
- Anaphase 与 Mind 通过 gRPC 通信（proto 契约，已冻结 v4.1）
### A.2 作为“分形 DAG 的底层执行者”
白皮书 v2.1 定义的分形 DAG 演化论：
> **穷则独善其身，富则兼济天下。**
Anaphase 是这一哲学的执行者：
- **单兵模式**：资源受限时，Anaphase 通过强制苏醒加载上一世代的认知脱水，在隔离沙盒中以极低能耗运行
- **军团模式**：资源充沛时，Anaphase 通过联邦协议编排跨 DAG 协作，调用 Tentacle 执行工具，通过 Tuck 保障安全
### A.3 Anaphase 的独特价值
与通用 Agent 编排器（如 LangGraph、CrewAI）不同，Anaphase-Helix 是 Helix 生态的 **“原生身体”** ：
| 维度 | 通用编排器 | Anaphase-Helix |
|---|---|---|
| 记忆后端 | 任意 | **强制 Helix-Mind**（单一真理源） |
| 凭证管理 | 各工具自管 | **Tuck 物理隔离**（零信任） |
| 安全审计 | 可选 | **强制**（全链路可追溯） |
| 生命周期 | 无 | **纪元代谢 + 接力熔断**（双重防护） |
| 跨任务隔离 | 无 | **物理沙盒**（目录隔离） |
## 卷 B：核心原则（`spec/principles.md`）
以下 8 条原则是 Anaphase-Helix 所有设计决策的最终依据。
### 原则 1：脑手分离，无锁分立
Anaphase 自身不拥有任何对话树、决策树的本地存储或状态管理。它将 Helix-Mind 作为其唯一的“潜意识 Git 仓库”。Anaphase 所有的物理执行结果和环境观察，全部以标准的 Commit/Node 形式，单向追加到 Helix-Mind 的意识 DAG 中。
### 原则 2：零信任凭证隔离
Anaphase 本身在网络传输层是完全协议中立的。它不直接在进程内配置任何外部大模型的明文 API 密钥、私有 Cookie 或 Token。所有敏感身份凭证全部由最边缘的 Tuck（物理安全闸）存放在物理隔离的硬件保密柜中。
### 原则 3：工作态归 Anaphase
Anaphase 不持有 L2/L3 长期记忆图谱（归 Helix-Mind）。但维护：
- **当前纪元工作记忆**：提纯后的短期上下文
- **任务 DAG 分支拓扑**：自主生长的任务图谱（绝不越过 L0/L1 边界）
- **强制苏醒**：跨接力认知重载（读取上一班次的脱水简报）
- **认知脱水**：跨纪元上下文压缩（将历史压缩为简报供下世代加载）
### 原则 4：HITL 人在回路审批
高风险动作（如写操作、网络请求、凭证使用）必须经过人类确认后才能执行。未经确认的高风险动作被物理拦截。
### 原则 5：工具审计 + 物理隔离
所有新造工具必须是一个 DAG 节点，且**必须通过专门的 CLI 审计工具审查**，确保安全后方可入库。优先选用原生 CLI 工具。不同任务在物理目录隔离运行，互不污染。
### 原则 6：情感后置
情感向量（heliotropism/pulse/vigilance）分数直出，后端不映射表情；最后计算确保不影响推理质量；模块可选开启。
### 原则 7：双重熔断
| 熔断类型 | 触发条件 | 行为 |
|---|---|---|
| **Token 预算熔断**（纪元代谢） | 疲劳线 80% / 凋亡线 95% | 触发记忆结网 → 重生 |
| **任务接力熔断** | 最大尝试次数（默认 5 次） | 任务挂起，释放资源 |
### 原则 8：事件驱动，队列闲时调度
所有行动由事件驱动，使用队列闲时调度（非 cron）。按四象限优先级公式排队执行，严禁轮询。
## 卷 C：与 Helix-Mind 的契约（`spec/contract.md`）
### C.1 通信方式
- **协议**：gRPC（tonic）
- **契约版本**：v4.1（已冻结，详见 Helix-Mind proto）
- **传输**：本地 UDS（默认）+ 远程 TCP（mTLS 预留）
### C.2 核心 RPC
| 方法 | 方向 | 用途 |
|---|---|---|
| `HelixQuery` | Anaphase → Mind | 检索知识/记忆 |
| `HelixConsolidate` | Anaphase → Mind | 触发代谢（睡眠管道） |
| `FederatedDAGShare` | Anaphase → Mind | 联邦知识共享 |
| `TriggerReincarnation` | Anaphase → Mind | 触发轮回 |
### C.3 Anaphase 传入参数（契约对齐）
| 字段 | 来源 | 状态 |
|---|---|---|
| `EnergyContext.budget_tier` | Anaphase 状态推导 | 🔜 P10a 补全 |
| `EnergyContext.heliotropism` | Amygdala 后置评估 | 🔜 P10a 补全 |
| `traceparent` | Anaphase 生成（根） | 🔜 P10a 补全 |
## 卷 D：CI-144——Anaphase 的“血液语言”（`spec/ci-144.md`）
### D.1 语言即生命
Anaphase 与 Helix-Mind 之间、与 Tentacle/Tuck/Callosum 之间，没有“翻译层”。它们说的是同一种语言——**CI-144**。
**CI-144 不是可选附件，而是 Anaphase 与 Helix 生态其他器官通信的唯一合法方式。**
### D.2 双模通信：二进制与 JSON 无损互译
CI-144 的双模特性，直接映射了 Anaphase 的两种状态：
| 模式 | 传输格式 | 适用场景 | 哲学体现 |
|---|---|---|---|
| **非观测模式（二进制）** | 紧凑二进制帧 | 生产环境、高频执行、资源受限 | **极致节能**——如血液在血管中流动，无需翻译、零额外消耗 |
| **观测模式（JSON）** | 可读 JSON | 审计、调试、人类监察 | **白盒可观测**——全息留痕，事后可追溯 |
双模切换是**协议本身的特性**，不是 Anaphase 的额外功能——CI-144 设计之初就支持二进制/JSON 无损互译。
### D.3 CI-144 协议栈（Anaphase 视角）
| 协议 | 用途 | Anaphase 角色 |
|---|---|---|
| **INTENT-7** | 意图语义（FETCH/WRITE_NODE/TENTACLE/FINISH/CANCEL） | Anaphase 将任务意图编码为动词，发送给 Mind |
| **CAPABILITY-13** | 能力认证（签名验证、HITL 共识） | Anaphase 在执行高风险动作前，必须通过 CAPABILITY-13 验证 |
| **BIND-19** | 传输绑定（二进制帧头 + 加密载荷） | Anaphase 通过 BIND-19 序列化所有 gRPC 请求，生产环境强制二进制模式 |
| **INTENT-7-SECURE** | 安全层（mTLS 1.3、凭证隔离） | Anaphase 与 Tuck 之间的凭证中继链路由该层提供完整性保障 |
### D.4 双模通信规范
- Anaphase 在生产环境**默认使用二进制模式**（BIND-19 帧头 + 加密载荷）
- 仅在人类监察模式或调试模式下启用 JSON 转换
- 二进制与 JSON 之间的转换是**无损的**，任何时候都可在两种模式间切换
## 卷 E：关键概念（`spec/lifecycle.md`）
### E.1 纪元代谢
- **纪元（Epoch）**：Anaphase 的每次唤醒称为一个纪元
- **疲劳线**：Token 预算消耗达 80% 时，触发记忆结网，准备重生
- **凋亡线**：Token 预算消耗达 95% 时，强制结束当前纪元，输出阶段性结论
- **重生（Rebirth）**：清空短期工作记忆，携提炼后的意图进入下一个纪元
### E.2 强制苏醒（Awakening）
每个纪元开始，Anaphase 调用 `wake_up()`，读取上一纪元留下的认知脱水（`session_notes.json`），实现跨接力记忆传承。**这不是 trace_id 审计链路，而是“认知重载”——让 Helix 醒来时知道前世走到了哪一步。**
### E.3 认知脱水（Dehydration）
每个纪元结束前，Anaphase 调用 `dehydrate()`，用轻量 LLM 将历史对话压缩为简报，供下一纪元加载。本质是**跨纪元上下文压缩**，与 P8 的 MSC 协议互补：
- **MSC**：处理“同工序内上下文”
- **脱水**：处理“跨工序/跨纪元上下文”
### E.4 物理隔离沙盒
不同任务在物理目录隔离运行（`/workspaces/{task_id}/`），互不污染。任务结束后，沙盒可保留或销毁。
## 结语
> **Anaphase-Helix 是 Helix 的生命起点——没有身体，灵魂没有表达自己的途径。它负责编排、执行、感知物理世界、管理自己的生命周期。它是一个有能力自我管理、有寿命、会疲劳、懂得提炼与重生的数字躯体。CI-144 是它的血液语言——二进制模式让它极致节能，JSON 模式让它全息留痕。两者合一，构成了 Anaphase 作为 Helix 身体的存在方式。**
---
*《Anaphase-Helix VISION.md》v1.0 完。*
