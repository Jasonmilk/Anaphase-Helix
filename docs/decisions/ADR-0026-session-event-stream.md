# ADR-0026：会话事件流——经历的确定性落盘

- **状态**: Accepted
- **日期**: 2026-09-07
- **决策范围**: Anaphase（事件流写入）/ Cellrix（ProveTrack turn 时间线渲染）
- **关联**: ADR-0023（会话即经历）、ADR-0019（stage events）、ADR-0003（ledger/trace_id）

## 1. 背景与问题

ADR-0023 确立"会话是 Helix 的经历"。经历需要一个**结构化的、确定性的、可回放**的本体落盘，而不只是网关调用链（Tuck 审计）与推理正文（reasoning trace）两处元数据/正文的割裂视图：

- Tuck 审计链只有 request/response（网关视角，无认知语义）
- reasoning trace 只有 prompt/response（正文，无轮次/工具/判据结构）
- 两处以 `trace_id` 为 join key，但缺**事件词汇**（turn 起止、工具调用对、判据结果）

DSH 的轨迹（deepseek-harness `packages/session/session-format`）给出参考形态：会话 = header + 事件流，事件 `{type, seq, time, data}`，类型为命名空间词汇（user/message、tool/call、turn/start……）。其事件词汇可借结构，**不借命名**——Helix 有自己的词汇表与哲学。

## 2. 决策

### D1: 每认知周期一个事件流文件

Anaphase run_cycle 每轮（一个认知周期）向 `session_events_path` 目录写一个 JSONL：

```
{dir}/{job_id}.events.jsonl
```

`job_id` 是 `derive_job_id(user_input)`（`run-<12hex>`）——与 reasoning trace、Tuck 审计链**同一 join key**（ProveTrack 三源一键）。

### D2: 事件词汇表（协议值，消费者精确匹配）

| type | data 摘要 | 语义 |
|---|---|---|
| `turn/start` | `{}` | 认知周期开始（进入状态机） |
| `user/message` | `{text}` | 人类原始输入（写前脱敏） |
| `context/inject` | `{nodes, chars, resume_from}` | 记忆/认知注入摘要（正文在 trace）；`resume_from` = 续接父 job_id（机器可读，ProveTrack 线程化）或旧格式摘要文本 |
| `assistant/think` | `{text}` | 私有推理（脱敏，仅展示） |
| `assistant/attempt` | `{text}` | Reasoning 输出（脱敏） |
| `tool/call` | `{tool, index, expect}` | 确定性工具调用 |
| `tool/result` | `{tool, ok, duration_ms, data}` | 执行结果（evidence 行摘要） |
| `check/status` | `{check_id, check, expect, actual, gate}` | 判据执行（hard/soft，judge=谁判的） |
| `verdict/status` | `{job_id, status}` | criteria 判据（MET/UNMET/blocked） |
| `assistant/reply` | `{text, chars, model}` | 交付物：最终回答（verdict 与 turn/end 之间；空回答也诚实发出）；`model` = 上游真实路由模型（ADR-0036，非 config 声明值，可为 null） |
| `assistant/usage` | `{prompt_tokens, completion_tokens, cached_tokens, reasoning_tokens, model}` | 上游计量（**ADR-0038**）；**只作披露，永不参与判据**；**词表 1.1.0（2026-09-15）起** —— 该行由 `Cellrix/web/tests/wordlist_parity_test.js` 守着（文档词表 == 代码词表） |
| `turn/end` | `{done, success, impasse, reply, model}` | 周期结束，回 Perception；`reply` = 最终回答冗余字段（消费端可直接取）；`model` 同 `assistant/reply` |

每行 `{type, seq, time, data}`：`seq` 周期内单调（确定性重放），`time` 注入时钟的 RFC3339（可重放）。

### D2b: 会话线程化（2026-09-14 修订）

