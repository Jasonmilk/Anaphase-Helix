# ADR-0005：候选 E——Reasoning 结构化 + run_cycle ↔ pipeline 完整 merge

## 状态
**Active**（2026-09-03，候选 E 执行记录；决策先于代码，经 E-T1 探查代码证据收敛）

## 问题
1. Reasoning 输出仍是自然语言 String + `contains("tool_call")/python/cli` + `contains("impasse")/unknown` 字符串匹配，与 M1 结构化 `calls[]` 基因不兼容（ADR-0003 决策 9 四点不兼容第 1 点未消除）
2. `suggested_actions: Vec<String>` 非结构化（P11b 链路产物），Execution 以 `join(", ")` 消费
3. M1 确定性 pipeline 六 stage 已建成但未接入 run_cycle 状态机（旁路未成永久事实——ADR-0003 决策 9 "M1.5 必做项"）
4. run_cycle 5 处硬编码（`0.7/0.3/0.2`、`"left_brain"`、`p_death>0.7`、`"echo"`、`0..7`）为 ADR-0002 已登记技术债，未消除

## 决策

### 决策 1：Reasoning 输出协议结构化（E-T2）
- **协议**：JSON `{"calls":[...],"impasse":bool}`（两字段均可选）或裸 `[...]` 数组；见 `docs/contracts/reasoning-output.md`
- **解析**：`contract::parse_reasoning_output`（唯一消费点），run_cycle 从输出解析
- **替换**：删除 Reasoning 状态全部 `contains(...)` 字符串匹配（tool_call/python/cli/impasse/unknown）
- **保持**：`ReasoningAdapter::reason()` trait 签名不变（ADR-0003 决策 4）——HttpReasoningAdapter / Noop / FlowModus 适配器零改动
- **impasse 结构化**：`{"impasse":true}` 或 `{"calls":[],"impasse":true}` → Impass；非法 JSON → NoToolNeeded（warn，兼容 Noop 纯对话输出）

### 决策 2：六 stage 落点（E-T3/T4/T5，ADR-0003 决策 9 映射表落地）
| pipeline stage | run_cycle 状态 |
|---|---|
| stage1 parse calls | Reasoning（`parse_reasoning_output`） |
| stage2 组装 tt_job | Reasoning 尾部（`context.job`，job_id 派生 + clock 时间戳） |
| stage3 gRPC execute | Execution（`execute_structured` → `Pipeline::execute_calls`） |
| stage4 evidence 落盘 | Execution 尾部（`record_evidence` + `context.evidence`） |
| stage5 criteria 校验 | Reflection（`Pipeline::check_results`） |
| stage6 ledger 写入 | Reflection 尾部（`build_verdict` + `ledger.append`） |

- **AgentLoop 持有 `pipeline: Option<Pipeline>`** + `with_pipeline()` builder；None 保持 legacy echo 路径（向后兼容，ADR-0004 决策 5 延续）
- **AgentContext 新增结构化字段**：`calls: Vec<Call>` / `job: Option<TtJob>` / `evidence: Vec<EvidenceRecord>`；`suggested_actions` 保留不动（P11b 测试依赖）
- **安全闸保留**：结构化路径同样过 HITL 执行闸（原则 4）+ safety 工具审计（原则 5），低风险工具零延迟放行
- **无计划语义**：无 pipeline 或 calls 为空 → 不产生 evidence / 不写 ledger（legacy 行为不变）

### 决策 3：确定性信封派生（E-T4）
- `job_id = FNV-1a(user_input)` 十六进制（`contract::derive_job_id`）——无 UUID（DNA 原则 11 / ADR-0003 决策 12），同输入同 id，可回放
- `created_at = unix_secs_to_rfc3339(ledger.clock_now())`（`ledger::unix_secs_to_rfc3339`，chrono，RFC3339 date-time 符合 tt_job.schema.json）
- `identity_labels` = 协议默认空 map（run_cycle 无调用方身份；ADR-0004 语义）

### 决策 4：run_cycle 五常量入 config（E-T6）
- 新增 `config::RunCycleConfig`（`Default` 即文档化协议值）：`amygdala_default_vector` / `reasoning_mode` / `soft_reflex_threshold` / `execution_placeholder` / `cycle_cap`
- `AnaphaseConfig.run_cycle`（`#[serde(default)]`），config.toml `[anaphase.run_cycle]` 可覆盖；main.rs 已接线
- **agent_loop.rs 零字面量**（grep 验证：5 处历史常量仅存在于 config.rs Default 与 config.toml）

### 决策 5：测试与验证（E-T7）
- `tests/run_cycle_pipeline.rs`（8 例，MockTentacle）：MET 全链路 / UNMET+retry_due / 无计划跳过 pipeline / 确定性回放（字节级一致）/ cycle_cap / soft_reflex_threshold / amygdala+mode / execution_placeholder
- contract +6、ledger +1 单测；`m1_e2e_live` 新增 `m1_5_live_run_cycle_structured_chain`（真实 Tentacle 全链路，#[ignore]）
- 既有 78 测试全绿保持（TriggerToolReasoning stub 输出升级为结构化协议）

## 影响
- 测试数：Anaphase 78 → **94 passed + 3 live（#[ignore]）**
- 验收判据：① run_cycle 一次循环走通六 stage 全链路 ✓ ② 无 `contains("tool_call")` 匹配残留（仅注释提及）✓ ③ 5 处硬编码全部有 config 来源 ✓ ④ 78 全绿 + 新增全绿 ✓
- 生态测试总数：1153 → **1168**

## 边界守约（任务书 4.5）
- ✅ 不改 Helix-Mind / FlowModus / Cellrix / Tentacle 代码（Anaphase 仓库内完成）
- ✅ 不引入 lodestone（M2）；不做守护进程、Auto 路由 / 小模型 Router（M3）
- ✅ echo fallback 保留向后兼容（legacy 路径 + RunCycleConfig.execution_placeholder）

## 已知相邻债（本 ADR 不处理）
- `src/adapters/mind.rs` `build_energy_context` 内 `token_budget/pulse/vigilance/latency_limit_ms/familiarity` 字面量（Mind 适配器域，非 run_cycle 5 处）
- `src/agent_loop.rs` MemoryRetrieval `impasse_level > 2`（既有字面量，不在任务书 E-T6 列明的 5 处清单内）
- `main.rs` 未接线 pipeline（`tentacle_endpoint` 消费；与 M1.5 一致，待后续）
- config.toml `[cognitive]` / `[immune]` 段未被 AnaphaseConfig 消费（历史遗留）
- `src/adapters/flowmodus.rs` `endpoint` 字段 dead_code 警告（既有，未触碰）

## 关联
- ADR-0003（决策 9 映射表 / 决策 11 结构契约）、ADR-0004（决策 5 渐进接线）、ADR-0002（DNA 原则 11）
- PLAN v1.9 / GROWTH v1.5 / README / ECOSYSTEM v1.6
