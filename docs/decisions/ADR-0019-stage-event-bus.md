# ADR-0019: Stage 事件总线——过程白盒（append-only 事件环 + 增量拉取）

- **状态**: Accepted
- **日期**: 2026-09-06
- **决策范围**: Anaphase（pipeline / run_cycle / HTTP 投影）
- **关联**: ADR-0003（六 stage pipeline）、ADR-0005（确定性 pipeline 全 merge）、
  ADR-0016（编排哲学 D5：轨迹三层）、ADR-0010（按需感知）、ADR-0018（rails 心智外铁轨）

## 1. 背景与问题

候选 E 后 Anaphase 已有三层白盒：**能力**（Manifest）、**状态**（SemanticSnapshot）、
**事实**（ledger + evidence）。缺一层：**过程**——一次 run_cycle 里六 stage 如何推进、
每步发生了什么、卡在哪。用户要求"全链路可查可最终，全程白盒，可审查"。

已有的 ledger 是**事实**（verdict MET/UNMET + retry_due），evidence 是**支撑**（每次
调用的原始记录）——但两者都不回答"流程走到哪了、走了几步、每步状态"。
2026 年前沿（OTel GenAI 语义约定 / ActiveGraph 式"log is the agent"）验证了方向：
**事件溯源 = agent 架构主流**。但集中式可观测后端（Langfuse/ClickHouse）、OTel SDK
插桩、嵌套 span 树全部违背零依赖哲学——Anaphase 只需要一条**轻量的过程记录线**。

**核心立场**：事件 = 过程，ledger = 事实，evidence = 支撑——三条线职责分明，
不互相替代，全部 append-only 可回放。

## 2. 决策

### D1: append-only 事件环（不是事件驱动背板）
- `EventRing`：`Vec<StageEvent>` + 单调 `seq`，只追加不修改
- **记录非控制流**：事件是投影，不是消息总线——没有订阅/推送/回调，消费方拉取
- cap 满时**拒绝并计数 dropped**（不重写、不循环覆盖），增量游标语义不破坏

### D2: 事件模型——stage/phase/detail
- `StageEvent{seq, ts, trace_id, stage, phase, detail}`
- `stage` = 1..=6（ADR-0003 六 stage 映射），`phase` ∈ begin/end/verdict
- `trace_id` = 确定性派生 job_id（ADR-0003，无 UUID）——一次 run_cycle 的
  全部事件共享同一条 trace，可整链重放
- `ts` 来自注入的 Clock（与 ledger 同一时间源）——FakeClock 下字节级可回放
- 语义命名对齐 OTel GenAI 信号精神（event/exception/metric），但不 import 任何
  依赖，用自家 stage/phase 词汇（VISION 命名空间独立）

### D3: 插桩点——六 stage 边界（零侵入）
| stage | 插桩位置 | 事件 |
|---|---|---|
| 1 parse calls | run_cycle Reasoning（structured + LLM 两路径） | begin/end（detail=calls=N） |
| 2 assemble tt_job | run_cycle Reasoning 尾部 | begin/end |
| 3 gRPC 执行 | pipeline.execute_calls（begin + 每 call 一条 end） | begin + per-call end（tool ok/err） |
| 4 evidence 落盘 | pipeline.record_evidence | begin/end（records=N） |
| 5 criteria 校验 | run_cycle Reflection | begin/end（reports=N） |
| 6 ledger 写入 | run_cycle Reflection 尾部 | begin/end（verdict=MET/UNMET） |

中断即事实：gRPC 失败提前返回时 stage3 end 缺失 = 环节没走完的信号，不补发。

### D4: 增量拉取 + HTTP 投影
- `after(seq)`：返回 seq 之后全部事件（0 = 全量）——客户端持有游标轮询
- HTTP：`GET /v1/agent/events?after=N` → `{events, dropped, last_seq}`
- 与 snapshot 同模式（Arc<Mutex> 共享槽），pipeline 装配时 clone，无新依赖
- 无 pipeline 接线时诚实返回 `{"status":"no pipeline"}`（fail-open）

### D5: 容量来源（零硬编码）
- `events_cap` 进 `PipelineConfig`，由 fixture-codex.json contract 提供
  （DNA 原则 11 / ADR-0002）——默认 1024（保守内存预算，一次 cycle ≈ 13 事件）

## 3. 备选方案与拒绝理由

| 备选 | 拒绝理由 |
|---|---|
| 集中式可观测后端（Langfuse/ClickHouse） | 重依赖、外部服务；违背零依赖/极致解耦 |
| OTel SDK 插桩 | 依赖 + 命名冲突；对齐精神即可，不引库 |
| 嵌套 span 树 | 复杂度过剩；六 stage 是线性 DAG，一条 trace 足够 |
| 事件驱动背板/推送 | 记录非控制流——推送会把投影变成耦合点 |
| 环形覆盖（丢最旧） | 破坏 append-only 与增量游标语义 |

## 4. 后果

**正面**:
- 过程白盒闭环：能力（Manifest）+ 状态（Snapshot）+ 事实（ledger）+ 过程（events）
  四层齐全——驾驶舱可查"这一轮走了哪几个 stage、每步什么结果"
- 确定性：同输入同时钟 → 事件流字节级一致（可回放、可审查）
- 增量拉取极致节能：客户端只取新增（`?after=N`），不重复传输
- 零新依赖、零控制流耦合

**负面/代价**:
- 事件是进程内环，重启即失（不持久化）——本轮范围是"过程投影"，持久化事件
  日志属 O-3 候选（按需再做）
- 失败路径的 stage3 end 缺失需消费方理解"缺失即中断"语义

**风险与对策**:
- 事件环长期膨胀 → cap + dropped 计数（满则拒，诚实可见）
- Mutex 锁竞争 → append-only 写极少，读走 clone，无实际竞争

## 5. 实现要点与状态

| 项 | 位置 | 状态 |
|---|---|---|
| EventRing（emit/after/to_jsonl/cap/dropped） | `src/events.rs` | ✅ 完成（3 单元测试） |
| PipelineConfig.events_cap（codex contract） | `src/pipeline/mod.rs` + fixture-codex.json | ✅ 完成 |
| Pipeline.events（Arc<Mutex>）+ emit_event（时钟单一来源） | `src/pipeline/mod.rs` | ✅ 完成 |
| stage3/4 插桩（execute_calls / record_evidence） | `src/pipeline/mod.rs` | ✅ 完成 |
| stage1/2/5/6 插桩（Reasoning / Reflection） | `src/run_cycle.rs` | ✅ 完成 |
| HTTP `GET /v1/agent/events?after=N` | `src/main.rs` | ✅ 完成 |
| 测试（全链路六 stage / 增量游标 / 确定性重放 / verdict 一致） | `tests/stage_events.rs` | ✅ 4 用例 |

**验证**：176 tests 全绿（169 + 3 events 单元 + 4 stage_events 集成）；真实二进制
`GET /v1/agent/events?after=0` → `{"status":"no pipeline"}`（无 tentacle 诚实降级）；
集成测试经真实 MockTentacle gRPC 验证全链路事件（六 stage begin/end + verdict）。

## 6. 一句话总结

> ledger 说"事实如何"，evidence 说"凭什么"，events 说"过程如何走到这"——
> 三条 append-only 的线，让 Helix 的每一次呼吸都可回放、可审查、可白盒。
