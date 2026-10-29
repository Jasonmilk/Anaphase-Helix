# ADR-0054：停的位置是执行点，以及第四态（bound-then-dark）

- **状态**：**Proposed（待人类批准；其中「停的位置」一句已于 2026-10-28 由园丁追认，见 §一.1）**
- **日期**：2026-10-28
- **决策范围**：anaphase-helix（`run_cycle` 的 Execution 分支 + `pipeline` 的装配结果词汇）
- **关联**：`ADR-0007`（D2 fail-open 装配，**本文不重开**，只对 D'-3 的注释做补注）、`ADR-0005`（candidate E：结构化执行通道）、`ADR-0022`（安全门 / K-033）、`Tuck:ADR-0004` D5/D7（旁路焊死）、`Tuck:ADR-0007`（审计词汇）
- **引用约定**：不带仓名的 `ADR-XXXX` 指 `anaphase:ADR-XXXX`；`Tuck:ADR-XXXX` 是他仓。

---

## 一、裁决

### 1.1 「停」的位置 = **执行点**（2026-10-28 园丁追认）

早期讨论里「Tuck 挂了 anaphase 应该停」有两个候选落点：

| 候选 | 后果 | 裁决 |
| --- | --- | --- |
| **入口停**（`/v1/chat` 返回 503） | 整机不可用：连"想、记、报"一起停 | ❌ 未采纳 |
| **执行点停**（工具调用拒不发出） | 降级为只读：还能想/记/报，只是不能做 | ✅ **采纳** |

采纳理由：生态耦合的本意是「绑了就焊死」，不是「绑了就整机断电」。一次网关故障不应让人连提问都不能问；而"做"才是需要外部信任的那一步，也正是内容安全层真正要守的那一步。

**追认留痕**：本裁决由园丁于 2026-10-28 追认（`m00818`「② 追认批 3：执行点停的行为翻转（2ab501a）」）。追认发生在实现之后——实现先于批准这件事本身是缺陷，记录在此以免后来者把提交时间当成批准时间：
- `d026db9`（判据，红着进仓）→ `2ab501a`（翻行为，判据转绿）→ 本条追认。

### 1.2 第四态：**bound-then-dark** 是一等状态

装配结果原本只有三态（`Unbound` / `Unreachable` / `Bound`，见 `ADR-0007` D'-3 补注与 `src/pipeline/mod.rs` 的 `PipelineBinding` / `BindingState`）。但运行期还会出现第四种：**装配时绑上了，运行期探针说 tentacle 不在了**。

这不是"第三种"的重复，因为它来自**不同的观察时刻**：三态是启动时的装配事实，第四态是运行期的物理事实。旧实现把它写成一行 `warn!("tentacle dark at runtime; pipeline path may fail-open")`，然后照样执行——一条日志不是判据（K22 族：东西在那儿，等于没有）。

裁决：**第四态与前三态同权**——一律具名拒绝，不落 legacy，不执行。

---

## 二、四态与处置

| 态 | 来源 | 拒绝文案（唯一出处 `BindingState::refusal`） | 处置 |
| --- | --- | --- | --- |
| `Unbound` | 端点缺席或空串（出厂态） | `execution pipeline unbound: no tentacle endpoint configured …` | 执行点拒绝 → Reflection（只读降级） |
| `Unreachable` | 配了连不上（环境故障） | `execution pipeline unreachable: tentacle at {endpoint}: {reason} …` | 同上 |
| `Bound` | 装配成功且运行期可见 | —（无拒绝） | 走 `execute_structured` |
| `Bound` 但 dark | 运行期探针 `GloveStatus::Unavailable` | `execution pipeline dark at runtime: bound at assembly, but the runtime probe says the tentacle is gone (bound-then-dark)` | 同上（拒绝） |

三条推论：

1. **拒绝不是循环结束**：新条件 `TransitionCondition::ExecutionRefused` 有一条表边 `(Execution, ExecutionRefused) → Reflection`，所以拒绝之后仍能想/记/报。**没有复用 `ReflexBlocked`**——反身门与执行点是两道门、两种处置，共用一个条件就是本仓反复记录的「两义坍缩」。（这个选择是被 `every_returned_condition_has_a_rule` 当场逼出来的：它报「Execution can return ReflexBlocked but the table has no edge for it」。）
2. **拒绝必须可断言**：`warn!` 只给人看，判据读 `context.last_execution_refusal`。单独一个槽位，不与 `last_reflex_block` 复用（两道门，两个槽位）。
3. **判据读 `binding`，不读 `pipeline.is_some()`**：后者把 `Unreachable` 塌缩成「没配」，拒绝文案就会指错原因。

---

## 三、证据链（K24 范式：先红、再修、再绿）

| 阶段 | 提交 | 读数 |
| --- | --- | --- |
| 判据（红着进仓） | `d026db9` | `2 failed` — `执行点没有停：calls=["echo"]`；`运行期变暗后仍执行了：evidence=[…ok: true…]` |
| 翻行为 | `2ab501a` | 同一判据文件**一行未改**：`3 passed; 0 failed` |
| 全量 | `2ab501a` | `--all-features --no-fail-fast` ⇒ 只剩 `K32` 的两条古红 |

判据文件 `tests/execution_gate.rs` 的三条：`unbound_execution_point_does_not_execute`（正）、`unbound_still_parses_the_calls_so_silence_is_not_absence`（守判据自己：calls 若本来就空，"没执行"就是假绿）、`dark_tentacle_at_runtime_is_not_a_silent_proceed`（第四态）。

---

## 四、后果（登记，不在本文私自清理）

1. **`tool_command` / `execution_placeholder` 对「做」已死**：它们现在只在 `calls` 为空时才会用到。这是本裁决的后果，不是顺带清理项；与 `K32` 的预算工作一起裁。
2. **legacy 的 echo 仍会输出执行计划**：K-127 / LEAK 4.8%（"说/做不分"）**与本裁决无关**，安全含义已由执行点接管，但噪声仍在。单记，不与安全混。
3. **`ADR-0007` D'-3 的文档注释原文**（`fail-open — an empty endpoint or a failed connection yields None`）把两义写在一句里 ⇒ 需要一条**补注**（不是修订：装配层的 fail-open 保留，`tests/replay_guard.rs` 原样）。

---

## 五、明确不做

1. 不改装配层的 fail-open（`resolve_binding` 不炸启动；`ADR-0007` D2 与 `tests/replay_guard.rs` 原样保留）。
2. 不加 break-glass 旁路（那就是被消灭的后门换了个持有人）。
3. 不做入口 503。
4. 不把 `ToolGateOutcome` / `HITL` 那条门与本条的门合并（K-033 的两条路径答案本就相反）。
5. 不在本文处理 K32 的 8 个超预算文件。

## 六、待批准

- 本条整体（`Proposed`）。其中 §1.1 的「停的位置 = 执行点」已获追认；§1.2 第四态、§二 的四态处置、§四.3 的补注仍待批。
