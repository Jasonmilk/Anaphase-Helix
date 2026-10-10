# ADR-0047：「会话」一词三义 —— period / lineage_path / slice 各归其位

- **状态**：**Accepted（2026-10-09，人类裁定：会话 = 切片；另两义改名）**
- **日期**：2026-10-09
- **决策范围**：anaphase-helix（`session_events` 的读取语义与命名）／ Cellrix（`web/assets` 消费侧：窗口 vs 展示）
- **关联**：ADR-0006（会话即经历）、ADR-0041（周期身份唯一化）、`Cellrix:ADR-0021`（链窗口 = 血缘路径）、`Cellrix:ADR-0022`（面板导航约束 N 系列）、`Cellrix:ADR-0048`（面板状态归属与渲染契约，§269/§270）
- **引用约定**：本文不带仓名的 `ADR-XXXX` 一律指 `anaphase:ADR-XXXX`。

---

## 一、背景：一个词，三个含义

「会话」在代码、面板与文档里**同时**指三样东西，而它们的**用途相反**，所以任何一处"修好"都会在另一处表现为"又坏了"。

| # | 概念 | 物理承载（实测锚点） | 该词服务什么 |
|---|---|---|---|
| 1 | **节点**（一轮执行） | `anaphase:src/session_events/types_and_stream.rs:141-147` —— 文件按 `period_id` 命名；`Cellrix:web/assets/period_normalize.js:65 normalize()` 处理**单段** | 身份 / 存储键 |
| 2 | **血缘路径**（祖先链） | `Cellrix:web/assets/period_normalize.js:226 chainJobIds()` —— 注释自述「The window is the LINEAGE PATH: root → the period the human opened」 | **上下文加载**（继续对话时注入什么历史） |
| 3 | **切片**（可达子图） | —— **尚无同一函数**；`panel_tree.js` 用原始 `parent`/`children` 渲染树 | **展示 + 继续**（要的那个 DAG） |

三义共用一词的后果是**假冲突**：把 (2) 当作"错的"去改成 (3)，(2) 的用途立刻回归缺陷。

---

## 二、决策（人类裁定 2026-10-09）

**一句话**：**会话 = 切片（slice）**；`period` 是**节点**，`lineage_path` 是**上下文加载专用的祖先链**。

| 规范词 | 定义 | 服务于 | 硬约束 |
|---|---|---|---|
| **`period`**（节点） | 一轮对话 = `run_cycle` 的一次执行；身份由 `period_id` 分配，**永不由内容派生** | 身份 / 存储 | 见 `ADR-0041` |
| **`lineage_path`**（血缘路径） | 祖先链：含祖先，**不含"尚未拥有的未来"** | **仅**上下文加载 | **不得**用于展示分组 |
| **`slice`**（切片）/「**会话**」 | 从**选中根出发的可达子图** | **展示 + 继续** | **不得**退化为整棵子树的 BFS |

**三词各归其位，禁止混用**：文档、注释、函数名里出现「会话」时，其含义**只能**是 `slice`；指 `period` 或 `lineage_path` 的场合一律用规范词。

---

## 三、理由

### 3.1 v1.97 的 `chainJobIds` 改动不是错

`ADR-0021` 把 `chainJobIds` 从"从根 BFS 走整棵子树"改为血缘路径，解决的是**上下文加载**：继续对话只应继承它**实际拥有过**的历史，不该继承一个"它当时还不存在的未来"。路径在那里是**正确答案**，本 ADR **不动它**。

`Cellrix:web/tests/chain_merge_test.js:200` 已用变异断言钉死：

```
MUTATION: the subtree walk would merge the sibling
```

⇒ 子树 BFS 若回归，这条会红并点名原因。

### 3.2 生产调用只有一个，且用法正确

`chainJobIds` 的**非测试调用点**全仓只有一处（2026-10-09 实测）：

| 调用点 | 用途 | 判定 |
|---|---|---|
| `Cellrix:web/assets/script.html:156`（`loadWindow`） | 组装窗口 = 上下文加载 | ✅ 正确 |

`panel_tree.js:279`、`session_list.js:296` 对 `chainJobIds` 的提及都在**注释**里，不是调用。

### 3.3 风险是**前瞻的**，不是既有的

`Cellrix:web/assets/session_list.js:296` 明确计划：

> 「The next step turns cards into **threads**, reusing the existing `chainJobIds` traversal.」

**threads = 展示分组**，它要的是 `slice`，不是 `lineage_path`。若照该注释实施，就会把概念 2 用在概念 3 的位置 —— **这正是"混用"的入口，也是 M3 的 I5 要防的点。** 本 ADR 在实施前把它标出来。

### 3.4 与 `ADR-0006` 的关系（收窄词义，不覆写原文）

`ADR-0006` 标题为「会话即经历」。本 ADR 生效后：

- 「**经历**」由 `episode`（跨轮次的经历边界）承载 —— `ADR-0006` 的实质主张**不变**；
- 「**会话**」一词被本 ADR **收窄**为 `slice`。

`ADR-0006` 正文**不回改**（Active 不可覆写），词义收窄由本 ADR 承担 —— 与 `ADR-0041` §7「原文留痕、修订另附」同一体例。

---

## 四、影响

### 4.1 命名欠账（**真实成本，落地见 M2**）

下列标识符用 "session" 指称 `period` 或"经历面板"，与本裁决不一致。**本 ADR 只立词义，不改码**（Anaphase 处于 CI 零余量冻结窗内，见 `DSH-冻结档-2026-10-10.md` §三）：

| 现有标识符 | 实指 | 规范词 | 处置 |
|---|---|---|---|
| `anaphase:src/session_events/`（含 `SessionEventStream`） | 每 `period` 一个事件流 | `period_events` / `PeriodEventStream` | **M2 候选**（冻结后再议） |
| `anaphase GET /v1/sessions` | 返回 `period` 摘要列表 | `GET /v1/periods` | **M2 候选**（改面须单独立 ADR） |
| `Cellrix:web/assets/session.html` / `session_list.js` | 经历面板 + 列表 | 面板名，非领域词 | 保留（属 UI 命名，不进入领域词汇） |

> **不做的事**：本判**不**授权新增任何「面」（端口 / 端点 / 代理）—— 改名 ≠ 新建，且新增面本身是六条硬禁令之一。

### 4.2 对后续里程碑的约束

- **M3 / I5**：切片断言 = "从选中根出发的可达闭包"，**与 `lineage_path` 是两条断言**，各自独立。
- **M2 验收**：确认展示层**没有**用 `lineage_path` 冒充 `slice`（当前实测：无既有混用；`session_list.js:296` 是实施前的拦截点）。
- **M7（UI 解冻）**：DAG 展示以 `slice` 为准，不以"整棵子树"或"血缘路径"替代。

---

## 五、状态

**Accepted — 2026-10-09。** 三词定义自本日起生效；命名欠账（§4.1）转入 M2，CI 冻结（至 2026-10-11）解除后统一处置。
