# ADR-0023: 按需认知注入——记忆折叠进推理请求（O-5）

- **状态**: Accepted
- **日期**: 2026-09-06
- **决策范围**: Anaphase（run_cycle Reasoning 组装 / config / main 输入来源）
- **关联**: ADR-0002（DNA 原则 11）、ADR-0016（O-1 结构化分诊）、ADR-0018（Rails 注入）、ADR-0022（认知工艺触发链）、ADR-0005（候选 E 结构化输出）

## 1. 背景与问题

O-5（原 O-3 定义）"按需加载落点：请求只带本轮所需"。物理探查（物理事实优先）
发现三个事实：

1. **记忆检索断裂**：`MemoryRetrieval` 阶段 `memory.query` 的结果存进
   `context.memory_nodes`，但 **Reasoning 阶段从未把它注入 LLM**——LLM 每轮只见
   原始 user_input。伙伴模式"带着记忆工作"（干活前检索自评）在 LLM 侧是断的：
   检索了，但白检索。
2. **对话窗口数据源缺失**：Anaphase 无多轮对话入口（HTTP 仅 snapshot/events 两
   端点），主循环是单任务循环（同一输入多轮状态机推进）。"窗口 L0 按轮取"在
   当前架构下**没有历史数据源**——诚实标注为待 UI 会话层接入，不在本 ADR 伪造。
3. **0 硬编码违规**：main.rs 硬编码演示输入 `"Calculate 2 to the power of 10"`
   （无来源）。

## 2. 决策

### D1: 按需认知注入——记忆折叠进推理请求
Reasoning 阶段组装 prompt = `user_input + [memory] 段`，记忆节点经
`fold_memory_nodes` 确定性折叠：
- 节点以分隔符拼接；
- 超过 `memory_inject_chars` 预算 → 截断到预算 + 显式折叠标记
  （`...[folded: more memory available on demand]`）；
- 预算 0 → 不注入（纯无状态，legacy 行为完全向后兼容）；
- structured（`!command`）与 rails 命中路径**不受影响**（0 tokens，不注入）。

"请求只带本轮所需"的物理语义：注入量 = min(记忆总量, 预算)——与轮数无关，
25 轮上下文近零增长（验收判据）。

### D2: 预算单一来源——config
`AnaphaseConfig.memory_inject_chars`（`[anaphase]` 段，serde default）协议默认
800（注释声明，ADR-0023）；main 装配进 `AgentLoop.memory_inject_chars`；
AgentLoop::new 内 const 协议默认（被 main 覆盖）。0 硬编码：run_cycle/main 无
字面量（除注释声明的协议默认）。

### D3: 演示输入来源化
`"Calculate 2 to the power of 10"` 从 main.rs 调用点移除：
- CLI `--input "<text>"` 最高优先；
- config `smoke_input`（`[anaphase]` 段）次之；
- `DEFAULT_SMOKE_INPUT` const（注释声明：协议默认演示任务）兜底。

### D4: 窗口 L0 诚实标注（不伪造数据源）
"窗口 L0 按轮取 + 磁石点开才读"的**多轮窗口部分**依赖对话历史数据源，Anaphase
当前无多轮对话入口——本 ADR 只落地"注入预算"（近零增长的物理机制），窗口切
片标注为待 UI 会话层（Cellrix/Tentacle 对话入口）接入后扩展。如无必要勿增
实体：不为不存在的数据源造字段。

## 3. 验收结果（物理验证）

| 判据 | 结果 |
|---|---|
| 记忆注入 LLM（断裂修复） | ✅ `memory_nodes_are_injected_into_reasoning_prompt`（prompt 含 `[memory]` + 节点原文） |
| 预算折叠 | ✅ `fold_memory_nodes_*` 三例（空/预算内/超预算截断+标记） |
| 预算 0 = 纯无状态 | ✅ `zero_budget_keeps_stateless_prompt`（无 `[memory]` 段） |
| 25 轮近零增长 | ✅ `twenty_five_rounds_context_stays_bounded`（25 轮注入段 ≤ 预算恒定） |
| 0 硬编码 | ✅ main.rs 调用点无演示字面量（grep 核对：仅注释声明的协议默认） |
| 冒烟 | ✅ `--input "hello test"` 生效（User: hello test） |
| 全量 | ✅ 189 passed / 0 failed（183 + 6） |

## 4. 备选方案与拒绝理由

| 备选 | 拒绝理由 |
|---|---|
| 记忆全量注入（不折叠） | 违背按需/节能——上下文随轮数增长 |
| 新建 WindowConfig 结构 | 勿增实体——单字段平铺进 AnaphaseConfig 足够 |
| 伪造窗口 L0（造历史缓冲） | 物理事实优先——无对话入口，数据源不存在；标注待接入 |
| 改 HttpReasoningAdapter 拼历史 | 解耦——组装在 run_cycle（状态机知情），adapter 保持无状态 |

## 5. 后果

**正面**：
- 伙伴模式记忆真正到达 LLM（检索不再白做）——认知工艺"干活前检索自评"落地；
- 注入量预算封顶，25 轮上下文近零增长（Memory-Efficient）；
- 演示输入可配置（`--input` / config），0 硬编码在输入源收口。

**负面/代价**：
- 每轮 LLM 请求多一段 ≤800 字符的记忆（有界成本，换取记忆参与决策）；
- 窗口 L0 尚未落地（无数据源，诚实标注）。

**风险与对策**：
- 折叠截断可能丢关键记忆 → 折叠标记显式声明"more memory available on
  demand"（磁石点开才读语义）；Mind 侧真实检索裁剪（top_k）后续在 P10a 接线时
  对齐。
