# ADR-0051：被 drop 的 shutdown 发送端会让 mock 服务器提前停机（"偶尔红"的真因）

- **状态**：**Accepted（2026-10-09）**
- **日期**：2026-10-09
- **决策范围**：anaphase-helix（`tests/`）
- **关联**：`KNOWN_ISSUES` K25 · `ADR-0050`（快照 golden 与判据诚实）· `tests/common/mod.rs` · `tests/mock_tentacle.rs:66`（正当用法）

## 根因

`tests/run_cycle_pipeline.rs`（及 `tests/stage_events.rs`）把 `spawn_mock_tentacle` 返回的
shutdown 发送端绑成 `_tx`：

    let (endpoint, _captured, _tx, _handle) = spawn_mock_tentacle(mock).await;

**下划线只表示"我不打算用它"，不阻止它在**本函数返回时**被 drop。**
而 shutdown 发送端一旦被 drop，oneshot 接收端就**立即** resolve ⇒
`serve_with_incoming_shutdown` 收到停机信号 ⇒ **服务器停止 accept**。

## 机制链（每一片证据都被认领）

| 证据 | 由谁认领 |
|---|---|
| `s1:begin("llm") → s1:end("calls=N")`、`s2:*` 都走完了 | 它们由 **`src/run_cycle/reasoning.rs:69-78 / 388-389`** 发出，是**推理/组装**阶段，**不碰网络、不经 mock** ⇒ 与停机无关（曾经的"孤儿证据"） |
| `s3:begin(calls=1) → s3:gate(gate=none)` 之后全无 | `s3` 才是唯一过网的 `Pipeline::execute_calls`；服务器已停 accept ⇒ 传输层失败 |
| `mock_captured = 0`（15/15） | 同上：请求根本没到服务器 |
| 无 `evidence`、无 per-call 事件 | 工具调用失败 ⇒ `execute_calls` 返回 `Err` |
| 失败**完全静音** | 该 `Err` 被 `run_cycle/mod.rs:1229` 降级成 `warn!`，而**测试二进制里没有日志订阅者**（`tracing_subscriber::fmt::init()` 只在 `src/main.rs`）⇒ 什么都不打印 |
| 失败数与集合**逐次波动（1–5）** | 每个测试**各有自己的 mock 与自己的 tx** ⇒ 各自独立地赢/输"停机生效 vs 调用发起"这场赛跑 ⇒ 总数 = 多个独立伯努利之和（**故不是全有全无**） |
| 与负载 / 线程 / env / 文件 / 端口 / 生态栈无关 | 它是纯赛跑，不是资源竞争 |

⚠️ **诚实标注**：中环（"停止 accept"与"已建连接被 tear down"的具体时机、GOAWAY 行为）
**没有直接观测**，依据是 **tonic/h2 的 graceful-shutdown 语义 + 端到端变异判据**（见下）。

## 判据（两条，都能红）

1. **基线对照**：修复前 `cargo test --test run_cycle_pipeline` **20 次约 25–28 个失败（~95% 红）**；
   修复后**并行 20/20、串行 20/20 全绿**；修复后再跑全套 **6/6 全绿**。
2. **变异验证**：删掉 `keep_mock_alive(tx)`（或 `_tx` 的持有）⇒ **8 次立刻累计 36 个失败**。

## 修法选择（为什么是"持有"，不是 `forget`，更不是"占位发送端"）

| 方案 | 判定 | 理由 |
|---|---|---|
| `std::mem::forget(tx)` | ❌ 能用但姿势不对 | 它的语义是"**永不关**"，而意图是"**活到我用完**" —— 抽象错位，读代码的人会问"为什么故意泄漏" |
| 让 `spawn_mock_tentacle` 返回**占位**发送端 | ❌ **被实测否证** | `tests/mock_tentacle.rs:66` 正当地用 `shutdown_tx.send(())` 来造传输错误 ⇒ 换掉真 API 立刻让那条测试红。**真 API 不能被伪装** |
| **调用方持有真发送端**（`keep_mock_alive`） | ✅ 采用 | 零泄漏语义、名字即意图；**且修复落在真正丢弃它的那个调用方** |

## 方法论（本轮用惨痛代价换来的五条）

1. **挂起任务没有线程栈帧**：任务被 poll 时 future 树在线程栈上展开，返回 `Pending` 的瞬间整棵栈退回堆。
   ⇒ **线程采样只看得见 busy spin**，看不见"挂起等 IO"与"任务已死"（后两者无帧可采）。
2. **静音仪器不能当证据**：我用 `grep 'Pipeline execution failed'` 得到 0 次并据此宣布"Err 臂已排除" ——
   **无效**，因为那个 `warn!` 在测试里根本不会输出。正解不是"记住仪器可能静音"，而是**让仪器不再静音**（`common::init_logs`）。
3. **凭记忆写 pattern 不能当证据**：grep 的 pattern 必须对着源码字面量逐字核过。
4. **"等一个逻辑上不可能到来的事件" ≠ "唤醒丢失"**：前者是调用**已经失败**、测试在等结果；
   后者的修法方向完全不同（查 waker vs 查调用为何失败）。二者外观都是"卡住不返回"。
