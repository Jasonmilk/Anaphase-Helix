# ADR-0045: 一个端点字符串，只能有一种解释 —— scheme 归一化 + 覆盖面无缺口

- **状态**: Proposed（待人类批准）
- **日期**: 2026-09-24
- **决策范围**: Anaphase 的端点解析（`src/health.rs` / `src/gloves.rs` / `src/adapters/tentacle.rs` / `src/main.rs` 的适配器选择）与配置覆盖面（`src/config.rs` 的 env 覆盖）
- **关联**: `Anaphase:ADR-0011`（一条命令起全栈 · env 覆盖）、`Anaphase:ADR-0040`（生态灯四态语义 · 生态事实单一来源）、`Anaphase:DNA` 原则 10（生态兼容 · `src/adapters/` 唯一 IO 边界）、`Anaphase:DNA` 原则 11（零硬编码）；跨仓：`Cellrix:ADR-0045`（探针必须看得见状态行 —— 判据层）、`Tuck:ADR-0006`（审计链必须被配置且可验证）
- **引用约定**: ADR 编号按仓内序号递增；跨仓引用必须带仓名

---

## 1. 背景

### 1.1 实测：同一个字符串，两个相反的结论

审查当日，真 `anaphase` 进程的 `/v1/health`：

```
flowmodus  configured=true  ok=false  bad endpoint: grpc://127.0.0.1:60054
tentacle   configured=false ok=true   not configured
mind       configured=false ok=true   not configured
tuck       configured=false ok=true   not configured
overall ok = False        governance.state = ungoverned
```

而同一个 `grpc://127.0.0.1:60054` **正是推理链路的要求**——`src/main.rs:735` 用
`endpoint.starts_with("grpc://")` 来选中 gRPC 适配器（上一笔提交 `6d3b1c2`
"修复 GrpcFlowModusAdapter 缺 scheme（对话契约断链最后一环）"刚把它设成这个形状）。

⇒ **健康面说这条腿是坏的，推理面说它必须长这样。** 同一个字段，两个解析器，两个结论。

### 1.2 根因：同一个字符串有**三个**解析器

| 位置 | 解析方式 | 对 `grpc://127.0.0.1:60054` 的结论 |
|---|---|---|
| `src/main.rs:735` | `starts_with("grpc://")` + `&endpoint[7..]` | **认得**（并硬切 7 字节） |
| `src/health.rs:21-29` | `trim_start_matches("http://"/"https://")` | `"grpc:"` ⇒ **bad endpoint** |
| `src/gloves.rs:168-180` | 再一份 `http/https/unix` 剥离 | 落入路径分支 ⇒ **Unavailable** |

实测的生态灯印证第三行：`/v1/agent/snapshot` 的 `ecosystem` 五项**全部 `Unavailable`**。

⇒ 三处各自拥有"怎么读一个端点"的知识，其中两处漏了 `grpc`。这不是三个 bug，是**一个知识被抄了三遍**。

### 1.3 第二个缺口：覆盖面不完整

`apply_env_overrides()` 只认 `TENTACLE` / `REASONING` / `MIND`。而 `config.toml` 被 gitignore
⇒ 换机重建会静默丢掉 `tuck_endpoint`（**既是审计腿、又是 fail-closed 闸门**）与
`flowmodus_endpoint`（推理入口），而**启动器没有任何通道**能补上它们。

⇒ 一个"启动器是链路唯一声明处"的设计，**只有在每个端点都可从文件外声明时才成立**。覆盖面不完整，声明处就必然回落到"手工编辑未跟踪文件"。

---

## 2. 决策

1. **一个解析器**：`health::endpoint_authority(endpoint) -> &str` —— 剥掉**任意** `scheme://`
   与路径，返回 authority（`host:port`，或以 `/` 开头的 Unix socket 路径）。
   `health` 与 `gloves` **都用它**；`gloves` 不再保留自己的剥离逻辑。
