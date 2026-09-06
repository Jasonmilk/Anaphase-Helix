
## 记录 24：P10d 预约制闹钟——Anaphase 唤醒侧接线（2026-09-06，ADR-0032）

### 触发条件
Mind 侧 ana_wakeup RPC 完成后，Anaphase 侧唤醒发起接线（每交互看表一次）。

### 变更性质
- **MemoryAdapter 三方法**：wakeup（列出 due）/ wakeup_ack（done/renewed）/ consolidate（helix_consolidate kind）——默认 Err 静默降级，GrpcMindAdapter 真实实现
- **run_cycle 入口 check_wakeup**：白名单 action → consolidate 链 → ack done；未知 → ack done 释放（不执行但释放，永不死锁）；失败 → ack done 记录；不可用 → 跳过
- **RunCycleConfig**：wakeup_enabled（默认 true）/ wakeup_jitter_minutes（默认 60）/ wakeup_actions（默认 [hibernate]），serde default 保老 TOML 兼容
- **测试**：+4（触发执行/白名单外/无预约/降级），21 套件全绿 0 warning，198→202

### 兼容性
零破坏：proto Append-Only；新字段 serde default；Noop 零改动静默降级。

### 验收
PLAN（P10d 段）｜ README（202 tests）｜ ECOSYSTEM v1.51（Anaphase 202，全生态 1414）

### 状态
🧬 已完成
## 记录 25：P10 收尾——gRPC 级闭环 + live 物理验证（2026-09-06）

### 触发条件
P10d 接线完成后收尾核对：发现 gRPC 级闭环测试缺失（MockMind 有 stub 无测试），补上真实通道验证。

### 变更性质
- **mind_integration +3**：craft_via_grpc（确定性 trace）/ wakeup+ack_via_grpc（due alarm 走真实 wire + ack 到达 mock）/ consolidate_via_grpc（睡眠复盘链）
- **mind_live.rs 新增（#[ignore] 手动联调）**：起真实 helix-mind-cli 二进制（临时 config + 临时库 + 随机端口）→ GrpcMindAdapter 真实客户端 → craft/wakeup/consolidate 全链路**物理验证通过**
- **测试**：202→205（+3 gRPC 级）+ 1 live（ignored），22 套件全绿 0 warning

### 兼容性
零破坏；live 测试不进入常规套件（需 Mind 二进制，文档写明运行方式）。

### 验收
README（205 + P10 live 段）｜ PLAN（gRPC + live）｜ ECOSYSTEM v1.52（Anaphase 205，全生态 1417）

### 状态
🧬 已完成
## 记录 26：命名纪律——阶段号测试名全量改能力名（2026-09-06）

### 触发条件
用户审查 p10_live.rs 命名后确立硬规则：**内部命名必须准确无歧义、按能力命名，经用户通过方可保留**；阶段代号（M1/M1.5/P10/D4）不得用作测试文件名。

### 变更性质
| 原名 | 新名 | 被测能力 |
|---|---|---|
| m1_e2e_live.rs | tentacle_live.rs | 真实 Tentacle 连通 + fixture 判据 |
| m1_5_d4_live.rs | plugin_live.rs | MCP 学习产物真实插件执行 |
| m1_e2e.rs | pipeline_e2e.rs | pipeline 双 mock 闭环 |

函数同步去阶段前缀（m1_5_live_met→tentacle_live_met 等 9 处）；README/PLAN/ADR-0004/0005/0009 现存引用全量修正（archive 历史记录保留原名——当时事实）。

### 兼容性
零行为变化（纯改名）；`git mv` 保留历史。

### 验收
22 套件全绿 0 warning；live 三套真实联调全绿：tentacle_live 3/3 + plugin_live 3/3（真实 tentacle 二进制）+ mind_live 1/1（真实 Mind 二进制）。

### 状态
🧬 已完成
