# Anaphase-Helix 生长记录
> **版本**：v1.8
> **日期**：2026-09-06
> **规则**：仅保留最近 3 条记录，超则归档至 `docs/archive/growth/`
> **归档策略**：历史随仓库版本化，永不删除


---

## 记录 9：候选 D'-2（Tuck 深度集成——SecurityGate 接线点）完成（2026-09-06）
**变异类型**：管控闭环咽喉落地（ADR-0023 的 Anaphase 侧实体）
**背景**：
- 候选 D' 目标（ADR-0007）：M1.5 深化四项；D'-1/D'-3 已完成；D'-2（Tuck 深度集成）此前标"阻塞（Tuck 侧接口）"——物理核验发现 Tuck P6-T3 的 TuckSecurityGate（anaphase_bridge.rs）早已就绪，本轮解除阻塞落地
- 前置：Tuck P0-P7 全部完成（P6-T3 AnaphaseBridge + TuckSecurityGate 是 D'-2 的 Tuck 侧接口）
**关键决策与发现**：
1. **接线点（D1）**：`Pipeline.security_gate: Option<Arc<dyn SecurityGate>>` + `with_security_gate()`；execute_calls 对每条 call 执行前过闸（三闸门之三：工具审计→HITL→Tuck 的执行前位置）；`None` = 无闸门 = 110 基线逐字节不变
2. **零依赖契约（D2）**：`src/security.rs` 定义 Anaphase 本地 `SecurityGate` trait + `GateCheck`（job/index/tool/args/labels 全事实）+ `GateVerdict`（Pass/Reject/HitlRequired/HardOverride）——发布库不依赖 tuck-core（极致解耦，适配在部署/测试层，对齐 Tuck 自身"transport handled by adapter"注释）
3. **决策语义（D3/D4）**：Pass/HardOverride 放行；Reject/HitlRequired 阻塞 call（不执行、不进 Tentacle）并写 ledger `Blocked` 记录（独立 record_type，**不改** VerdictStatus/既有 Verdict JSON 形状——ADR-0003 append-only 兼容）；错误信息带闸门 reason
4. **确定性（D5）**：trace_id 用 `Uuid::new_v5`（name-based 确定性，同一 job#index → 同一 gate 请求序列）；无新增 UUID v4
5. **验证**：tests/security_gate.rs 6 例（mock 闸门：无闸门兼容/Pass 放行/HardOverride 放行/Reject 阻塞+Blocked 落账+未触达 wire/HitlRequired 阻塞/事实全量透传）+ tests/tuck_gate.rs 3 例（真实 TuckSecurityGate：Low→Pass 执行 / Catastrophic→Reject 阻塞 / Critical→HitlRequired 映射；dev-only git 依赖 tuck-core，tuck-core 的 InMemoryCredentialStore 是 #[cfg(test)] 不可用 → 测试侧实现真实 CredentialStore trait）
**状态**：✅ 完成（121 测试全绿——110 基线 + 2 security lib + 6 security_gate + 3 tuck_gate；live 3 条 #[ignore]；生态合计 1201）

