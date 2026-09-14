## [2026-09-14] 完成：上游计量捕获——按次落盘，按需派生（ADR-0038）

### 变更性质
- 根因：证轨状态栏八个统计格中三个恒为 `—`（TOKENS / 缓存命中 / TOK/S）。不是没实现，是**没有数据源**——上游 `usage` 从未被捕获。
- 捕获（写入端只记事实）：适配器契约新增 `last_meta()`，在**实现层**收敛 ADR-0036 的 `last_model()`（降为派生默认方法，契约不变，故 ADR-0036 不被取代）；`http_reasoning` 的 `capture_model` → `capture_meta`，一次加锁内同取 model + usage；逐字段「存在才写」，`null` 与缺失同义（部分 OpenAI 兼容网关会填 `null`，不得当成"清零"）。
- 落盘（Append-Only 扩展）：新增 `assistant/usage` 事件，**每次上游往返一条**。一个周期有 3 处调用点（重试循环流式/缓冲 N 次 + 工具证据 finalize）。若挂在 `assistant/reply` 上，重试与 finalize 的成本会静默丢失（违反物理事实优先）。
- 派生（读取端纯函数）：Cellrix `derivePeriodUsage(events)` 按需聚合，纯函数零状态，可重放，零模型调用。**不做写入端累加器**（会丢明细，且把聚合混进写入端）。
- 三条口径：①**不相交计数** `输入 = prompt − cached`（上游 `prompt_tokens` 已含缓存命中，直接展示会把同一批 token 计两遍）②可选桶**全有或全无**（不是每次调用都报就整桶省略，不给残缺的和）③**缺失即省略**，永不折算估算。
- 不存储 `total_tokens`：它等于 `prompt + completion`，留着就是一个事实两个来源（改为派生 + 溢出校验）。
- 解耦：解析实现抽出独立模块 `src/adapters/usage.rs`（契约层 `mod.rs` 不装实现），可脱离 HTTP 单测。
- 边界：usage 为 display-only，**不进判据**；原则 7 的 token 预算熔断走 `EnergyContext.token_budget`，是另一条数据路径（ADR-0038 D9 显式划界，防未来接线误用）。

### 验收
- 新增 12 条测试：`usage.rs` 6 条（不相交口径 / 缺失与 `null` 同义 / 两拼写回退 / 矛盾桶拒收 / 残缺记录拒收 / 派生溢出拒收）+ `http_reasoning.rs` 5 条（流式 usage 只出现在终 chunk 的顺序陷阱 / 无 usage 报 None / `null` 不清除已捕获值 / 缓冲路径 / 非 SSE 回退 / **跨调用不重放上一次（D12）**）。
- 零回归（HEAD 现算）：测试属性 254 → 266，净增 12，**一条基线测试都没丢**；`cargo test --no-fail-fast` = 257 passed / 0 failed / 9 ignored。
- **实测补丁（D12）**：读侧发现真漏洞——适配器跨调用复用，上游本次未报 usage 时会把**上一次**的数字留在字段里 → 周期总和静默翻倍（造假事实）。先写测试证伪（修复前失败：`left: Some(..10, 2..) / right: None`），再以 `begin_round_trip()` 在每次往返开始时丢弃上一次计量。`model` 刻意不清（描述路由而非本次调用）。
- live 端到端：Tuck 网关 → anaphase `--stdio` → 真实上游（`X-Route-Tier: free`）。真实产出 `assistant/usage` = `{prompt_tokens:664, cached_tokens:256, completion_tokens:171, reasoning_tokens:148, model:"agnes-2.5-flash"}`；不相交输入 408，total 835。
- Cellrix 数据层真实回放：用上述 live 事件文件在 node 下回放 `prove_track.data.js`，27 项断言全绿（含「计量事件不扰动既有 dur」逐项相同）。

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

## [2026-09-08] 完成：SSE 事件序运行时焊死（ADR-0030）

### 变更性质
- 终态通道 oneshot → mpsc：oneshot 在 complete 后重复 poll 触发 tokio panic（`called after complete`）→ unfold 流在 done 行发出前中断 → 浏览器只收到 attempt 裸 JSON delta（用户视角"Helix 回复是计划文本不是答案"）。mpsc `recv()` 可安全重复 poll，done 行确定性到达。
- 事件序契约不变（ADR-0028：delta 先、done 后、drain flush 尾部）；只换通道原语，不换语义。
- sender 未发送即 drop（周期崩溃）→ 流静默结束，客户端保留已流式内容，不伪造 done。

### 验收
- 浏览器实测：8^3 → Chat 显示 `calc: {"ok":true,"result":"512"}`（此前裸 JSON）；
- 连续两次 curl SSE 均收到 `"done":true`（此前 done 行丢失）；
- `grep -c panicked` = 0（此前每请求后 panic）；`cargo test` 全绿 0 failed。

