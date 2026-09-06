# ADR-0020: O-3 事件轨迹持久化——跨重启可回放的过程白盒

- **状态**: Accepted
- **日期**: 2026-09-06
- **决策范围**: Anaphase（main 装配 / events.rs）
- **关联**: ADR-0019（O-2 事件总线）、ADR-0016（白盒四层）、ADR-0022（"中途崩溃不丢经历"）

## 1. 背景与问题

O-2（ADR-0019）建立了六 stage 事件环（`EventRing`，append-only + 增量游标 + 确定性 JSONL），
但事件只活在进程内——**重启即失**。白盒四层（能力/状态/过程/事实）里，过程层
（stage events）是唯一无法跨会话追溯的一层：

- 能力（Manifest）：来自代码/配置，天然持久；
- 状态（Snapshot）：候选 G 共享槽，随时可重建；
- 事实（ledger + evidence）：JSONL 已落盘；
- **过程（events）：进程内存，重启清零。**

Helix 的哲学（ADR-0022 D2）是"中途崩溃不丢经历"——L3 摄取实时逐轮而非会话结束批量，
正是为了崩溃时不丢。事件轨迹若也实时逐轮落盘，则每次呼吸的过程都可跨重启回放。

## 2. 决策

### D1: 事件环纯数据结构，持久化归装配方（极致解耦）
`EventRing` 不碰文件。新增 `EventRing::from_jsonl(s, cap)` 纯函数（与既有
`to_jsonl` 配对）：
- 按 append 顺序重建事件，`seq` 接续最大已恢复序号——**增量游标跨重启连续**；
- 坏行**失败关闭**（fail-closed）：损坏的轨迹宁可拒绝恢复也不静默截断历史；
- 恢复后事件数超过 `cap`（来自 codex contract）则拒绝——配置不一致被显式暴露，
  而非悄悄裁剪。

文件 IO 全部在 `main.rs`（装配方），与 session_notes 同模式。

### D2: 默认开启，路径可配（跟随 session_notes_path 先例）
`AnaphaseConfig.events_log_path: Option<String>`，`None` = 默认 `"events.jsonl"`
（轨迹持久化默认开启，跨重启回放是默认行为；显式设路径可迁移）。

### D3: 实时逐轮追加（增量 flush）
主循环每轮 `run_cycle` 结束后，把 `after(flushed_seq)` 的新事件**追加**到日志文件
（`OpenOptions::append`，只写增量）：
- 崩溃最多丢**当前在飞的那一轮**，已落地的轨迹一条不丢；
- `flushed_seq` 是调用方维护的游标——与消费端 `?after=N` 同一语义，无新实体。

### D4: 恢复失败开启（fail-open on restore）
启动时日志缺失/损坏 → 打印 warning，**从空轨迹开始**，绝不让历史问题阻塞启动
（session_notes 先例：`读取失败（降级为无历史）`）。

## 3. 备选方案与拒绝理由

| 备选 | 拒绝理由 |
|---|---|
| EventRing 内部自管理文件 | 破坏"事件环是纯数据结构"的解耦；测试与回放都要碰磁盘 |
| 每次 flush 全量重写 | 浪费 IO；`after(seq)` 增量已存在，全量重写是重复实现 |
| 恢复失败即崩溃 | 违背按需驱动的容错哲学——历史是审计材料，不是启动依赖 |
| 单独 events 数据库/OTel 后端 | 零新依赖（O-2 已拒一次）；JSONL 追加已满足审计需求 |

## 4. 后果

**正面**：
- 过程白盒四层全部可跨重启追溯——Helix 的呼吸从"当前会话"变成"连续生命史"；
- 单测覆盖恢复语义（round-trip 字节一致 / seq 接续 / 坏行拒绝 / cap 强制）；
- 零新依赖、零新 crate、零新实体——`from_jsonl` 一个函数 + 一个 config 字段。

**代价**：
- 每轮一次小文件 append（毫秒级，可忽略；失败静默降级为仅内存轨迹）。

## 5. 验收

- `cargo test` 全绿：176 → **180**（+4 events 单元：restores_round_trip /
  keeps_cursor_continuous / refuses_corrupt_line / enforces_cap）
- 无回归：180 passed / 0 failed
- smoke：真实二进制启动正常（rails 0-token 短路 + 完整循环）