---
## 记录 10：候选 D'-4（真实场景插件）完成（2026-09-06）
**变异类型**：生态链路全通（MCP-Learner 学习 → post_learn 审查 → Tentacle 加载 → Anaphase 执行 → 判据 → 账本）
**前置修复**：MCP-Learner 1 个失败测试——根因是测试断言滞后（断言旧的无 `.manifest` 后缀文件名），实现产出 `{name}.manifest.json` 是生态契约（Tentacle 插件扫描依赖，联调修复 #2）；修断言，42+1f → 43 passed
**关键决策与发现**：
1. **Expect::Ok 判据（D1）**：`contract::Expect` 新增 `Ok` 变体（serde lowercase）——真实插件工具无统一数值形状，判据=纯结构断言 `exec_ok(ok_flag, echoed)`（零阈值零硬编码）；字段来源=执行体契约 `mcp_proxy.js`（`{ok:true, data:{tool, params}}`）；现有 Numbers/Rate/Text 不动（向后兼容）
2. **未知工具边界（D2）**：Tentacle grpc 未注册工具返回 `Status::not_found` → transport Err → pipeline Err（M1 single-pass：执行错误报错不落账不重试）；UNMET 仅用于"工具存在但判据不过"——物理核验修正了我初始的 UNMET 误判
3. **live 验收（D3）**：tests/m1_5_d4_live.rs 3 例（#[ignore]）——插件目录参数化（TENTACLE_PLUGINS_DIR 默认 /tmp/d4-learn/stable）；真实插件 MET / 未知工具 Err / run_cycle 全链路 MET；**实测 3/3 全绿**（真实 tentacle 二进制 + node + 学习产物）
4. **执行体占位如实标注**：mcp_proxy.js 是占位实现（echo 参数），真实 MCP 代理执行属 ECOSYSTEM 第二优先级 #4——D'-4 证明"链路真实"，不冒充"执行真实"
**状态**：✅ 完成（124 测试全绿——lib 61 + integration 16 + m1_e2e 3 + mind 9 + mock 4 + run_cycle_pipeline 8 + episode 10 + replay_guard 4 + security_gate 6 + tuck_gate 3；live 6 条 #[ignore]；生态合计 1228）
## 记录 15：G-7 配置向导（2026-09-06）

**变异类型**：LLM 引导输入——用户（2026-09-06）"Anaphase 有没有引导我输入 api key?"

- ADR-0015：up 菜单选项 4 = 配置 LLM（base_url/model/api_key 一问一答，Enter 跳过保持现值）
- api_key 输入不回显（stty -echo 包裹，零新依赖；pty 实测捕获输出无 key 明文）
- 写盘前备份 config.toml.bak；行级替换其余字节保留；空输入不写盘；字段缺失诚实 warn
- 实测：菜单 → 4 → 三字段写入 → 备份存在 → 还原往返一致
- 140 tests 全绿（135 + 5）；下一步 G2 Web 优化

## 记录 14：G-6 交互菜单（2026-09-06）

**变异类型**：一条命令之后只有选择题——用户（2026-09-06）"我觉得应该让用户做选择题，一个命令之后，最好就别出现命令了吧？！"

- ADR-0013：`up` 启动后端后进入交互菜单（tty 时），四选项：1 打开驾驶舱（Enter 默认）/ 2 查看状态 / 3 配置说明 / 4 停止退出（q）
- 状态 = 物理探测（TcpStream）+ 真实 snapshot 摘要（手写 HTTP GET /v1/agent/snapshot，ADR-0010 契约，无新依赖）
- 非 tty 自动降级挂起（is_terminal 物理判断）；parse_choice 纯函数 3 新单测（未知输入重提示，不猜测）
- 实测（pty）：菜单 → 选 2 状态（Tentacle/Anaphase 运行中 + Perception + ledger 0 条）→ 选 4 退出 → 端口全清
- 135 tests 全绿（132 + 3）；下一步 G2 Web 面板（SaaS 种子）

## 记录 13：G-5 易用引导 UX（2026-09-06）

**变异类型**：`up` 从 debug 输出升级为首跑引导——"打开就会用"

- ADR-0012：引导四段式（欢迎 banner / 前置检查 / 启动 / 下一步），中文输出（用户母语），代码注释英文
- `check_prereqs()` 纯函数：缺失项带具体构建命令（`cd helix-tentacle && cargo build`），3 单测
- Anaphase 缺失 = 致命（无本体无从启动）；Tentacle/Cellrix 缺失 = fail-open（离线 Noop 不阻塞）
- Noop 引导：reasoning 未配置时明确提示 + 配置方式（消除"驾驶舱为什么没数据"疑惑）
- 实测：正常场景（全就绪 + 中文模式标签 + 下一步三选项）/ 缺失场景（Tentacle 构建提示 + Anaphase 照常就绪）双验证通过
- 132 tests 全绿（129 + 3）；退出端口全清
- 下一步候选：G2 Web 面板（SaaS 种子，浏览器即开）或候选裁决

## 记录 12：候选 G 完成 + G-4 bootstrap（2026-09-06）

