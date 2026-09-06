# ADR-0017: Anaphase CI-144 传输层（驾驶舱闭环咽喉）

- **状态**: Accepted
- **日期**: 2026-09-06
- **决策范围**: Anaphase（stdio 传输协议）/ Cellrix（驾驶舱消费端）
- **关联**: ADR-0010（快照契约）、ADR-0014（驾驶舱 G2）、ADR-0016（编排哲学）、BIND-19（CI-144 传输家族）、Cellrix README §6.4（缺口记录）
- **取代**: Anaphase `--stdio` 的 JSON-lines 临时协议（退役）

## 1. 背景与问题

驾驶舱（Cellrix）TUI 链路实测：handshake MessagePack → 全屏渲染循环已通（mock-agent 验证）。
但接真实 Anaphase 时发现**接口契约断裂**：

- Cellrix `StdioTransport` 期望：CIB/1.0 文本握手 → 长度前缀（LE u32）→ MessagePack 帧（AgentEvent）
- Anaphase `--stdio` 现状：JSON-lines 临时协议（`connect`/`get_snapshot`/`act`/`exit` 文本命令）

README §6.4 已诚实标注该缺口。本 ADR 补齐它——Anaphase 实现 CI-144 传输层，
驾驶舱对真实 Anaphase 闭环。

## 2. 决策

### D1: 协议类型 vendored，不跨仓库依赖
Anaphase 是独立仓库（哲学：极致解耦），不依赖 `cellrix-protocol` crate。
所需协议类型（`AgentEvent`/`CapabilityManifest`/`Action`/`SecurityClass`/
`SemanticSnapshot`/`SemanticNode`/`NodeType`/`ActionRequest`/`ActionResponse`）
**vendored 到 `src/ci144/`**（与 `proto/tentacle.proto` vendored 先例一致），
serde 结构逐字段对齐 Cellrix 契约（字段名/tag/rename 完全一致），MessagePack 互操作。

### D2: 握手与帧协议（CIB/1.0 + LE 长度前缀）
- 握手：读 client 首行（`CIB/1.0 ...`）→ 回 `CIB/1.0 MSGPACK\n`（与 Cellrix `WireFormat` 一致）
- 帧：4 字节 LE u32 长度前缀 + MessagePack payload
- 心跳：BIND-19 约定 19s 素数间隔（防系统共振）

### D3: 事件流（首帧 Manifest，随后 Snapshot 推流）
- 首帧 `AgentEvent::Manifest(CapabilityManifest)`——anaphase 能力声明
  （agent_name="anaphase-helix"、version、actions 列表）
- 随后按节律推 `AgentEvent::Snapshot(SemanticSnapshot)`——驾驶舱渲染数据源
- 节律可配置（config），不硬编码（DNA 原则 11）

### D4: 快照映射——真实状态 → 语义树投影
`AgentSnapshot`（mode/state/episode/ledger/ecosystem，ADR-0010）映射为
`SemanticSnapshot`：
- `status` = mode（driving/partner/survival）
- `metrics` = 生态点亮摘要（六组件 status 计数）
- `semantic_tree` = 驾驶舱视图投影：认知状态（state_tree）+ 状态详情（text_panel）
  + 生态组件（metrics 节点）
- `epoch_time` = 时钟（注入，可测）

### D5: ActionRequest 处理——协议层业务无关，动作由注入回调承接
- 协议层（`server::run_loop`）不感知任何业务动作：`handle_action` 回调把
  `ActionRequest` 映射为 `ActionResponse`（极致解耦——协议只管帧与类型）
- launcher（main.rs）注入真实处理器：`status` 返回当前快照摘要；
  `send_message` 走真实 `run_cycle`（单周期循环，cap 尊重 config cycle_cap）；
  未知动作返回可恢复错误帧
- 动作路由对接 run_cycle 属候选 O 系列后续扩展，本 ADR 保证协议层闭环

### D6: JSON-lines 临时协议退役
无消费者（Cellrix 只认 MessagePack）。`--stdio` 标志保留，协议切换为 CI-144。

## 3. 后果

**正面**:
- 驾驶舱 TUI 对真实 Anaphase 闭环（语义树通道 + HTTP 快照通道双通）
- CI-144 生态治理落地到 Anaphase（BIND-19 传输家族成员）
- 类型 vendored 保持仓库自洽（极致解耦）

**负面/代价**:
- vendored 类型需随 Cellrix 协议演进手动同步（如 snapshot 加字段——serde 向后兼容默认容忍新增字段，风险低）
- 心跳/快照推流有常驻任务开销（按需节律，config 可调）

**风险与对策**:
- 协议字段漂移 → vendored 文件头注明来源 commit；serde 兼容（`#[serde(default)]`）兜底
- 快照推流过度（200ms 全帧）→ 默认节律放缓（1s），config 按需

## 4. 实现要点（2026-09-06 已落地）

| 项 | 内容 | 状态 |
|---|---|---|
| `src/ci144/mod.rs` | vendored 类型（AgentEvent/Manifest/Snapshot/Action/NodeType/ActionRequest/ActionResponse/ViewHash）+ 握手 + LE u32 帧编解码 + 投影（AgentSnapshot→SemanticSnapshot） | ✅ |
| `src/ci144/server.rs` | `run_loop`（select 单任务：推流 tick + ActionRequest 帧处理，biased 确定性）+ `run_stdio` | ✅ |
| `run_stdio_mode` 重写 | JSON-lines → CI-144：握手→Manifest→推流→Action 响应；`status`/`send_message`（真实 run_cycle）注入 | ✅ |
| 依赖 | `rmp-serde = "1.1"`（MessagePack 帧） | ✅ |
| 单元/集成测试 | `tests/ci144_transport.rs` 6 用例（握手 x2 / 帧往返 / 投影形状 / vendored serde 形状 / duplex 全协议会话） | ✅ 6 passed |
| live 实测 | `tests/ci144_live.rs`（#[ignore]）：真实二进制 `--stdio` 全链路（握手→Manifest→Snapshot→status→send_message→EOF 退出） | ✅ 1 passed |
| 快照推流节律 | `SNAPSHOT_PUSH_INTERVAL = 1s`（ADR 决策：config 可调、不硬编码；1s 为默认值） | ✅ |
| 文档链 | ADR-0017 + PLAN + GROWTH + README + ECOSYSTEM | ✅ 本记录内 |

**实现回写**：`handle_action` 回调签名（协议层业务无关）与 select 单任务事件循环
（无 spawn、无 Send 体操、biased 确定性）为落地时的两处优化，已并入 D5/D3 表述。
测试基线：154 → 160 全绿（+6 传输层）。

## 5. 一句话总结

> Cellrix 说的是 CI-144 方言（MessagePack + 握手），Anaphase 之前只会说 JSON 土话——
> 本 ADR 让 Anaphase 学会生态共同语，驾驶舱从此听得到它的心跳。