2. **scheme 仍可用来分派，但不再有第二个解析器**：适配器选择按 scheme 分派是**正当的**
   （scheme 就是通道的声明），判据是 `!endpoint.starts_with("http://")` ⇒ gRPC，
   `http://` ⇒ 旧 HTTP 适配器（其自身已声明 deprecated，退役归 `DEPRECATE.md`）。
   本 ADR **不**引入 `endpoint_scheme()`：它只服务一个调用点，而 `health.rs` 的行预算**零余量**
   （见 §6.3）。多一个 5 行的公开函数只为省一次 `starts_with` 不划算；**解析**共用才是本 ADR 的要点。
3. **删掉 `&endpoint[7..]`**：`len("grpc://")` 这个量原先是硬编码的第二份，紧挨着已经知道它的解析器
   （`0 硬编码`）。现在分派只问「是不是 http」，不再自己数前缀长度。
4. **适配器也走同一个解释**：`GrpcTentacleAdapter::new` 把任意写法归一成 tonic 能接受的
   `http://<authority>`。修前它与 FlowModus 适配器**不对称**：后者两种写法都收，前者只收带 scheme 的。
5. **覆盖面补全**：新增 `ANAPHASE_TUCK_ENDPOINT`、`ANAPHASE_FLOWMODUS_ENDPOINT`、
   `ANAPHASE_CELLRIX_ENDPOINT`。启动器从此可以声明**整条链**而不碰任何文件。
   （空值忽略的既有权衡不变；机密不在本仓，故不涉密。）
6. **顺手清掉仓库里的注入残留**：`src/health.rs` 末尾有 `// SYNTAX ERROR TEST` 与
   `// change marker 1788769106`（`5626c38` 带入，全生态仅此一处、无任何引用）。
   变异注入的**还原半边**没做干净。删除。

---

## 3. 理由

- **单一事实来源**：端点的读法是一份知识。抄三遍就会有两遍过期——本 ADR 的实测就是证据。
- **物理事实优先**：`grpc://` 与 `http://` 底下的 socket 是同一个 TCP 地址；让健康面按 scheme 判死活，是在拿**命名**冒充**事实**。
- **极致复用**：`gloves` 复用 `health` 的解析器，而不是"两份看起来一样的代码"。
- **0 硬编码**：`&endpoint[7..]` 是第二份 `len("grpc://")`。
- **生态灯先说真话**：灯坏了没人知道；灯**说谎**会让人去修不存在的问题（本 ADR 的起点正是如此）。

---

## 4. 不做什么（Non-Goals）

- **不**引入 `grpcs://` / mTLS。传输层仍由 tonic 决定（当前 `http` 之上）。
- **不**删除 deprecated 的 HTTP `FlowModusAdapter`。它的退役由 `DEPRECATE.md`（DEP-002）决定，
  本 ADR 只保证它**只**在显式 `http://` 时才被选中。
- **不**改 `decide()` / 硬实时路径。
- **不**把 `unix://` 从探测路径里去掉（UDS 是 Mind 的既有姿势）。
- **不**在本 ADR 里解决"推理该不该经 Tuck"——那是跨仓架构决策（见 §7）。

---

## 5. 影响

- **正向**：`/v1/health` 与生态灯第一次能如实报告一条**已接线**的链；`grpc://` 在四类消费点（health / lamp / reasoning / executor）里含义一致。
- **代价**：`endpoint_authority` / `endpoint_scheme` 成为 `pub`（二进制 `src/main.rs` 要用）。这是 lib 的公开面扩张，但换来的是"一份知识"。
- **风险**：以前被"http/https 之外一律当路径"**误判为 Unavailable** 的端点，现在会按 TCP 判死活；若某处真的把路径当端点传进来，它的灯态会变。已核对的写入点：`config.toml`、`up` 的注入（`http://…`）、以及本 ADR 新增的三个 env。

---

## 6. 验收（判据必须能红 —— 铁律 9）

| # | 判据 | 期望 |
|---|---|---|
| 1 | 单元：`endpoint_authority` 对 `grpc://h:p` / `http://h:p` / `https://h:p` / 裸 `h:p` / 带路径 / `unix:///p` / 带空白 | 全部得到 authority |
| 2 | 单元：`endpoint_addr` 接受生态实际写的每一种 scheme | 全 `Ok` |
| 3 | 单元（**反面对照**）：`endpoint_addr("not-an-endpoint")` / `("grpc://")` / `("")` | **`Err`** —— 证明"scheme 无关"没有退化成"什么都收" |
| 4 | 单元：三个新 env 覆盖（**一条测试三段**：文件值 → 覆盖 → 空值忽略） | 端点可全部从文件外声明 |
| 5 | 活体：四个端点**全部由 env 声明**、四个服务在听 | `/v1/health`：`tentacle`/`mind`/`tuck`/`flowmodus` 全 `configured=true && ok=true`；`ok=true` |
| 6 | 活体：生态灯 | `ecosystem` 五盏不再是 `Unavailable` |