**变异类型**：一条命令起全栈——从 4 条 CLI 收敛为 1 条（易用性，用户 2026-09-06 明确）

- G-3（Cellrix 侧）：transport 帧契约修复（mock-agent 双通道字节序对齐，Cellrix ADR-0010）——驾驶舱 TUI 双通道真实渲染
- G-4 bootstrap `up`（ADR-0011）：tentacle（grpc :50051 派生自 endpoint/协议默认 + fixtures）→ anaphase（`ANAPHASE_TENTACLE_ENDPOINT` env 注入，config.toml 零改动）→ 物理探测（TCP 端口就绪）→ 可选 `--cockpit` 拉驾驶舱
- config.rs `apply_env_overrides`（12-factor env 优先 + fail-open，空值忽略）：3 单测（串行锁防 env 竞争）
- 实测：tentacle/anaphase 双就绪 + 退出端口全清 + 129 tests 全绿（126 + 3 env）
- 健康快照：全生态 1242（Cellrix 316 + Anaphase 129 + Tuck 316 + BIND-19 142 + Mind 98 + Tentacle 153 + Glove 45 + MCP 43）

## 记录 11：候选 G-T2（Anaphase 驾驶舱快照投影端点）完成（2026-09-06）
**变异类型**：共享快照投影——HTTP 端点与 agent 内部极致解耦
**背景**：
- 候选 G（正名：Anaphase 驾驶舱，非 Helix 驾驶舱——监控意识层，灵魂本体不驾驶）：Cellrix 白盒驾驶舱（模式栏 + 经历时间线 + Ledger 审查视图）
- G-T2 = Anaphase 服务端：真实状态端点替换静态演示 JSON
**关键决策与发现**：
1. **`AgentLoop::capture()`**：从 pub 字段投影 `AgentSnapshot`（mode/state/episode/ledger），无第二事实源
2. **共享槽 `Arc<Mutex<Option<AgentSnapshot>>>`**：run_cycle 每轮刷新，HTTP 端点只读（booting = 未刷新）；HTTP 禁用时槽为 None（按需加载）
3. **ledger 原样序列化**：serde tag=`record_type` 即协议契约，Cellrix 同形状反序列化；零投影代码
4. **零硬编码**：`token_consumed: 1234` 静态演示 JSON 消除（DNA 原则 11）
5. **live 验证**：真实 Anaphase（cap_http 50061）→ curl 输出真实 `{"mode":"partner","state":"Perception","episode":null,"ledger":[]}`；Cellrix HttpAnaphaseClient 真实解析（anaphase_live.rs）
**状态**：✅ 完成（126 passed + 6 live ignored——lib 63 + 新 capture 2；生态合计 316 + 126 + 其余）

## 记录 12：编排哲学显式化 ADR-0016（2026-09-06）
**变异类型**：哲学决策固化——防止知识腐烂，编排策略从研讨结论升级为 ADR
**背景**：
- 候选 G 完成后，编排策略仍散落在多轮研讨中（分诊/认知工艺挂点/按需感知/依赖边界/轨迹三层）
- 依据：Anthropic《Building Effective Agents》、DSH 内核源码比对（事件词汇表/surface 投影/工具调度）、Claude Code 架构分析（简单循环 + 周围系统）
**关键决策**：
1. **确定性优先分诊**：六 stage 只有"理解自由文本/生成表达"两处必须 LLM，其余 0 tokens 通道
2. **认知工艺四拍挂点**：检索自评→想、风格对齐→动、预期校准→量、差距评估→记账
3. **按需感知**：任务前/升级 LLM 前各感知一次（看口袋），不持续轮询
4. **依赖边界**：并行池/窗口感知→FlowModus；前缀稳定→Callosum；编排层只保证同输入同输出
5. **轨迹三层**：ledger + evidence + 会话 DAG + stage 事件（比 DSH 多"经历"维度）
**状态**：✅ ADR-0016 已立（Proposed）+ 同日修订——核对 Helix-Mind ADR-0021/0022/0010 后修正两处越界：①四拍/五工序（含批判性）全归 Mind，Anaphase 只触发 helixQuery（既有契约）不实现工序；②"看口袋"对齐 ADR-0010 = 设置 budget_tier 随请求传入，非新实体；新增三层递进边界（执行层分诊→System 0 门控→五工序）。VISION 补编排哲学指针，PLAN 增候选 O 系列（O-1..O-4 + 两条 FlowModus/Callosum 等待项），生态 1242 测试不变（纯文档轮）

