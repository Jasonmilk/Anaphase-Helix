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
