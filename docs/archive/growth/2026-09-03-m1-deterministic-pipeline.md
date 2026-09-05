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
