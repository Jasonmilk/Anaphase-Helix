# ADR-0001：Anaphase-Helix 契约对齐 + 方法论迁移
## 状态
**Active**（2026-08-28，已审查通过）
## 问题
1. Helix-Mind P0-P7 已完成，契约 v4.1 已冻结（含 `budget_tier=9` / `traceparent=7` / `activation_vector=13`）
2. Anaphase-Helix 契约停留在旧版本，缺 3 个 Append-Only 字段
3. Anaphase-Helix 尚无 DNA 方法论框架，所有改动缺乏决策锚点
4. 认知工艺 P8-P9 已完成，但 Anaphase 不传 `EnergyContext.budget_tier`，认知工艺的预算输入永远为空
## 决策
1. **方法论迁移**：将 DNA 方法论 v2.0（五件套 + 归档 + ADR 两态）迁移到 Anaphase-Helix
   - 方法论机制（PLAN/GROWTH/ADR 流转）完全复用
   - VISION/DNA 内容按 Anaphase 哲学重写（不拷贝 Helix-Mind）
   - 早期 Python 代码中的“强制苏醒/认知脱水/接力熔断”并入 DNA 原则
2. **契约对齐（P10a）**：Anaphase 侧补全 3 个缺失字段
   - `EnergyContext.budget_tier`：从 Anaphase 状态推导（用户层级/任务复杂度）
   - `traceparent`：Anaphase 作为根，生成 W3C traceparent 透传给 Mind
   - `HelixQueryResult.activation_vector`：预留接收（P4 已兑现）
   - 硬编码改状态推导（suggested_mode/autonomy_level）
3. **CI-144 v2.0 不阻塞**：PAL 是身体层协议，独立演进，不阻塞 P10a
   - P10a 只对齐已冻结的 v4.1 契约
   - 等 CI-144 v2.0 冻结后再接入生态手套路由
4. **触发链路**：`mind_endpoint` 接线 + 集成测试（起 Mind 服务连真实端点，验证闭环）
## T3 执行决议（2026-08-28 追加，ADR-0001 的执行细节补充，非新决策）
以下为 T1→T3 执行中确认的工程决策，属 ADR-0001 范围内落地细节：
- **build.rs `build_server(true)`**：生成 server trait，供集成测试实现 mock Mind（Anaphase 本身仅作 client，不启用 server）。集成测试**不依赖真实 Helix-Mind 二进制**（跨仓库解耦、CI 可行），mock 验证契约闭环。
- **mock server 优雅停机机制**：`serve_with_incoming_shutdown` + oneshot 信号，确保已接受连接也优雅关闭（`abort()` 只停外层 task，不停已接受连接）。
- **`resolve_memory_adapter` 抽取为可测装配**：从 main.rs 抽至 `src/adapters/mod.rs`，语义为"空端点→Noop / 连接失败→warn+Noop（fail-open）/ 成功→GrpcMindAdapter"。对应 DNA 铁律 6（所有依赖必须有降级策略）。
- **契约对齐验收**：31 测试全绿（10 单元 + 14 既有集成 + 7 T3 集成），覆盖正常闭环 / trace 透传 / budget_tier 传递 / Mind 离线 fail-open。

若 Anaphase 契约对齐导致现有行为重大偏差，或集成测试覆盖不足产生回归，可回滚至对齐前状态，重新评估实施范围。
## 关联
- Helix-Mind ADR-0010（预算路由）
- Helix-Mind ADR-0020（CI-144 对齐）
- Helix-Mind ADR-0021（认知工艺）