5. **`_` 与 `_x` 的 Drop 语义陷阱（Rust 经典陷阱的 async 爆炸形态）**：
   `let _ = x` **立即** drop；`let _x = x` 活到作用域结束 —— 而 oneshot 发送端的 `Drop` 带**协议副作用**（触发 shutdown）。
   现有 lint（如 `let_underscore_future`）不覆盖"丢弃带 Drop 副作用的字段"。

## 新增调用者的义务

**任何新调用 `spawn_mock_tentacle` 的测试都必须持有返回的发送端**（`common::keep_mock_alive(tx)`）。
`tests/stage_events.rs` 就是漏了这一处而间歇红（同一根因的第二例，由本 ADR 的判据独立重发现）。

---

## 附：结案后的四条补记（2026-10-09，reviewer 审查后）

### 附① 分母说清（原来"全套 6/6"没给分母）

**确切命令**：`cargo test --all-features --no-fail-fast`
**真实分母**：**35 个测试二进制**（`Running` 行）· **483 条测试用例** · 36 个结果块。
**差一的来源已查明**：第 36 块是 `Doc-tests anaphase`（doctest 不打印 `Running` 行）⇒ **35 + 1 = 36**。

**基线只写红率，不写失败计数** —— 各轮口径不一：一次 20 连跑的全套里 19/20 次运行有红、
累计 **67** 个失败（≈3.5/次，**全库范围**）；而另一处 **25–28** 是
`--test run_cycle_pipeline` **单文件** 20 次的累计。**两者分母不同，不可并列**。
⇒ 故本文的基线表述统一为：**修复前红率 ≈95%（20 连跑 19 次有红）→ 修复后 0/35 二进制红、483/483 绿。**

### 附② `the_override_does_not_affect_the_real_run` 的销案（从"断言"升级为"证据"）

它**不是**被本 ADR 的 tx 修复修好的（它所在文件 `tests/ci_line_budget.rs` 根本不调用
`spawn_mock_tentacle`）。它的正文是：

    fn the_override_does_not_affect_the_real_run() {
        let (code, _, stderr) = run_checker(&[]);
        assert_eq!(code, 0, "the tree is clean: {stderr}");
    }

**主证是结构性论证（路径不相交）**：该文件既不 `use` 也不调用 `spawn_mock_tentacle`，
更不构造 `Pipeline`/`AgentLoop` ⇒ **它的失败路径与本条修复的路径不相交** ⇒ 本条的修复
**在构造上不可能**修好它。
**佐证（时序，标注为推断而非观测记录）**：它所在文件跑的是 `run_checker(&[])`，其真因是
行预算的修复（登记 `K25-LEDGER-SNAPSHOT` 坑 + `[[fix_window]]`）；**时序上它与预算转绿同步，
但我没有为"同步"单独留过观测记录** ⇒ 故此处**只作为佐证**，不作为主证。
**⇒ "消失了"必须追因，不能算作"顺带修好"**（第 6 条的姊妹条，见附④）。

### 附③ 修法补上一半：`_handle` 的"死亡无人知晓"已在**源头**清掉

`keep_mock_alive` 只持 `tx` 时，`handle` 仍被调用方丢成 `_handle` ⇒ 服务器任务若 **panic**
或异常结束，无人知晓（H5 的沉默形态）。本轮在 **`spawn_mock_tentacle` 源头**加看守者：

    let watched = handle;
    let handle = tokio::spawn(async move {
        if let Err(e) = watched.await {
            eprintln!("[K25] mock tentacle 任务异常结束: {e}（panic={}）", e.is_panic());
        }
    });

⇒ 死亡**具名**（含 panic，经 `JoinError::is_panic`），且**不改变** `tests/mock_tentacle.rs:66`
的正当用法（那里只用发送端）：实测 `--test mock_tentacle` 仍 **4 passed**。

### 附④ 方法论第 6、7 条

**第 6 条（本轮最贵的遗漏）：绿是小样本抽样，红率 N≥20 才有资格说话。**
它正是"等它红是伪命题"的来源 —— 我曾据"8/8 绿"宣布"它现在不红了"，而同一命令
20 连跑是 **19/20 有红**。**对"变绿"与"消失了"同样适用**（附②就是实例）。

**第 6 条的姊妹条：绿要计数，消失要追因。** "某测试消失了/好了"与"绿了"同属**成功断言**，
同样有第三态（小样本绿 / 他因被顺手修掉）⇒ 必须给出**结构性或观测性**的因果，不能记作"顺带修好"。
（附②就是这条的实例：主证是路径不相交，时序只是佐证。）

**第 7 条：每个二分先问第三态。** 本轮的三个二分各自漏了第三态，且漏的那一支先验最高：
- "断言读数为 0" 的二分漏了 **C（断言早于完成）**；
- "采样看见什么" 的二分漏了 **挂起任务没有线程栈帧**（死/挂/busy 三分）；
- "grep 无输出" 的二分漏了 **仪器本身静音**（没有订阅者）。
**⇒ 对成功断言同样适用**："绿了 / 消失了"也有第三态（小样本绿 / 他因被顺手修掉），附②即实例。

**（记账，不重做）** 本 ADR 的改动是三件不可分的东西（修复 / 判据仪器 / ADR 三本账），
故以 `[large]` 具名提交。**习惯上更优的姿势是：三笔各 <50 行的提交互相引用**（对 `git bisect` 友好）。
