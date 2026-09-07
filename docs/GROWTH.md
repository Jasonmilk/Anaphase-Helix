## [2026-09-07] 完成：P10 注入打通（ADR-0033 联动）

### 变更性质
- L3 经历化：Reflection note 改 `User said: {}\nCycle completed. …`（记"这一轮经历了什么"，不是账本行）
- 注入折叠：fold 剥离 `\nCycle` 账本尾行（provenance 不进 prompt）；标签 `[memory: Helix's past experiences — true history, answer from them]`
- serde 默认修复：`memory_inject_chars` `#[serde(default)]` 对 usize 反序列化为 0 的 bug（协议默认 800 只在 impl Default）→ 单一常量 `DEFAULT_MEMORY_INJECT_CHARS`，Default 与 Deserialize 同源（0 硬编码）
- 诊断：`[MemoryRetrieval] N memory node(s)` 预览 + `inject_chars` 一行日志（白盒审计）
- daemon 保活：`sh -c 'nohup … &'` double-fork（直接 nohup & 在 bash 退出后不稳定）

### 验收
端到端：`"我叫什么名字？"` → 14 memory node(s)（User said 排前）→ "你叫Jason。"；新增 `fold_strips_bookkeeping_tail_keeps_experience`；228 passed 0 failed。


## 记录 25：Helix-Mind 物理打通（live）（2026-09-07）

### 触发条件
用户实测"Helix 还是 DeepSeek，并没有真正连接 Helix-Mind"——排查发现 Mind 服务一直在跑（:50052，24 节点），但 Anaphase `mind_endpoint` 为空 → Noop 离线，对话从不触达潜意识层。

### 变更性质
- config.toml `mind_endpoint = "http://127.0.0.1:50052"`（gitignore 保护，本地生效）
- 重启 anaphase daemon + web 面板；**物理验证全链**：glove `mind: Available` → `[MemoryRetrieval] Querying memory`（helix_query 真实 gRPC）→ `craft note (0 tokens)`（认知工艺确定性编排）→ Reflection `memory.remember` → Mind L3 append
- **诚实缺口**：检索 FTS5 短语匹配（"我叫Jason你记得我吗" 与库中"你好,我是Jason" 整句不匹配→零命中）；语义 onnx 模型未加载。写入全通、读取召回待 P10 增强

### 状态
🧬 已完成（写入链路）｜ 召回增强待 P10

## 记录 24：P10d 预约制闹钟——Anaphase 唤醒侧接线（2026-09-06，ADR-0032）

### 触发条件
Mind 侧 ana_wakeup RPC 完成后，Anaphase 侧唤醒发起接线（每交互看表一次）。

### 变更性质
- **MemoryAdapter 三方法**：wakeup（列出 due）/ wakeup_ack（done/renewed）/ consolidate（helix_consolidate kind）——默认 Err 静默降级，GrpcMindAdapter 真实实现
- **run_cycle 入口 check_wakeup**：白名单 action → consolidate 链 → ack done；未知 → ack done 释放（不执行但释放，永不死锁）；失败 → ack done 记录；不可用 → 跳过
- **RunCycleConfig**：wakeup_enabled（默认 true）/ wakeup_jitter_minutes（默认 60）/ wakeup_actions（默认 [hibernate]），serde default 保老 TOML 兼容
- **测试**：+4（触发执行/白名单外/无预约/降级），21 套件全绿 0 warning，198→202

### 兼容性
零破坏：proto Append-Only；新字段 serde default；Noop 零改动静默降级。

### 验收
PLAN（P10d 段）｜ README（202 tests）｜ ECOSYSTEM v1.51（Anaphase 202，全生态 1414）

### 状态
🧬 已完成
## 记录 25：P10 收尾——gRPC 级闭环 + live 物理验证（2026-09-06）

### 触发条件
P10d 接线完成后收尾核对：发现 gRPC 级闭环测试缺失（MockMind 有 stub 无测试），补上真实通道验证。

### 变更性质
- **mind_integration +3**：craft_via_grpc（确定性 trace）/ wakeup+ack_via_grpc（due alarm 走真实 wire + ack 到达 mock）/ consolidate_via_grpc（睡眠复盘链）
- **mind_live.rs 新增（#[ignore] 手动联调）**：起真实 helix-mind-cli 二进制（临时 config + 临时库 + 随机端口）→ GrpcMindAdapter 真实客户端 → craft/wakeup/consolidate 全链路**物理验证通过**
- **测试**：202→205（+3 gRPC 级）+ 1 live（ignored），22 套件全绿 0 warning

