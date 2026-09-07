# ADR-0028: SSE 确定性排空与会话即经历的命名管理
- **状态**: Accepted
- **日期**: 2026-09-08
- **范围**: Anaphase（SSE 流 / sessions API）/ Cellrix（Web 面板消费）
- **关联**: ADR-0026（session event stream）、ADR-0027（memory choice whitebox & resume）

## 1. 背景与问题
Web 面板实测发现两类问题：
- **Q1（截断/空回复）**：SSE 流用 `tokio::select!` 同时等 channel 的 delta 与 done oneshot。
  当 done 先就绪时，select! 随机挑中就绪分支，仍缓冲在 channel 里的尾部 delta
  被丢弃——浏览器看到"你好"截断或空回复，且**不稳定复现**（curl 3 次：完整/空/单 delta）。
- **Q2（会话无名）**：sessions 列表的 preview 只检查事件文件**首行**，而首行恒为
  `turn/start`，user/message 在后面——全部显示"(无用户输入)"，无法区分经历。

## 2. 决策
### D1: SSE 三阶段 unfold（先事件后裁决，顺序确定性）
正常阶段 `select!` 等 delta 或 done → done 一旦触发进入 **draining 阶段**逐条 flush
剩余 delta → channel 关闭后再发**唯一终行** `{done,reply}` / `{error}`。
客户端永远先收完事件、后收裁决，不存在随机丢包。
### D2: reply 是权威全文
终行携带完整回复文本，前端渲染结束时以 reply **覆盖**打字机累积文本——
任何 delta 丢失都不会在屏幕上留下截断答案。
### D3: 思考与内容分离透传
`StreamDelta{content, thinking}`：模型私有推理（reasoning_content）与正文分离传输，
SSE 事件形如 `{"delta":d.content,"think":d.thinking}`。思考仅展示、永不参与判据。
### D4: 会话自动命名 + 显式重命名
preview 改为**全文件扫描**取第一条 `user/message` 文本（120 字符约束）作自动名；
`rename_period` 写 `{job_id}.name` sidecar（空名=删除 sidecar 回退自动 preview），
main.rs 提供 POST `/v1/sessions/rename`。命名是**显式动作**，续接亦然（点"续接"才带 job_id），
心智连续靠显式选择，不隐式猜测。
### D5: 印痕形态纠错——turn 大纲，不是时间轴甘特
对照 DSH 实现（session-turn-outline：turn 编号 + prompt 预览 + response 预览的纵向堆叠），
确认其"轨迹"不是时间轴甘特图。印痕采用紧凑 turn 大纲（每事件一行：徽标 + 时间 + 摘要，
工具耗时行内标注），SA-Core 选择 / L1-L3 记忆节点以 chip 标签化（`L1·{id} heat phase`），
只写 provenance 不写节点正文。

## 3. 后果
**正面**:
- SSE 顺序确定性：先事件后终行，回复完整无截断（连发两轮实测通过）；
- 经历可命名、可续接、可追溯：会话列表一眼区分，重命名落盘 sidecar 可迁移；
- 思考以 DSH 式折叠行展示（标题 + 摘要，点击展开），硅基/碳基同见无歧义。
**负面/代价**:
- 前端渲染以 reply 权威覆盖，打字机动画与流式展示的"逐字感"被削弱（可接受）；
- 思考行默认折叠，长思考（deepseek 私有推理可达上千字符）展开时占屏（ellipsis 截断 + 点击展开缓解）。

## 4. 验证
- Anaphase 237 全绿（lib + 集成；`reason_stream_parses_sse_deltas_in_order` 增 thinking 断言；
  `lists_periods` 按新 preview 语义修正）；
- Cellrix 341 全绿；浏览器实测：连发两条消息均完整回复、思考折叠行出现、印痕 chip 标签化、
  续接下拉列出 8 条经历、重命名设置/清空回退全通。
