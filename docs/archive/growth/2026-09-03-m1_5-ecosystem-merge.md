## 记录 5：M1.5 生态合流核心完成（2026-09-03）
**变异类型**：跨仓库联调 + 真实连通 + 语义定义 + run_cycle 渐进接线
**背景**：
- M1.5 目标（ADR-0004）：Tentacle `--transport grpc` + fixture 插件 + 真实连通 + 语义定义 + run_cycle 渐进接线
- 前置：M1 mock 闭环完成（ADR-0003 决策 7 定义 M1.5 范围）
**关键决策与发现**：
1. **Tentacle grpc 接线（Tentacle d902151）**：main.rs 新增 `--transport grpc` + `--grpc-port`（默认 50051），复用 scan_plugins + ProcessTool 实例化 → TentacleGrpcService + tonic serve；tentacle crate 新增 tentacle-transport-grpc + tonic 依赖
2. **fixture 插件（Tentacle d902151）**：fixtures/numbers + rate（manifest+js，SHA-256 完整性校验，参数化 MET/UNMET）。**联调发现**：fixture 默认值必须满足判据契约——numbers 20 序列 sum=210 超 high=100、rate 10/20 的 cross_check 不过，修正为 1..=10 与 {10,10}（= M1 mock 形状）
3. **真实连通（Anaphase 1c1e723）**：`tests/m1_e2e_live.rs`（#[ignore]，手动联调）spawn 真实 tentacle 二进制 → pipeline 全链路，m1_5_live_met + m1_5_live_unmet 全绿——M1 mock → M1.5 真实无缝切换
4. **语义定义（Anaphase 6ac3a0e）**：identity_labels（纯标签供 Tuck 审计/渐进披露，绝不传凭证，BTreeMap 确定性注入）+ seen_entropy_bloom（可选重放守卫，当前不启用）；execute_tool_with_labels 新增
5. **run_cycle 渐进接线（Anaphase 4d7e3ec）**：tool_command（Option<String>）——配置后 Execution 派发真实工具名，未配置保持 echo fallback（向后兼容）；with_tool_command() builder；RecordingToolAdapter 双测试
**状态**：✅ 完成（78 测试全绿——lib 46 + integration 16 + m1_e2e 3 + mind 9 + mock 4；live 2 条 #[ignore] 手动验证）
---