## 记录 13：O-1 落地——结构化分诊 + 生态点亮感知（2026-09-06）
**变异类型**：编排哲学首个物理落点（ADR-0016 D1/D3 从纸面到代码）
**背景**：候选 E/F/G 完成、编排哲学 ADR-0016 显式化后，O 系列第一项开工。
**关键决策**：
1. **结构化输入零 LLM**：`!tool {"json"}` 在 Perception 分诊为 calls，Reasoning 跳过 LLM 直接组装 tt_job——0 tokens 通道有了第一个物理证明（计数 adapter 断言零调用）
2. **生态点亮 = 物理事实**：probe_ecosystem 任务开始前一次探测（TCP connect/UDS 文件存在性），fail-open 不阻塞；Cellrix = Native 手套
3. **感知点落位**：Reasoning 前（升级 LLM 前看一眼口袋）+ Execution 对 tentacle 未点亮记录降级事实
4. **不越界**：探测只做点亮状态不建连接；budget_tier 仍由 mind.rs 内部 derive（O-1 未覆盖外部 tier 透传）
**验证**：152 tests 全绿（+12：结构化解析 x4 / LLM 零调用 x2 / 生态探测与投影 x6）；free-text 无回归
**状态**：✅ 完成，commit ecc1924

## 记录 14：run_cycle 单周期原语化 + 模块改名（2026-09-06）
**变异类型**：语义归位——名字与职责一致（ADR-0016 D1 落地深化）
**背景**：用户指出"我们不是 run_cycle 吗？"——模块叫 agent_loop 但状态机函数叫 run_cycle，名实不符；且 run_cycle 内部自带 for 循环（cycle_cap=7）是"内置完整循环"形态（黑盒），与"单周期原语"（循环归调用方）的哲学不符。
**关键决策**：
1. **run_cycle = 单周期原子原语**：7 状态 DAG 走一圈返回 CycleOutcome{done/success/impasse}；周期步数上限 = HelixState::ALL.len()（枚举派生，非字面量）
2. **循环归调用方**：main 与测试各自 while（cap 用 config 的 cycle_cap）；防死循环责任随循环权转移——cap 语义从"内部截断"变"调用方保险丝"
3. **模块改名 agent_loop → run_cycle**（git mv，11 文件）；AgentLoop 类型保留（它是跑循环的 agent 主体，非循环本身）
4. **cycle_cap 来源落地**：config 注释写明 7 = 本地 LLM 上下文预算保守默认（初代设计动机：防死循环 + 控上下文 + 本地 API 条件有限）
**验证**：154 tests 全绿（+2 单周期语义测试：outcome 报告 + 调用方循环 episode step 每周期 +1）；cap 测试从"内部截断"重写为"调用方循环尊重 cap"
**状态**：✅ 完成，commit 3c8349c

## 记录 15：CI-144 传输层——驾驶舱闭环咽喉（2026-09-06）
**变异类型**：接口契约归位——Anaphase 学会生态共同语（ADR-0017 落地）
**背景**：Cellrix TUI 驾驶舱链路实测发现契约断裂——Cellrix `StdioTransport` 说
CIB/1.0 MessagePack（握手 + LE u32 帧），Anaphase `--stdio` 只会 JSON-lines 土话
（connect/get_snapshot/act/exit）。README §6.4 诚实标注缺口，本记录补齐。
**关键决策**：
1. **协议类型 vendored**（`src/ci144/`，与 proto/tentacle.proto 先例一致）：serde 逐字段
   对齐 Cellrix（tag="event"/content="data"/snake_case/开放枚举降级），MessagePack 互操作，
   不跨仓库依赖（极致解耦）
