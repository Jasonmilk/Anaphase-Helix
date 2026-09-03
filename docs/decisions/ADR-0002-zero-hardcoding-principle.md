# ADR-0002：DNA 原则 11 零硬编码（Zero Hardcoding）新增

## 状态
**Active**（2026-09-03，用户提议 + 严肃审查通过）

## 问题
1. Anaphase DNA 现有 10 条原则 + 6 条防腐化铁律，**均无对硬编码/魔法数的约束**
2. `src/agent_loop.rs` 实测存在 5 处硬编码：`amygdala_vector = (0.7, 0.3, 0.2)` / `reason(..., "left_brain")` / `p_death > 0.7` / `command = "echo"` / `for _ in 0..7`
3. M1 里程碑（tt_job 流水线）将新增大量阈值、占位、契约字段，若无顶层约束，硬编码风险将随 pipeline 扩散
4. 历史修正：M1 审查早期曾误引"DNA 铁律 7：消灭魔法"，经核验该条不存在——本 ADR 补上真实缺口

## 决策
1. **DNA 新增原则 11（零硬编码）**：代码中任何字面量（阈值/占位/模型名/循环上限/坐标向量/字符串常量）必须有来源——配置 / 常量表 / 派生规则。禁止裸硬编码。协议可选字段用协议默认空值（空 map / 空串），不造假占位。
2. **工程映射**：M1 起所有阈值/占位/魔法数走 `config` 或 `knowledge_base/` 契约文件。
3. **run_cycle 技术债**：既有 5 处硬编码为已知技术债，M1 旁路不动，M1.5 随 pipeline 接入 run_cycle（ADR-0003 决策 9 映射表）一并消除。
4. **层级关系**：原则 = 不可变公理（顶层）；防腐化铁律 = 防腐化规则（分层）。原则 11 约束所有设计决策，铁律不重复列举。

## 影响
- M1 的 `GrpcTentacleAdapter` 重写：`identity_labels` / `seen_entropy_bloom` 用协议默认空值（空 map / 空串），不造假占位
- M1 的 criteria 判据参数、retry_policy 全部来自 `knowledge_base/fixture-codex.json`
- 新增硬编码 = 违反 DNA 原则 11，代码审查可据此否决

## 关联
- Anaphase-Helix DNA.md 原则 11（本文档落地）
- ADR-0003（M1 里程碑执行架构，待 T9 落档）
- Helix-Mind ADR-0020（trace 根生成，原则 9 来源）
