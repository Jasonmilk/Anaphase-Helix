# Anaphase-Helix 生长记录
> **版本**：v1.5
> **日期**：2026-09-03
> **规则**：仅保留最近 3 条记录，超则归档至 `docs/archive/growth/`
> **归档策略**：历史随仓库版本化，永不删除


## 记录 4：M1 确定性流水线完成（2026-09-03）
**变异类型**：M1 里程碑（tt_job 确定性流水线）+ DNA 原则 11 新增
**背景**：
- M1 目标：Anaphase 独立完成可回放闭环——mock LLM → tt_job → mock Tentacle → evidence → criteria → ledger（Tentacle 字面零改动）
- 前置：M1 任务书经四轮严肃审查收敛（路径臆造 → proto 断裂 → mock 传播缺位 → 循环语义半句 + 引擎归属）
**关键决策与发现**：
1. **T0 proto 对齐**：vendor Tentacle v1 权威 proto（tentacle.v1，完整拷贝 + 溯源注释）；GrpcTentacleAdapter 重写（ExecuteTool/execute_tool，ToolAdapter trait 保留为 run_cycle 兼容 shim；perceive 明确报错）；MockTentacle + TcpListener mock server（复刻 mind_integration 模式）
2. **T2-T5 四模块**：contract（tt_job 类型 + parse_llm_calls 三例）/ evidence（append-only + evidence_id 派生 + expect 自包含）/ criteria（六纯函数 + expect→criteria 映射 + 规则数据分离）/ ledger（JSONL + retry_due + parent_id 谱系 + Clock 注入）
3. **T8 闭环**：pipeline 六 stage（禁巨型 run）+ m1_e2e 三用例（MET / UNMET retry_due=4600 + reopen 扫描 / 同输入同时钟字节级一致）
4. **DNA 原则 11 新增（ADR-0002）**：零硬编码——run_cycle 5 处硬编码（0.7/0.3/0.2、left_brain、p_death>0.7、echo、0..7）为已知技术债，M1.5 接线时消除；协议可选字段用默认空值
5. **ADR-0003 落档**：决策 9（旁路 + 六 stage 映射表）/ 10（UNMET + retry_due + 谱系 + 两循环物种）/ 11（pipeline 结构契约）+ 执行决策 1-8、12-15
**状态**：✅ 完成（76 测试全绿——lib 46 + integration 14 + m1_e2e 3 + mind 9 + mock 4，M1 验收三判据全过）
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
