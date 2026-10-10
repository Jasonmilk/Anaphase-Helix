# ADR-0048：跨仓 join 键改用 `period_id`（B18）

- **状态**：**Accepted（2026-10-09）** —— 决策本体由 `ADR-0041` §7.2（人类裁定 2026-09-18）作出；本文是该裁定的**实现 ADR**，不重新论证。
- **日期**：2026-10-09
- **决策范围**：anaphase-helix（join 键的产生与传播）／ Tuck（审计链消费，**零改动**）／ Cellrix（证轨消费与展示）
- **关联**：`ADR-0041`（周期身份唯一化，§7.2 D3 修订 / §7.3 / §7.5）、`ADR-0004`（证轨半体 + Tuck 链）、`ADR-0026`（会话事件流）、`ADR-0037`（术语更名）、`Cellrix:ADR-0021`
- **引用约定**：本文不带仓名的 `ADR-XXXX` 一律指 `anaphase:ADR-XXXX`。

---

## 一、背景：`ADR-0041` 登记了但没排期的另一半

`ADR-0041` §7.2 已裁定：

> **join 键一律改用 `period_id`**；`job_id` 退回它本来的角色（内容摘要 / 重放句柄）。
> **在 B18 完成前，跨来源 join 记为已知未修缺陷，不得写成"设计约束"。**

`ADR-0041` 修完的是**存储侧**（文件按 `period_id` 命名）。**跨来源侧未动**：join 键仍是输入摘要。

### 1.1 实测：碰撞是 219:1，不是理论风险

`M0.5` 现状 dump（`helix-mind/docs/helixECO/M0.5-dump-2026-10-09.md`）：

> **462 个 period 只来自 56 个 `job_id`**，最大一组 **219 个 period 共用同一个键**。

### 1.2 键的产生点（2026-10-09 逐个实测，共 6 处）

| # | 位置 | 当前键 | 去向 |
|---|---|---|---|
| 1 | `src/run_cycle/reasoning.rs:144` | `derive_job_id(user_input)` | → `reason(..., trace_id)` → **Tuck `x-tuck-trace`** |
| 2 | `src/run_cycle/reasoning.rs:324` | 同上 | → **正文轨迹** `trace.record` |
| 3 | `src/run_cycle/reasoning.rs:36`（结构化路径） | `derive_job_id(user_input)` | → `Pipeline::assemble_tt_job(job_id)` → **ledger** |
| 4 | `src/run_cycle/reasoning.rs:381`（LLM 路径） | 同上 | 同上 |
| 5 | `src/run_cycle/mod.rs:637`（`emit_cycle`） | 同上 | → 阶段事件环 trace_id |
| 6 | `src/trace.rs`（读取侧） | 按上面的键过滤 | `/v1/trace?trace_id=` → Cellrix `/api/trace` |

消费侧：Tuck `crates/tuck-gateway/src/gov.rs:80` 把 `x-tuck-trace` 当**不透明字符串**存（缺省 `"local"`）⇒ **Tuck 零改动**。

---

## 二、决策

**D1：`period_id` 是本工程的 join 键。** 凡"跨来源关联一次执行"的场合，键一律是 `period_id`；
`job_id`（输入摘要）只用于确定性 replay 与基名可读性。

**D2：`period_id` 在周期入口**无条件**分配**，不再与"事件流是否配置/打开成功"绑定。
理由：D1 的键不能因为某个可选通道未配置就退回一个会碰撞的摘要。

**D3：唯一回退**：分配失败（身份层拒绝，B17'）⇒ 该周期**没有 join 键**，记为缺失，
**不**静默回退到 `job_id`。回退会让"缺失"与"碰撞"同名 —— 同一形状的原则（读者拒绝脏数据）。

**D4：`job_id` 的位置不变**：仍写进事件流的 `job_id` 字段、仍是 replay 句柄（`ADR-0041` §7.5：
replay 断言比对 body 且**排除** `period_id`）。

---

## 三、影响

### 3.1 正面

- 219:1 的合流消失：同输入两次独立对话在 **Tuck 链 / ledger / 正文轨迹**三处得到**两个不同的键**。
- Cellrix 证轨按 `period_id` join，不再把多周期事件合成一条。

### 3.2 需要同时守住的两条既有约束

1. **replay 等价**（`ADR-0003` / `ADR-0041` §7.5）：正文轨迹比对必须排除 `period_id`，
   否则确定性 replay 会因身份不同而红。**本 ADR 不改这条**，并新增断言钉住它。
2. **append-only**：历史行（旧 `job_id` 键）**不动**，由读者接受"两种键"并存。

### 3.3 已知残留（**明说，不掩盖**）

- **历史数据**：已落盘的审计链/轨迹行仍带旧键。**不迁移**（append-only）。
  ⇒ 面板需同时接受两种形状；`/v1/trace` 的过滤保持字符串相等语义。
- **Tuck 侧无校验**：`gov.rs` 不校验键的形状，故"传错键"不会被 Tuck 拒绝 ——
  唯一的守卫在生产者（本 ADR 的 D1–D3 与随附测试）。

---

## 四、验收判据（可证伪）

> 同一用户输入的**两次独立对话**，在 **Tuck 审计链 / Anaphase ledger / Cellrix 证轨** 三处
> 各自产生**不同的** join 键；任意一个 period 的键在全局唯一。

**反证（变异）**：把 join 键改回 `derive_job_id(user_input)` ⇒ 同一输入两次对话的两个键**相等** ⇒ 断言必须变红。
**说不清"改哪行会红"的测试不算交付。**

---

## 五、冻结处置（明说）

本 ADR 的实现会触达 6 个**在册且零余量**的 ratchet 目标
（`DSH-冻结档-2026-10-10.md` §三：任何一行增长即红）。处置：

- **优先净零**：本次改动伴随**必须重写的注释**（`reasoning.rs:145-156` 的注释在本 ADR 生效后
  **即为错误** —— 它写着「`job_id` stays the join key」，且本身有一处**重复**）。
  ⇒ 以"删旧错注 + 加新码"抵账，力求每个文件 **NCLOC 不增**。
- 若无法净零 ⇒ 走 `DSH-冻结档` §三 的三步（`--emit-ratchet` + `ci K≥3` + 记入关账档），
  **不静默**。

---

## 六、状态

**Accepted — 2026-10-09。** D1–D4 自本日起生效；`ADR-0041` §7.2 的"已知未修缺陷"登记
在实现落地并通过 §四 验收后关闭。
