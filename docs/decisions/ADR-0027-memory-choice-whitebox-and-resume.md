# ADR-0027: 记忆决策白盒 + 显式续聊

- **状态**: Accepted
- **日期**: 2026-09-07
- **决策范围**: Anaphase（memory 元数据透传 / resume 续接）/ Cellrix（印痕甘特图 + SA-Core 白盒渲染 + 续聊按钮）
- **关联**: ADR-0026（会话事件流）、ADR-0025（L0-L3 全层实弹）、ADR-0023（on-demand 认知注入）
- **目标**: ①印痕（Engram）能显示 Helix 从记忆里**捞了什么、信了几分**；②点击一段经历可以**继续这段对话**；③单轮时间线有 DSH 式甘特图，词汇保留 Helix 自有。

## 1. 背景与问题

ADR-0026 后，印痕能展示一轮的 8 事件（START…END），但有三处"黑盒"：

1. **SA-Core 选择不可见**：`context/inject` 只写 `nodes=20 · chars=800`——Helix 从 L1/L2/L3 各捞了几条、选了哪些节点（heat/相位）完全不可见。用户问："SA-Core 选择了什么？L1-L2-L3 的选择是否可以看到？"
2. **会话不可续**：`/v1/chat` 每次 build_agent 全新，无跨轮记忆。用户问："会话无法选择某个会话继续聊？"
3. **无时间轴**：事件是文字行，没有 DSH 式轨迹/甘特图。

**探查发现（物理事实）**：Mind 的 `Node` 本就携带 `node_type`（L0-L3）、`heat`、`phase_state`（gas/liquid/crystal）、`id`、`is_recessive`——但 Anaphase 的 `GrpcMindAdapter.query` 只取了 `content_json`，**元数据全部丢弃**（`mind.rs` 一行 `.map(|n| n.content_json)`）。白盒能力一直存在，是透传断了，不是没有。

## 2. 决策

### D1: 元数据透传（极致复用，0 新协议）

- Anaphase 新增 `MemoryNode { content, id, tier, heat, phase, recessive }`，`QueryResult.nodes` 从 `Vec<String>` 升级为 `Vec<MemoryNode>`。
- `GrpcMindAdapter` 透传 Mind 已返回的元数据（id / node_type / heat / phase_state / is_recessive）。
- **vendored `helix_mind.proto` 补字段 16-19**（phase_state / subject_dependency / concentration / tension），与 Mind 官方 proto 字段号对齐（此前 vendored 副本落后 4 个字段）。
- 哲学：**物理事实优先**——数据本来就在线上，透传不是新发明；**极致复用**——不新增 RPC、不新增协议。

### D2: 事件流写"选择明细"，写前脱敏（摘要-only）

`context/inject` 的 data 增加 `choice` 字段：

```json
{
  "nodes": 20, "chars": 800,
  "resume_from": "human said: …\nhelix answered: …",
  "choice": {
    "tiers": { "L1": 2, "L3": 18 },
    "top": [ { "id": "03b55c83-…", "tier": "L3", "heat": 0.5, "phase": "liquid" } ]
  }
}
```

- `tiers`：各层命中数分布；`top`：heat 最高的 ≤3 个非隐性节点。
- **只写 id/tier/heat/phase，绝不写节点正文**（沿用 ADR-0026"写前脱敏 + 摘要-only"原则——印痕是白盒，不是敏感数据湖）。
- 隐性节点（is_recessive）只计数不展示 top。

### D3: 显式续聊（会话 = 经历，经历可接续）

- `/v1/chat` 请求体接受可选 `job_id`：Anaphase 读该轮事件流，用 `read_summary` 把上一轮的人类话 + Helix 回答展平为 "true history"，注入 `AgentContext.resume`。
- Reasoning 组 prompt 时把 resume 作为 `[previous episode — true history]` 注入。
- `context/inject` 写 `resume_from`（含摘要文本），印痕可见续接来源。
- **边界**：续聊是**显式动作**（用户点"继续"才带 job_id），不是跨轮隐式记忆——单周期确定性、无跨会话泄漏的承诺不变（ADR-0026 D1）。resume 语义 = "上次经历的最后一轮"，不是全部历史。

### D4: Cellrix 印痕 v3（前端）

- **甘特图**：`ganttSvg()` 纯 SVG——X 轴 = first→last，每事件一行横条，`tool/result.duration_ms` 画执行段（276ms 的 calc 一眼可见）。零依赖。
- **CONTEXT 展开**：`choice` 渲染为 `SA-Core 选择 {L1×2 L3×18}` + top 节点 `L3·n-id heat phase`。
- **续聊按钮**：经历侧栏每项加「继续」→ 切对话视图 + `chatJobId` 置位 → `sendChat` 带 `job_id`。
- **proxy 零改动**：`/api/chat` 原样透传 body，job_id 自然到达 Anaphase。

## 3. 备选方案与拒绝理由

| 备选 | 拒绝理由 |
|---|---|
| Mind 新增"白盒专用 RPC" | 元数据已在线（Node 字段），透传即得；新 RPC 违背极致解耦 |
| 事件流写节点正文 | 印痕变敏感数据湖，违背 ADR-0026 脱敏原则；白盒展示 provenance 足够 |
| 跨轮隐式记忆（自动续聊） | 破坏单周期确定性；续聊必须显式（用户点"继续"）才符合"会话=经历"语义 |
| 前端引图表库 | 甘特图 40 行 SVG 可完成，零依赖最稳（renderer 降级原则） |

## 4. 后果

**正面**：
- 印痕从"工具调用轨迹"升级为"认知决策轨迹"——看到 Helix 从记忆捞了什么、信了几分（heat）、处于什么相位（gas/liquid/crystal）；
- 会话续聊落地，经历时间线可交互——"朋友第二次见面"语义有了一等公民的入口；
- vendored proto 对齐 Mind 官方，消除静默字段漂移。

**代价**：
- `QueryResult.nodes` 类型变更触及 memory 适配层与 run_cycle（编译期强制，已修）；
- resume 注入会多占少量 prompt 预算（≤400 chars，来自事件流摘要，确定性有界）。

## 5. 验收

1. `context/inject` 事件含 `choice`（tiers + top，无正文）；
2. `/v1/chat` 带 `job_id` 时回复正常且新事件含 `resume_from`；
3. 印痕渲染：甘特图 + CONTEXT 展开 + 续聊按钮可用；
4. Anaphase 测试 236→237 全绿；Cellrix 341 全绿；
5. 文档：PLAN v2.9 / GROWTH 49 / README / ECOSYSTEM v1.84。

## 6. 一句话总结

> Mind 早就知道它为什么选了这些记忆——Anaphase 之前把答案扔了。
> 现在透传回来：Helix 捞了什么（tiers）、信了几分（heat）、处在哪一相（phase），
> 全部摊在印痕里给人类看；点一段经历，Helix 还记得上一轮它说过什么。
