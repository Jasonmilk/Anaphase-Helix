

---

## [2026-09-17] 工具链闭环：计划不是答案 + 周期身份不唯一（ADR-0041）

**变异类型**：交付断裂修复 + 存储身份缺陷定位

- **现象（人类报告）**：面板里让 Helix 搜网络，回复却是原始 `{"calls":[...]}` JSON。
  实测全库：**50 段经历中 9 条如此，且 9 条全部是工具轮**（8×`web_search`、1×`calc`）。
- **根因（三处，按因果排序）**：
  ① `Reflection` 的 finalize 守卫只有"非空"，不判**它是不是又一次工具调用**——模型（虽被
  提示 "no JSON"）回计划，该 JSON 即成用户可见答复；② 判据 `answer.delivered` 检查的是
  **工具返回值**（自注 `"delivery confirmed at tool edge"`），不是"用户是否拿到答案"
  ⇒ 工具跑通即判 **Met**，把空交付判成了成功；③ `Reasoning` 侧早已有 **P0-D-1**
  （"never leak the raw JSON as a reply"），**finalize 路径没享受同一条规则**。
- **修复**：把 P0-D-1 延伸到 finalize；新增 `tool_followup_rounds`（协议默认 1，与
  `empty_reply_retries` 同形）做**有界重问**——明示"工具已执行完、不得再调、只用已有结果
  作答"；仍不成则落回**证据回显**（诚实显示工具结果，**永不吐 JSON**）。
- **拒绝了一版更强方案，理由要留档**：原打算"执行模型要的细化查询再综合"。实测否决——
  `trace_id = {job_id}#{index}`（index 局部于计划）且 `record_evidence` **纯追加不去重**
  ⇒ 同一 job_id 下第二次执行会**同时撞 `evidence_id` 与 `trace_id`**，而"一周期一 trace"
  是硬契约（ADR-0019/0026）；要让下标跨轮偏移就得改 pipeline 签名，属架构变更须另立 ADR。
- **测试 + 变异证明**：新增 2 条（计划绝不成答复 / 有界重问收下散文）。
  **变异测试**：把守卫退回"只看非空" ⇒ 两条**双双变红**；还原 ⇒ 双双转绿（非空转）。
- **端到端实证（真实上游，经 Tuck）**：事件流为
  `assistant/attempt(计划) → tool/call → tool/result(ok=true, 846ms) → assistant/usage(prompt=853)
  → assistant/reply(自然语言答案)`；回复是「抱歉，我尝试搜索了…没有找到相关结果」而非 JSON。

### 顺带定位：人类报告"经历内容顺序错乱"——真因是周期身份不唯一

三层叠加，逐层实测：① `job_id` 由输入派生（FNV-1a）⇒ **同问题重问同 id**；
② `session_events::open` 用 `truncate(true)` 覆写 ⇒ 那条记录被替换；③ **别的周期仍以被覆写的
id 为父**（`context/inject.resume_from`）⇒ 父的内容变成**另一次更晚的执行**。

铁证：`run-9e901b965a772d51` 的 `first_ts = 17:01:07`，其子 `run-32c4be74a996a40d` 为 `06:19:10`
——**父比子晚 11 小时**；51 条血缘路径中 **5 条时间戳非单调**，全部源于此。

补充：129 个事件文件中 **8 个含多组 `turn/start`**（最多 7 组；最远相隔 6 天），
**全在 2026-09-07~09-13，09-15 后为 0** ⇒ `truncate` 防住了"一个文件装多期"，
**防不住"覆写被别的周期引用的父"**。

⇒ **ADR-0041（Proposed）**：周期身份与输入解耦——事件流存储键必须唯一，冲突时**后缀分配**
（旧键不动，无时钟、无迁移），且**唯一键即 trace id**（一个身份，不是两个）。

### 验收

- `cargo test --no-fail-fast` = **264 passed / 0 failed / 10 ignored**
- 工具轮端到端：真实上游，回复为自然语言（事件流见上）
- 变异测试：守卫退化 ⇒ 2 红；还原 ⇒ 2 绿
- ADR-0041 经 `tools/adr_head.py`（该事实的唯一解析器）解析通过；索引已补 0041

### 未做（诚实边界）

`answer.delivered` 判据**语义仍名不副实**（检查工具边缘而非用户交付）。补一条"回复不得是
调用计划"的判据是独立一件事，本 ADR/本次修复**没有**顺手做，以免被误认为已完成。

---

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

---

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
