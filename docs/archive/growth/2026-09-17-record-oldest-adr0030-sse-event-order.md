# Anaphase 生长记录归档

> **归档于 2026-09-17**：`docs/GROWTH.md` 保留最近 3 条，补记"工具链闭环 +
> 周期身份不唯一"时归档日期最早一条。**历史永不删除 —— 以下为原文完整保留。**

---

## [2026-09-08] 完成：SSE 事件序运行时焊死（ADR-0030）

### 变更性质
- 终态通道 oneshot → mpsc：oneshot 在 complete 后重复 poll 触发 tokio panic（`called after complete`）→ unfold 流在 done 行发出前中断 → 浏览器只收到 attempt 裸 JSON delta（用户视角"Helix 回复是计划文本不是答案"）。mpsc `recv()` 可安全重复 poll，done 行确定性到达。
- 事件序契约不变（ADR-0028：delta 先、done 后、drain flush 尾部）；只换通道原语，不换语义。
- sender 未发送即 drop（周期崩溃）→ 流静默结束，客户端保留已流式内容，不伪造 done。

### 验收
- 浏览器实测：8^3 → Chat 显示 `calc: {"ok":true,"result":"512"}`（此前裸 JSON）；
- 连续两次 curl SSE 均收到 `"done":true`（此前 done 行丢失）；
- `grep -c panicked` = 0（此前每请求后 panic）；`cargo test` 全绿 0 failed。