### 兼容性
零破坏；live 测试不进入常规套件（需 Mind 二进制，文档写明运行方式）。

### 验收
README（205 + P10 live 段）｜ PLAN（gRPC + live）｜ ECOSYSTEM v1.52（Anaphase 205，全生态 1417）

### 状态
🧬 已完成
## 记录 26：命名纪律——阶段号测试名全量改能力名（2026-09-06）

### 触发条件
用户审查 p10_live.rs 命名后确立硬规则：**内部命名必须准确无歧义、按能力命名，经用户通过方可保留**；阶段代号（M1/M1.5/P10/D4）不得用作测试文件名。

### 变更性质
| 原名 | 新名 | 被测能力 |
|---|---|---|
| m1_e2e_live.rs | tentacle_live.rs | 真实 Tentacle 连通 + fixture 判据 |
| m1_5_d4_live.rs | plugin_live.rs | MCP 学习产物真实插件执行 |
| m1_e2e.rs | pipeline_e2e.rs | pipeline 双 mock 闭环 |

函数同步去阶段前缀（m1_5_live_met→tentacle_live_met 等 9 处）；README/PLAN/ADR-0004/0005/0009 现存引用全量修正（archive 历史记录保留原名——当时事实）。

### 兼容性
零行为变化（纯改名）；`git mv` 保留历史。

### 验收
22 套件全绿 0 warning；live 三套真实联调全绿：tentacle_live 3/3 + plugin_live 3/3（真实 tentacle 二进制）+ mind_live 1/1（真实 Mind 二进制）。

### 状态
🧬 已完成
## 记录 27：up 全栈——潜意识层接入 + 真实见面冒烟（2026-09-06）

### 触发条件
用户问"测试是否全栈跑通、想和 Helix 见面"→ 发现 up 只启 Tentacle+Anaphase，缺 Mind（潜意识层），全栈差核心一环。

### 变更性质
- **config.rs**：env 覆盖加 `ANAPHASE_MIND_ENDPOINT`（12-factor 同款，+1 单测）
- **up.rs**：Mind 一键接入——`HELIX_MIND_BIN`/`HELIX_MIND_CONFIG` env 覆盖 > Helix-Mind/config.toml > `.helix/mind/` 最小默认配置（gene_lock 来自仓库 example，永不臆造）；端口从 config 解析、与 Tentacle 冲突自动 +1（零硬编码）；Mind 缺失 fail-open（无潜意识不阻塞意识层）；preqreq 检查 + 测试同步（+Mind）
- **真实冒烟验证**：三进程全起（Tentacle :50051 + Mind :50052 + Anaphase :50061 partner）→ snapshot 显示 tentacle/mind **Available**（物理探活）→ events 白盒完整记录 run_cycle 状态机迁移（Perception→PreAssessment→MemoryRetrieval→Reasoning，trace_id 确定性派生）
- **测试**：205→206（+mind env override），22 套件全绿

### 兼容性
零破坏：Mind 缺失 fail-open；新 env 可选；up 菜单/非 tty 行为不变。

### 验收
README（up 全栈 + Mind prereq）｜ PLAN（全栈）｜ ECOSYSTEM v1.54（Anaphase 206，全生态 1418）

### 状态
🧬 已完成
## 记录 28：驾驶舱真身——stdio 全栈装配 + WebUI 一键接入（2026-09-06）

### 触发条件
用户配好 LLM 后启动 up：驾驶舱主面板是 mock-agent（演示），Anaphase 只是右下角只读投影；WebUI 没起。用户预期驾驶舱能直接与 Helix 对话。

