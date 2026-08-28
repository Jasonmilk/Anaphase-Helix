# Anaphase-Helix 与 Mind 的契约（spec/contract.md）

> **分卷 v1.0**（2026-08-28）｜ 继承 VISION 卷 C + ADR-0001 ｜ 方法论 DNA v2.0

## C.1 通信方式

- **协议**：gRPC（tonic）
- **契约版本**：v4.1（已冻结，详见 Helix-Mind proto `helix_mind.proto`）
- **传输**：本地 UDS（默认）+ 远程 TCP（mTLS 预留）
- **对齐原则**：优先对齐 Helix-Mind v4.1（最新）；与 Anaphase 旧蓝图冲突时，以 Helix-Mind 为准，冲突需双方研讨权衡

## C.2 核心 RPC

| 方法 | 方向 | 用途 |
|---|---|---|
| `HelixQuery` | Anaphase → Mind | 检索知识/记忆 |
| `HelixConsolidate` | Anaphase → Mind | 触发代谢（睡眠管道） |
| `FederatedDAGShare` | Anaphase → Mind | 联邦知识共享 |
| `TriggerReincarnation` | Anaphase → Mind | 触发轮回 |

## C.3 Anaphase 传入参数（契约对齐，ADR-0001）

| 字段 | 来源 | 状态 |
|---|---|---|
| `EnergyContext.budget_tier` | Anaphase 状态/负载推导 | ✅ P10a T2 已实现 |
| `EnergyContext.system_load` | 系统探针（sysinfo） | ✅ P10a T2 已实现 |
| `traceparent` | Anaphase 生成（根，W3C） | ✅ P10a T2 已实现 |
| `HelixQueryResult.activation_vector` | Mind 返回（预留接收） | 🔜 待接入 |

## C.4 触发链路

- `mind_endpoint` 接线（config）+ 集成测试（起 Mind 服务连真实端点，验证闭环）
- 降级链：`docs/design/dependency-fallback.md`（Mind 不可用 → fail-open；Tuck 除外）

## C.5 测试契约（ADR-0001 T3 决议）

- Anaphase ↔ Mind 契约闭环通过 **mock Mind server** 验证（`build.rs build_server(true)` 生成 server trait，测试内实现），**不依赖真实 Helix-Mind 二进制**（跨仓库解耦、CI 可行）
- mock 覆盖：正常闭环 / trace 透传（W3C）/ budget_tier 传递 / Mind 离线 fail-open
- mock 停机用 `serve_with_incoming_shutdown` + oneshot（优雅关闭已接受连接）

## C.6 记忆契约语义（body-agnostic）

Mind 不产出动作，只通过 gRPC 记忆契约被身体驱动。契约不烘焙任何身体特定假设——Helix-Mind 兼容通用生态（harness、openclaw 等），Anaphase 是其中一个身体实现。

## C.7 预算路由划界（P10b T1 确认，ADR-0010 / Helix-Mind cognitive-craft）

- **链路已验证**：Anaphase 前置推导 `budget_tier` → proto 传递 → Mind `layer3.rs` 接收并映射 core `BudgetTier` → 传入 `retrieval.query`（P10b T1 消费点核查确认）
- **划界**：预算路由（外部前置，ADR-0010）决定检索**扫描范围**（相态）；System 0（Mind 内）决定思考**深度**。二者正交
- **retrieval 层相态过滤**是 Helix-Mind 侧独立任务（Mind 侧 P 阶段），Anaphase 不强迫其实现（勿过度设计）；Anaphase 职责 = 正确推导并传递 budget_tier

## C.8 HITL 审批通道与工具审计串联（P10b T3 决议，ADR-0001）

**三层闸门**：工具审计（入库门）→ HITL（执行闸）→ Tuck（边缘物理闸）。

| 闸门 | 时机 | 对象 | 职责 |
|---|---|---|---|
| **工具审计**（DNA 原则 5） | 工具首次入库前（一次性） | 工具本身 | `ToolAuditor.approve()` CLI 审查，安全后方可入库 |
| **HITL 审批**（DNA 原则 4） | 高风险动作**每次执行前** | 具体动作 | `check_hitl_approval(action, params)` 挂起至人类确认；低风险零延迟 |
| **Tuck**（边缘闸） | 出网/凭证使用 | 物理边界 | 凭证隔离、硬拦截；启用后不可停摆（可配置硬依赖） |

**串联语义**：审计管"工具能不能入库"，HITL 管"这次动作能不能执行"，Tuck 管"物理边界与凭证"。三者正交，不可互相替代。高风险动作 = 写操作 / 网络请求 / 凭证使用。
