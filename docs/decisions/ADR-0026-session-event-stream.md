# ADR-0026：会话事件流——经历的确定性落盘

- **状态**: Accepted
- **日期**: 2026-09-07
- **决策范围**: Anaphase（事件流写入）/ Cellrix（Engram turn 时间线渲染）
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

`job_id` 是 `derive_job_id(user_input)`（`run-<12hex>`）——与 reasoning trace、Tuck 审计链**同一 join key**（Engram 三源一键）。

### D2: 事件词汇表（协议值，消费者精确匹配）

| type | data 摘要 | 语义 |
|---|---|---|
| `turn/start` | `{}` | 认知周期开始（进入状态机） |
| `user/message` | `{text}` | 人类原始输入（写前脱敏） |
| `context/inject` | `{nodes, chars}` | 记忆/认知注入摘要（正文在 trace） |
| `assistant/attempt` | `{text}` | Reasoning 输出（脱敏） |
| `tool/call` | `{tool, index, expect}` | 确定性工具调用 |
| `tool/result` | `{tool, ok, duration_ms, data}` | 执行结果（evidence 行摘要） |
| `verdict/status` | `{job_id, status}` | criteria 判据（MET/UNMET/blocked） |
| `turn/end` | `{done, success, impasse}` | 周期结束，回 Perception |

每行 `{type, seq, time, data}`：`seq` 周期内单调（确定性重放），`time` 注入时钟的 RFC3339（可重放）。

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
- Engram 可渲染 DSH 式 turn 时间线（USER/CONTEXT/ATTEMPT/TOOL/VERDICT 徽标）单一来源
- 判据（verdict）与行动（tool）首次进入同一可回放时间线——DSH 没有判据维度
- 确定性：同输入同轮 → 同事件序列（seq/ts 注入可重放）

**代价**：
- 每轮一个文件（小文件数量随轮增长）——Cellrix 侧按目录列目录即会话视图，成本可控
- 事件词汇需跨端同步（Anaphase 写、Cellrix 读）——词汇表在本 ADR 冻结为协议值

## 6. 一句话总结

> Tuck 审计链是"网关看见了什么"，reasoning trace 是"Helix 想了什么"，
> 会话事件流是"Helix 经历了什么"——判据与行动、输入与注入，都在一条可回放的时间线上。