连续对话 = 一个根周期 + 一串续接：`/v1/chat` 的 done 事件携带 `job_id`（`derive_job_id(input)`），客户端将其设为下一次的 resume 锚点；后端把该 job 写入 `context.resume_job`，`context/inject.resume_from` 优先携带 job_id（机器可读父链），`resume` 字段仍承载人读摘要用于 prompt 注入。会话列表按 `resume_from` 聚合（根卡片 + 续接子条目）。事件文件按周期 truncate：同输入重发 = 同 job（ADR-0006 确定性），周期账本保留**最近一次执行**，完整历史在 Tuck 审计链。

### D3: 写前脱敏（不建敏感数据湖）

事件 payload 递归字符串红act（复用 reasoning trace 的 Redaction：内置凭证形态 + config 字面量）。**事件流永不包含凭证原文**；正文只在 reasoning trace（同受脱敏约束）。

### D4: 摘要为主，正文不重复

事件只带摘要；完整 prompt/response 正文在 reasoning trace（同一 job_id）。**一个事实只在一个载体**（极致复用、极致节能）。

### D5: 非致命写入

与 reasoning trace 同契约：事件写失败不终止认知循环（记录后继续）。失败路径显式降级（无流 = 诚实降级）。

## 3. 与 ADR-0023 的关系

- 这是 D2"实时逐轮数据流"在 Anaphase 侧的本体落盘（Mind 侧的 L3 情景写入继续走 helix_write）
- 会话 = 经历：**事件流是经历的时间性载体**，trace_id 是其身份，judgement（verdict）与行动（tool）都进了同一时间线
- 未来会话聚合（多轮 = 一段经历）在 Cellrix 侧做（按时间/session 前缀分组），不在数据层预构建

## 4. 备选方案与拒绝理由

| 备选 | 拒绝理由 |
|---|---|
| Cellrix 从审计链+trace 投影出 turn 时间线 | 投影有损：审计链无工具/判据语义，trace 无轮次结构；事件在源头才保真 |
| 复用 DSH 命名（user/message 等） | 命名是 Helix 自己的协议值，不借外名（内部命名准确简短） |
| 事件流存全量正文 | 与 trace 重复（一个事实两个载体），浪费存储与读取成本 |
| 每会话一个大文件（多轮聚合） | 会话边界未定（多轮=经历还无协议）；每周期一文件与 trace_id 一一对应，join 最简 |

## 5. 后果

**正面**：
- ProveTrack 可渲染 DSH 式 turn 时间线（USER/CONTEXT/ATTEMPT/TOOL/VERDICT 徽标）单一来源
- 判据（verdict）与行动（tool）首次进入同一可回放时间线——DSH 没有判据维度
- 确定性：同输入同轮 → 同事件序列（seq/ts 注入可重放）

**代价**：
- 每轮一个文件（小文件数量随轮增长）——Cellrix 侧按目录列目录即会话视图，成本可控
- 事件词汇需跨端同步（Anaphase 写、Cellrix 读）——词汇表在本 ADR 冻结为协议值

## 6. 一句话总结

> Tuck 审计链是"网关看见了什么"，reasoning trace 是"Helix 想了什么"，
> 会话事件流是"Helix 经历了什么"——判据与行动、输入与注入，都在一条可回放的时间线上。

### D6: 模型标签与 finalize（ADR-0036，2026-09-14）

1. **模型标签 = 物理事实**：`model` 字段取自上游 OpenAI 兼容响应的 `model` 字段（http_reasoning 在 buffered / JSON 降级 / SSE 三路径捕获），由 `ReasoningAdapter::last_model()` 默认 None、HTTP 适配器覆盖——**显示的是真实路由结果，不是 config 声明的名字**。会话列表（`PeriodSummary.model`）同样捕获，前端消息头与列表卡片显示 `Helix · <model>`。
2. **finalize 交付物**：工具轮执行后，证据回显**不是**回答。Reflection 用一次轻量 finalize 调用（原始问题 + 工具结果 → 自然语言回答）组织交付物；失败降级为证据回显（绝不编造、绝不沉默）。工具成功 ≠ 任务完成（answer.delivered 判据语义）。
