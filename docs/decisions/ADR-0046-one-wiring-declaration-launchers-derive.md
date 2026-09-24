# ADR-0046: 链路的接线事实只有一份声明 —— 启动器派生，不得复述

- **状态**: Proposed（待人类批准）
- **日期**: 2026-09-24
- **决策范围**: Helix 链的接线声明（`ecosystem/chain.json`）与所有启动器
  （`anaphase-helix/src/bin/up.rs`、`Cellrix/web/src/bin/up.rs`、`Cellrix/web/tests/start-panel.sh`）
- **关联**: `Anaphase:ADR-0011`（一条命令起全栈 · env 覆盖）、`Anaphase:ADR-0045`（一个端点字符串一种解释 —— **本 ADR 的前置**：端点可声明才有意义）、
  `Anaphase:DNA` 原则 10（生态兼容）、`Anaphase:DNA` 原则 11（零硬编码）；
  跨仓：`Tuck:ADR-0006`（审计链必须被配置）、`Cellrix:ADR-0045`（探针必须看得见状态行 —— 本 ADR 的验收依赖它）
- **引用约定**: ADR 编号按仓内序号递增；跨仓引用必须带仓名

---

## 1. 背景

### 1.1 实测：三个启动器，三套接线

审查当日逐行核对（真进程 + 源码）：

| 启动器 | 起 tentacle/mind | 起 flowmodus | 注入 `ANAPHASE_*_ENDPOINT` | 面板 `--flowmodus-url` |
|---|---|---|---|---|
| `anaphase-helix/src/bin/up.rs` | ✅ | ❌ | **✅（唯一的）** | — |
| `Cellrix/web/src/bin/up.rs`（`--restart`） | ✅ | ✅（双服务） | ❌ | ✅ |
| `Cellrix/web/src/bin/up.rs`（默认引导路径 `main`） | ❌ | ❌ | ❌ | ❌ |
| `Cellrix/web/tests/start-panel.sh` | ✅ | ⚠️ 只起 `serve`，**不起 gRPC Reason** | ❌ | ✅ |

⇒ **只有 anaphase 自己的 `up` 会闭合链路**，而它不是人平时用的那条（面板的 `up` 才是）。

### 1.2 为什么"六个端口全亮"却是开的

`Anaphase` 在启动时解析 tentacle/mind/tuck/flowmodus：**端点未配置 ⇒ 静默换 Noop 适配器，不报错**
（`src/adapters/mod.rs:195`）。于是：

```
实测（start-panel.sh，改动前）：
  tentacle  configured=false ok=true   not configured
  mind      configured=false ok=true   not configured
  tuck      configured=false ok=true   not configured
  flowmodus configured=true  ok=false  bad endpoint: grpc://127.0.0.1:60054
```

六进程健康、健康检查全绿、生态灯全灭或说谎 —— **环是开的，而没有任何一处说它开着。**

### 1.3 根因不是五个 bug，是一份事实被抄了三遍

"哪些组件、在哪些端口、注入哪些 env"**是一份知识**。三个启动器各自抄了一份，
于是必然漂移：一个抄漏了 flowmodus，一个抄漏了 env，一个抄漏了 gRPC 监听。

---

## 2. 决策

1. **建立唯一声明**：`anaphase-helix/ecosystem/chain.json`。每个组件声明
   `name / process / role / kind / port / order`，需要注入 Anaphase 的再声明
   `anaphase_env` + `anaphase_value`；另有三种可选字段：
   `health_path`、`note`，以及 **`start_env`** —— 组件**自己被启动时**需要的环境
   （例如 Tuck 的 `TUCK_GATEWAY__AUDIT_PATH`，见 `Tuck:ADR-0006`）。
   `start_env` 的值支持 `<workspace>` 占位符，因为声明里的事实可以是工作区相对的。
   **只放事实，不放 argv**：怎么拼命令是各启动器自己的机制（它们的 UX 不同），
   但"起什么、在哪、注入什么"不许各自回答。
2. **格式选 JSON，理由是可解析性**：三个消费者都已具备 JSON 解析器 ——
   `cellrix-web` 有 `serde_json`（既有依赖，**不新增**）、`anaphase` 有 `serde_json`、
   shell 侧有 `python3`（测试 harness 既有依赖）。**没有为"统一声明"引入任何新构建依赖**
   （`Anaphase:ADR-0022 §1.4`：借形状，不借容器）。
3. **声明放在 anaphase-helix**：它是链的枢纽，**唯一必须知道全部四个端点的组件**，
   且本仓已经是邻仓契约的持有者（`proto/helix_mind.proto`、`proto/tentacle.proto`、
   `proto/flowmodus.proto` 都已 vendored 在此）。链的接线声明放这里与既有形状一致。
4. **启动器只能派生**：不得复述声明里的端口/组件/env。判据见 §6。
5. **分期**：`start-panel.sh` 已转换（本 ADR 的第一笔）；两个 Rust 启动器随后。
   **验收判据的 `CONVERTED` 清单就是分期表** —— 转换一个，判据强一分，不需要新机制。
6. **顺带修掉一个真实缺口**：`start-panel.sh` 此前**从不启动 FlowModus 的 gRPC Reason**，
   却把 Anaphase 的推理入口指向它 —— 声明里有这个组件，派生即自动补上。

---

## 3. 理由

- **单一事实来源**：一份知识抄三遍，必然有两遍过期。本 ADR 的实测就是证据。
- **0 硬编码**：端口、组件集、env 名都不该出现在启动器里。
- **物理事实优先**：判据问的不是"启动器看起来对不对"，而是"它有没有复述一份已有的事实"。
- **极致复用**：一个声明 + 三个派生，而不是三份手抄 + 三次漂移。
- **可审计**：声明是 JSON，人和机器都能读；启动器的分歧一眼可查。

