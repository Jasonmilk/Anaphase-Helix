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
