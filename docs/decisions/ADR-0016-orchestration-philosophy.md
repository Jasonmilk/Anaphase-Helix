# ADR-0016: 编排哲学——确定性优先分诊 + 认知工艺挂点 + 按需感知
- **状态**: Proposed
- **日期**: 2026-09-06
- **决策范围**: Anaphase run_cycle（六 stage 流水线）的编排策略
- **关联**: ADR-0003（六 stage 流水线）、ADR-0006（会话即经历）、ADR-0010（驾驶舱快照投影）、Helix-Mind ADR-0021（认知工艺）、DNA 原子原则 7（双重熔断）/ 8（事件驱动）/ 9（trace 根）

## 1. 背景与依据

候选 G 完成后，编排策略需要显式化，防止知识腐烂。2026-09-06 研讨结论（基于 Anthropic《Building Effective Agents》、DSH 内核源码比对、Claude Code 架构深度分析）：

1. **全世界最强 agent 骨架一致**：简单循环 + 周围系统（Claude Code 官方论文原话："core is a simple while-loop… most code lives in the systems around this loop"）。Anaphase 六 stage 流水线即此形态，**骨架不动**。
2. **传统循环每个 stage 都调 LLM**（连"结果好不好"都让 LLM 评）；Helix 的差异化是**先问要不要 LLM**。
3. **tokens 是稀缺资源**：0 tokens 优先 > 少 tokens/小 LLM > 多 tokens/大 LLM。
4. **编排的确定性 = 对自身状态完整感知后的决策**（"看口袋过日子"：像出门坐车前看表，不是时刻看表）。
5. **DSH 短板实证**：无跨会话记忆（会话隔离容器）、压缩靠 LLM 摘要（不可复现）、无 turn budget（官方承认）、无元认知。Helix 在记忆连续性/确定性压缩/元认知三处压倒；四个待补短板：事件词汇表、surface 投影落地、turn/step 分层、并行调度（后两项有依赖边界，见 D4）。

## 2. 决策

### D1: 确定性优先分诊（0 tokens 优先）

六 stage 中只有两处必须 LLM：**理解没说清的话**（自由文本）、**生成要说的话**（表达）。其余 stage 全部先走 0 tokens 确定性通道：

| stage | 0 tokens 通道 | 升级 LLM 条件 |
|---|---|---|
| 想（parse） | 结构化输入直接解析（磁石/命令/状态/协议） | 自由文本理解 |
| 装（assemble） | 确定性拼装 tt_job | 永不（结构性错误才升级诊断） |
| 动（execute） | 工具/脚本/CLI/Tentacle-MCP（复用通道，不造轮子） | 无可用工具且需生成性动作 |
| 记（evidence） | append-only 落盘 | 永不 |
| 量（criteria） | 六把尺子（纯函数） | 永不（判据不可表达时才升级） |
| 记账（ledger） | 确定性 JSONL 写入 | 永不 |

### D2: 认知工艺触发点（不实现——四拍与五工序全归 Mind）

**边界修正（2026-09-06 核对 Helix-Mind ADR-0021/0022 后）**：认知工艺是 Mind 的器官（ADR-0021：Mind=编排建议，CognitiveService=执行；ADR-0022：四拍是伙伴模式元认知）。**Anaphase 不实现任何工序，只负责在正确时机触发，并把 Mind 的建议（effective_mode / impasse / suggested_actions）用于执行编排**。既有 `src/adapters/mind.rs` 契约即触发通道。

| Mind 侧（权威定义） | 内容 | Anaphase 职责 |
|---|---|---|
| **四拍**（ADR-0022 伙伴模式元认知） | 干活前检索自评 / 干活中风格对齐 / 交作业预期校准 / 收反馈差距评估 | 触发 helixQuery 让 Mind 走四拍，消费其输出，不自行实现 |
| **五工序**（ADR-0021 思考工序） | 结构性 / **批判性** / 创造性 / 情境意图解析 / 元批判 | 同上——**"批判性思维判断"是 Mind 批判工序，不是 Anaphase 工序** |

### D3: 按需感知 = 设置 budget_tier（对齐 ADR-0010，非新实体）

**边界修正**：ADR-0010 已规定**身体（Anaphase/Callosum）决定 `budget_tier`**，随 `HelixQueryRequest.energy_context.budget_tier` 传入，Mind 只执行（按 tier 选扫描范围）。"看口袋过日子"的物理落点 = Anaphase 正确设置 tier，不是新机制。