---

## 4. 不做什么（Non-Goals）

- **不**把启动命令（argv）搬进声明。各启动器的引导方式不同（面板要问用户、脚本要静默），
  强行统一会让声明变成第二个启动器。**声明是事实，不是脚本。**
- **不**引入新的配置格式或依赖（JSON + 既有解析器）。
- **不**改各启动器的 UX（引导问答、`--restart`、交互菜单）。
- **不**把面板端口写进声明：面板端口由启动器决定（`$1`/`--port`），
  而 `anaphase.cellrix_endpoint` 未配置时 `gloves.rs:160-164` 已有回退，故意留空。
- **不**在本 ADR 里决定"推理该不该经 Tuck"（跨仓架构决策，见 `Tuck:ADR-0004` D7）。

---

## 5. 影响

- **正向**：`start-panel.sh` 起出的栈**第一次是闭合的**（实测见 §6.1）；
  新增启动器/新增组件时，漏接会被判据抓住，而不是靠人记得。
- **代价**：多一个跨仓文件（`anaphase-helix/ecosystem/chain.json`）；
  `start-panel.sh` 依赖 `anaphase-helix` 检出（缺失时**明确报错**，不静默降级 —— 见 §6 SKIP 约定）。
- **风险**：声明一旦与实际二进制不符（例如某组件改名），启动器会照声明去起而失败 ——
  这是**响的失败**，优于现在"起不来但灯全绿"的静默失败。

---

## 6. 验收（判据必须能红 —— 铁律 9）

判据：`Cellrix/web/tests/chain_wiring_test.js`（入 `run_all.js` 回归网）。

| # | 判据 | 期望 |
|---|---|---|
| 1 | 声明自洽：每个组件有 name/port/kind/order | 无缺项 |
| 2 | 端口不重复 | 无重复 |
| 3 | `anaphase_env` 与 `anaphase_value` 成对 | 无半对 |
| 4 | 启动顺序是致密的 1..N | 致密 |
| 5 | **已转换的启动器不复述任何已声明端口**（可执行行内） | 0 处复述 |
| 6 | 已转换的启动器**消费** `anaphase_env` 字段（派生而非复述 env 名） | 引用该字段 |
| 7 | 已转换的启动器 `spawn` 到**每一个**声明组件 | 全覆盖 |
| 8 | 已转换的启动器把每个声明组件放进 `SERVICES`（`--stop` 才覆盖得到） | 全覆盖 |

**变异注入（3 处，逐条实测）**：

- 在可执行行里复述一个已声明端口（`wait_port 60052 tuck 15`）⇒ **判据 5 变红**（报出 `tuck:60052`）；还原 ⇒ 转绿。
- 把 `spawn mind` 改名 ⇒ **判据 7 变红**；还原 ⇒ 转绿。
- 判据自带**合成输入自检**（铁律 9 的「结构性自检」）：坏输入（复述端口）必须被报、好输入（派生端口）必须不被报、注释里的端口必须被忽略。

> ⚠️ **一处已修正的弱判据（留痕）**：判据 7 的第一版搜"组件名是否出现在文件里"，
> 于是把 `spawn mind` 改成 `spawn mindX` **仍然通过** —— 因为 `helix-mind` 里含有 `mind`。
> **一个碰巧在别处出现的子串，不是"这个组件被启动了"的证据。** 改为解析真正的
> `spawn <name>` 调用后，变异才被抓住。这正是铁律 9 说"判据要能红"的现场例子。

### 6.1 实测记录（2026-09-24，本机）

`./start-panel.sh 18932`（声明派生版）：

```
--- what Anaphase actually wired (its own /v1/health) ---
  tentacle   configured=true  ok=true   ok
  mind       configured=true  ok=true   ok
  flowmodus  configured=true  ok=true   ok
  tuck       configured=true  ok=true   ok
  governance.state = indeterminate        （改动前：ungoverned）

--- ecosystem ---
  cellrix=Available, tentacle=Available, mind=Available, tuck=Available, flowmodus=Available

一次真实回合：
  State transition: MemoryRetrieval --Success--> Reasoning
  [Reasoning] inject_chars=800 memory_nodes=4        ← 记忆真的召回并折进了 prompt
  WARN Reasoning failed: Aborted "…agnes-ai.com/v1/chat/completions: 403"
  （失败只在上游凭据，不在接线）
```

对照改动前：`tentacle/mind/tuck configured=false`、`flowmodus ok=false bad endpoint`、
生态灯全灭。⇒ **同一个脚本，从"六进程健康但环是开的"变成"四个端点全部接通"。**

### 6.2 跨仓缺失时的行为

声明在 `anaphase-helix` 仓内。若该仓未检出，`start-panel.sh` **明确报错并退出**，
判据则按 `NEEDS-INPUT`（退出码 3）登记为 SKIP 并写明原因 —— 不静默降级
（与 `Cellrix:ADR-0045` 的 SKIP 约定同一条纪律）。

---

## 7. 分期（本 ADR 的开放项）

| 期 | 内容 | 状态 |
|---|---|---|
| **C1** | 声明 + `start-panel.sh` 派生 + 判据入网 | ✅ **本笔** |
| **C2** | `Cellrix/web/src/bin/up.rs` 派生（含默认引导路径补齐组件 + 注入 env + `start_env`） | ✅ **已落地** |
| C3 | `anaphase-helix/src/bin/up.rs` 派生 | ⏳ |
| C4 | `session_events_path` 进声明/覆盖面（证轨白盒，见 `Anaphase:ADR-0045` §6.2） | ⏳ |

**分期不靠人记**：判据的 `CONVERTED` 清单是本表的机器可读版本；每转换一个启动器，
判据的覆盖面随之扩大，漏接即红。
