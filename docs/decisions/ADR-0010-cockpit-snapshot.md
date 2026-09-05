# ADR-0010: Anaphase 驾驶舱快照投影（候选 G-T2）

- **状态**: Accepted
- **日期**: 2026-09-06
- **关联**: ADR-0006（会话即经历）、ADR-0007（D'-3 接线）、ADR-0023（CI-144 全局治理）
- **仓库**: anaphase-helix（服务端）/ Cellrix（消费端）

## 1. 背景

候选 G 目标：Cellrix 作为 **Anaphase 驾驶舱**（监控意识层状态，非驾驶 Helix-Mind 灵魂本体），
把认知循环的全链路白盒呈现给人类——模式、经历（episode）、ledger（append-only 验证账本）。
旧 `/v1/agent/snapshot` 返回静态演示 JSON（`token_consumed: 1234` 硬编码），
不反映任何真实状态。物理事实优先：端点必须输出真实投影。

## 2. 决策

### D1: 共享快照投影——HTTP 端点不触碰 agent 内部

`AgentLoop::capture()` 从 pub 字段投影 `AgentSnapshot`（mode / state / episode / ledger），
写入 `Arc<Mutex<Option<AgentSnapshot>>>` 共享槽；run_cycle 每轮后刷新；
HTTP 端点只读共享槽（booting = 未刷新）。**极致解耦**：HTTP 层不依赖 agent
生命周期与内部可变性；**按需驱动**：有 HTTP 消费方才建槽（`None = HTTP 禁用`）。

### D2: ledger 原样序列化——serde 契约即协议

`AgentSnapshot.ledger` 直接投影 `LedgerRecord`（serde tag = `record_type`），
不做二次扁平化。协议契约 = Anaphase 的 serde 形状；Cellrix 端用同形状
serde 反序列化（未知字段忽略，容忍内部演进）。零投影代码、零复制逻辑。

### D3: mode 序列化契约——snake_case

`Mode` 沿用既有 `#[serde(rename_all = "snake_case")]`（partner/drive/survive）；
Cellrix `InteractionMode` 消费端对齐同一契约（live 联调抓到不一致后修正）。

## 3. 备选与拒绝

| 备选 | 拒绝理由 |
|---|---|
| 端点直接持有 `&AgentLoop` | 破坏解耦；HTTP 层与 agent 生命周期强耦合 |
| ledger 二次扁平化 | 重复建模；serde tag 形状已稳定，Cellrix 直接消费 |
| 事件推送（EventSeq） | 如无必要勿增实体；轮询快照已覆盖白盒需求，M3 再议 |

## 4. 后果

**正面**：白盒可审查（真实 ledger）、零硬编码（`token_consumed: 1234` 消除）、
协议契约稳定（serde 形状）、live 联调验证（真实 Anaphase ↔ HttpAnaphaseClient）。

**代价**：快照是轮询语义（≤2s 延迟），非推送；HTTP 未启用时无驾驶舱数据。

## 5. 验收（已通过）

1. `cargo test` 全绿（126 passed + 6 live ignored）
2. live：真实 Anaphase（cap_http 50061）→ `curl /v1/agent/snapshot` 输出真实
   `{"mode":"partner","state":"Perception","episode":null,"ledger":[]}`
3. Cellrix `HttpAnaphaseClient::get_snapshot` 真实解析成功（transport/tests/anaphase_live.rs）