### 变更性质
- **main.rs**：装配提取为共享 `build_agent(config)`——daemon 与 CI-144 stdio 驾驶舱复用同一装配（Mind gRPC + LLM 链 + Tentacle pipeline + rails + judge + mode + events ring）。之前 stdio 模式是精简 Noop（memory=Noop、无 pipeline）——驾驶舱对话没有潜意识也没有手，现在与 daemon 完全同体（极致复用，一个装配两个门面）
- **up.rs**：
  - cockpit 主 agent 换成 **Anaphase 本体**（`--exec "anaphase --mode stdio"`，CI-144 帧流，可发消息跑 run_cycle）；mock-agent 回归 demo 用途
  - **WebUI 一键接入**：cellrix-web 后台启动（:8080，WEB_PORT 可覆盖），菜单状态行显示 URL；失败 fail-open（web 是窗不是墙）
  - 端点注入改**进程级 set_var**（ANAPHASE_MIND_ENDPOINT / ANAPHASE_TENTACLE_ENDPOINT / HELIX_CODEX 绝对路径）——daemon、驾驶舱子进程、web 一律继承，不再逐命令复制
- **main.rs**：codex 路径支持 `HELIX_CODEX` env 覆盖（12-factor；默认相对路径保持 repo cwd 行为）——修复从任意 cwd 启动时 pipeline 装配失败
- **验证**：stdio 握手真实通过（MessagePack 帧 + Manifest 收达，agent_name=anaphase-helix，action=status）；带 HELIX_CODEX 后 codex warning 消失（pipeline 装配成立）；四端口冒烟（50051/50052/50061/8080）全开；测试 206 全绿

### 兼容性
零破坏：mock-agent 仍可手动用 cellrix-cli 拉起；env 覆盖向后兼容；stdio 装配升级是纯增益（原 Noop 无任何外部依赖可依赖）。

### 验收
README（驾驶舱=Anaphase 本体 + WebUI + env 注入）｜ PLAN（共享装配）｜ ECOSYSTEM v1.55

### 状态
🧬 已完成
## 记录 30：推理流量过 Tuck 之门 + 凭证治理（2026-09-07）

### 触发条件
Tuck 内容治理网关 v1 完成后，旁路焊死最后一环——Anaphase 真实推理流量接入唯一出口。

### 变更性质
- **零代码改动**：config 两行切换——`reasoning_endpoint` → `http://127.0.0.1:60052/v1`（Tuck 网关）、
  `reasoning_api_key` → `tk-local-gate`（Tuck 身份凭证）；真实 deepseek key 移入 Tuck config（gitignored）
- **live 验证**：deepseek 真实响应经 Tuck 之门返回；审计链 request/response 双记录（hash 链）
- **凭证治理**：发现真实 key 曾进本仓库 git 历史（60df6f8）→ `config.toml`/`.bak` untrack
  （磁盘保留，gitignore 机制防再犯），`config.toml.example` 保留为模板
- **依赖**：Tuck reqwest 补 rustls-tls（https 上游必须，mock 掩盖的真伤）

### 验收
PLAN（旁路焊死段）｜ ECOSYSTEM v1.61（Tuck 369 + 全生态物理核对）

### 状态
🧬 已完成（key 轮换待用户执行后更新 Tuck config）

## 记录 31：Engram 正文轨迹——推理 round trip 落盘（2026-09-07）

### 触发条件
Cellrix Engram（印痕）面板落地后，审计链只有元数据（正文不落盘是 ADR-0004 安全决策）——
"以适配 ai 的输入输出展示轨迹"需要正文半体，记录在构造 prompt 的一侧（Anaphase）。

### 变更性质
- **`src/trace.rs`**：`ReasoningTrace`（append-only JSONL，seq 文件行数续启）+ `Redaction`
  （sk-/Bearer /api_key=/token=/password= 内建 + config 附加字面量，token 级匹配不误伤散文）
  + 截断预算（脱敏后 max_chars）
- **run_cycle 接线**：reason 调用 Ok 后记录（trace_id = `derive_job_id(user_input)`——
  与审计链/ledger 同一 join 键；ts 来自注入 Clock——确定性回放；写失败非致命）
- **config**：`reasoning_trace_path`（None=默认关闭）/ `reasoning_trace_max_chars`
  （None=协议默认 4096）/ `reasoning_redact_patterns`——零硬编码
- **测试**：trace 5（脱敏/附加模式/append/截断/续启 seq）+ reasoning_trace 2
  （join 键 + 凭证不落盘 + 默认关闭）；**206→211 全绿**

### 验收
`cargo test` 211/211 全绿（0 failed）｜ trace 文件脱敏实测（sk-4056 不落盘）

### 补丁（同日）：三键合一——x-tuck-trace 头
- **物理发现**：真实联调时审计链 trace_id 为 `local`（请求未带头），与正文 trace 的
  `run-{fnv}` 对不上——印痕 join 断裂
