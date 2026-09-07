
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
