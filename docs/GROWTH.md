# Anaphase-Helix 生长记录
> **版本**：v1.4
> **日期**：2026-09-03
> **规则**：仅保留最近 3 条记录，超则归档至 `docs/archive/growth/`
> **归档策略**：历史随仓库版本化，永不删除


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
*（记录 2 已归档至 docs/archive/growth/2026-09-03-p10c-lifecycle-ecosystem.md）*
