# Anaphase-Helix 生长记录
> **版本**：v1.6
> **日期**：2026-09-05
> **规则**：仅保留最近 3 条记录，超则归档至 `docs/archive/growth/`
> **归档策略**：历史随仓库版本化，永不删除


---
## 记录 5：M1.5 生态合流核心完成（2026-09-03）
**变异类型**：跨仓库联调 + 真实连通 + 语义定义 + run_cycle 渐进接线
**背景**：
- M1.5 目标（ADR-0004）：Tentacle `--transport grpc` + fixture 插件 + 真实连通 + 语义定义 + run_cycle 渐进接线
- 前置：M1 mock 闭环完成（ADR-0003 决策 7 定义 M1.5 范围）
**关键决策与发现**：
1. **Tentacle grpc 接线（Tentacle d902151）**：main.rs 新增 `--transport grpc` + `--grpc-port`（默认 50051），复用 scan_plugins + ProcessTool 实例化 → TentacleGrpcService + tonic serve；tentacle crate 新增 tentacle-transport-grpc + tonic 依赖
2. **fixture 插件（Tentacle d902151）**：fixtures/numbers + rate（manifest+js，SHA-256 完整性校验，参数化 MET/UNMET）。**联调发现**：fixture 默认值必须满足判据契约——numbers 20 序列 sum=210 超 high=100、rate 10/20 的 cross_check 不过，修正为 1..=10 与 {10,10}（= M1 mock 形状）
3. **真实连通（Anaphase 1c1e723）**：`tests/m1_e2e_live.rs`（#[ignore]，手动联调）spawn 真实 tentacle 二进制 → pipeline 全链路，m1_5_live_met + m1_5_live_unmet 全绿——M1 mock → M1.5 真实无缝切换
4. **语义定义（Anaphase 6ac3a0e）**：identity_labels（纯标签供 Tuck 审计/渐进披露，绝不传凭证，BTreeMap 确定性注入）+ seen_entropy_bloom（可选重放守卫，当前不启用）；execute_tool_with_labels 新增
5. **run_cycle 渐进接线（Anaphase 4d7e3ec）**：tool_command（Option<String>）——配置后 Execution 派发真实工具名，未配置保持 echo fallback（向后兼容）；with_tool_command() builder；RecordingToolAdapter 双测试
**状态**：✅ 完成（78 测试全绿——lib 46 + integration 16 + m1_e2e 3 + mind 9 + mock 4；live 2 条 #[ignore] 手动验证）
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
