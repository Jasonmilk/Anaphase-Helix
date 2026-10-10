# ADR-0053：会话锚是【已存在的 `conversation_id`】—— K15 走【A · 分组下沉查询层】

- **状态**：**Accepted（人类 2026-10-10 裁决选 A）**
- **日期**：2026-10-10
- **决策范围**：anaphase（查询层的**消费方式**，或持久化层的键）
- **关联**：`K15` · `ADR-0006`（Episode 边界）· `ADR-0020`（事件轨迹持久化）· `ADR-0047`（lineage_path）

## 先查的结论（实测，非推测）

`src/session_events/query.rs`：
- `:132` **`pub conversation_id: Option<String>`** —— 它已经是**查询层的一等字段**；
- `:421` `s.conversation_id = roots.get(&s.period_id).cloned().flatten();` —— **由 roots 映射填充**；
- 且**已有判据**（`query_tests.rs`）：`:347` **"a root names itself"** · `:349` **"a continuation inherits its root"**；
- `refs.rs:9` 的注释也说：*"into is **DERIVED from the lineage (`conversation_id`)**, never duplicated here."*

**⇒ 所以"一个会话"这件事，本仓【已经知道】** —— 它由**客户端设的 `resume_from`**（`main.rs:665` /
`Cellrix/web/assets/chat.js:340`："next resume_from, so **consecutive messages stay ONE conversation**"）
构成链，链**根**即 `conversation_id`。

## 因此 K15 的缺口被重新定位（这是本 ADR 的价值）

| 原表述（K15 旧文） | 实测后的真表述 |
|---|---|
| "`job_id` 锚在内容上 ⇒ 会话永远只有一轮" | ⚠️ **部分已缓解**：客户端设 `resume_from` ⇒ **连续消息已在同一会话**（实测 `context/inject.resume_from` 覆盖 **34/103** 个 period ⇒ **链是部分覆盖**，不是全无） |
| "正解：`session_id` 与 `job_id` 分离" | **锚已存在**：`conversation_id`（派生自 lineage）⇒ **要做的不是发明它** |
| —— | **真缺口：文件与视图仍以 `period_id` 为键**（`{period_id}.events.jsonl`），而**查询层已知道 `conversation_id`** |

## 两条路（**待裁决**）

| 路 | 做法 | 行为面 | 风险 / 代价 |
|---|---|---|---|
| **A · 纯视图消费**（**推荐**） | **DSH 界面/侧栏按 `conversation_id` 分组**（查询层已提供该字段）⇒ 一个会话显示一行 | **不改持久化接口**（只是**消费已有字段**） | **ADDITIVE、可逆、无需迁移** —— 旧文件一字不动 |
| **B · 改持久化键** | 新写入按 `{conversation_id}.events.jsonl` 追加，读取侧合并 | **改持久化接口**（行为面） | 需迁移案 + 下游读取者清单（`session_events/` 10+ 文件） |

**⇒ 本 ADR 的判断**：**先做 A**（它可能**完全满足 DSH 的目标**——"界面上一段对话是一段"），
**而 B 只在 A 不够时才做**（例如需要"一个会话一个文件"给外部消费者）。
**⇒ 这也把 K15 从"必须改行为面"降级为"可能只是消费一个已有字段"** —— **这正是 `ADR-0006` 那次
"不用突变，只需向光"的第二次实证。**

## 判据

- **A 的判据**：DSH 侧栏对同一 `conversation_id` 的 N 个 period 显示**一行**；且 `conversation_id` 由**查询层**给出（不在前端重算）。
- **B 的判据**（若走）：`tests/session_convergence.rs` 的 `k15_target_one_episode_lands_in_one_file`
  **取消 `#[ignore]` 后转绿**（该判据已在 `anaphase 4f58c21` 落地，当前**红且具名**）。

---

## 裁决与三附件（人类 2026-10-10：**选 A**）

> **★ "已备"不是状态翻转** —— 本 ADR 由 Proposed 转 **Accepted(A)**，实现随之开工。

| 附件 | 内容 | 判据 |
|---|---|---|
| **一（最重）** | **分组逻辑【下沉到 anaphase 查询层】**，Cellrix **只消费** —— **不是"去 Cellrix 做侧栏分组"**。否则 **K15 会从"挡 DSH"反转成"被 DSH 挡"**（把地基放进 UI 里，UI 就变成地基的前置） | **纯 Rust 单测**：fixture ⇒ 分组输出（不碰 UI、不碰网络） |
| **二** | **B 路线的红测试留 `#[ignore]` 作长期债标记** | `k15_target_one_episode_lands_in_one_file` 保持 `#[ignore]` + 具名理由（**它今天真能红**，已实测） |
| **三** | **`34/103` 进观察态判据**，盯**合并率** | 一条读数：有 `resume_from` 的 period 占比（链覆盖率的趋势） |

**⇒ 所以本 ADR 的落地物是【anaphase 查询层的一个纯函数 + 它的单测】，Cellrix 侧不动。**
