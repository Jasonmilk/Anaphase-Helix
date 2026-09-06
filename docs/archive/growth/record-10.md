## 记录 10：候选 D'-4（真实场景插件）完成（2026-09-06）
**变异类型**：生态链路全通（MCP-Learner 学习 → post_learn 审查 → Tentacle 加载 → Anaphase 执行 → 判据 → 账本）
**前置修复**：MCP-Learner 1 个失败测试——根因是测试断言滞后（断言旧的无 `.manifest` 后缀文件名），实现产出 `{name}.manifest.json` 是生态契约（Tentacle 插件扫描依赖，联调修复 #2）；修断言，42+1f → 43 passed
**关键决策与发现**：
1. **Expect::Ok 判据（D1）**：`contract::Expect` 新增 `Ok` 变体（serde lowercase）——真实插件工具无统一数值形状，判据=纯结构断言 `exec_ok(ok_flag, echoed)`（零阈值零硬编码）；字段来源=执行体契约 `mcp_proxy.js`（`{ok:true, data:{tool, params}}`）；现有 Numbers/Rate/Text 不动（向后兼容）
2. **未知工具边界（D2）**：Tentacle grpc 未注册工具返回 `Status::not_found` → transport Err → pipeline Err（M1 single-pass：执行错误报错不落账不重试）；UNMET 仅用于"工具存在但判据不过"——物理核验修正了我初始的 UNMET 误判
3. **live 验收（D3）**：tests/m1_5_d4_live.rs 3 例（#[ignore]）——插件目录参数化（TENTACLE_PLUGINS_DIR 默认 /tmp/d4-learn/stable）；真实插件 MET / 未知工具 Err / run_cycle 全链路 MET；**实测 3/3 全绿**（真实 tentacle 二进制 + node + 学习产物）
4. **执行体占位如实标注**：mcp_proxy.js 是占位实现（echo 参数），真实 MCP 代理执行属 ECOSYSTEM 第二优先级 #4——D'-4 证明"链路真实"，不冒充"执行真实"
**状态**：✅ 完成（124 测试全绿——lib 61 + integration 16 + m1_e2e 3 + mind 9 + mock 4 + run_cycle_pipeline 8 + episode 10 + replay_guard 4 + security_gate 6 + tuck_gate 3；live 6 条 #[ignore]；生态合计 1228）