2. **握手 + 帧**：CIB/1.0 首行 → 回 `CIB/1.0 MSGPACK\n`；4 字节 LE u32 长度前缀 + MessagePack
3. **事件流**：Manifest 首帧 → 1s 节律 Snapshot 推流（`SNAPSHOT_PUSH_INTERVAL`，config 可调不硬编码）
   → ActionRequest 响应
4. **协议层业务无关**：`run_loop(reader, writer, snapshot, handle_action, interval)`——
   action 回调注入；launcher 挂 `status`/`send_message`（真实 run_cycle，cap 尊重 cycle_cap）
5. **select 单任务事件循环**：biased select 合并推流 tick 与帧读取——无 spawn、无 Send 体操、
   确定性顺序（落地时对 spawn 方案的优化）
**验证**：160 tests 全绿（+6：握手 x2 / 帧往返 / 投影形状 / vendored serde 形状 / duplex 全协议会话）；
live 实测（`cargo test --test ci144_live -- --ignored`）真实二进制全链路：
握手→Manifest→Snapshot→status→send_message→unknown→EOF 退出 ✅
**状态**：✅ 完成（commit 见 git log）

## 记录 16：Rails 心智外铁轨——人类知识 DAG 只读引用（2026-09-06，ADR-0018）
**变异类型**：新器官——人类权威知识的确定性引用通道（驾驶/伙伴/生存三模式通用，只读）
**背景**：驾驶模式查法典刚需——律文必须原文引用、零编造（行业实测 19% 引用幻觉率，
律师因 AI 幻觉引用被法院制裁是真实失败模式）。通用 LLM 无铁轨不可控。
**关键决策**：
1. **rails = 人类资产**：knowledge_base/rails/<kb>/ markdown 原汁原味，SHA-256 版本冻结；
   RailScope 只有 Read 变体（类型级无写侧）
2. **mddag 轻量解析**：标题树=节点、链接=边、确定性 id（{doc}#{heading}，无 UUID）、
   断链即构建错误——完整 lodestone 协议留 M2
3. **确定性导航**：整句 + 词项 + CJK bigram（≥2 不同 bigram 才计分，防单 bigram 噪声）；
   无嵌入（概率性+依赖，违背确定性/极致节能）；visited set 做 provenance
4. **引用契约**：verify_reference 纯函数（节点存在 + visited + 原文子串）三判据；
   引不到答 NO_RAIL_CONTENT（graceful refusal，零编造）
5. **接线 MemoryRetrieval**：注入 + rail_mode；与心智记忆两条知识线物理分开
   （心智会消化遗忘，铁轨不消化不遗忘）
6. **与熟练模式同源**：心智内软铁轨（EMA 权重会进化）vs 心智外硬铁轨（冻结）；
   硬度 = 错误的代价
**物理验证（2026-09-06）**：
- 同目录两次 build_index 字节级一致（确定性回放）✅
- 断链 kb 构建报错（dangling link）✅
- 导航"数据归属条款是什么"→ 命中"第 1 条 数据归属"（bigram 主题判定）✅
- 验证器：原文+visited 过；未访问/改写/未知节点全拒 ✅
- run_cycle e2e：rail 命中注入原文节点 + rail_mode 置位 ✅
- 真实二进制：Rails mounted: knowledge_base/rails/demo ✅
**发现并修复**：kb_dir 指向容器根导致跨文档链接断链（doc id 被子目录前缀污染）
→ 修正为 kb_dir 指向具体 kb（one kb per directory）
**输出契约层（ADR-0018 后续，同日完成）**：rail 命中时 Reasoning 短路 LLM——回答 =
`assemble_rail_answer` 确定性拼装（0 tokens，无 LLM，无编造空间，验证器天然满足因为
回答就是铁轨原文）；e2e 断言 LLM 调用数=0 + 回答含节点 id + 原文逐字引用。代码注释
ADR-XXXX 占位全部替换为 ADR-0018。
**状态**：✅ 完成（169 tests 全绿 = 160 基线 + 9 新增；commit 见 git log）

## 记录 19：ADR-0021 模式无关事件环——驾驶模式黑匣子（2026-09-06）

