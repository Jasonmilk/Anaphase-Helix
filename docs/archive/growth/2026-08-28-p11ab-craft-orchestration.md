# Anaphase-Helix 生长记录（归档）

## 记录：P11a/P11b 认知工艺双向复用轨道调研完成（2026-08-28）

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

*归档于 2026-09-03（GROWTH v1.5 超出 3 条保留上限）*
