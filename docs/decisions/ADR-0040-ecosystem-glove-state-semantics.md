# ADR-0040: 生态灯四态语义 + 生态事实单一来源 + 工具索引式披露

- **状态**: Accepted（2026-09-15 用户拍板：采纳诊断、驳回处方，四案按此冻结）
- **日期**: 2026-09-14
- **决策范围**: anaphase-helix（`gloves.rs` / `main.rs` 生态块与工具意识块）／ Cellrix（面板生态板）
- **关联**: ADR-0002（零硬编码）、ADR-0016（编排哲学 D3 生态点亮）、ADR-0023（按需加载落点）、
  ADR-0039（判断点单一来源，同属"一个事实一个来源"）
- **取代**: 无（扩展 ADR-0016 D3 的探测语义；D3 的「一次物理探测、0 token、无轮询」不变）

## 1. 背景与问题

### 1.1 用户已定的灯语义（照抄，不改动）

**绿** = 已联通 ／ **黄** = 正在连接 ／ **红** = 错误或失败 ／ **灰** = 未检测到。
手套为空则**不亮**；**所有灯通用**；**鼠标可点击**看到实际上有多少手套。

### 1.2 现状与这套语义的差距（全部实测）

**差距一：状态枚举一个值承担两种含义。**
`anaphase-helix/src/gloves.rs:19-26`：

```rust
pub enum GloveStatus { Unknown, Available, Unavailable }
```

- `gloves.rs:141`：**端点未配置** → `Unavailable`
- `gloves.rs:176-178`：**TCP 连不上** → `Unavailable`
- `gloves.rs:184-186`：**UDS 文件不存在** → `Unavailable`

「未配置」（用户要的**灰**）与「连不上」（用户要的**红**）写的是同一个值。
而 `Unknown`（本可作为灰的槽位）**没有任何生产者**——是个死变体。

**差距二：「黄 = 正在连接」在当前形态下无法表达。**
`probe_endpoint`（`gloves.rs:168-188`）是**同步阻塞**的一次性 `TcpStream::connect_timeout`
（400ms，`gloves.rs:176`），塞在一个 `async fn` 里（`probe_ecosystem`，`gloves.rs:126`）。
没有"在途"这个状态可言，且会阻塞 runtime 线程（5 个端点 × 400ms）。

**差距三：探测点早于自己的监听点 ⇒ cellrix 恒不亮。**
`main.rs:53` 调 `build_agent()` → `main.rs:698` 执行 `probe_ecosystem`；
而 CAP listener 在 `main.rs:523` 才 `TcpStream`/`TcpListener::bind`。
`cellrix_probe_target`（`gloves.rs:159-165`）的兜底目标恰恰是
**anaphase 自己的 CAP 端口**（`127.0.0.1:{cap_http_port}`）——
在它 bind 之前探测它，**恒 `ECONNREFUSED`** ⇒ cellrix 恒不亮，而它其实活着。
按用户定的语义，这不是「灰」而是「红」——**组件活着却报未检测到，是错的信号**。

**差距四：同一个事实有三个来源，且词汇互不相同。**

