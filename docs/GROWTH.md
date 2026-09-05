# Anaphase-Helix 生长记录
> **版本**：v1.7
> **日期**：2026-09-05
> **规则**：仅保留最近 3 条记录，超则归档至 `docs/archive/growth/`
> **归档策略**：历史随仓库版本化，永不删除


---
## 记录 6：候选 E（Reasoning 结构化 + run_cycle ↔ pipeline 完整 merge）完成（2026-09-03）
**变异类型**：认知状态机 ↔ 确定性流水线合一 + 零硬编码收口
**背景**：
- 候选 E 目标（ADR-0005）：替换 `contains("tool_call")` 字符串匹配，suggested_actions 结构化，六 stage 完整落点 run_cycle（ADR-0003 决策 9 映射表）
- 前置：M1.5 完成 tool_command step1 接线（ADR-0004 决策 5）
**关键决策与发现**：
1. **Reasoning 输出协议结构化（E-T2）**：JSON `{"calls":[...],"impasse":bool}` 或裸数组；`contract::parse_reasoning_output` 唯一解析点；删除全部 contains 字符串匹配；trait 签名不变（Http/Noop/FlowModus 零改动）
2. **六 stage 落点（E-T3..T5）**：Reasoning=stage1+2（parse+信封），Execution=`execute_structured`→execute_calls+record_evidence（HITL/审计闸保留），Reflection=check_results+build_verdict+ledger.append；AgentLoop 持 `pipeline: Option<Pipeline>`（None 保持 legacy echo 向后兼容）
3. **确定性信封（E-T4）**：job_id=FNV-1a(user_input)（无 UUID）；created_at=clock→RFC3339（chrono）；identity_labels=协议默认空
4. **零硬编码收口（E-T6）**：`config::RunCycleConfig` 承载 5 常量（amygdala 向量/模式/阈值/占位/循环上限），config.toml `[anaphase.run_cycle]` 可覆盖；agent_loop.rs 零字面量（grep 验证）
5. **验证**：`tests/run_cycle_pipeline.rs` 8 例（MET/UNMET/无计划跳过/确定性回放/cap/阈值/向量+模式/占位）+ contract 6 + ledger 1 + live `m1_5_live_run_cycle_structured_chain`（真实 Tentacle 全链路 MET）
**状态**：✅ 完成（94 测试全绿——lib 54 + integration 16 + m1_e2e 3 + mind 9 + mock 4 + run_cycle_pipeline 8；live 3 条 #[ignore] 手动验证；生态合计 1168）
---
*（记录 3 已归档至 docs/archive/growth/2026-08-28-p11ab-craft-orchestration.md）*
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
## 记录 8：候选 D' 部分（重放守卫指纹 + 启动接线）完成（2026-09-05）
**变异类型**：不阻塞项先行（D'-1 seen_entropy_bloom 指纹 + D'-3 pipeline 启动装配）
**背景**：
- 候选 D' 目标（ADR-0007）：M1.5 深化四项；D'-2（Tuck 深度集成）依赖 Tuck 侧接口、D'-4（真实场景插件）依赖 MCP-Learner 升级——阻塞，本轮做不阻塞的 D'-1 / D'-3
- 前置：候选 F 完成（ADR-0006，fnv64 共享派生原语就绪）
**关键决策与发现**：
1. **物理核验（D'-1 落点）**：Tentacle proto 有 `seen_entropy_bloom=5` 字段（optional）但 grpc 服务仅透传 `Option<String>`（无消费逻辑）——Anaphase 侧把 `""` 占位升级为真实确定性指纹；bloom 检测语义属于执行体（Tentacle），信封层只携带特征；**不做内部 bloom filter**（破坏 pipeline 无状态确定性，ADR-0003 验收判据）；Callosum 不参与（职责=上下文内存分配器，勿增实体）
2. **熵指纹（D'-1）**：`contract::derive_seen_bloom(tool, params)` = `bl-` + fnv64(`{tool}#{params}`)——复用 fnv64 共享原语（run-/ep-/bl- 前缀家族）；同 call 同指纹（确定性回放）、异 call 异指纹；execute_calls 透传真实指纹
3. **启动接线（D'-3）**：`pipeline::resolve_pipeline(endpoint, config)` fail-open（空 endpoint/连接失败 → None + warn，DNA 铁律 6，与 resolve_memory_adapter 同模式）；main.rs `tentacle_endpoint` 非空 → with_pipeline（六 stage 替代 echo）；SystemClock（production）注入
4. **验证**：tests/replay_guard.rs 4 例（wire 层指纹断言/重放稳定性/fail-open/接线成功）+ contract 1 例（golden）；MockTentacle 加 captured_bloom 捕获（测试基建）
**状态**：✅ 完成（110 测试全绿——lib 56 + integration 16 + m1_e2e 3 + mind 9 + mock 4 + run_cycle_pipeline 8 + episode 10 + replay_guard 4；live 3 条 #[ignore]；生态合计 1184）


---
## 记录 7：候选 F（会话即经历）完成（2026-09-05）
**变异类型**：经历边界 + 三模式参与度（生态级哲学首落码 Anaphase）
**背景**：
- 候选 F 目标（ADR-0006）：Helix 无会话概念——对话是 Helix 的经历（L3 情景），Mind 应能"看到"会话（元认知）；驾驶/伙伴/生存三模式有效运行
- 前置：候选 E 完成（Reasoning 结构化 + pipeline merge）；ADR-0022/0023 草案经严肃审查拦截（ADR 编号冲突 0022-0030 已占用、编造 spec §15 引用、协议版本失实 v0.6/v0.7-draft vs 实测 v1.0.0-RFC-4），不落库
**关键决策与发现**：
1. **复用点全核验**（物理事实优先）：Mind L3 `content: JSON` 保留结构化记录 + 默认 PRIVATE + 突触切断语义；认知工艺已有"元批判"工序与独立会话隔离（ADR-0021）；INTENT-7 已有 FINISH（认知循环结束→L3 收尾）与 autonomy_level=AGENT/OPEN/SURVIVAL；main.rs 已有 NoopMemoryAdapter（驾驶基础）——**不新建 crate / 协议 / RPC / L3 schema 字段**
2. **Episode 边界（D1）**：`contract::fnv64` 提取共用派生原语 + `derive_episode_id`（前缀 `ep-`，与 job 的 `run-` 同模式，确定性回放无 UUID）；AgentLoop `episode: Option<Episode>`；Reflection 写入带 `{id}#{step}` provenance 的结构化 JSON（无 episode 时原样——严格向后兼容 94 测试）
3. **经历收束（D2）**：`end_episode` 生成 EpisodeDigest（id/turns/first_input）经既有 remember 通道写 L3（语义对应 INTENT-7 FINISH）；`begin_episode` 自动收束旧 episode（不丢经历）；幂等
4. **三模式（D3）**：`config::Mode { Drive, Partner, Survive }`（serde snake_case，默认 Partner=Helix 本体）；Drive=Noop 装配（已有路径），Partner=GrpcMind+episode 生命周期，Survive=枚举占位（反向驱动待 Mind P10a）；**运行期零 if 分支**（Noop 天然隔离，极致解耦）
5. **验证**：tests/episode_lifecycle.rs 10 例（golden 派生/生命周期/自动收束/幂等/provenance/兼容/mode serde）+ contract 1 例（fnv64 共用断言）
**状态**：✅ 完成（105 测试全绿——lib 55 + integration 16 + m1_e2e 3 + mind 9 + mock 4 + run_cycle_pipeline 8 + episode 10；live 3 条 #[ignore] 手动验证；生态合计 1179）
---
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
