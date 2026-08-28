# Anaphase-Helix 核心原则（spec/principles.md）

> **分卷 v1.0**（2026-08-28）｜ 继承 VISION.md 卷 B + DNA.md ｜ 方法论 DNA v2.0
> **冻结源**：`docs/DNA.md`（宪法，10 条不可变原则）。本文档为原则的规格化与工程映射，不重复宪法全文。

## 10 条不可妥协的公理（工程映射）

| # | 原则 | 工程映射（rs 分支） |
|---|---|---|
| 1 | **脑手分离，无锁分立** | `src/adapters/mind.rs`（gRPC 契约，只读响应）；记忆写入走 `HelixConsolidate` RPC |
| 2 | **零信任凭证隔离** | `src/adapters/tuck.rs`（预留）只传凭证标签，明文凭证永不进入内存；出网强制经 Tuck 代理 |
| 3 | **工作态归 Anaphase** | 工作记忆 `src/working_memory.rs`（待建）；任务 DAG `src/task_dag.rs`（待建）；强制苏醒/认知脱水 `src/lifecycle.rs`（待建）；**生命体征感知** `src/adapters/mind.rs` `probe_system_load()`（已实现） |
| 4 | **HITL 人在回路审批** | `src/lifecycle.rs`（待建）`check_hitl_approval()` 在 `SYS_EXECUTE` 前强制调用 |
| 5 | **工具审计 + 物理隔离** | `src/audit.rs`（待建）`ToolAuditor.approve()`；`src/sandbox.rs`（待建）`/workspaces/{task_id}/` 独立目录 |
| 6 | **情感后置** | `src/amygdala.rs`（待建）`assess_affect()` 在推理后计算，仅用于 `EnergyContext`，不注入推理 |
| 7 | **双重熔断** | Token 熔断（疲劳线 80% / 凋亡线 95%）+ 任务接力熔断（默认 5 次），`src/lifecycle.rs`（待建） |
| 8 | **事件驱动，队列闲时调度** | `src/scheduler.rs`（待建）事件入队 + 空闲消费，严禁轮询 |
| 9 | **trace 根生成**（ADR-0020） | 每个外部请求入口生成 W3C traceparent，Mind 只透传不生成 |
| 10 | **生态兼容** | 优先 Helix 生态，兼容通用生态；adapter 可替换/可降级，编排层只依赖契约 |

## 关键澄清（感知职责）

原则 3 显式包含感知职责：Anaphase 是身体的感知器官，持续感知宿主系统与资源状态（生命体征）作为 `EnergyContext` 输入 Mind。生态手套（MCP/宇树/Unity/鸿蒙）可用性感知为 P10b 渐进扩展（勿增实体）。

## 依赖降级（DNA 铁律 6）

所有依赖必须有降级策略，组件独立降级；**已启用的 Tuck 不可降级**（fail-closed）。是否启用 Tuck 由用户决定（不强迫用户，debug 可关闭）。见 `docs/design/dependency-fallback.md`。
