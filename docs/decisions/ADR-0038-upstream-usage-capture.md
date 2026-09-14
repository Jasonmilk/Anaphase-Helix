# ADR-0038: 上游计量捕获——按次落盘，按需派生

- **状态**: Accepted
- **日期**: 2026-09-14
- **决策范围**: anaphase-helix（推理适配器 + 会话事件流）／ Cellrix（证轨数据层 + 证轨状态栏）
- **关联**: ADR-0006（确定性回放）、ADR-0026（会话事件流）、ADR-0029（证轨链条完整性）、ADR-0034（空回复预算重试）、ADR-0036（物理模型捕获）
- **取代**: 无（在**实现层**收敛 ADR-0036 的 `last_model()`；其**契约不变**，故 ADR-0036 保持 Accepted，不被覆写）

## 1. 背景与问题

证轨状态栏（Cellrix `prove_track`）有八个统计格，其中三个恒为 `—`：

| 格 | 状态 |
|---|---|
| `TOKENS` | 硬编码占位 |
| `缓存命中` | 硬编码占位 |
| `TOK/S` | 硬编码占位 |

按 DNA **原则 11**「协议可选字段用协议默认空值（如空 map / 空串）而非造假占位」，
**占着位置却不表达任何事实的槽位本身就是造假占位**。填真值不是新增功能，是消除既有违规。

根因在数据源：anaphase 完全不解析上游响应里的计量字段。

### 三个实测陷阱（决定了本决策的形状，非推断）

1. **`prompt_tokens` 含缓存命中**——上游语义为
   `prompt_tokens = cache_hit + cache_miss`。若直接展示，同一批 token 会被计入两个桶。
2. **流式 usage 只在终 chunk**——实测一轮 41 个 chunk，usage 出现在**第 41 个**（41/41），
   前 40 个均无 `usage` 键。逐 chunk 无脑覆写会把终值抹成空。
3. **一个周期有 3 处上游调用**——`run_cycle.rs` 的重试循环含两条路径
   （流式 / 缓冲，可重试 N 次），工具跑完后另有一次 `finalize` 收口调用。
   而 `assistant/think` 与 `assistant/attempt` 都在重试循环**之外**、每周期只发一次，
   且重试的中间输出被直接丢弃。**若只把计量挂在交付物上，重试与收口的成本会静默消失。**

## 2. 决策

### D1: 捕获在写入端，聚合在读取端

上游 usage 是**瞬时事实**——响应体不落盘，推理 trace 只存 prompt/output，
**不即时捕获即永久丢失**。而聚合是**可重算的派生**。

故二者分离：**写入端只落原始事实，聚合由读取端纯函数按需派生。**

这不是"预计算"（按需加载所禁止的），而是"抢救瞬时事实"；真正按需的是聚合。

### D2: 新增 `assistant/usage` 事件（Append-Only）

每次上游调用落一条，携带该次调用的路由事实与计量事实。

**不改 `assistant/attempt` 的既有语义**（每周期一次）。把它移进循环使其"每次尝试一次"
是**行为变更**，违反铁律 2「契约冻结不可静默修改」；新增事件类型才是 append-only 扩展。

副作用收益：该事件**不进 Cellrix 的 `TYPES` 映射**，`buildSession` 的
`if (!map) return;` 自动跳过它——**不污染轨迹表，零额外渲染成本**。

### D3: 不相交计数

内部一律采用**不相交**计数：

```
输入 = prompt − cached
缓存读 = cached
输出 = completion
```

三者可加且互不重叠，且满足自校验不变式 `输入 + 缓存读 = prompt`。

**`cached` 缺失时，`输入` 也不给。** 因为无法判定上游是否采用了含缓存的计数口径，
此时给出 `prompt − 0` 就是**猜数**。

### D4: 可选桶「全有或全无」

可选桶（`cached` / `reasoning`）只有在**参与聚合的每一次调用都报告了该桶**时，
才给出聚合值；否则**整桶省略**。

明细恒完整保留在事件流中——省略的是**不完整的和**，不是事实。
（若 3 次调用只有 2 次报了缓存，其"总和"是残缺的，给出即误导。）

### D5: 不存储 `total_tokens`

OpenAI 形状下 `total ≡ prompt + completion`。存储它即**一个事实两个来源**，
且当上游给出矛盾值时会产生无法裁决的冲突。

派生 + 安全整数守卫；溢出或不可信即**省略**（不给近似值）。

### D6: `null` 与缺失同义

部分 OpenAI 兼容网关会以 `null` 填充未提供的字段。

**`null` 表示"无更新"，永不表示"清零"。** 捕获逻辑必须把"键不存在"与"值为 `null`"
视为同一语义，均不覆写已捕获的值。

### D7: 两个拼写回退

缓存读 token 有两个拼写，均需识别：

