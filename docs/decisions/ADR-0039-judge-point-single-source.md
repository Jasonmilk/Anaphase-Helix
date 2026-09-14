# ADR-0039: 判断点单一来源——规则归 FlowModus，Anaphase 不再自算

- **状态**: Accepted（2026-09-15 用户拍板：采纳诊断、驳回处方，四案按此冻结）
- **日期**: 2026-09-14（2026-09-15 修订：补行为变更声明 / 组合函数 / 超时与错误码 / 降级可观测）
- **决策范围**: anaphase-helix（`judge.rs` / `adapters/mind.rs` / `run_cycle.rs`）／ FlowModus（`serve` 面，见 `FlowModus:ADR-0102`）
- **关联**: `anaphase:ADR-0002`（零硬编码）、`anaphase:ADR-0016`（编排哲学：确定性优先分诊）、`anaphase:ADR-0024`（判断点后端可配化）、`FlowModus:ADR-0102`（判定面）
- **取代**: **部分取代 `anaphase:ADR-0024` 的「失败回退 Rules」一条**（见 §4 放弃项）。其余条款（Judge trait、`judge_backend` 可配、SmallLlm 可选）不变，ADR-0024 保持 Accepted。

> **跨仓引用一律仓名限定**：`anaphase:ADR-0016`（编排哲学）与 `Cellrix:ADR-0016`（证轨资产解耦）
> **是两份不同的 ADR**；`ADR-0017` 同理（`anaphase:` CI-144 传输层 / `Cellrix:` 资产语言）。

## 1. 背景与问题

契约 `FlowModus/docs/engineering-manual/judge-points-contract.md`（v1.1-draft）规定：
**消费方 = Anaphase，提供方 = FlowModus**。JP-1 输出 `suggested_mode`（simple / moderate / complex），
JP-2 输出 `budget_tier`（endogenous / augmentable / exogenous）。

但同一个事实在生态里有 **三套阈值**，而且**互不相等**：

| # | 位置 | 长度阈值 | 输出 |
|---|---|---|---|
| ① | `flowmodus-rs/src/judge_points.rs`（契约指定的权威） | `skilled_len = 48` / `anchor_len = 192` | `SuggestedMode` / `BudgetTier` |
| ② | `anaphase-helix/src/judge.rs::RulesJudge` | 来自 `MindConfig`：**10 / 40** | `u8` 1/2/3 |
| ③ | `anaphase-helix/src/adapters/mind.rs::derive_suggested_mode` | 同 `MindConfig` 10 / 40（`complexity == 0` 时的兜底） | `CognitiveMode` |

⇒ 同一句话，两处会给出**不同的**复杂度。这不是"重复实现"，是**同一个事实的多个互相矛盾的来源**。

根因不是粗心，是**物理不可达**：FlowModus 的规则实现**没有机器可读面**——
`flowmodus-rs/src/serve_cmd.rs:71-73` 只路由 `GET /healthz` 与 `GET /api/status`，
`judge` 只存在于 CLI（`flowmodus judge <text>`）。Anaphase **拿不到**权威判定，于是自己重算了一遍。

现状链路（`run_cycle.rs:719-720`）：

```
self.judge.assess_complexity(&user_input)   // ② 本地重算
  → self.memory.set_complexity(tier)        // adapter 内 AtomicU8
  → 下次 query() 时 derive_suggested_mode   // ③ 又一次本地派生
  → 作为 suggested_mode 告知 Mind
```

**范围边界（本 ADR 不动）**：`helix-mind/crates/helix-mind-cognitive/src/value.rs::ValueAssessor`
（阈值 20 / 60，输出 `ValueGrade → Mode`）与 JP-1 同构，但它回答的是**"值不值得深想"**，
属 Mind 的「怎么想」域，不是「问题有多复杂」。它的归属另议，本 ADR 不触碰。

## 2. 决策

### D0｜本决策同时采用 FlowModus 阈值，属**行为变更**

收敛到唯一来源的代价是：阈值从 Anaphase 的 **10 / 40** 变为 FlowModus 的 **48 / 192**
（约 **4.8 倍**）。这是一个**行为变更**，不是纯重构：

- 绝大多数输入会被判为**更低**的复杂度档位（原 >40 字即 moderate，现需 >48；原 >40 即 complex 边界，现需 ≥192）。
- 直接影响：`suggested_mode` → 模型路由、`budget_tier` → 预算、以及**成本**。

⇒ 因此 **T0 双算 shadow run 是强制前置**（见 §6），不是可选项。
阈值生效日期必须落 judge-points 契约（v1.2）并写入生效日期——**现在全生态查不到任何数字，
行为变更就不可审计**。

### D1｜规则实现唯一来源 = FlowModus

`flowmodus-rs/src/judge_points.rs` 是 JP-1 / JP-2 规则的**唯一**实现。阈值只有 `JudgePointsConfig` 一处。

### D2｜FlowModus 暴露机器可读判定面

