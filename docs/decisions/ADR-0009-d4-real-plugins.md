# ADR-0009: 真实场景插件——MCP-Learner 产物接入确定性执行（候选 D'-4）

- **状态**: Accepted
- **日期**: 2026-09-06
- **关联**: ADR-0003（六 stage 确定性流水线）、ADR-0004（M1.5 生态合流）、ADR-0005（候选 E：run_cycle↔pipeline 全 merge）、ADR-0007（D'-1/D'-3）、ADR-0008（D'-2 Tuck SecurityGate）、Helix-MCP-Learner post_learn 管道
- **决策范围**: Anaphase（Expect::Ok 判据扩展 + live 集成测试）；MCP-Learner（失败测试修复，43 passed）；Tentacle 零改动

## 1. 背景与问题

候选 D' 四项中，D'-4（真实场景插件）是最后一项：把 pipeline 的执行通道从 fixture 插件（numbers/rate，数值契约）扩展到**真实场景插件**——MCP-Learner 学习 mock MCP Server 后经 post_learn 审查产出的 `stable/` 工具（`mock-filesystem.list_files` 等 4 个，`.manifest.json` 后缀 + SHA-256 校验）。

前置阻塞解除（本轮物理核验）：
- MCP-Learner 有 1 个失败测试（`test_process_and_write_creates_directories`）——根因是**测试断言滞后**：断言旧的无 `.manifest` 后缀文件名，而实现产出 `{name}.manifest.json`（生态契约，Tentacle 插件扫描依赖该后缀，联调修复 #2）。实现正确，断言过时 → 修断言，43 passed / 0 failed。
- 真实插件执行体 `mcp_proxy.js` 目前是**占位实现**（echo 参数，输出 `{ok:true, data:{tool, params, message}}`），真实 MCP 代理执行是 ECOSYSTEM 第二优先级 #4，不属于 D'-4 范围。

问题：
- Q1: 真实插件工具的输出形状（`{ok, data:{...}}`）与现有判据契约（Numbers/Rate/Text 的数值形状）不匹配——真实工具用什么判据？
- Q2: 真实插件链路如何验收？live 测试的模式是什么？
- Q3: 未知工具名的行为边界？

## 2. 决策

### D1: Expect::Ok —— 结构化执行成功判据（最小增量）

`contract::Expect` 新增 `Ok` 变体（serde lowercase → `"ok"`）。判据零阈值、零硬编码：**纯结构断言**——

```rust
pub fn exec_ok(ok_flag: bool, echoed: bool) -> CheckReport {
    let passed = ok_flag && echoed;
    CheckReport { check: "exec_ok".into(), passed, detail: format!("ok={ok_flag} echo={echoed}") }
}
```

字段来源 = 执行体契约：`mcp_proxy.js` 占位实现明确输出 `{"ok": true, "data": {"tool", "params", ...}}`（物理事实，非臆造）。`run_for_expect(Ok)` 读 `data.ok`（bool）+ `data.data.params`（args 回显非 null）。

为什么是结构断言而非数值断言：真实场景工具（list_files 等）没有统一数值形状；"工具被真实调用 + 参数真实透传"是执行成功的物理证据，数值判据留给 fixture 契约（Numbers/Rate/Text 不变，向后兼容）。

### D2: 未知工具 = pipeline 错误，不是 UNMET

Tentacle gRPC 对未注册工具返回 `Status::not_found` → transport 层 Err → pipeline `?` 传播 → `run()` 返回 Err。这符合 M1 single-pass 哲学（ADR-0003 decision 5）：**执行错误报错，不落 evidence、不重试**；UNMET verdict 仅用于"工具存在但判据不过"。边界写入测试（`m1_5_d4_live_unknown_tool_errors`）。

### D3: live 验收模式（#[ignore] 手动联调）

新增 `tests/m1_5_d4_live.rs`（3 用例，#[ignore]，与 m1_e2e_live 同模式）：
- 插件目录参数化：`TENTACLE_PLUGINS_DIR`（默认 `/tmp/d4-learn/stable`，MCP-Learner 学习产物）
- `m1_5_d4_live_plugin_met`：真实插件 `mock-filesystem.list_files` + expect ok → MET（retry_due none）
- `m1_5_d4_live_unknown_tool_errors`：未知工具 → pipeline Err（含 "not found"）
- `m1_5_d4_live_run_cycle_real_plugin`：run_cycle 全链路（Reasoning→structured calls→Execution 真实 grpc→Reflection criteria→ledger）走真实插件 → Verdict::Met

**实测 3/3 全绿**（真实 tentacle 二进制 + node + 学习产物）。

### D4: 产物链路（MCP-Learner 学习 → post_learn → Tentacle 加载 → pipeline 执行）

```
mcp-learner learn --command python3 --args tests/mock_mcp_server.py \
  --name mock-filesystem --output /tmp/d4-learn
    → stable/ 4 工具（.manifest.json + SHA-256 + mcp_proxy.js）
tentacle --transport grpc --plugins-dir /tmp/d4-learn/stable
    → scan_plugins 校验哈希 → ProcessTool(node) 实例化
anaphase pipeline → execute_tool_with_labels → 真实执行 → evidence → criteria(ok) → ledger
```

Tentacle 零改动（M1 约束延续）；MCP-Learner 仅修过时断言（非实现改动）。

## 3. 备选方案与拒绝理由

| 备选 | 拒绝理由 |
|---|---|
| 给真实工具做数值判据（映射到 Numbers/Rate） | 真实工具无统一数值形状；强行映射是臆造契约 |
| 升级 mcp_proxy.js 为真实 MCP 执行 | 属 ECOSYSTEM 第二优先级 #4（mcp_proxy.js 升级），D'-4 范围 = 链路成立，执行体占位如实标注 |
| 把学习产物固化为测试资产 | 产物依赖 node + mock server，固化会引入生成时点歧义；live 模式 + env 参数化保持单一事实来源 |

## 4. 后果

**正面**：
- D'-4 完成，候选 D' 四项全部落地（D'-1 重放守卫 / D'-2 Tuck 闸门 / D'-3 接线 / D'-4 真实插件）
- 生态链路"学习→审查→加载→执行→判据→账本"全通，fixture 与真实插件双轨并行
- 判据扩展向后兼容（现有 3 变体不动，110→124 基线全绿）

**负面/代价**：
- `Expect::Ok` 是结构判据，不校验业务语义（占位执行体下只证明"被调用+透传"）
- live 测试依赖 node + 学习产物，CI 不跑（#[ignore]），需手动联调

**风险与对策**：
- mcp_proxy.js 占位实现被误当生产执行 → README/ADR 显式标注占位 + 升级任务挂 ECOSYSTEM 第二优先级 #4
- 未来真实 MCP 执行改变输出形状 → Ok 判据的字段读取以执行体契约为准，契约变更走 ADR

## 5. 一句话总结

> fixture 证明"判据是确定性的"，真实插件证明"链路是真实的"——
> MCP-Learner 学到的每一个工具，都能经 Tentacle 的手，走完 Anaphase 的六 stage，落进账本。
