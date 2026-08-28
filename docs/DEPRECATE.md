# 凋亡清单（Anaphase-Helix）

规则：每条必须有明确死期。已安葬项移入 `archive/deprecated/`。

---

## DEP-001: 早期 Python 代码（legacy 桥接层）

- **原因**：方法论迁移至 Rust（rs 分支），Python 原型中部分逻辑并入 DNA 原则（强制苏醒/认知脱水/接力熔断），其余为历史遗留
- **替代**：rs 分支模块（`src/lifecycle.rs`、`src/working_memory.rs`、`src/task_dag.rs` 待建）
- **截止**：2026-09-30
- **状态**：⏳ 迁移中

## DEP-002: FlowModus 桥接依赖

- **原因**：FlowModus 与 Helix-Mind 哲学不同（Helix-Mind 侧已物理删除 flowmodus.rs 轮询循环）；Anaphase 的 FlowModus adapter 待状态明确后澄清
- **替代**：待 FlowModus 状态明确或 P10a 推进时再澄清（ADR-0001 记录）
- **截止**：待定（不阻塞 DNA 方法论）
- **状态**：⏳ 观察中

## DEP-003: RESTful 记忆接口（蓝图 v11.0 遗留）

- **原因**：架构蓝图 v11.0 以 REST（`/v1/mind/search` 等）对接 Helix-Mind；ADR-0001 对齐冻结的 gRPC v4.1 契约
- **替代**：gRPC（tonic）`HelixQuery` / `HelixConsolidate` 等
- **截止**：2026-09-30
- **状态**：⏳ 迁移中