- **修复**：`ReasoningAdapter::reason(prompt, model, trace_id)` 签名加 trace_id
  （10 处实现机械同步）；HttpReasoningAdapter 注入 `x-tuck-trace` 头
- **验证**：真实调用后审计链 seq 4/5 = `run-8580fa8f91688134` == 正文 trace_id（同键）

### 状态
🧬 已完成（Cellrix Engram 正文 join 为下一步）
## 记录 29：驾驶舱真对话——send_message 输入 + 真实 LLM 回复（2026-09-06）

### 触发条件
用户配好 LLM 后驾驶舱仍无法对话：无输入框（UI 只渲染 ActionButton 空参数触发、Anaphase 投影无 action 节点、manifest 只暴露 status）。

### 变更性质
- **ci144/server.rs**：manifest 暴露 `send_message`（参数声明 `message: string`）
- **ci144/mod.rs**：project_snapshot 的 semantic_tree 增加 ActionButton（`send_message` 带 `needs_input: true`、`status`）——声明式协议扩展，UI 无需 manifest 知识
- **config.rs**：`ANAPHASE_CONFIG` env 覆盖 config 路径（12-factor）——驾驶舱子进程任意 cwd 加载同一 config（此前相对路径 → Cellrix cwd 下 Noop 无 LLM，真实对话失败根因）
- **Cellrix（协作仓）**：AppState 输入三字段 + handler 输入模式 + 输入行渲染（回复展示）
- **真实对话验证**：send_message 帧 → run_cycle → deepseek API 真实调用 → "我是 DeepSeek 的 AI 助手..."（非 mock 非 Noop）
- **测试**：Anaphase 206 + Cellrix 321（+2 输入字段测试），全生态 1420

### 兼容性
零破坏：needs_input 可选；ANAPHASE_CONFIG 可选（默认相对路径不变）。

### 验收
README（对话 + ANAPHASE_CONFIG）｜ PLAN｜ ECOSYSTEM v1.56

### 状态
🧬 已完成

## 记录 32：全文回放端点——/v1/trace（2026-09-07）

### 触发条件
印痕两半体（正文 + 链）同键后，用户拍板①：全文回放——Engram 从"链完整性"升级为"每轮思考正文可查"。

### 变更性质
- **trace.rs `query_file`**：只读按需查询（trace_id 过滤 / 无参取最新 N 条尾部窗口；坏行跳过不致命）——append-only 存储不建热索引
- **cap_http 加 `/v1/trace`**（axum）：`trace_id` + `limit`（端点协议默认 20）；未配置 trace path → `{"configured":false}` 永不 500
- **Cellrix web `/api/trace` 代理**：透传 trace_id query → Anaphase；detail 面板加"正文回放"区（点击审计条目 → 该轮 prompt/response，脱敏由写入侧保证）
- **测试**：query_file 1（过滤/尾部窗口/不存在→空），Anaphase 211→**212**；Cellrix web route +1
- **真实全链路验证**：anaphase --input "hello" → 正文落盘 → `/v1/trace?trace_id=run-a430d84680aabd0b` → web `/api/trace` 代理 → 返回真实 prompt+response（物理成立）

### 验收
README（Read-back endpoint 节）｜ GROWTH｜ ECOSYSTEM v1.62（Anaphase 212）

### 状态
🧬 已完成

## 记录 33：Fail-closed Tuck 门禁（2026-09-07）

### 触发条件
用户要求：绑定后无 Tuck 要提醒人类并停止工作（SPOF 落地）。

### 变更性质
- **gate_ok()**（health.rs）：tuck_endpoint 配置了 → tcp_reachable 探测；未配置 → pass（按需驱动，空串不评判）
- **main 门禁**：run_cycle 每轮前检查——Tuck 不可达 → `⚠️ Tuck 不在岗，已停止工作` + break（进程存活继续显示状态，只停推理）
- **config.toml**：`tuck_endpoint = "http://127.0.0.1:60052"`（对齐 LLM 已走 Tuck 网关的现状）
- **测试**：+3（未配置 pass / 不可达 block / 可达 pass），Anaphase 216→**219**
- **真实验证**：gate-fail（tuck 不可达）→ ⚠️ 停止 + 引导恢复；gate-ok（60052）→ 正常 run_cycle + snapshot tuck=Available

