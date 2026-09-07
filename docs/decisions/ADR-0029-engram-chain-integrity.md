# ADR-0029: 印痕链条完整性——物理事实 → 确定性判据 → 可审计记账

- **状态**: Accepted
- **日期**: 2026-09-08
- **决策范围**: Anaphase（run_cycle / pipeline / session_events）
- **关联**: ADR-0003（六 stage 流水线）、ADR-0019（事件环）、ADR-0026（事件流）、ADR-0027（白盒续接）、ADR-0028（SSE 确定性）

## 1. 背景与问题

印痕（Engram）已能渲染一轮经历的时间线（USER / CONTEXT / ATTEMPT / TOOL / VERDICT），
但链条存在三处"断点"，导致**无法从账本复演一轮经历**：

1. **产出物丢失**：`tool/result` 只记 `ok + duration_ms`，不记工具返回的**物理结果**（如计算器输出
   `40353607`）。没有产出物，复演无从对合——用户两次提问都看不到 calc 的结果。
2. **判决裸奔**：`verdict/status` 只有一个 `Met/Unmet` 标签，没有"判了什么、谁判的、依据什么"。
   没有 reason 的判据只是标签。
3. **成功自报**：`turn/end.success` 由状态机迁移条件派生，模型自报成功——与判据脱钩。
   工具轮判据失败（Unmet）时 `success` 仍可能为 true，账本自相矛盾。

用户明确要求："不能用补丁式思维，把哲学捋顺，把完整链条关系搞清楚，实现哲学自洽、逻辑闭环。"

## 2. 决策

### D1: 链条范式——物理事实 → 判据 → 记账

每一轮经历统一为三段式，任何环节不得缺位：

```
物理事实 (outcome + outcome_sha)   ← 工具执行的产出，字节级可对合
    ↓
确定性判据 (CHECK: judge/gate/expect/actual/reason)  ← 0-token 纯函数，失败即 fail-closed
    ↓
记账 (VERDICT 派生 + END.success ≡ VERDICT)         ← 判决由判据派生，禁止自报
```

### D2: tool/result 携带产出物

`tool/result` 事件补 `outcome`（执行返回全文）与 `outcome_sha`（SHA-256 前 4 字节 hex，
8 字符指纹，`trace::short_sha`）。同一产出物有稳定短引用，印痕行无需二次存正文。

### D3: 新增 check/status 事件

criteria 的每个确定性报告写一条 `check/status`：

| 字段 | 含义 |
|---|---|
| `check_id` | `{job_id}#c{index}`（确定性派生，无 UUID） |
| `check` | 判据名（threshold / sample_size / …） |
| `passed` | 是否通过 |
| `judge` | 谁判的：`rule`（确定性判据）或模型后端 |
| `gate` | 严格度：`hard`（fail-closed）/ `soft`（只记账）。0-token 判据默认 hard |
| `expect` | 契约名（numbers/rate/text/ok） |
| `evidence_id` | 被读的证据行 `{job_id}#{index}` |
| `reason` / `actual` | 为什么（判据 detail） |

`CheckReport` 结构同步扩展（judge/gate/expect/evidence_id），纯函数构造处给协议默认值，
调用点（run_for_expect / check_results）填充真值——**判决自带身份证，终结"谁来审计审计者"套娃**。

### D4: END.success ≡ (VERDICT ≠ Unmet)

`turn/end.success` 由本轮 `last_verdict` 派生：

- 有工具轮：`success = (verdict == Met)`——判据失败即失败，状态机迁移无关。
- 无工具轮（纯对话）：`success = (迁移条件 == Success)`——无判据可依，回归状态机语义。

`turn/end` 事件补 `verdict` 字段，派生链对客户端可见。

### D5: 思考进印痕——assistant/think 事件

