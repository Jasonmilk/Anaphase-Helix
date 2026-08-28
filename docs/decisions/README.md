# Anaphase-Helix 架构决策记录（ADR）

> **DNA 方法论 v1.0** ｜ **ADR 两态法则**：架构决策只有 Draft（草案）和 Active（生效）两种状态。Active 后不可覆写，只能新建 ADR 并标记旧者为 `Superseded`。

## 规范

1. **文件命名**：`NNNN-<kebab-case-title>.md`（序号递增，如 `0001-mind-contract-alignment.md`）。
2. **状态**：每个 ADR 顶部声明 `Status: Draft | Active | Superseded`。
3. **生命周期**：
   - `Draft` → 讨论/草拟，可修改。
   - `Active` → 已生效冻结，不可覆写；发现错误 → 新建 ADR 标记旧者 `Superseded`。
   - `Superseded` → 被新 ADR 取代（保留原文，永不删除）。
4. **内容模板**：决策背景 → 决策 → 理由 → 影响 → 状态。
5. **与三层联动**：生态级变更必须同时更新 N 层（VISION/DNA）、D 层（ADR）、A 层（代码）。

## 决策索引

| # | 标题 | 状态 | 日期 |
|---|---|---|---|
| 0001 | 契约对齐 + 方法论迁移（budget_tier/traceparent/activation_vector 补全 + P10a 触发链路） | Active | 2026-08-28 |

---

*ADR 是记忆不可篡改在工程层的投射。*
