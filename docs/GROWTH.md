# Anaphase-Helix 生长记录
> **版本**：v1.2
> **日期**：2026-08-28
> **规则**：仅保留最近 3 条记录，超则归档至 `docs/archive/growth/`
> **归档策略**：历史随仓库版本化，永不删除


## 记录 1：P10b 认知工艺触发链路完成（2026-08-28）
**变异类型**：认知工艺触发链路 + HITL 审批通道 + 跨项目裁决
**背景**：
- P10b 目标：打通 Anaphase → Mind 认知工艺触发链路
**关键决策与发现**：
1. **T1 System 0 门控经 budget_tier 前置路由验证**：确认 Mind `layer3.rs` 接收映射 `budget_tier` → core `BudgetTier` → `retrieval.query`（链路已通）；划界：预算路由（外部前置）决定扫描范围 / System 0（Mind 内）决定思考深度，二者正交
2. **T2 状态机驱动 suggested_mode**：`MemoryAdapter::set_complexity` 默认钩子 + `GrpcMindAdapter` AtomicU8 复杂度 + `derive_suggested_mode(query, complexity)` 状态优先（1→Skilled/2→Anchor/3→Imagination），0 兜底长度启发式
3. **T3 HITL 审批通道**：`src/hitl.rs`（is_high_risk 写/网络/凭证判定 + check_approval，默认 fail-closed）；Execution 接入执行闸；三层闸门串联：工具审计（入库门）→ HITL（执行闸）→ Tuck（边缘物理闸）
4. **跨项目裁决（认知工艺双向复用备忘录审查）**：方向采纳 + 4 条修正——重编号 P11a→P11d（避免与主线 P10c 撞号）、CraftQuery 暂缓（P10b 间接触发已覆盖第一层，勿增实体）、措辞"模式同构+接口复用"（脑手分离铁律不变）、trait 归属推迟
5. **生态对齐**：Cellrix = 原生生态手套（优先级最高）；CI-144 v2.0 等待冻结不阻塞；Tentacle Rust 重构硬性对齐（凭证标签/布隆过滤器/异步沙箱/动态共识/多传输）
**状态**：✅ 完成（37 测试全绿，P10b 验收通过）
---
## 记录 2：P10c 生命周期实体化 + 生态感知完成（2026-08-28）
**变异类型**：生命周期实体化 + 任务 DAG + 生态感知 + 跨项目裁决
**背景**：
- P10c 目标：把 Anaphase 的"身体本能"实体化（强制苏醒/认知脱水、任务 DAG、生态手套感知）
**关键决策与发现**：
1. **T1 强制苏醒/认知脱水**：`src/lifecycle.rs` `SessionNotes`——`wake_up()` 跨纪元认知重载（读上一纪元 `session_notes.json` 简报）+ `dehydrate()` 确定性压缩持久化（0 Token，LLM 端口预留）。工作态归 Anaphase，不触碰 L3 不可篡改
2. **T2 任务 DAG 分支拓扑**：`src/task_dag.rs`——`dag_branch_create(parent, branch_name, intent, knowledge_ref)` 自主生长；L0/L1 边界守卫（基因锁/自画像保护区不可作为父节点）；`add_subtask`/`attach_leaf` 分化挂载
3. **T3 生态手套可用性感知**：`src/gloves.rs`——Cellrix 原生手套优先（`GloveTier::Native`）→ MCP 等通用；只做状态注册/查询，**不实现协议**（独立扩展位，勿增实体）
4. **认知工艺双向复用备忘录裁决落地**：四阶段重编号 P11a→P11d（避免撞号）；CraftQuery 为第二阶段显式调用暂缓（P10b 间接触发已覆盖第一层）；措辞"模式同构+接口复用"（脑手分离铁律不变）；trait 归属 P11c 时裁决
5. **Tentacle Rust 重构**：硬性对齐纳入（凭证标签流转/布隆过滤器/异步沙箱/动态共识/多传输），P10b 后自然启动
**状态**：✅ 完成（49 测试全绿，P10c 验收通过）
---
## 记录 3：P11a/P11b 认知工艺双向复用轨道调研完成（2026-08-28）
**变异类型**：跨项目调研 + 裁决 + 链路验证
**背景**：
- 认知工艺双向复用备忘录（v1.0）四阶段轨道：P11a CraftAdapter → P11b 编排 → P11c OrchestrationCore → P11d 双向
- P11a 启动后先调研 Mind 侧认知工艺 RPC 现状（用户指令：CraftQuery 第二阶段不急）
**关键决策与发现**：
1. **P11a 裁决：CraftAdapter 不建**。Mind v4.1 冻结契约无显式认知工艺 RPC；P10b 间接触发（suggested_mode + budget_tier → System 0 门控）已覆盖第一层；符合意志优先 + 勿增实体
2. **P11b 裁决：OrchestrationAdapter 不建**。Anaphase 侧单向编排链路已就位：`HelixQueryResult.suggested_actions`(12) → GrpcMindAdapter 消费 → MemoryRetrieval 注入 → Execution（HITL 闸就位）
3. **P11b 验证策略**（战术微调采纳）：mock Mind 返回 suggested_actions → 断言流转到 Execution；Mind 就绪用真实、未就绪 mock 不阻塞；mock 验证 Anaphase 侧流转，Mind 侧产出为独立验证项
4. **P11c/P11d 暂缓**：OrchestrationCore trait 归属待 Mind 认知工艺显式化后裁决；双向复用 = 模式同构 + 接口复用，职责不合并（脑手分离铁律不变）
5. **P11b 验证测试通过**：`p11b_suggested_actions_flow_to_execution`（adapter 消费 + agent_loop 全流程流转），9 集成测试全绿，全量 50 passed
**状态**：✅ 完成（P11a/P11b 裁决 + 验证闭环）
---
## 记录 4：预留
*（按 DNA v2.0 SOP，新记录追加至此，旧记录自动归档）*
