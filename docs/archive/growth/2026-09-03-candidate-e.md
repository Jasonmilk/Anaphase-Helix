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
