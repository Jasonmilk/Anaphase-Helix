# 归档：GROWTH 第 4 条超出上限时移出（方法论：保留最近 3 条）

## [2026-09-08] 完成：回答被思考吞掉——token 预算共享修复（ADR-0034）

### 变更性质
- 根因：reasoning 模型思考与回答共享输出 token 预算（DeepSeek 家族已知行为）；`max_tokens=2048` 下思考 7104 字符（>2048 tokens）耗尽预算 → content 空 → 用户只见思考不见回答（47 条记录 8 条空，17%）
- 根因修：`reasoning_max_tokens` 2048→8192（config 单一来源，零硬编码）
- 兜底：`RunCycleConfig.empty_reply_retries`（默认 1，0=永不）——空输出 → 重试 + 直答指令 `[direct answer required — no reasoning]`（解除思考需求释放预算），有界不风暴
- 诚实终态：attempt 事件加 `empty` 标记（重试后仍空 → 前端明确提示，不假装空行是答案）
- 边界：重试只追加指令不重注入（think sink 清空、craft note 不重复）

### 验收
- 复现问题实测：think 5816 + attempt 103（`empty=False`）——回答落地；浏览器显示完整回答
- 新测试 `empty_reply_retries_with_direct_answer_directive`：空→直答重试，断言 2 次调用 + 指令存在
- 240 passed 0 failed（此前 239 + 新增）
