# ADR-0008: Tuck 深度集成——SecurityGate 接线点（候选 D'-2）

- **状态**: Accepted
- **日期**: 2026-09-06
- **关联**: ADR-0003（六 stage 确定性流水线）、ADR-0004（M1.5 生态合流）、ADR-0007（D'-1/D'-3 确定性指纹与启动接线）、Tuck ADR-0003（Cellrix 状态流）、Tuck P6-T3（AnaphaseBridge/TuckSecurityGate）
- **决策范围**: Anaphase pipeline（执行闸接线点）；Tuck 侧零改动

## 1. 背景与问题

候选 D' 四项中，D'-2（Tuck 深度集成）是 ADR-0023 管控闭环的咽喉：Anaphase pipeline 执行工具前，应经过 Tuck 的边缘物理闸（三闸门之三：工具审计 → HITL → Tuck）。Tuck 侧接口已就绪（P6-T3 `TuckSecurityGate::process(SecurityGateRequest) -> SecurityGateResponse`，serde wire 格式，catastrophic→Reject / critical→HITL 的默认策略）。

问题：
- Q1: Anaphase 的 pipeline 在哪里接 Tuck 闸门？stage 3（gRPC execute）之前。
- Q2: 如何接线而不破坏极致解耦？Tuck 明确"不依赖 anaphase-helix"；反向也不应让 anaphase 硬依赖 tuck-core crate（发布依赖）。
- Q3: 无 Tuck 时 pipeline 怎么办？现有 110 测试必须全绿（向后兼容）。

## 2. 决策

### D1: 接线点 = stage 3 执行前，Option 注入

`Pipeline` 新增 `security_gate: Option<Arc<dyn SecurityGate>>` 字段 + `with_security_gate()` builder。
`execute_calls` 对每条 call，在执行前先过 gate（若已注入）。`None` = 无闸门 = 既有行为，110 测试零影响。

### D2: 本地类型，零 Tuck 依赖（极致解耦）

新建 `src/security.rs`，定义 Anaphase 侧的闸门契约——不引入 tuck-core 类型：

```rust
pub struct GateCheck {
    pub job_id: String,
    pub index: u32,              // trace 确定性派生源 {job_id}#{index}（ADR-0003）
    pub tool: String,
    pub args_json: String,       // 审计用动作描述
    pub identity_label: Option<String>, // 审计标签透传（ADR-0004）
}
pub enum GateVerdict { Pass, Reject(String), HitlRequired(String), HardOverride }

#[async_trait]
pub trait SecurityGate: Send + Sync {
    async fn check(&self, check: &GateCheck) -> GateVerdict;
}
```

Tuck 适配（`SecurityGateRequest` 构造 + `Uuid v5` 确定性 trace_id + verdict 映射）属于部署/测试层，不进发布库。哲学对齐 Tuck 自身注释："transport handled by adapter layer in the deployment environment"。

### D3: 决策语义——Pass 放行，Reject/HITL 阻塞并记 ledger

| GateVerdict | pipeline 行为 |
|---|---|
| Pass | 放行（正常执行） |
| HardOverride | 放行（紧急放行，语义同 Pass 的执行路径） |
| Reject(reason) | 该 call 不执行，返回 `Err(reason)`，ledger 记 `Blocked` |
| HitlRequired(reason) | 该 call 不执行，返回 `Err(reason)`（升级信号），ledger 记 `Blocked` |

### D4: ledger 新增 Blocked 记录类型（不破坏 append-only）

`LedgerRecord` 增加独立变体 `Blocked { job_id, tool, index, reason, identity_label }`（record_type="blocked"）。
**不改** `VerdictStatus`（Met/Unmet 语义不动），**不改**既有 `Verdict` 变体的 JSON 形状——ADR-0003 决策 10 的 append-only 兼容原则。被闸门拦下的动作不是"判定未达标"，是"物理未执行"，必须诚实分型。

### D5: 测试双轨

- `tests/security_gate.rs`：mock gate（Pass 放行 / Reject 阻塞 + Blocked 落账 / HitlRequired 阻塞 / 无 gate 兼容）——验证接线逻辑，零外部依赖。
- `tests/tuck_gate.rs`：真实 `TuckSecurityGate` 连通（dev-dependency `tuck-core`，git branch rs）——验证 ADR-0023 管控闭环咽喉：Low PFP → Pass 放行执行，Catastrophic PFP → Reject 阻塞。
  - trace_id 确定性：`Uuid::new_v5(NAMESPACE, "{job_id}#{index}")`（name-based，非随机 v4——Tuck 内部 AuditLog 的随机 id 是 Tuck 既有行为，不扩散）。
  - PFP 构造：测试用 fixture 表（`{tool → PFP 字节}`），fixture 是合法来源（ADR-0003 的 fixture 契约精神），不引入运行时硬编码。

## 3. 备选方案与拒绝理由

| 备选 | 拒绝理由 |
|---|---|
| Anaphase 发布依赖 tuck-core crate | 破坏极致解耦——Tuck 不依赖 Anaphase，反向亦然；适配应留在部署层 |
| gate 决策逻辑复制进 Anaphase（本地策略引擎） | 重复实现 Tuck 的 PFP 决策；PFP 知识属于 Tuck（第一个消费者） |
| Blocked 并入 VerdictStatus 枚举 | 破坏 Met/Unmet 语义与 JSON 形状；append-only 兼容性受损 |
| 无 gate 时 fail-closed（默认拒绝） | 破坏 110 测试基线；D'-2 是可选增强，不改变既有确定性流水线语义 |

## 4. 后果

**正面**:
- 管控闭环咽喉落地：pipeline 执行路径物理上可被 Tuck 闸门拦截（PFP→决策→审计链）；
- 极致解耦保持：Anaphase 发布库零 Tuck 依赖，适配在测试/部署层；
- 向后兼容：无 gate 时行为逐字节不变，110 基线全绿；
- 确定性优先：trace_id 用 Uuid v5 name-based 派生，同一 job 同一调用序列 → 同一 gate 请求序列。

**负面/代价**:
- 真实 Tuck 连通测试依赖 git dev-dependency（首次 `cargo test` 需拉取 tuck-core）；
- 生产部署仍需一个适配器实现（本 ADR 不引入，属部署层债）。

**风险与对策**:
- tuck-core git 依赖不可达 → 该测试 `#[ignore]` 或失败可定位（dev-only，不影响发布库）。
- PFP 构造权在适配层 → fixture 表显式声明，运行时零硬编码。

## 5. 验收判据

1. `cargo test` 全绿：110 基线 + 新增测试（mock 4+ + tuck 连通 2+）；
2. Reject/HitlRequired 的 call 不执行且 ledger 出现 `blocked` 记录；
3. 无 gate 时既有测试逐字节通过（向后兼容）；
4. 代码注释全英文；无 UUID v4 新增使用（trace 派生用 v5）。
