# ADR-0042: 400 行红线债务清偿 —— D11 的"另行开轮"到期

- **状态**: Accepted
- **日期**: 2026-09-18
- **决策范围**: anaphase-helix（`session_events.rs` → `session_events/`，`run_cycle.rs` → `run_cycle/`）
- **关联**: `ADR-0038` D11（本次不做：不解耦 400 行红线既有违规）· `ADR-0041`（周期身份，其代码落在 `session_events.rs`）· DNA 红线「代码超 400 行必须解耦」

---

## 一、背景：被推迟的债务，以及它没有被"不扩大"

`ADR-0038` D11 原文：

> **不解耦 400 行红线既有违规**——`run_cycle.rs`、`session_events.rs` 均已超 DNA 红线，属**既有技术债**；本轮只加入必要的最小代码，**不扩大**，**另行开轮**。

**实测（2026-09-18）**：

| 文件 | 行数 | 相对 400 |
|---|---|---|
| `run_cycle.rs` | **2093** | 5.2× |
| `session_events.rs` | **1545** | 3.9× |
| `main.rs` | 1171 | 2.9× |
| `contract/mod.rs` | 703 | 1.8× |
| `adapters/http_reasoning.rs` | 540 | 1.35× |
| `rails.rs` | 488 | 1.2× |
| `adapters/mind.rs` | 474 | 1.19× |

**⇒ D11 的两个条件都没被满足**：既没有"不扩大"（`session_events.rs` 在本轮的身份改动中又长了约 250 行），"另行开轮"也一直没有发生。

**⇒ 本 ADR 是那"另行开轮"。**

---

## 二、决策

### D1：清偿顺序——先 `session_events.rs`，再 `run_cycle.rs`

- **先拆 `session_events.rs`**：它在本轮被改动最多，契约（身份 vs 重放句柄）刚刚稳定、测试 284 全绿 ⇒ **回归网最密**，是最安全的窗口。
- **`run_cycle.rs` 随后拆**，方式见 D2。
- **两者都拆完，才继续 B18**（身份传播到 Tuck）—— 否则 B18 要在 1545 行里找位置，且会再扩大一次。

### D2：拆分方式——沿用仓内既有目录式模块，不发明新结构

仓内已有 8 个目录式模块（`adapters/` / `contract/` / `pipeline/` / `ledger/` / `evidence/` / `criteria/` / `ci144/` / `bin/`），均以 `mod.rs` 为入口。本次沿用。

**`session_events.rs` → `session_events/`**

| 文件 | 承接 |
|---|---|
| `mod.rs` | 重导出 + `EventType` + `SessionEvent`（对外契约类型） |
| `identity.rs` | `job_digest` / `try_allocate_period_id` / `allocate_period_id` / `is_period_id` / `is_job_id` / `PeriodRef` / `Resolved` / `resolve_period` / `resolve_one` |
| `stream.rs` | `SessionEventStream`（开流 / `emit` / redact / `emit_period_start`） |
| `query.rs` | `list_periods` / `read_period` / `read_summary` / `PeriodSummary` |
| `naming.rs` | `rename_period` / `period_name` / `freeze_name`（`.name` sidecar） |
| `crystallize.rs` | `crystallize` / `CrystalSuggestion` |

**`run_cycle.rs` → `run_cycle/`**：按 `HelixState` 状态机分段，`mod.rs` 只留 `AgentLoop` 与调度。
**⇒ 注意**：反射那一段**不得**命名 `reflex.rs`——`src/reflex.rs` 已存在（`ReflexArc`）；用 `reflex_check.rs`。

### D3：验收判据（可证伪）

1. **没有任何源文件 > 400 行**（含 `mod.rs`）—— 命令：`find src -name '*.rs' -exec wc -l {} + | sort -rn | head`
2. **纯移动**：拆分前后**公开 API 不变**（类型名、函数签名、可见性），除非一并记录为独立决定。
3. **测试数不变**：拆分不得删改测试（277→284 那条基线只增不减）。
4. **`cargo test` 全绿**（当前 284/0）。
5. **无死代码**：拆分后不得留下未被引用的 `pub` 重导出或空模块。

### D4：本 ADR 不清偿的（诚实边界）

- `main.rs`（1171）、`contract/mod.rs`（703）、`adapters/http_reasoning.rs`（540）、`rails.rs`（488）、`adapters/mind.rs`（474）**仍超线**。
  **⇒ 本 ADR 只清偿被 D11 点名的两个文件**；其余另行开轮，**不得因为"拆过一轮"就宣称红线清零**。

---

## 三、影响

### 正面
- 本轮改动最多、契约刚稳定的两个文件获得结构性边界，后续改动（B18/B17）不再往单体里塞。
- 身份逻辑（`identity.rs`）从 1545 行里独立出来 ⇒ **"唯一正确性前提"变得可单独审查**（K-031 的教训）。

### 代价与风险
- **大 diff**：纯移动仍会产生一次数百行的 diff，审查成本高 ⇒ 必须**分段提交**（一个模块一次），且**每段跑一次全量测试**。
- **重导出层可能掩盖耦合**：若 `mod.rs` 变成"什么都转发"的出口，则只是把单体换成了目录。**⇒ 判据 5 就是为此设的。**
- **`use` 路径变动会波及 `main.rs` 与测试**：预期 `pub use` 保持外部路径不变，如不可行则记录为独立决定。

---

## 四、不解决什么

- **不改变任何行为**。本 ADR 是结构决定，不含语义决定；语义若有变动，必须另立 ADR 或写进对应坑记录。
- **不偿还其余 5 个超线文件**（见 D4）。
- **不为"行数"而拆**：拆分边界按**职责**划，不按行数切齐。若某职责拆完仍超 400，**如实记录并说明**，不用硬切制造假达标。

---

*一个文件长到 2000 行时，读它的人已经不再是在读一个模块，而是在读一个目录的目录。*