- **感知点**：任务开始前一次、关键决策点（升级 LLM/触发 Mind 前）一次。
- **感知内容**：tokens 预算（EnergyContext）、系统内存、生态项目点亮状态（Cellrix/Tuck/手套可用性）。
- **tier 映射**：`AUGMENTABLE`（常规）/ `ENDOGENOUS`（0-token 紧急，仅晶体+高相关胶体）/ `EXOGENOUS_REQUIRED`（探索）/ `VOID`（无认知）。
- **原则**：不持续轮询、不随时读取——避免分散注意力、干扰行动（物理类比：出门坐车前看表）。驾驶舱（候选 G）向用户展示状态即可，Helix 自身按需读取。

### D2.5: 三层递进边界（0 tokens 逐层，极致解耦）

```
Anaphase 执行层分诊（D1：要不要调 LLM/工具）      ← 身体自主
    ↓ 触发 helixQuery（带 budget_tier）
Mind System 0 门控（ADR-0021：要不要认知工艺）    ← 0 Token 纯逻辑
    ↓ 命中则
Mind 五工序编排（ADR-0021：怎么思考，含批判工序）  ← CognitiveService 执行
```

三层各自决策、互不侵入：Anaphase 不猜认知深度（System 0 的活），Mind 不执行物理动作（README §1.1：零物理动作、只输出建议）。

### D4: 依赖边界（FlowModus / Callosum）

| 能力 | 归属 | 状态 |
|---|---|---|
| 并行工具调度（并行池 + 独占屏障） | FlowModus（管理 API 与 tokens） | ⏳ 等待 FlowModus，完成前串行可工作 |
| 上下文窗口感知 | FlowModus | ⏳ 等待；当前"给用户看就够了" |
| 请求前缀稳定 / KV 缓存复用 | Helix-Callosum（上下文压缩、KV 缓存复用） | 能做就做，不做不勉强；编排层只保证**同输入同输出**（已满足） |

### D5: 轨迹三层（白盒可查，DNA 原则 9 延伸）

- **ledger**（做了什么，JSONL 全量，append-only）
- **evidence**（为什么这么做，证据落盘）
- **会话 DAG**（经历了什么，mddag 磁石结构）
- **stage 事件总线**（六 stage 边界发确定性事件，Cellrix/Tuck 订阅——对应 Claude Code hooks / DSH 事件词汇表；此为轨迹可视化的机制地基）
- **trace_id 贯穿**：stage 事件携带 W3C traceparent（DNA 原则 9 + ADR-0021 §4：认知工艺共享同一 trace_id，全息留痕、跨器官可追溯）
- 轨迹比 DSH 多一层"经历"维度；DSH 只有事件流回放。

## 3. 备选方案与拒绝理由

| 备选 | 拒绝理由 |
|---|---|
| 每个 stage 都调 LLM（传统 ReAct） | 费 tokens、不可复现、与 0 tokens 优先哲学冲突 |
| 时刻感知预算（持续读取状态） | 分散注意力、干扰行动；违背按需驱动 |
| 并行池提前实现 | FlowModus 未就绪，API/tokens 管理职责不清；违背如无必要勿增实体 |
| 认知工艺作为第二循环 | 违背极致解耦与按需驱动；四拍是挂点不是并行器官 |

## 4. 后果

**正面**：
- 编排确定性最大化（同输入同输出，预算可见的最优分配）
- tokens 开销按预算分配（0 tokens 默认，高价值任务可升级）
- 轨迹三层 + stage 事件 = 用户完全了解 AI 看到什么（白盒）
- 认知工艺触发点清晰（helixQuery 契约），伙伴模式复利（L3→L2→L1）有确定载体

**负面/代价**：
- 自由文本理解仍是 LLM 依赖（不可消除，但可按需感知控制成本）
- FlowModus 完成前并行调度缺位（串行可工作，不阻塞）
- stage 事件总线是新增机制，需与 ledger 职责分清（事件 = 过程，ledger = 事实）

**风险与对策**：
- 0 tokens 通道误判（把需要理解的问题当结构化处理）→ 对策：升级 LLM 前必须过"按需感知"检查（口袋/资源/任务性质）
- 事件总线膨胀 → 对策：事件只发 stage 边界与判定结果，不发内部细节；消费方按需订阅

## 5. 一句话总结

> 心跳是骨架，四拍五工序是 Mind 的灵魂，六哲学是土壤；
> 先看口袋，再走通道，0 tokens 优先，尺子说话。