| 优先 | `usage.prompt_tokens_details.cached_tokens`（OpenAI 兼容拼写） |
|---|---|
| 回退 | `usage.prompt_cache_hit_tokens`（原生拼写） |

不因拼写差异丢事实。推理 token 同理取 `completion_tokens_details.reasoning_tokens`。

### D8: 适配器元数据合并（收敛 ADR-0036 的实现）

`model` 与 usage 取自**同一次上游响应**、在**同 3 个位置**捕获、在**同一处**消费。
分设两套字段 + 两个捕获函数 + 两个 getter 会使调用点从 3 个翻倍到 6 个。

故合并为单一：

```rust
pub struct UpstreamMeta { pub model: Option<String>, pub usage: Option<UsageSnapshot> }
fn last_meta(&self) -> UpstreamMeta          // 默认空；唯一覆写点
fn last_model(&self) -> Option<String>       // 派生默认方法：self.last_meta().model
```

**`last_model()` 的契约逐字保留**（仍返回上游响应的 `model` 字段，ADR-0036 D6 不变），
仅从"覆写点"降为"派生视图"。故 ADR-0036 不被取代、不被覆写。

### D9: 定位——display-only，不进判据

`assistant/usage` 与 `assistant/think` 同定位：**只作披露，永不参与判据**
（复用 `EventType::Think` 的既有表述，不新造措辞）。

**边界声明**：原则 7 的 token 预算熔断走 `EnergyContext.token_budget`，
与本事件**不是同一数据通路**。本 ADR 显式划界，防止未来接线时把显示数据当决策数据。

### D10: 粒度声明（防双源误判）

`model` 会同时出现于两处，**粒度不同、不是冗余**：

| 位置 | 语义 | 粒度 |
|---|---|---|
| `assistant/reply.model`（ADR-0036 冻结） | 产出交付物的那次调用的路由 | **聚合**（归属） |
| `assistant/usage[].model` | 第 i 次调用的路由事实 | **明细** |

N=1 时两者相等；N>1 时不等。不携带明细则"重试时上游切换"这一物理事实**永久丢失**，
而静默丢失物理事实正是本 ADR 否决"只挂交付物"方案的理由。

### D11: 本次不做

- **不引入任何估算或启发式**——无 usage 即 `—`，不用字符数折算 token
- **不做计费口径**——本事件是审计与披露，不是账单
- **不改 `list_periods`**——其 `count` 如实包含 usage 事件（它就是事件流的一行），不做特殊化
- **不扩 `PeriodSummary`**——列表级摘要不需要详情级指标（按需加载）
- **不解耦 400 行红线既有违规**——`run_cycle.rs`、`session_events.rs` 均已超 DNA 红线，
  属**既有技术债**；本轮只加入必要的最小代码，**不扩大**，另行开轮

### D12: 计量归属单次往返（读侧）

D6 只规定了**写侧**（`null`/缺失 = 无更新）。**读侧**另有独立风险，实测暴露：

适配器实例跨调用复用，`last_meta()` 的值在调用之间**不会自动消失**。若上游本次
**未报** usage，写侧的"不覆写"会让**上一次**的数字留在字段里 → `emit_usage()`
把它当作本次的账读走 → **同一批 token 被计入两次**，周期总和静默翻倍。
这是"造假事实"，不是"四舍五入"。

**规则：`usage` 描述且仅描述"刚结束的那一次往返"。适配器必须在每次往返开始时丢弃
上一次的计量**（`begin_round_trip()`，在 `reason` / `reason_stream` 两个入口的
`post_chat` 之前调用）。

`model` **刻意不清**：它描述的是**路由**（一条线路），不是**这一次调用**；
且 ADR-0036 在重试循环之外还要读它。

> **实测证据**（先证伪、再修）：`a_call_without_usage_does_not_replay_the_previous_call`
> ——修复前断言失败，输出为
> `left: Some(UsageSnapshot { prompt_tokens: 10, completion_tokens: 2, .. }) / right: None`；
> 修复后通过。

## 3. 备选方案与拒绝理由

