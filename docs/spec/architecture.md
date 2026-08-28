# Anaphase-Helix 核心架构（spec/architecture.md）

> **分卷 v1.0**（2026-08-28）｜ 继承架构蓝图 v11.0 + VISION 卷 C/E ｜ 方法论 DNA v2.0

## 模块化编排（大脑分区）

Anaphase 将认知动作映射为大脑分区模块，全部通过 DTO 契约通信（见 `spec/contract.md` 的 gRPC 对齐后版本）。

| 模块 | 别名 | 职责 | 关键 DTO |
|---|---|---|---|
| **杏仁核** | `helix.amygdala` | 前置优先级评估 + 后置情感评估（情感后置，原则 6） | `PriorityAssessment` / `AffectVector` |
| **前额叶** | `helix.prefrontal` | 核心推理（无情感干扰的清澈上下文） | `ReasoningRequest` / `ReasoningDraft` |
| **突触** | `helix.synapse` | 工具执行（先审计，原则 5） | `ExecutionRequest` / `ExecutionResult` |
| **海马体** | `helix.hippocampus` | 记忆检索/写入（由 Helix-Mind 实现） | `MemoryQuery` / `MemoryFragments` |

> **契约演进注记**：架构蓝图 v11.0 曾以 RESTful API（`/v1/mind/search` 等）对接 Helix-Mind；ADR-0001 起对齐 Helix-Mind 冻结的 **gRPC v4.1 契约**。当前及未来以 gRPC 为准，REST 为历史遗留参考。

## Agent Loop 时序（无情感干扰原则）

```
1. 前置评估（Amygdala.assess_priority → priority_score / intent）
2. 记忆加载（Hippocampus.fetch_relevant，按 intent + task 检索 L1/L2/L3）
3. 核心推理（Prefrontal.reason，上下文不含情感向量）
4. 后置情感评估（Amygdala.assess_affect，仅在即将回复时）
5. 回复合成（AffectVector 注入润色，不影响工具执行）
6. 工具执行（Synapse.execute，经 ToolAuditor 审计）
7. 记忆写入（Hippocampus.record_hxr 追加 L3）
```

## 动态硅基代谢（资源感知）

- **纪元（Epoch）**：每次唤醒为一个纪元，生命力由 Token 预算决定
- **疲劳线 80%**：触发记忆结网（提炼核心结论与未尽事宜 → 海马体），准备重生
- **凋亡线 95%**：强制结束纪元，输出阶段性结论
- **工作记忆提纯**：突触原始输出绝不直接进上下文，必须预压缩（见 `spec/lifecycle.md`）

## 可插拔扩展机制（核心稳定，扩展可插拔）

| 挂载件 | 挂载点 | 要求 |
|---|---|---|
| `AffectEnricher` | 后置评估后、回复合成前 | 无副作用，可随时禁用 |
| `MemoryAnnotator` | HXR 写入时异步 | 非阻塞，失败不影响主流程 |
| `ToolAuditor` | 首次调用/非白名单源时 | 可配置严格程度，默认宽松 |

## 物理隔离沙盒

不同任务在物理目录隔离运行（`/workspaces/{task_id}/`），互不污染。任务结束后沙盒可保留或销毁。