### 验收
README（fail-closed 节）｜ GROWTH｜ ECOSYSTEM v1.67（Anaphase 219）

### 状态
🧬 已完成

## 记录 34：/v1/chat——伙伴模式对话端点（2026-09-07）

### 触发条件
小白全流程实测：面板无输入框、无对话入口——打开只能看状态，无法与 Helix 说话。

### 变更性质
- **cap_http 新增 POST /v1/chat**：{message} → gate_ok（Tuck 不可达 503）→ build_agent（同装配、同潜意识、同黑盒）→ 单周期 run_cycle → {reply, done}
- 每次请求装配全新 AgentLoop：无共享可变状态、无跨会话串话；对话连续性 = 未来 Memory（L3 情景）职责，v1 不承诺
- 绑定后自动进 auth_mw 门禁（非白名单端点）
- **真实验证**：面板 /api/chat → 200 1.0s 真实 LLM 回复（走 Tuck 网关审计）；无凭据 401（继承绑定门禁）

### 状态
🧬 已完成

## 记录 45：第二次对话 EAGAIN 修复 + 资源基准目录（2026-09-07）

### 触发条件
①WebUI 第一次对话正常、第二次报 `Helix⚠ Resource temporarily unavailable (os error 35)`；②TUI 启动日志混入 DEBUG 与 fixture-codex 加载失败。

### 根因与修复
- **EAGAIN（os error 35）**：HttpReasoningAdapter 复用 reqwest 连接池，网关/上游关闭 keep-alive 后第二次调用复用死连接 → EAGAIN。修复：`pool_max_idle_per_host(0)`——每次调用新连接（本地 LLM 无 TLS 代价低；正确性优先）。
- **fixture-codex 找不到**：TUI stdio 子进程 cwd=Cellrix，`knowledge_base/fixture-codex.json` 相对路径解析失败。修复：`--config`/`ANAPHASE_CONFIG` 确定后 chdir 到配置所在目录——repo 相对资源（codex/rails）从配置目录解析，daemon（cwd 已是 repo 根）无副作用。
- **TUI 日志污染**：transport stdio 的 DEBUG eprintln 直接打到 TUI 终端。修复：`CELLRIX_DEBUG` 环境变量门控，默认静默。

### 验证
- 两次真实 chat：1.1s / 1.4s 全成功，无 EAGAIN
- TUI 启动输出：无 DEBUG、无 codex warning、Rails 正常挂载
- 测试：Anaphase 225 / Cellrix 337 全绿

### 状态
🧬 已完成

---
## 记录 46：SSE 流式对话（打字机 + 超时根治）2026-09-07
### 背景
用户报 WebUI 偶发 `Resource temporarily unavailable (os error 35)` 与 `replayed nonce`。排查发现**真根因是旧进程残留**：pkill -f 未杀掉 18:32 旧 anaphase / 18:20 旧面板，新二进制因端口占用启动失败，面板一直在连旧版（无 SSE、连接池修复前行为）。按 PID 强杀后 SSE 端到端全通，5 连发无 EAGAIN / 无 replay。
### 变更
- `ReasoningAdapter` trait 新增 `reason_stream(..., deltas: mpsc::UnboundedSender<String>)`——channel 传输 delta，默认实现=缓冲（全部 adapter 兼容，极致解耦）
- `HttpReasoningAdapter` 真流式：`stream=true` + SSE 行解析（`data:`/`[DONE]`），网关忽略 stream 时诚实回退 JSON 一次性输出
- `run_cycle`：`stream_tx: Option<UnboundedSender>`——有 sink 走流式，无 sink 走缓冲（同一契约两种传输）
- `/v1/chat`：`Accept: text/event-stream` 分流——SSE 分支 spawn run_cycle + unfold 流（delta 行 + done 行带全量 reply），JSON 路径保留兼容 curl/旧客户端
- 测试 +2：SSE 解析顺序断言 + JSON 回退断言（mock 网关，字节级契约固定）
### 验证
- 直连 50061：`data: {"delta":"好"}` 逐字流式 + done 行 ✓
- 面板 8080：`content-type: text/event-stream` + chunked 透传 ✓
- 5 连发 chat：全成功，无 EAGAIN / replayed nonce
- 测试：Anaphase 227 全绿

### 状态
🧬 已完成