在既有 `serve` HTTP 面新增 `POST /api/judge`：请求 `{input}`，响应 `{suggested_mode, budget_tier, rule_version}`。
0 token、纯函数、无状态、按需触发（**不是**心跳探测，不违反 FlowModus 铁律 0）。
`rule_version` = 该份 `JudgePointsConfig` 的 hash（**回传 hash 而非阈值本身**——
既让判定可复现，又不把阈值变成第二份来源）。详见 `FlowModus:ADR-0102`。

### D3｜Anaphase 的 Rules 后端改为判定面客户端

`judge.rs` 保留 `Judge` trait 与 `SmallLlmJudge`（ADR-0024 的**语义**判断点能力，属 Anaphase 侧），
但 `RulesJudge` **不再持有启发式**，改为调用 D2 的判定面。阈值字段从 `RulesJudge` 移除。

### D4｜删除 query 派生的规则重算；负载是**独立字段**，不并入 `budget_tier`

- 删除 `adapters/mind.rs::derive_suggested_mode` 的**长度兜底**（那是 JP-1 的第三份拷贝）。
- 删除 `derive_budget_tier` 的 **query 派生部分**（那是 JP-2 的第二份拷贝）。
- **负载不并入 `budget_tier`。** 原设计是「FlowModus 判定 **叠加** 本地负载调制」——
  但 `budget_tier` 是三值枚举，两个来源合成一个字段而不给代数式，等于**把多来源从跨仓
  搬进了同一个字段内部**，而且让 `budget_tier` 依赖运行时负载 → **不可复现**。

⇒ 组合函数（具名纯函数，写死）：

```text
budget_tier      = FlowModus 判定（纯 input 派生，可复现，随 rule_version 归档）
load_gate        = Anaphase 自己的物理事实（sysinfo 系统负载），独立字段
effective_tier   = apply_gate(budget_tier, load_gate)      // 具名纯函数，见下

apply_gate(tier, gate):
    gate == 0        -> tier                       // 负载正常，不调制
    gate == 1        -> one_step_down(tier)        // 高负载，下调一档（ exogenous -> augmentable -> endogenous ）
```

- `one_step_down(Endogenous) == Endogenous`（已到底，不再降 —— 降级必须有底）。
- 阈值（`load_high`）来自 `MindConfig`，零字面量。

### D5｜降级链：无建议，不猜 —— 但**必须可见**

FlowModus 判定面不可用 / 超时 / 熔断时，Anaphase **不本地重算**，判为**无建议**。

**"无建议"必须与"判为 simple"可区分**，否则就是**静默降级**。落一条显式标记（三处同时）：

| 落点 | 内容 |
|---|---|
| 日志 | `judge_unavailable` + 原因（timeout / 熔断 / 错误码 / 解析失败） |
| 证轨 | 五元组（见下） |
| 面板 | 一处可见提示 |

**证轨五元组**（可复现的最小事实集）：

```text
{ input, flowmodus_verdict, rule_version, load_gate, effective }
```

`flowmodus_verdict` 为缺失时写协议默认空值（**不是** `"simple"`）。
`effective` 因 `load_gate` 调制而**不等于** `flowmodus_verdict` —— 这必须在记录里显式可辨，
不得把两者合并成一个字段。

### D6｜不做共享 crate，不做 gRPC

- **不做共享 crate**：会把两个独立仓的**构建**耦合起来（FlowModus 自述 "Zero Helix dependency"；
  反向的 crate 依赖会让 Anaphase 的构建需要 FlowModus 源码树在场）。两仓既有的唯一耦合方式
  就是网络面，保持不变。
- **不做 gRPC**：FlowModus proto（gossip / metrics / registry / routing / supplier）**没有 judge 服务**；
  新增要两仓 codegen 同步。HTTP 骨架已在，改动最小。

### D7｜客户端必须有超时 / 熔断 / 错误码表

判定面在**每轮推理的关键路径**上。`FlowModus:ADR-0102` 的 `serve` 是手写 std HTTP/1.1
（串行 accept），一次慢请求会挂住整轮。故：

- **超时**：判定请求超时来自配置（零字面量），默认短于推理超时。
- **熔断**：连续失败 N 次 → 打开熔断，窗口内不再请求（直接走 D5 无建议），半开后试探一次。
  N 与窗口来自配置。
- **错误码 → 消费方行为表**（两份 ADR 共引用）：

| 响应 | 消费方行为 |
|---|---|
| `200` + 合法 JSON | 采用判定 |
| `404` | 判定面未上线 → 无建议 + 熔断计数 |
| `413` | 输入超限 → **无建议**（**不得**截断后照常判定） |
| `5xx` | 无建议 + 熔断计数 |
| 超时 / 连接失败 | 无建议 + 熔断计数 |
| `200` 但 JSON 非法 / 字段缺失 | 无建议 + 熔断计数 + 记诊断（**不得**按默认值猜） |

## 3. 备选方案与拒绝理由