**变异注入**：

- 把 `endpoint_authority` 还原成 http/https-only ⇒ **判据 1 与 2 必红**（实测报出 `left: "grpc:"`）；还原 ⇒ 转绿。
- 删掉任一新 env 覆盖 ⇒ **判据 4 必红**；还原 ⇒ 转绿。
- 判据 3 是**反面对照**：它证明本次放宽没有把判据变成橡皮图章。

### 6.1 实测记录（2026-09-24，本机）

**改动前**（真进程）：

```
flowmodus  configured=true  ok=false  bad endpoint: grpc://127.0.0.1:60054
tentacle/mind/tuck  configured=false ok=true  not configured
overall ok = False        governance.state = ungoverned
ecosystem: 五项全 Unavailable
```

**改动后**（四个端点**全部由 env 声明**；Tuck 带 `TUCK_GATEWAY__AUDIT_PATH`，FlowModus `grpc --port 60054`）：

```
tentacle   configured=true  ok=true  reachable
mind       configured=true  ok=true  reachable
flowmodus  configured=true  ok=true  reachable
tuck       configured=true  ok=true  reachable
overall ok = True         governance.state = indeterminate
ecosystem: cellrix/tentacle/mind/tuck/flowmodus 全 Available
```

**单元、集成与变异**（判据在 `tests/`，故 `--lib` 计数比把判据放在源文件里时少几条）：
`cargo test --lib` → **216 passed / 0 failed**；
`tests/endpoint_parsing.rs` → **3 passed**；`tests/env_overrides.rs` → **1 passed**（内含三段）。
变异（还原 scheme 剥离）⇒ `endpoint_authority_is_scheme_agnostic` 与
`endpoint_addr_accepts_every_scheme_the_ecosystem_writes` **变红**，报出 `left: "grpc:"`；还原 ⇒ 转绿。

### 6.2 一次真实回合（这是本 ADR 的**副产品证据**，也是下一阶段的起点）

`POST /v1/chat {"message":"reply with the single word: pong"}` 的真实路径：

```
Pipeline wired to Tentacle endpoint (deterministic execution channel active)
State transition: MemoryRetrieval --Success--> Reasoning
[Reasoning] inject_chars=800 memory_nodes=2          ← 记忆**真的**召回了 2 个节点并折进 prompt
[Reasoning] Left-brain reasoning...
WARN Reasoning failed: Aborted, "调用上游失败: https://apihub.agnes-ai.com/v1/chat/completions: 403"
周期以 impasse 结束（如实）
```

⇒ **链路的物理通路第一次被走通**：anaphase → FlowModus gRPC :60054 → 上游。
失败只在**上游凭据**（agnes-ai 返回 403），不是接线。

**同时暴露三件尚未闭口的事**（本 ADR 不解决，登记以免丢失）：

1. **Tuck 被旁路**：该回合之后 `GET /v1/audit` = `{"count":0,…}`，链文件 0 字节
   ⇒ 「唯一门」没记账。这是**跨仓架构决策**（`Tuck:ADR-0004` D7「其下游必经 Tuck」），
   需要人类裁决出口归属。
2. **`session_events_path` 未配置** ⇒ `/v1/events?job_id=…` = `{"configured":false,"missing":true}`
   ⇒ 面板的**证轨白盒**对这一轮没有数据。这是"闭环"的第三条未接线。
3. **经 Tuck 的推理路径已存在但未启用**：`reasoning_endpoint` 是**优先级 1**
   （`HttpReasoningAdapter` → Tuck `/v1/chat/completions`），当前配置没设它，于是落到优先级 2
   （FlowModus gRPC 直连）。⇒ 第 1 条的技术手段**已经在代码里**，缺的是**决策与配置**。

---

