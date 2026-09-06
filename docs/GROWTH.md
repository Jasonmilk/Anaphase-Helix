
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
