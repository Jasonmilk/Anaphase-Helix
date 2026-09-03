# Anaphase-Helix 生长记录
> **版本**：v1.3
> **日期**：2026-09-03
> **规则**：仅保留最近 3 条记录，超则归档至 `docs/archive/growth/`
> **归档策略**：历史随仓库版本化，永不删除


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
*（记录 1 已归档至 docs/archive/growth/2026-09-03-p10b-cognitive-craft-link.md）*