| # | 位置 | 来源 | 条目数 | 状态词汇 | 去向 |
|---|---|---|---|---|---|
| ① | `gloves.rs::probe_ecosystem` | TCP 探测 | 5（cellrix/tentacle/mind/tuck/flowmodus） | `Unknown/Available/Unavailable` | AgentSnapshot + 一处 `warn!`，**不进 prompt** |
| ② | `main.rs:1078-1097` | **读配置**（不探测） | 5（mind/flowmodus/tentacle/tuck/cellrix） | 无状态，只有 `name@endpoint` | **进 prompt** |
| ③ | Cellrix `web/src/server.rs:220-276` | 面板**自己再探一遍** | 5（tentacle/mind/anaphase/tuck/**panel**） | `ok / starting / off` | 面板生态板 |

三份清单的**成员不同**（②③ 无 cellrix、③ 无 flowmodus、③ 多 panel），
**状态词汇三套**，**面板与 anaphase 各探各的**。

**差距五：按需披露是反的。**
`main.rs:1017-1046` 把每个工具的**完整签名 + 描述**全部倒进 prompt：

```rust
let sig = if params.is_empty() { name.clone() } else { format!("{}({})", name, params.join(",")) };
let line = format!("{}: {}", sig, desc);
```

按 source tag（fixture / mcp / other）分组后用 ` | ` 连接成一整块。
这是**「全倒出来」，不是柜桶**——与「按需加载：知道柜桶里有什么，需要时再打开」相反。

## 2. 决策

### D1｜四态状态机，语义由枚举承载，颜色由视图映射

```rust
pub enum GloveStatus {
    Unknown,     // 灰 —— 未检测到：未探测，或端点未配置
    Connecting,  // 黄 —— 正在连接：探测在途
    Available,   // 绿 —— 已联通
    Failed,      // 红 —— 错误或失败：已配置但连不上 / 探测报错
}
```

- **删除 `Unavailable`**（一个值两义的源头）。
- `Unknown` 从死变体变成**未配置**与**未探测**的正式表达。
- **颜色不写进枚举**（`GloveStatus` 只出语义）；颜色是**视图层**的映射，映射表放视图侧一处。
  ⇒ 0 硬编码：换配色不改协议。

### D2｜探针改异步事件驱动

`probe_ecosystem` 从「一次性同步 connect」改为**异步探测**：
先置 `Connecting`（黄），完成后 resolve 为 `Available` / `Failed`。

- 修掉同步阻塞（当前 `connect_timeout` 会阻塞 runtime 线程）。
- **不轮询**：无事件时零能耗。触发点是「启动就绪」与「显式刷新」，不是定时器。
- ADR-0016 D3 的「0 token、无持续轮询」不变。

### D3｜探测点必须在自己的监听点之后

探测移到 CAP listener `bind` 之后（或由「服务就绪」事件触发）。
同时 `cellrix_probe_target` 的兜底改为**读配置**，与其余组件**同一规则**——
组件自己探自己是一种自引用，删掉它，缺陷三自然消失，不需要特例。

### D4｜生态事实单一来源：状态由 Anaphase 的探测唯一产生

**更正一处诊断错误**：①（探测结果）与 ②（配置声明）**不是同一件事的两份副本，是两个事实**。

| 事实 | 谁产生 | 确定性 | 去哪 |
|---|---|---|---|
| **配置声明**：我被告知要和谁说话 | 配置 | 确定性、可回放 | **进 prompt**（②保留） |
| **可达性**：它现在活着吗 | `gloves.rs` 探测（①） | 运行时、非确定 | **只进灯 / 快照 / 只读面** |

⇒ 因此：

- **状态不进 prompt**。把探测结果塞进 prompt，会让同一输入在不同机器状态下得到不同 prompt，
  证轨不可回放。这是「确定性优先」对「状态可视化」的硬边界。
- **真正冗余的只有 ③**：面板自己再探一遍。③ 退役——面板改读 Anaphase 的只读面，
  `ok/starting/off` 词汇退役。
- 新增只读面 `GET /v1/agent/gloves`（存在性：每项名称 / 层级 / 状态 / 计数），
  这是「鼠标可点击看到有多少手套」的数据来源。
- **成员清单以 SSOT 为准**。`helix-mind/docs/helixECO/ECOSYSTEM.md`（v1.94，自述 Helix 生态唯一真相源）
  是生态成员的唯一来源，其余各处是它的投影。该 SSOT 自身若存在多份互相打架的清单，
  **先收敛 SSOT**，再谈投影一致。

### D5｜存在性与能力是两条事实，分开呈现

- **存在性**（端口可达）：`gloves.rs` 探 → 决定灯的颜色。
- **能力**（有哪些手）：Tentacle `list_tools` → 决定展开后的清单。

两者**不互相冒充**（**端口亮 ≠ 有手**）。点击展开显示的是**能力清单**，
两条事实在界面上分别标注来源，不合并为一个值。

### D6｜按需披露：工具意识块改索引式

prompt 里只放**索引**：`name` + 一句话用途。
完整签名（参数表 + 描述）**按需取**——工具调用时由协议校验，或 `!tool NAME` 显式查询。

⇒ 直接对应「按需加载」：知道柜桶里有什么，需要时再打开。

### D7｜空则灭灯

手套集合为空 → 该组件的灯**不显示**（而不是显示灰）。

**判定边界（以 D1 为准，D7 不另立一套）**：

- **灰（`Unknown`）只留给「未配置」与「未探测」。**
- **已配置但探不到 = 红（`Failed`）**，不是灰。

> 本条原稿写「灰保留给『已配置但探测不到』」，与 D1 的「已配置但连不上 = 红」
> 对同一物理情形给出两种判定。按用户语义（红 = 错误或失败）**D1 对、D7 错**，已更正。
>
> 推论：**配额耗尽 / 认证失败 / 端点存在但拒绝连接**等「已配置但用不了」的情形，
> 一律归**红**，不归灰。灰只表示「我还没有关于它的任何物理事实」。

## 3. 备选方案与拒绝理由

| 方案 | 拒绝理由 |
|---|---|
| **A. 只加一个 `Connecting`，其余不动** | 不解决「未配置」与「连不上」同值（用户要的灰/红分不开），也不解决三份清单 |
| **B. 保留同步探测，用假状态凑出黄灯** | 黄灯需要一个真实的在途状态；用假占位填状态槽位违反「禁止造假占位」 |
| **C. 面板继续自己探** | 面板与 anaphase 各探各的 = 同一事实两个来源，且两者会给出**相反**结论（cellrix 恒不亮就是例证） |
| **D. 把存在性与能力合并成一个"组件可用"布尔** | 端口亮 ≠ 有手；合并后无法回答"它到底能做什么"，会再次造出一个一值两义 |

## 4. 放弃了什么

- **放弃了「探测一次就够」的简单性。** 四态 + 异步 + 就绪事件，比一次同步 connect 复杂。
  换来的是黄灯**成立**，以及 cellrix 不再假灭。
- **放弃了面板的自足性。** 面板不再自己探端口，改为读 Anaphase 的只读面 ——
  面板因此**依赖 anaphase 在线**才能显示生态状态。代价：anaphase 不在时面板生态板无数据
  （此时面板**必须显示"未知"而不是"全灭"**，否则又造出一个错误信号）。
- **放弃了把工具完整签名常驻 prompt 的"省一次查询"便利。** 索引式后，
  模型要调用工具时可能需要多一步取签名。换来的是 prompt 不被工具清单稀释。
- **放弃了 `Unavailable` 这个名字。** 它被三个地方引用（含测试），改名是一次原子重构。

## 5. 后果

**正面**
- 四色语义在枚举层成立，颜色是视图映射（0 硬编码）。
- cellrix 假灭（缺陷三）消失，且不需要特例。
- 生态事实从三份收敛为一份，成员与词汇一致。
- prompt 体积不再随工具数量线性膨胀。

**负面**
- 面板对 anaphase 产生读依赖（需正确处理"无数据 ≠ 全灭"）。
- 探测从同步变异步，`probe_ecosystem` 的调用点（`main.rs:698`）需改为事件驱动。
- 四态枚举改名触及 `gloves.rs` 与其测试（原子重构）。

## 6. 实施追踪

| 任务 | 仓 | 状态 |
|---|---|---|
| T1 `GloveStatus` 四态化（删 `Unavailable`，加 `Connecting`/`Failed`），同步改测试 | anaphase-helix | 待实施 |
| T2 `probe_ecosystem` 异步化 + `Connecting` 在途态 | anaphase-helix | 待实施 |
| T3 探测点移到 CAP listener bind 之后；`cellrix_probe_target` 兜底改读配置 | anaphase-helix | 待实施 |
| T4 生态块（`main.rs:1078-1097`）保留为**配置投影**（确定性、进 prompt）；**状态一律不进 prompt** | anaphase-helix | 待实施 |
| T5 新增只读面 `GET /v1/agent/gloves` | anaphase-helix | 待实施 |
| T6 面板生态板改读 Anaphase 只读面，删自探逻辑与 `ok/starting/off` 词汇 | Cellrix | 待实施 |
| T7 工具意识块改索引式（`main.rs:1017-1046`） | anaphase-helix | 待实施 |
| T8a 颜色映射表落到 **Cellrix**（视图层，一处） | Cellrix | 待实施 |
| T8b 回归网（**派生式**）：`GloveStatus` 变体 ↔ 颜色映射表 一一对应，无遗漏无多余 | anaphase-helix + Cellrix | 待实施 |
| T8c 回归网：生态块 **不得**含 `GloveStatus` 字段（状态不进 prompt 的守卫） | anaphase-helix | 待实施 |
| T9 A/B 验证：修复前 cellrix 恒 `Failed`（因 bind 顺序）→ 修复后 `Available`。**前提：配置中存在可达的 cellrix 端点**（D3 之后 cellrix 走配置；无可配置端点时正确结果是**灰**，T9 不适用） | 工作区 | 待实施 |

提交信息关联：`(ADR-0040 §T1)` … `(ADR-0040 §T9)`

## 7. 参考

> 跨仓引用一律**仓名限定**：`anaphase:ADR-0016`（编排哲学）与 `Cellrix:ADR-0016`（证轨资产解耦）
> **是两份不同的 ADR**；`ADR-0017` 同理（`anaphase:` CI-144 传输层 / `Cellrix:` 资产语言）。

- `anaphase:ADR-0016`（编排哲学 D3 生态点亮）
- `anaphase:ADR-0023`（按需加载落点）
- `anaphase:ADR-0039`（判断点单一来源，同一病根的另一面）
- `anaphase-helix/src/gloves.rs` / `anaphase-helix/src/main.rs`
- `Cellrix/web/src/server.rs:220-276`
- `helix-mind/docs/helixECO/ECOSYSTEM.md` v1.94（生态成员 SSOT）
