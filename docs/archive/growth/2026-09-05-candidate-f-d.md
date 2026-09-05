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


---## 记录 7：候选 F（会话即经历）完成（2026-09-05）
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
