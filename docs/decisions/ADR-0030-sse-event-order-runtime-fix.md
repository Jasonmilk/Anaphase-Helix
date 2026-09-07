# ADR-0030: SSE 事件序契约的运行时焊死（oneshot → mpsc）

- **状态**: Accepted
- **日期**: 2026-09-08
- **决策范围**: Anaphase（/v1/chat SSE 流）/ Cellrix（WebUI 消费端）
- **关联**: ADR-0028（SSE 确定性事件序）、ADR-0029（印痕链完整性）、ADR-0016（单周期原语）
- **取代**: 无（ADR-0028 的补强）

## 1. 背景与问题

ADR-0028 确立了契约：**done 行必须晚于所有 delta 行到达**，客户端永远先见事件、后见判决。实现采用
`tokio::sync::oneshot` 传递最终 reply，`select!` 在 delta 与 done 之间竞态，done 收讫后进入 drain 阶段。

真实联调（浏览器 WebUI）暴露运行时缺陷：

1. **done 行丢失**：同一请求反复出现
   `thread 'tokio-rt-worker' panicked at .../tokio/.../oneshot.rs:1289:13: called after complete`。
   oneshot 的 Receiver 在已 complete 之后被再次 poll（select! 先选了 delta 分支、done future 被 drop，
   下一轮重新 poll 已完成的 oneshot）→ panic → unfold 流中断 → 客户端只收到 attempt 的裸 JSON delta，
   永远等不到最终 reply 覆盖。用户视角：**Helix 回复显示为 `{"calls":[...]}` 计划文本，不是答案**。
2. **Anaphase 侧替换已生效**（`reply replaced: calc: {...}`），缺陷在传输层事件序，不在判据/替换逻辑。

## 2. 决策

### D1: 终态通道从 oneshot 改为 mpsc

`done_tx/done_rx` 由 `tokio::sync::oneshot` 改为 `mpsc::unbounded_channel<Result<String, String>>`：

- mpsc 的 `recv().await` **可安全重复 poll**，sender 未发送时返回 `None`，无 panic；
- oneshot 在 complete 后再次 poll 是未定义行为（panic），这正是竞态的爆点；
- 语义不变：done 行仍是唯一终态行，`Ok(reply)` → done 行，`Err(e)` → error 行。

### D2: 事件序契约保持 ADR-0028 不变

delta 先、done 后、drain 阶段 flush 尾部 delta——本 ADR 只更换通道原语，不改变事件序语义。
修复的是"契约在运行时被竞态破坏"，不是契约本身。

### D3: sender 未发送即 drop（周期崩溃）→ 流静默结束

客户端保留已流式内容并正常 finish，不伪造 done、不二次报错。传输故障归驾驶舱（Cellrix 已有
`j.error` 兜底），Helix 的消息空间不被运维噪音污染。

## 3. 备选方案与拒绝理由

| 备选 | 拒绝理由 |
|---|---|
| select! 加 `biased;` 固定顺序 | 只调优先级不除根：done future 仍可能被多次创建/poll，panic 源仍在 |
| 保留 oneshot + `futures::future::Fuse` | 引入额外依赖，mpsc 是 tokio 既有原语，极致复用更优 |
| 客户端忽略裸 JSON delta、只信 done | 掩盖传输缺陷，丢失"流式可见"这一白盒价值；应修源头 |

## 4. 后果

**正面**：
- done 行确定性到达，浏览器 Chat 显示替换后的最终 reply（实测 `calc: {"ok":true,"result":"512"}`）；
- 连续请求无 panic、无丢失（实测两次 SSE 均收到 done）；
- 事件序契约（ADR-0028）第一次在真实浏览器链路被验证为"物理事实"。

**代价**：
- 一个通道原语变更，无 API/协议影响，向后兼容（done/error/delta 事件形状不变）。

## 5. 与 CI-144 的关系（本次用户提问的裁决）

用户问"这个问题能否复用 CI-144 的特征"。裁决：

- **哲学同源，实现异层**。本缺陷是"确定性事件序"被运行时竞态破坏——与 CI-144 的确定性优先
  哲学同源；但爆点在 tokio 异步运行时，不在协议语义。协议层（CI-144/SSE）无法约束运行时
  内部竞态，修实现是根治。
- **BIND-19 帧定界是将来可选的硬通道**。SSE 以 `data:\n\n` 文本行定界，切行/丢尾是弱边界；
  BIND-19 以 8 字节帧头 + 长度定界，帧完整性由协议保证。若将来 WebUI 通道升级为 BIND-19 帧
  传输（人机交互表面本就在 CI-144 治理范围），此类"尾行丢失"在协议层即被消除。
- **按需加载**：当前 HTTP/SSE 对 Web 界面是合适的既有通道，事件序契约已由实现焊死，
  不因"更硬"而提前引入帧协议。列入候选，不阻塞现网。

## 6. 验证

- `cargo test` 全绿（133 lib + 集成，0 failed）；
- 浏览器实测：8^3 → Chat 显示 `calc: {"ok":true,"result":"512"}`（此前显示裸 JSON）；
- 连续两次 curl SSE（Accept: text/event-stream）：均收到 `"done":true`（此前 done 行丢失）；
- 日志 `grep -c panicked` = 0（此前每次请求后 panic）。

## 7. 一句话总结

> 契约是 ADR-0028 立的，本次把契约焊死在运行时上——oneshot 的竞态爆点换成 mpsc 的确定性收尾，
> done 行从此必然到达；协议层将来可以更硬（BIND-19 帧），但今天这一层已经物理成立。