### 6.3 行预算棘轮：本 ADR 做到**零新增债务**

`anaphase` 的 `ci_line_budget` 把每个文件的 NCLOC 钉在历史最小值上（`min(previous, current)`，只降不升），
而本 ADR 要动的三个文件**余量都接近零**。做法是让每一处改动**净增 0 或由既有 allowance 吸收**：

| 文件 | 天花板 | 改动后 | 结论 |
|---|---|---|---|
| `src/health.rs` | 199（+1 既有 allowance） | 200 | **+1，被既有 allowance 吸收** |
| `src/config.rs` | 350 | 350 | **0** |
| `src/gloves.rs` | 246 | ≤246 | **≤0**（删掉自带剥离逻辑，变短） |
| `src/adapters/tentacle.rs` | 80 | ≤80 | **≤0** |
| `src/main.rs` | 941 | 941 | **0** |
| `src/adapters/flowmodus.rs` | 52 | 57 | **+5 —— 先于本 ADR，非本次引入** |

**A/B 实测（同一份脚本，两次运行）**：

```
A) 纯净 HEAD（本次改动 stash 后）：
   FAIL  1 file(s) grew past their ratchet:  src/adapters/flowmodus.rs  52 -> 57
B) 带本 ADR 的改动：
   FAIL  1 file(s) grew past their ratchet:  src/adapters/flowmodus.rs  52 -> 57
```

⇒ **两侧逐字相同**：本 ADR **没有新增任何棘轮债务**，因此**不需要 PIT、不需要 fix_window**。

**为此做的三处收敛**（都不是为了骗过预算，而是本来就该这样）：

- env 覆盖改成**闭包表**：从「每字段一对 `if let`」变成「一张表 + 2 个分支」，
  顺带把 `config.rs` 的 DATA-ONLY 分支数从 13 降回 ≤10（该 marker 曾被本次改动破坏）。
- 去掉多余的 `{ }` 作用域（NLL 已在最后一次使用处结束借用）⇒ `config.rs` 净增归零。
- 判据从源文件移出到 `tests/endpoint_parsing.rs` 与 `tests/env_overrides.rs`：
  **判据代码不是棘轮要约束的对象**，且 `tests/` 不在棘轮前缀集内。
- `health.rs` 里不新增只为单一调用点服务的 `endpoint_scheme()`（见 §2 决策 2），
  并把 authority 的 if/else 收成一行 ⇒ 净增压到 +1。

> ⚠️ **试过但走不通的路**：把解析器挪进新文件 `src/endpoint.rs`。检查器直接报
> `src/endpoint.rs: no [[seed]] entry … A seed is a decision` —— 新文件需要一条**更强**的治理行为
> （seed = 定基线）。净增为零的原地收敛比新文件更省，故不采用。

### 6.4 本轮顺带查出的两件事

1. 🔴 **`run_cycle_pipeline` 在纯净 HEAD 上就是抖的**：连跑 3 次，15 条里失败 **1 / 5 / 4** 条不等
   （`git stash` 后同样抖，与本 ADR 无关）。按 `Anaphase:DNA` 的「确定性优先」，
   **抖的判据比没有判据更糟**；而 `BUILD.md` / `HANDOFF.md` 只记录了 K-111（`ci_guards_policy`）。
2. 🔴 **`ci_line_budget` 会改写自己的基线**：`ci/ratchet.gen.toml` 由检查器在运行时重写
   （"GENERATED — do not edit"），跑一次就变；且 HEAD 里提交的值与实测已不一致
   （实测中 `src/gloves.rs` 242 vs 文件里的 246、`src/adapters/tentacle.rs` 79 vs 80）。
   一个**边判边改自己基准**的闸门需要单独审视 —— 本 ADR 的工作区里该文件已复原，不把生成漂移混进功能提交。



## 7. 与链路闭环的关系

本 ADR 交付的是"**四个端点可以被一个启动器完整声明，且四个消费点对同一个字符串的理解一致**"。
它本身不接通上游凭据，也不决定出口归属。

顺序：`Cellrix:ADR-0045`（判据）→ `Tuck:ADR-0006`（账本）→ **本 ADR**（链路声明与解释）
→ 出口归属裁决（跨仓）→ `session_events_path` 接线 → E2E 判据。
