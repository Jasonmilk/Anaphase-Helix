# Anaphase-Helix 生长记录
> **版本**：v1.1
> **日期**：2026-08-28
> **规则**：仅保留最近 3 条记录，超则归档至 `docs/archive/growth/`
> **归档策略**：历史随仓库版本化，永不删除
## 记录 1：方法论体系补全（2026-08-28）
**变异类型**：方法论补全（对齐 Helix-Mind docs 结构）
**背景**：
- 参照 Helix-Mind docs 结构，Anaphase 缺 SPEC.md / RNA.md / DEPRECATE.md / spec/ 分卷
- VISION.md 为一体式（卷 A-E 内嵌），分卷导航指向 spec/ 但目录未建（断链）
**关键决策**：
1. **SPEC.md 新建**：一粒种子的自白（完整叙事）+ 分卷导航，与 Helix-Mind SPEC 角色对齐
2. **spec/ 分卷 6 个**：position / principles / architecture / contract / ci-144 / lifecycle，从 VISION 卷 A-E 拆分 + 架构蓝图 v11.0 提炼
3. **RNA.md 新建**：三层加载协议（活跃档案/分卷/考古）+ AI 协作铁律 + 大版本 SOP
4. **DEPRECATE.md 新建**：凋亡清单（DEP-001 Python legacy / DEP-002 FlowModus / DEP-003 REST 遗留）
5. **VISION.md 重构为根索引**（v2.0）：原子原则表补全至 10 条（对齐 DNA 实际 10 条），生态位置/组件仓库索引
6. **DNA.md 文档生态 SOP 补全**：加入 VISION/SPEC/RNA/DEPRECATE/spec 职责
7. **GROWTH 归档**：最旧"初始化"记录移入 `docs/archive/growth/2026-08-28-initialization.md`
**状态**：✅ 方法论体系补全完成，T2 提交前已就绪
---
## 记录 2：方法论迁移 + P10a 契约对齐（2026-08-28）
**变异类型**：方法论迁移 + 生态对齐
**背景**：
- Helix-Mind P0-P7 已完成，契约 v4.1 已冻结
- Anaphase-Helix 尚未建立 DNA 方法论框架，契约存在漂移（缺 budget_tier/traceparent/activation_vector）
- 需要将 DNA 方法论 v2.0 迁移至 Anaphase-Helix，并执行 P10a 契约对齐
**关键决策**：
1. 方法论迁移遵循"骨架复用，内容重写"原则——PLAN/GROWTH/ADR 流转机制完全复用，VISION/DNA 按 Anaphase 哲学独立编写
2. 优先对齐 Helix-Mind v4.1 契约，CI-144 v2.0 不阻塞（独立演进线）
3. 早期 Python 代码中的"强制苏醒/认知脱水/接力熔断"已正确并入 DNA 原则
4. DNA 8 条原则冻结，后续补齐至 10 条（trace 根生成 / 生态兼容），纳入 VISION.md 和 DNA.md
**状态**：✅ 方法论已建立，P10a 契约对齐进行中
---
## 记录 3：预留
*（按 DNA v2.0 SOP，新记录追加至此，旧记录自动归档）*