| 方案 | 拒绝理由 |
|---|---|
| **A. 收敛到 Anaphase**（FlowModus 只留 CLI） | 违背契约「提供方 = FlowModus」；且 JP-3/4/5（Rails 命中 / 命令解析 / 判据核验）本就在 Anaphase 侧，会变成两张权威表 |
| **B. 收敛到 Mind**（Mind 判定复杂度） | Mind 的职责是「怎么想」不是「问题有多复杂」；且会让 Mind 成为判定链的硬前置，与「按需驱动」冲突 |
| **C. 共享 crate**（两仓链接同一份实现） | 构建耦合；见 D6 |
| **D. 保留现状 + 加一致性测试** | 治标：三个阈值互不相等是**设计**问题不是漂移；测试只能断言三者相等，那等于要求三处同步修改——仍然是三个来源 |
| **E. 负载并入 `budget_tier`**（原稿 D4） | 一个字段两个来源 + 依赖运行时值 → 不可复现。已改为 D4 的独立 `load_gate` |

## 4. 放弃了什么

- **放弃了 FlowModus 离线时仍有复杂度判定。** 今天 Anaphase 能自己算（虽然算的是另一套数）；
  D5 之后 FlowModus 不在 = 无建议。这是刻意的：**猜出来的复杂度不是事实**。
  代价是 `suggested_mode` 会在 FlowModus 缺席时缺失——因此 FlowModus 必须进入标准启动序。
- **放弃了 `anaphase:ADR-0024` 的「失败回退 Rules」一条。** 该条保障随本 ADR 生效而消失
  （故本 ADR 标为**部分取代** ADR-0024，而非「取代：无」）。
- **放弃了原 10 / 40 阈值所隐含的分档行为**（D0）。这是一次真实的行为变更，需 shadow run 标定。
- **放弃了一次本地函数调用的延迟优势。** 判定改为一次 loopback HTTP 往返，且位于每轮推理之前。
  接受：本机 HTTP，判定是纯函数无 I/O，且有超时与熔断兜底。
- **放弃了对 `value.rs` 那份阈值（20 / 60）的顺手统一。** 它属 Mind 域，强行合并会把
  「怎么想」塞进「有多复杂」，是错误解耦。

## 5. 后果

**正面**
- 一个事实一个来源：复杂度阈值只剩 `JudgePointsConfig` 一处。
- 跨语言可消费：Cellrix（JS）可读同一判定，不必在 JS 里再写一套。
- 可审计：判定可被 Tuck 审计链 / 证轨记录（五元组 + `rule_version`）。
- 契约归位：契约说的「提供方 = FlowModus」在**物理上**成立，而不只是纸面成立。

**负面**
- FlowModus 从「可选组件」升级为「判定链上的必需组件」（缺席 → 无建议，**非阻塞但可见**）。
- 多一次网络往返，且需超时 / 熔断配套（否则关键路径被拖住）。
- 阈值跳变 ~4.8 倍，需 shadow run 标定后才可切换。

## 6. 实施追踪

| 任务 | 仓 | 状态 |
|---|---|---|
| **T0 双算 shadow run**：新旧阈值并行跑 N 条真实输入，输出 mode 分布差异 + 成本预估；差异超阈值先重标定再切 | anaphase-helix | **待实施（前置）** |
| T1 `serve` 新增 `POST /api/judge`（含 `rule_version`、超时、错误码） | FlowModus | 待实施（`FlowModus:ADR-0102`） |
| T2 `RulesJudge` 改为判定面客户端；阈值字段移除 | anaphase-helix | 待实施 |
| T3 删除 `derive_suggested_mode` 的长度兜底，改透传 | anaphase-helix | 待实施 |
| T4 `derive_budget_tier` 只产出 `load_gate`；`apply_gate` 具名纯函数落 `mind.rs` | anaphase-helix | 待实施 |
| T5 降级可见：日志 + 证轨五元组 + 面板提示（D5） | anaphase-helix + Cellrix | 待实施 |
| T6 超时 / 熔断 / 错误码→行为表（D7）；消费端可回退开关 | anaphase-helix | 待实施 |
| T7 回归网（**派生式**）：全仓不得再出现第二处 `skilled_len` / `anchor_len` 定义 | anaphase-helix | 待实施 |
| T8 阈值落 judge-points 契约 v1.2 + 写入生效日期 | FlowModus | 待实施 |
| T9 FlowModus 进入标准启动序并验证生态条点亮 | 工作区 | 待实施 |

**顺序硬约束**：`FlowModus:ADR-0102`（T1）必须先于 T2 上线并验证，
否则 T2 一合并就是**全量降级**。故本 ADR 与 `FlowModus:ADR-0102` **必须成对冻结**。

提交信息关联：`(ADR-0039 §T0)` … `(ADR-0039 §T9)`

## 7. 参考

> 跨仓引用一律**仓名限定**：`anaphase:ADR-0016` ≠ `Cellrix:ADR-0016`；`ADR-0017` 同理。

- `FlowModus/docs/engineering-manual/judge-points-contract.md`（v1.1-draft）
- `anaphase:ADR-0024`（判断点后端可配化 —— 本 ADR 部分取代其「失败回退 Rules」）
- `anaphase:ADR-0016`（编排哲学：确定性优先分诊）
- `FlowModus:ADR-0102`（判定面：把 judge-points 规则放到已有的 serve 面上）
- `anaphase:ADR-0040`（生态状态灯 —— 与本 ADR 同属"一个事实一个来源"）
