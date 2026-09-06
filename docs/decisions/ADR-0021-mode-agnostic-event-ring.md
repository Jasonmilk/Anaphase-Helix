# ADR-0021: 模式无关事件环——驾驶模式黑匣子

- **状态**: Accepted
- **日期**: 2026-09-06
- **决策范围**: Anaphase（run_cycle / main 装配 / events.rs 语义）
- **关联**: ADR-0019（O-2 事件总线）、ADR-0020（O-3 事件轨迹持久化）、ADR-0016（白盒四层）

## 1. 背景与问题

O-2/O-3（ADR-0019/0020）把事件环挂在 **pipeline 装配**上：`tentacle_endpoint`
未配置（驾驶模式 / Noop 装配）→ `shared_events = None` → 事件不产生、
`events.jsonl` 不写。物理核验确认：**驾驶模式没有 WAL**。

白盒四层在驾驶模式下：状态快照（候选 G，模式无关 ✓）、**过程事件 ✗**、
**事实 ledger ✗**。而驾驶模式恰恰最需要审计轨迹——人类开车时车必须记录
黑匣子（事故数据），且驾驶模式无 Mind 审查，安全边界靠 Tuck，但"发生了什么"
没有任何记录。这不是可接受的缺口：**Helix 的身体无论谁驾驶，呼吸都要可回放**。

## 2. 决策

### D1: 事件环是模式无关的，从 pipeline 提升到 AgentLoop 级
- 事件环在 `main.rs` **总是创建**（cap 来自 codex contract，与装配无关）；
- `AgentLoop.with_events(ring)` 注入——**无论是否装配 pipeline**，每次
  `run_cycle` 都记录 cycle 级事件；
- pipeline 装配时**复用同一个环**（`pipeline.events = shared_ring`），六 stage
  事件（stage 1..=6）与 cycle 级事件（**stage 0**，本决策保留语义）在
  **同一条流、同一个 `?after=` 游标**里——消费端无需合并两个源。

### D2: cycle 级事件契约（黑匣子四拍）
每次 `run_cycle`（一次周期）发射：
| phase | 内容 | detail |
|---|---|---|
| `begin` | 周期开始 | `{input, mode}` |
| `state` | 每次状态迁移 | `{from, condition, to}` |
| `tool` | 工具执行结果（echo fallback 路径） | `{tool, ok, result\|error}` |
| `end` | 周期结束 | `{done, success, impasse}` |

trace_id = `derive_job_id(input)`（确定性派生，一次周期一条 trace——与
stage 事件同源）。

### D3: 时间戳分权（两个确定性需求，两个来源）
- **cycle 级 ts = 墙钟**（`Utc::now` RFC3339）：黑匣子的审计真值是"何时发生"；
- **stage/ledger ts = 注入 FakeClock**：可回放契约（字节级一致）属于
  pipeline/ledger 层，已有测试锁定。
  同一个环里两类 ts 并存，各自服务各自的不变性——不强行统一成一个来源，
  因为"审计时间"与"回放时间"是两种不同的正确性。

### D4: flush/恢复语义不变，挂载点换到 agent
O-3 的增量 flush 与启动恢复逻辑不变，只是数据源从
`agent.pipeline.events` 改为 `agent.events`——驾驶模式同样持久化，
黑匣子跨重启可回放。

## 3. 备选方案与拒绝理由

| 备选 | 拒绝理由 |
|---|---|
| 驾驶模式单独建一个轻量轨迹（新 JSONL 格式） | 新实体；破坏"一个 ?after 游标"的统一语义 |
| 两个事件环（cycle 环 + pipeline 环） | 消费端要合并两流；"如无必要勿增实体" |
| 驾驶模式不记录（维持现状） | 白盒四层在驾驶模式缺两层；黑匣子是驾驶的安全底线 |

## 4. 后果

**正面**：
- 白盒四层**模式无关**：驾驶 = 黑匣子（cycle 轨迹），伙伴 = 黑匣子 + 六 stage；
  生存模式（将来）同构；
- 一个环、一个游标、一个端点——零新端点、零新实体；
- 驾驶模式事件同样走 O-3 持久化——崩溃跨重启可回放。

**代价**：
- 每次 cycle 多几次内存 emit + 每轮一次 flush（毫秒级，O-3 已接受）。

## 5. 验收

- `cargo test` 全绿：180 → **181**（+1 `drive_mode_black_box_records_cycle_events_without_pipeline`：
  Noop 装配跑 `run_cycle` → 环里 begin/state/end、stage=0、一条 trace）
- 物理验证：无 tentacle 装配真实二进制 → `events.jsonl` 写入 7 条
  stage=0 cycle 事件（begin → 多 state → end），trace_id 确定性派生
- 无回归：181 passed / 0 failed