| 备选 | 拒绝理由 |
|---|---|
| 在网关层（Tuck）捕获 usage | Tuck 是**可配置**的（铁律 6：是否启用由用户决定，debug 模式可不启用），anaphase 亦可直连上游。放网关会让"无网关"场景下 usage **永久缺失**，违反物理事实优先。捕获必须在**离响应最近且必然经过**处 |
| 在 `run_cycle` 内累加后只写总和 | 丢失每次调用的明细；且把聚合逻辑混入写入端，违反按需驱动与极致解耦 |
| 把 `assistant/attempt` 移进重试循环 | 行为变更，违反铁律 2；且 Cellrix 侧渲染行为随之改变，改动面失控 |
| 把 usage 挂在 `assistant/reply` 上 | 重试与 finalize 的成本静默丢失（陷阱 3） |
| 用 `unwrap_or(0)` 处理可选桶 | 把"缺失"与"真是 0"抹平，且给出残缺的和 |
| 直接存储并展示 `total_tokens` | 一个事实两个来源；上游矛盾时无法裁决 |
| 逐 chunk 覆写 usage | 中间 40 个 chunk 会把终值抹成空（陷阱 2） |
| 用 `Instant::now()` 测周期耗时 | 不可注入的第二时钟，破坏 ADR-0006 确定性回放；且事件时间戳已可派生耗时 |
| 在状态栏引入字符数折算估算 | 违反物理事实优先；没有就是没有 |
| 让 `last_meta()` 改为一次性消费（`take_usage()`） | 修的不是真缺陷：缺陷是"值**陈旧**"（跨调用残留），不是"被读两次"。消费式 API 会让合法的二次读取（如 trace）也拿不到值，并把契约面从 1 个 accessor 扩成 2 个 |
| 读侧比对"与上次发射值是否相同"来去重 | 连续两次**真实相同**的调用（同 prompt 重放）会被误判为重复，丢掉一次真账 |
| 在适配器里给 usage 加"代次"计数，读侧比对代次 | 为同一个缺陷引入第二套状态与第二处判据；`begin_round_trip()` 一处即可，符合极致解耦 |

## 4. 测试

假网关（零 token、零外部依赖）：

1. 流式终 chunk 带 usage → 捕获值正确（含 `cached` / `reasoning` 嵌套字段）
2. 流式全程无 usage → `last_meta().usage` 为 `None`（诚实留空）
3. **顺序陷阱**：中间 chunk 无 usage + 终 chunk 有 → 终值**不被抹掉**
4. `usage` 键值为 `null` → 视为无更新，不覆写已捕获值
5. `cached` 两个拼写各自命中
6. 非流式路径捕获
7. 非 SSE 回退路径（网关忽略 `stream=true`）捕获
8. **读侧归属（D12）**：同一适配器连续两次往返，第 1 次报 usage、第 2 次不报 →
   第 2 次读到 `None`（**不重放**上一次的数字）。**此条为实测补丁**：修复前该测试失败

词表稳定性：`EventType::Usage.as_str()` 断言为 `"assistant/usage"`。

Cellrix 侧：`derivePeriodUsage` 为纯函数，以事件数组为唯一输入，
断言不相交分解、全有或全无、缺失即省略三条语义。

零回归（HEAD 现算，不引用历史摘要）：测试属性 **254 → 266**（净增 12），逐文件核对无丢失；
`cargo test --no-fail-fast` = **257 passed / 0 failed / 9 ignored**。

> 已知既有 flaky（非本轮引入）：`session_events::query_tests::lists_periods_newest_first`
> 使用**固定**临时目录 `anaphase-session-events-test-3`，而该目录被两个测试共用，
> 并行执行下竞态。HEAD 即已如此，与本 ADR 无关，另行开轮修。

## 5. 验证

- **live 端到端**（Tuck 网关 → anaphase `--stdio` → 真实上游 `X-Route-Tier: free`）：
  实测事件流 `{turn/start, user/message, context/inject, assistant/usage, assistant/think,
  assistant/attempt, assistant/reply, turn/end}` 各 1 条；`assistant/usage` 为真实数字
  `{prompt_tokens:664, cached_tokens:256, completion_tokens:170, reasoning_tokens:41,
  model:"agnes-2.5-flash"}`，不相交输入 408、total 834；交付物为真实回答。
  **D12 修复后重跑仍 PASS**，usage 条数 = 往返次数（1:1）
- **确定性**：同一事件流多次派生结果一致（纯函数）
- **降级**：上游不给 usage 时，事件流不出现该字段，状态栏显示 `—`，链路继续运行

## 6. 后果与已知限制

### 后果

- 证轨状态栏三个占位格获得真实数据源，既有违规消除
- 每次上游调用的成本完整可审计；重试与收口的开销不再静默丢失
- 计量口径（不相交计数）单一化，消除"同一批 token 计两遍"的误读

### 已知限制

- **`list_periods` 的 `count` 会因 usage 事件而增大**。这是如实计数（它就是事件流的一行），
  但会稀释"事件数"作为轨迹密度的直观性。若未来需要区分，另开一轮加字段，不在本轮特殊化。
- **usage 不进熔断**。原则 7 的 token 预算熔断仍走 `EnergyContext.token_budget`；
  两条数据通路尚未打通（D9 显式划界）。
- **400 行红线违规未消除**。`run_cycle.rs` / `session_events.rs` 仍超红线，
  本轮只保证**不扩大**。
- **不做估算**。上游不给 usage 的部署，状态栏恒为 `—`——这是刻意的诚实，
  不是缺陷。
