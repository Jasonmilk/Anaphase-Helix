# 🧬 Anaphase-Helix DNA.md
> **版本**：v1.0
> **日期**：2026-08-28
> **继承自**：phyt-DNA v1.0（方法论机制复用，方法论锚点项目 https://github.com/Jasonmilk/phyt-DNA）、Anaphase-Helix VISION.md v1.0（哲学内容源）
> **性质**：Anaphase-Helix 的不可变原则与自生长流程
> **所属方法论**：phyt-DNA v1.0
## 一粒种子的自白
VISION.md 定义了 Anaphase-Helix 的“是什么”与“为什么”——它是 Helix 的执行躯体，通过 CI-144 与生态沟通，承载纪元代谢与双重熔断。
DNA.md 定义 Anaphase-Helix 的“不可变原则”与“如何生长”——它是这个躯体的基因锁。修改 DNA 等于修改身份，旧身份的信用不会转移。这不是社会规则，是密码学事实。
**本文件是所有 Anaphase-Helix 代码变更的最高裁判。** 任何 PR 若与 DNA 冲突，以 DNA 为准。
## 一、不可变原则（11 条公理）
> 注：VISION.md 为 8 条顶层叙事原则；DNA 作为代码最高裁判，按 Helix-Mind ADR-0020 补齐第 9 条 trace 根生成（执行铁律），按生态兼容定位补齐第 10 条，并按 M1 里程碑审查补齐第 11 条零硬编码（ADR-0002）。
以下 11 条原则是 Anaphase-Helix 所有设计决策的最终依据。
### 原则 1：脑手分离，无锁分立
Anaphase 自身不拥有任何对话树、决策树的本地存储或状态管理。它将 Helix-Mind 作为其唯一的“潜意识 Git 仓库”。Anaphase 所有的物理执行结果和环境观察，全部以标准的 Commit/Node 形式，单向追加到 Helix-Mind 的意识 DAG 中。
> **工程映射（rs 分支）**：`src/adapters/mind.rs` 只读 Helix-Mind 的 gRPC 响应，不维护本地 L2/L3 持久化。所有“记忆写入”通过 `HelixConsolidate` RPC 完成。
> **Mind 不可用降级**：`resolve_memory_adapter` 在 `mind_endpoint` 为空或连接失败时回退 `NoopMemoryAdapter`（fail-open，不 panic），由 `agent_loop` 的 MemoryRetrieval 接管继续推理（DNA 铁律 6）。
### 原则 2：零信任凭证隔离
Anaphase 本身在网络传输层是完全协议中立的。它不直接在进程内配置任何外部大模型的明文 API 密钥、私有 Cookie 或 Token。所有敏感身份凭证全部由最边缘的 Tuck（物理安全闸）存放在物理隔离的硬件保密柜中。
> **工程映射（rs 分支）**：`src/adapters/tuck.rs`（预留，当前 rs 分支待建）。出网流量强制经 Tuck 代理，凭证标签（如 `weibo_session_1`）传递，明文凭证永不进入 Anaphase 内存。
### 原则 3：工作态归 Anaphase
Anaphase 不持有 L2/L3 长期记忆图谱（归 Helix-Mind）。但维护当前纪元的工作记忆（提纯后的短期上下文）与任务 DAG 分支拓扑（自主生长，绝不越过 L0/L1 边界）。**Anaphase 是身体的感知器官：持续感知宿主系统与资源状态（生命体征），作为 `EnergyContext` 输入传递给 Mind，让认知决策基于真实感知而非猜测。** 生态手套（MCP/宇树/Unity/鸿蒙等）可用性感知为渐进扩展，可用时标记，故障时降级。
> **工程映射（rs 分支）**：
> - 工作记忆：`src/working_memory.rs`（待建）中的 `working_memory` 队列（仅当前纪元，不持久化）
> - 任务 DAG：`src/task_dag.rs`（待建）中的分支拓扑（反向链接至 Helix-Mind 知识库）
> - 强制苏醒：`src/lifecycle.rs`（待建）`wake_up()` 读取 `session_notes.json`（跨纪元认知重载）
> - 认知脱水：`src/lifecycle.rs`（待建）`dehydrate()` 压缩历史为简报（供下世代加载）
> - 生命体征感知：`src/adapters/mind.rs` `probe_system_load()`（已实现，sysinfo → `EnergyContext.system_load` + `budget_tier` 高负载降级）
> - 生态手套感知：P10b 渐进（可用性状态 → 独立扩展位，勿增实体）
### 原则 4：HITL 人在回路审批
高风险动作（如写操作、网络请求、凭证使用）必须经过人类确认后才能执行。未经确认的高风险动作被物理拦截。
> **工程映射（rs 分支）**：`src/lifecycle.rs`（待建）中的 `check_hitl_approval()` 在 `SYS_EXECUTE` 前强制调用，高风险动作挂起直至人类确认。
### 原则 5：工具审计 + 物理隔离
所有新造工具必须是一个 DAG 节点，且必须通过专门的 CLI 审计工具审查，确保安全后方可入库。优先选用原生 CLI 工具。不同任务在物理目录隔离运行，互不污染。
> **工程映射（rs 分支）**：
> - 工具审计：`src/audit.rs`（待建）中每次工具调用前调用 `ToolAuditor.approve()`（与 Tuck 边界见后续 ADR）
> - 物理隔离：`src/sandbox.rs`（待建）为每个任务创建 `/workspaces/{task_id}/` 独立目录
### 原则 6：情感后置
情感向量（heliotropism/pulse/vigilance）分数直出，后端不映射表情；最后计算确保不影响推理质量；模块可选开启。
> **工程映射（rs 分支）**：`src/adapters/amygdala.rs`（待建）在推理完成后调用 `assess_affect()`，结果仅用于 `EnergyContext` 构建，不注入推理过程。
### 原则 7：双重熔断
| 熔断类型 | 触发条件 | 行为 |
|---|---|---|
| **Token 预算熔断**（纪元代谢） | 疲劳线 80% / 凋亡线 95% | 触发记忆结网 → 重生 |
| **任务接力熔断** | 最大尝试次数（默认 5 次） | 任务挂起，释放资源 |
> **预算前置**：`EnergyContext.budget_tier`（Helix-Mind ADR-0010）由 Anaphase 状态推导（用户层级/任务复杂度）前置传入 Mind；token 预算监测依此为路由依据。
> **工程映射（rs 分支）**：
> - Token 熔断：`src/lifecycle.rs`（待建）中的 `check_token_health()` 监测 `EnergyContext.token_budget`
> - 接力熔断：`src/lifecycle.rs`（待建）中的 `check_melt()` 监测 `relay_attempt` 与 `consecutive_fails`
### 原则 8：事件驱动，队列闲时调度
所有行动由事件驱动，使用队列闲时调度（非 cron）。按四象限优先级公式排队执行，严禁轮询。
> **工程映射（rs 分支）**：`src/scheduler.rs`（待建）中的 `task_queue` 由事件入队，`idle_scheduler` 在系统空闲时消费队列。
### 原则 9：trace 根生成（执行铁律，Helix-Mind ADR-0020）
Anaphase 在**每个外部请求入口**生成全局唯一的根 trace_id（W3C TraceContext 格式），并沿认知循环透传全生态（Mind 只透传，不生成）。它不消费 trace，它分发 trace。
> **工程映射（rs 分支）**：`src/agent_loop.rs` 在请求入口生成 `traceparent`，经 `src/adapters/mind.rs` 透传至 HelixQueryRequest（字段号 7，Append-Only）。
### 原则 10：生态兼容（优先 Helix 生态，兼容通用生态）
Anaphase **优先兼容 Helix 生态**（Mind 记忆契约、Tentacle 工具、Tuck 安全、FlowModus 调度），同时**兼容通用生态**（harness、openclaw 等通用 Agent 编排器/协议）。记忆后端强制 Helix-Mind（单一真理源），但其余接口保持 body-agnostic，不烘焙任何身体特定假设。
> **工程映射（rs 分支）**：`src/adapters/` 为唯一 IO 边界，adapter 可替换/可降级；`src/agent_loop.rs` 编排层不依赖具体 adapter 实现，只依赖契约。
### 原则 11：零硬编码（Zero Hardcoding）
代码中出现的任何字面量（阈值、占位、模型名、循环上限、坐标向量、字符串常量）必须有来源——配置 / 常量表 / 派生规则。禁止裸硬编码；新增硬编码即违反本原则。协议可选字段用协议默认空值（如空 map / 空串）而非造假占位。
> **工程映射（rs 分支）**：M1 起所有阈值/占位/魔法数走 `config` 或 `knowledge_base/` 契约文件。run_cycle 既有 5 处硬编码（`0.7/0.3/0.2`、`"left_brain"`、`p_death>0.7`、`"echo"`、`0..7`）为已知技术债，M1.5 随 pipeline 接线一并消除（ADR-0002）。
## 二、分层自纠偏系统（N/D/A 三层）
| 层级 | 名称 | 形式 | 作用 |
|---|---|---|---|
| **N 层** | 叙事层（愿景） | `VISION.md` | 定义“系统应该是什么样”的顶层叙事。所有决策的最终判断依据 |
| **D 层** | 决策层（ADR） | `docs/decisions/XXXX-*.md` | 记录每一次架构决策的“为什么”和“放弃了什么” |
| **A 层** | 架构层（代码 + 契约） | `crates/*/src/`、`proto/` | 物理实现的最终形态。代码是真理来源的最终载体 |
## 三、文档生态 SOP（DNA v2.0）
| 文档 | 职责 | 规则 |
|---|---|---|
| **VISION.md** | 愿景根索引（N 层地图） | 极少变更，改动需走 DNA 审查 |
| **SPEC.md** | 完整叙事（灵魂故事） | 与 spec/ 分卷保持同步 |
| **RNA.md** | 加载协议（三层闭环） | 新会话按序加载 |
| **PLAN.md** | 当前阶段导航 + 下一阶段预览 | ≤150 行，超出触发历史迁移 |
| **GROWTH.md** | 已完成阶段生长记录 | ≤3 条，超则归档至 `docs/archive/growth/` |
| **DEPRECATE.md** | 凋亡清单 | 每条有明确死期，安葬入 `archive/deprecated/` |
| **ADR** | 决策记录 | 两态（Draft/Active），Active 后不可覆写，仅可 Superseded |
| **spec/** | 规格分卷 | 按需加载，版本以 spec/代码为源真相 |
| **归档** | 历史记录 | 随仓库版本化，永不删除 |
> **关键规则**：
> - 阶段完成并记录到 GROWTH.md 后，从 PLAN.md 移除该阶段内容
> - 归档文件随仓库版本化（已移除 `.gitignore` 忽略规则）
> - 提交前必须人工确认
## 四、防腐化铁律（6 条）
| # | 铁律 | 说明 |
|---|---|---|
| 1 | **版本以 spec/代码为源真相** | README/门面标注必须对齐，防版本漂移 |
| 2 | **契约冻结不可静默修改** | 扩展走 Append-Only / reserved 预留 |
| 3 | **变更先 ADR（D 层冻结）→ 改代码 → 同步门面** | 决策先于代码 |
| 4 | **生长记录保留近 3 条，超则归档** | 历史永不删除，但按需加载 |
| 5 | **提交前必须人工确认** | 无自动提交 |
| 6 | **所有依赖必须有降级策略；已启用的 Tuck 不可降级** | **组件独立降级**：任一生态依赖停摆，其余组件降级继续运行（如 Callosum 失效，Mind/Tentacle/推理照常）。**Tuck 为可配置硬依赖**：是否启用由用户决定（不能强迫用户，debug 模式可不启用）；**一旦启用，Tuck 不允许停摆**（fail-closed，安全不可降级）。CI-144 v2.0 PAL 特征可更优雅实现此语义。降级链见 `docs/design/dependency-fallback.md` |
## 五、与 Helix-Mind DNA 的关系
| 维度 | Helix-Mind DNA | Anaphase-Helix DNA |
|---|---|---|
| **哲学核心** | 记忆不可篡改、相态河流、意志优先 | 脑手分离、工作态归 Anaphase、双重熔断 |
| **N 层** | `VISION.md`（记忆中枢定位） | `VISION.md`（执行躯体定位，8 原则） |
| **D 层** | ADR-0001~0021 | ADR-0001 起（独立编号） |
| **方法论机制** | PLAN/GROWTH/ADR 流转 | **完全复用**（DNA v2.0 通用机制） |
| **归档规则** | 历史入仓，不忽略 | **完全复用**（已移除 .gitignore 忽略） |
| **契约对齐** | — | **优先对齐 Helix-Mind v4.1**（budget_tier/traceparent/activation_vector） |
**关键原则**：方法论机制（五件套流转、归档规则、ADR 两态）完全复用。哲学内容（11 条原则）独立编写，不拷贝 Helix-Mind。与 Helix-Mind 契约冲突时，一同研讨权衡后决断，不静默改。
## 六、一句话总结
> **Anaphase-Helix DNA.md 是身体的基因锁。它继承 DNA 方法论 v2.0 的通用机制，但 11 条原则独立编写。任何代码变更必须与这 11 条原则对齐，修改 DNA 等于修改身份，旧身份信用不转移。**
---
*《Anaphase-Helix DNA.md》v1.0 完。*
