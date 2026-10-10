# ADR-0052：执行期失败必须留下【具名的行】，而不是被咽掉的 warn!

- **状态**：**Accepted（2026-10-09）**
- **日期**：2026-10-09
- **决策范围**：anaphase-helix（`ledger` 的记录面 + `pipeline::execute_calls` 的记录方式）
- **关联**：坑 `K26-EXECUTION-FAILED-ROW` · `run_cycle/mod.rs:722`（DECLARED ROW）· `K25`（静音仪器）· `K19`（一名两物）· `ADR-0050`/`ADR-0051`

## 根因

`Pipeline::execute_calls` 的工具调用那一跳失败时**只返回 `Err`**；`run_cycle/mod.rs:1229`
把它降级成 `warn!` + `Ok(TransitionCondition::Failure)`。**没有账本行。**
⇒ 与 `run_cycle/mod.rs:722` 自己的规矩相悖：*"A REFUSAL IS A DECLARED ROW, NOT A MISSING NODE"*。

**实测代价**（K25 排查全程）：那句 `warn!` 在**测试二进制里没有订阅者**
（`tracing_subscriber::fmt::init()` 只在 `src/main.rs`）⇒ 完全静音
⇒ 我一度用「grep 不到它」错判"该 Err 分支未走"，白走好几轮。
**⇒ 它是本次排查成本的最大单一放大器。**

## 决定

1. **新增 `LedgerRecord::ExecutionFailed { job_id, tool, index, class, detail, identity_label }`**
   （+ 构造函数 `execution_failed(...)`）。`class` 是**稳定可 grep 的指纹**（如 `"transport"`），
   `detail` 携带原始错误文本 ⇒ 消费者不必解析散文。

2. **为什么【不】复用 `Blocked`**：`Blocked` 的语义是**"被闸门拦下"**；这里是**"已派发但通道失败"**。
   共用一个名字就是**一名两物**（K19 的病）。

3. **记录点选在【知道 tool/index 的那一层】**：`Pipeline::execute_calls` 里工具调用那一跳，
   用 `map_err` 追加记录后**原样返回 `e`**。`run_cycle` 的 Err 臂**不知道是第几个调用**，故不能记在那里。
   **语义红线：只改记录方式，不改后续处理**（仍是 `Ok(Failure)`）。

4. **为什么不需要 skip 条件（设计变更，须记名）**：证据链三件 ——
   ① 该 `Err` **只有一处构造**（`src/pipeline/mod.rs:228`：`format!("blocked by security gate: {reason}")`，
   逐字核过，非凭记忆）；② 它**必带** `"blocked by security gate: "` 前缀；
   ③ **控制流不可达**：闸门检查在工具调用**之前**，故闸门拦截的 Err **根本走不到**那一跳。
   ⇒ **防双记由控制流结构保证，由守护测试钉死。**
   ⚠️ 这一句是给后来人的：**不要"好心"加回一个 skip 条件** —— 那会引入一个**不需要维护的假设**。

## 判据（两条，都实测能红）

- **③-a 具名行可见**（`tests/mock_tentacle.rs::execution_failure_leaves_a_named_ledger_row`）：
  断言**行的内容**（`job_id` 可归因 · `tool` · `index` · **`class == "transport"`** · `detail` 非空），
  且**从测试断言层可见，不靠日志订阅**。
  **能红实测**：临时撤掉那一跳的具名记录 ⇒ **FAILED**；还原 ⇒ 5 passed。
- **③-b 反双记守护**（`tests/security_gate.rs`，补在既有 `reject_gate_blocks_and_records_blocked` 之旁）：
  闸门拦截场景 ⇒ `blocked` 行恰 **1** 条（既有）**且 `ExecutionFailed` 行恰 0 条**（新增）。
  **守护的是【结果不变量】，不关心靠什么机制维持** —— 因为"闸门在调用前 return"是**当前控制流**的属性，
  将来任何重构（挪动 gate 检查、或给调用失败也加记录）都可能**无声打破**它：
  **双记不会让任何既有断言变红**。
- **全套不变绿**：`cargo test --all-features --no-fail-fast` = 35 个二进制 / 483 条用例，**0 红**。

## 账（行预算机制自己的 DECLARED ROW）

新增源码行 **+45**（`src/ledger/mod.rs` +28 · `src/pipeline/` +14 · `src/run_cycle/reflection.rs` +3）。
**性质：功能代价，不是腐烂** —— 它买到的是"失败从静音变成具名的行"。
`[[fix_window]]` 是**窗口期**：cap 按**实测**值登记（不用 spike 的估计），且**一个目标只能有一条允许量**。

**★ 守卫生态的第三类捕获（资产）**：检查器拒绝了我不小心留下的**第二条**同目标 `fix_window`——
*"two fix windows target 'src/ledger/mod.rs' … the second would **silently replace** the first"*。
⇒ 这类失败与 `[large]` 同族：**发生了，但没人看见**（静默覆盖 / 静默大改）。
守卫能捕获的第三类 = **"会发生但不会报错的破坏"**，值得单独记名。
