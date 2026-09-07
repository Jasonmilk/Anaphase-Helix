# ADR-0034: 回答被思考吞掉——token 预算共享与有界直答重试

- **状态**: Accepted
- **日期**: 2026-09-08
- **决策范围**: Anaphase（Reasoning 状态 / config）/ Cellrix（attempt 渲染）
- **关联**: ADR-0029（思考不参与判据）、ADR-0030（SSE 事件序）、ADR-0016（单周期原语）
- **取代**: 无

## 1. 背景与问题

真实联调（`run-1dc862da882521dc`）复现用户观察："回答被思考吞掉了"。

印痕事件链物理事实：

```
[user/message]   "你猜猜我我在做什么?"
[assistant/think] len=7104   ← 思考 7104 字符（已起草好回答）
[assistant/attempt] {"text": ""}   ← 最终输出为空
[turn/end] done=true, success=true   ← 还报成功
```

跨轮统计：47 条推理记录中 8 条 response 为空（17%），非个例。

**根因（已查证）**：`reasoning_max_tokens = 2048`，而 reasoning 模型的
**思考与回答共享同一个输出 token 预算**（DeepSeek 家族已知行为——默认上限下开启高思考
模式，"极大概率导致所有 Token 额度被推理过程耗尽，最终返回空响应"；检索证据：
DeepSeek 技术社区实测、GitHub worldmonitor/MiroShark 两个 issue 确认同一根因——
`max_tokens` 对思考模型 = thinking + response 共享，模型无法预知要留多少给回答）。
思考 7104 字符 ≈ 3500+ tokens > 2048 → content 无预算 → 输出空。

**成熟客户端（DSH / Cherry Studio）的做法**：预算给足 + 思考单独渲染（reasoning_content
折叠展示，与 content 分离）。本决策吸收同源哲学并加工程兜底。

## 2. 决策

### D1: 根因——输出预算给足（config 单一来源）

`config.toml` `reasoning_max_tokens = 2048 → 8192`（deepseek-v4-flash 输出上限 8K）。
思考与回答共享预算，预算必须覆盖两者；零硬编码——值来自 config，非代码字面量。

### D2: 兜底——有界直答重试（RunCycleConfig）

`RunCycleConfig.empty_reply_retries`（协议默认 1，0 = 永不重试，config 可配）：

- Reasoning 输出 `trim().is_empty()` 且重试次数未用尽 → 重试一次；
- 重试 prompt 追加确定性指令 `[direct answer required — output your final answer directly, no reasoning]`
  ——**解除思考需求**，把释放的预算留给回答；
- 重试后仍空或达到次数 → 接受空输出（不无限重试，不做重试风暴）。

### D3: 诚实终态——空回复显式标记

attempt 事件新增 `empty: bool`（`output.trim().is_empty()`）：
- 重试后仍空时，客户端渲染明确提示（"本次未生成回答"），不假装空行是答案；
- 状态机周期完成（done）照常——周期完成是事实，回答为空是输出质量问题，
  用事件标记区分，不改状态机语义（极致解耦）。

### D4: 边界——重试不重注入

重试 prompt 是**原 prompt + 直答指令**；思考 sink（thinking_sink）在重试前清空，
`[think-first]` craft note 不重复触发——重试只针对"空输出"这一种失败模式。

## 3. 备选方案与拒绝理由

| 备选 | 拒绝理由 |
|---|---|
| 从 thinking 提取回答作 fallback | 违反 ADR-0029（思考不参与判据/消费）；思考不是承诺，且含内部推理 |
| 空输出时静默返回 | 用户实测痛点正是"只有思考没有回答"还报成功——不诚实 |
| 重试次数无上限 | 违背极致节能与确定性；一次直答重试已覆盖预算饥饿根因 |
| 仅靠调大预算 | 其它模型/时刻仍可能饿死——重试是独立于预算的物理兜底 |

## 4. 后果

**正面**：
- 同一问题（"你猜猜我我在做什么？"）实测：think 5816 字符 + attempt 103 字符
  （`empty=False`）——回答正常落地；
- 根因修复（预算 8192）直接解决 17% 空回复率；重试为其它模型兜底；
- 空回复有诚实标记，前端可明确提示。

**代价**：
- 每次空回复多一次 LLM 调用（tokens）——由 retries=1 封顶，且只在空输出时触发；
- config 增加一个字段（协议默认值，向后兼容）。

## 5. 验证

- `cargo test` 全绿（200 passed，含新测试 `empty_reply_retries_with_direct_answer_directive`：
  第一次空、第二次直答——断言调 2 次、retry prompt 含直答指令）；
- 浏览器实测复现问题 → 回答正常显示（见 D1 后果）；
- 印痕事件链：`assistant/attempt` 带 `empty=false`。

## 6. 一句话总结

> 思考不是免费的——它和回答抢同一个预算；预算给足是根因修，直答重试是物理兜底，
> 空则明说是不说谎。DSH 靠预算和分离渲染，我们在同一条路上把兜底焊成了确定性的。