- **健康快照**：181 passed / 0 failed（180 + 1 drive black box 测试）
- **新能力**：事件环从 pipeline 提升 AgentLoop 级——无 tentacle 装配（驾驶/Noop）
  每次 run_cycle 也记录 cycle 轨迹（stage=0：begin/state/tool/end，trace=derive_job_id）；
  pipeline 装配复用同一环（stage 1..=6 同流同游标）；flush/恢复挂载点改 agent.events
- **哲学落地**：白盒四层模式无关（驾驶=黑匣子，伙伴=黑匣子+六 stage）；
  **审查修正**：初版 cycle ts 用墙钟 → 违背极致复用/确定性优先，改为复用 ledger
  `Clock` trait（单一时间源，FakeClock 下黑匣子字节级可回放，+1 测试锁定）
- **物理验证**：无 tentacle 真实二进制 → events.jsonl 7 条 stage=0 事件（确定性 trace）
- **零新增**：无新 crate / 无新端点 / 一个字段 + 一个 builder + 五处 emit

## 记录 18：O-3 事件轨迹持久化——跨重启可回放（2026-09-06，ADR-0020）

- **健康快照**：180 passed / 0 failed（176 + 4 from_jsonl 单元）
- **新能力**：`EventRing::from_jsonl`（round-trip 字节一致 / seq 接续 / 坏行失败关闭 / cap 强制）；
  `AnaphaseConfig.events_log_path`（默认 events.jsonl，跟随 session_notes 先例）；
  main 装配恢复历史（fail-open）+ 主循环增量 flush（崩溃最多丢在飞轮）
- **哲学落地**：白盒四层全部跨重启可追溯（能力/状态/过程/事实）；实时逐轮追加 =
  ADR-0022 "中途崩溃不丢经历" 的物理兑现
- **零新增**：无新 crate / 无新依赖 / 无新实体——一个函数 + 一个 config 字段

## 记录 17：O-2 stage 事件总线——过程白盒第四层（2026-09-06，ADR-0019）
**变异类型**：新器官——六 stage 过程的 append-only 事件投影（事件=过程，ledger=事实，evidence=支撑）
**背景**：候选 E 后白盒三层（能力 Manifest / 状态 Snapshot / 事实 ledger）缺"过程"层；
前沿（OTel GenAI / log-is-the-agent）验证事件溯源方向，但集中式后端/OTel SDK 违背零依赖。
**关键决策**：
1. **append-only 事件环**：EventRing + 单调 seq；记录非控制流（无推送/订阅）；cap 满拒并
   计数 dropped（不重写不覆盖，游标语义不破坏）
2. **事件模型**：StageEvent{seq, ts, trace_id, stage(1..=6), phase(begin/end/verdict), detail}；
   trace_id = 派生 job_id（一次 cycle 一条 trace，整链可重放）；ts 来自注入 Clock（与 ledger
   同一时间源，FakeClock 下字节级可回放）
3. **六 stage 边界插桩**：stage1/2 Reasoning（structured+LLM 双路径）、stage3 execute_calls
   （begin + per-call end：tool ok/err）、stage4 record_evidence、stage5/6 Reflection
   （criteria + verdict）；中断即事实（gRPC 失败不补发 end）
4. **增量拉取**：after(seq) + GET /v1/agent/events?after=N → {events, dropped, last_seq}；
   与 snapshot 同 Arc<Mutex> 共享槽模式；无 pipeline 时诚实 {"status":"no pipeline"}
5. **容量来源**：events_cap 进 PipelineConfig，codex contract 提供（零硬编码）
**物理验证（2026-09-06）**：
- run_cycle 全链路：六 stage 各一条 begin + end，stage3 每 call 一条 end ✅
- stage6 end 携带 verdict（MET/UNMET）与 ledger 最后一条一致 ✅
- seq 单调无间隙；after(last)=空、after(last-1)=末条 ✅
- 同输入同时钟 → 事件流字节级一致（确定性重放）✅
- 真实二进制 GET /v1/agent/events?after=0 → {"status":"no pipeline"}（诚实降级）✅
**状态**：✅ 完成（176 tests 全绿 = 169 + 3 events 单元 + 4 stage_events 集成；commit 见 git log）