模型的私有推理（thinking）流式送前端的同时，经 sink 聚合为 `assistant/think` 事件落盘。
- 脱敏写入（复用 trace 红action），**显示专用——判据永不消费思考**（判据只看物理产出）。
- 前端用统一 fold 组件呈现：点击展开、再点关闭、悬浮预览（见 D7）。

### D6: 结晶闭环——Unmet 是矿，规则是产品

新增 `crystallize(dir, limit)`：扫描最近经历，把每个 Unmet 轮的
`tool/result + check/status + verdict` 折叠成一条规则建议，写入 `{dir}/crystallized/rule-{job_id}.json`。
- **0 token**：纯确定性扫 JSONL，无 LLM 参与。
- **机器只建议，人不审核不上线**：绝不自动注入 Tuck 规则（fail-human，never fail-machine）。
- HTTP 端点 `POST /v1/crystallize`（limit 默认 50，上限 500）。

### D7: 统一 fold 能力（点击/关闭/悬浮预览）

前端一个 `foldHtml(label, preview, full)` 原语服务所有可折叠行：
think 全文、check 依据、tool 产出物、长 reason——**一个能力，处处复用，拒绝逐点打补丁**。
点击 toggle `.open`，`title` 承载悬浮预览。

### D8: 断连根因修复（proxy 传输层）

`Cellrix/web/src/lib.rs`：
- 读超时 30s → 180s（deepseek 家族长思考是物理事实，30s 边界制造假断连）。
- `read_line_from` EOF 不再报 `origin ended mid-line`：EOF 时冲刷缓冲剩余（可能是半行），
  干净结束——半行也转发，绝不丢数据。
- chunked 中途 EOF：冲刷剩余 payload 后正常返回。
- 前端 `sendChat`：收到 `error` 且已流式渲染过内容 → 静默 finish（保留已答内容）；
  仅"无任何内容即断"才弹 toast——传输故障是驾驶舱的事，不是 Helix 在说话。

## 3. 判据分层（本轮落地的边界）

| 层 | judge | gate | 成本 | 本轮 |
|---|---|---|---|---|
| criteria 确定性判据 | `rule` | `hard` | 0 token | ✅ 已落地（CHECK 事件） |
| Tuck 内容治理规则 | `tuck` | `hard` | 0 token | ⏳ 请求侧（既有），CHECK 记账对接待后续 |
| 模型软判据（编排判断） | 模型后端 | `soft` | 按需 | ⏳ O-6 判断点按 ROI 决定，本轮不接 |

## 4. 备选方案与拒绝理由

| 备选 | 拒绝理由 |
|---|---|
| RESULT 里塞完整 trace 引用 | outcome 就地在事件里，引用反而要二次读取；sha 是给对合用的短引用 |
| success 由模型/状态机自报 | 自报无法审计；铁律 `success ≡ verdict` 让账本自洽 |
| 思考只在 trace 不落事件 | 用户明确要求思考进印痕可点展；trace 与事件同源但事件是白盒主视图 |
| 结晶自动注入 Tuck | 机器自决策改变生产行为，违背"人审核"安全边界 |

## 5. 后果

**正面**：
- 账本自洽：产出物 → 判据 → 判决 → 成功，一条链全部可追溯、可复演、字节可对合。
- 白盒深化：思考、CHECK 依据、工具产出全部可点展可见，硅基碳基无歧义。
- 审计资产化：连续 Unmet 自动析出规则建议，0-token 拦在前面（生存伦理）。

**负面/代价**：
- 事件流新增 2 个事件类型（think/check）+ 3 个字段（outcome/outcome_sha/verdict），
  历史印痕文件不兼容（旧轮无这些字段——前端已做缺失容错）。
- CheckReport 结构变化影响 ledger 序列化，确定性测试已同步。

## 6. 一句话总结

> 印痕不是标签流水账——它是"物理事实 → 确定性判据 → 可审计记账"的闭环，
> 思考可点展，产出可对合，判决有身份证，失败是矿。
