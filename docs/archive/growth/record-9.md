## 记录 9：候选 D'-2（Tuck 深度集成——SecurityGate 接线点）完成（2026-09-06）
**变异类型**：管控闭环咽喉落地（ADR-0023 的 Anaphase 侧实体）
**背景**：
- 候选 D' 目标（ADR-0007）：M1.5 深化四项；D'-1/D'-3 已完成；D'-2（Tuck 深度集成）此前标"阻塞（Tuck 侧接口）"——物理核验发现 Tuck P6-T3 的 TuckSecurityGate（anaphase_bridge.rs）早已就绪，本轮解除阻塞落地
- 前置：Tuck P0-P7 全部完成（P6-T3 AnaphaseBridge + TuckSecurityGate 是 D'-2 的 Tuck 侧接口）
**关键决策与发现**：
1. **接线点（D1）**：`Pipeline.security_gate: Option<Arc<dyn SecurityGate>>` + `with_security_gate()`；execute_calls 对每条 call 执行前过闸（三闸门之三：工具审计→HITL→Tuck 的执行前位置）；`None` = 无闸门 = 110 基线逐字节不变
2. **零依赖契约（D2）**：`src/security.rs` 定义 Anaphase 本地 `SecurityGate` trait + `GateCheck`（job/index/tool/args/labels 全事实）+ `GateVerdict`（Pass/Reject/HitlRequired/HardOverride）——发布库不依赖 tuck-core（极致解耦，适配在部署/测试层，对齐 Tuck 自身"transport handled by adapter"注释）
3. **决策语义（D3/D4）**：Pass/HardOverride 放行；Reject/HitlRequired 阻塞 call（不执行、不进 Tentacle）并写 ledger `Blocked` 记录（独立 record_type，**不改** VerdictStatus/既有 Verdict JSON 形状——ADR-0003 append-only 兼容）；错误信息带闸门 reason
4. **确定性（D5）**：trace_id 用 `Uuid::new_v5`（name-based 确定性，同一 job#index → 同一 gate 请求序列）；无新增 UUID v4
5. **验证**：tests/security_gate.rs 6 例（mock 闸门：无闸门兼容/Pass 放行/HardOverride 放行/Reject 阻塞+Blocked 落账+未触达 wire/HitlRequired 阻塞/事实全量透传）+ tests/tuck_gate.rs 3 例（真实 TuckSecurityGate：Low→Pass 执行 / Catastrophic→Reject 阻塞 / Critical→HitlRequired 映射；dev-only git 依赖 tuck-core，tuck-core 的 InMemoryCredentialStore 是 #[cfg(test)] 不可用 → 测试侧实现真实 CredentialStore trait）
**状态**：✅ 完成（121 测试全绿——110 基线 + 2 security lib + 6 security_gate + 3 tuck_gate；live 3 条 #[ignore]；生态合计 1201）

---
