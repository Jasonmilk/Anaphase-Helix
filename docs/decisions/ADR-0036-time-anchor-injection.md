# ADR-0036: 时间锚注入——物理时钟，0 tokens，本地时区

- **状态**: Accepted
- **日期**: 2026-09-09
- **决策范围**: Anaphase（AgentContext.input_at / Reasoning prompt 组装 / ledger 渲染）
- **关联**: ADR-0021（单一时间源：ledger Clock）、ADR-0022（O-4 craft）、ADR-0023（O-5 注入）
- **取代**: 无

## 1. 背景与问题

Helix 必须知道时间——用户消息到达时刻 + 本周期回答时刻。没有时间锚，
跨天/跨会话的记忆会错乱（用户原话："否则没有时间的锚，会记忆错乱！"）。
同输入派生同 job id（`derive_job_id` 确定性），不同时刻的相同问题会追加
到同一经历文件——模型必须靠时间区分两次调用。

## 2. 决策

- **来源**：单一注入时钟 `AgentLoop.clock`（ADR-0021），`run_cycle` 入口
  记录 `context.input_at = clock.now()`（用户消息到达的物理时刻）。
- **渲染**：新增 `ledger::unix_secs_to_human_local`——宿主本地时区 +
  数值偏移，`2026-09-09 05:00:52 +0800`。完整年月日永不塌缩为 HH:MM。
  为什么本地而非 UTC：用户的物理时钟就是本地时间；UTC 20:59 在
  Asia/Shanghai 是次日 04:59，跨日注入会错位记忆锚（真实验证发现）。
- **注入点**：prompt 组装头部（identity_block 之后、user_input 之前）：
  ```
  [time anchor — physical clock, 0 tokens]
  user message at 2026-09-09 05:00:52 +0800
  now: 2026-09-09 05:00:52 +0800
  ```
- **成本**：0 tokens（~120 字符，确定性字符串，无 LLM）。
- 机器契约（tt_job RFC3339）不动；新渲染函数与 `unix_secs_to_rfc3339`
  并行，同一时间源。

## 3. 测试

- `ledger::unix_secs_to_human_utc_carries_full_date` / `..._local_...`：
  完整年月日 + 偏移结构（不依赖宿主时区）。
- `run_cycle_pipeline::time_anchor_injected_with_full_date`：
  FakeClock 固定时刻，断言 prompt 含锚段与完整日期。

## 4. 验证

真实 tokens 联调：trace prompt 含
`user message at 2026-09-09 05:00:52 +0800 / now: 2026-09-09 05:00:52 +0800`。
