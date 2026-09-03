# ADR-0003：M1 里程碑执行架构（确定性流水线 + 引擎归属 + 重入语义）

## 状态
**Active**（2026-09-03，经四轮严肃审查收敛定稿）

## 问题
1. v2 任务书假设 `src/reasoning/adapter.rs` 路径，实测不存在（真实为 `src/adapters/tentacle.rs` / `src/adapters/http_reasoning.rs`）
2. Anaphase 自带 proto 与 Tentacle 真实 proto 完全不匹配（Execute/Perceive vs ExecuteTool/ListManifests/GetManifest），GrpcTentacleAdapter 从未真正连通
3. run_cycle 状态机（LLM 驱动 + 字符串匹配 + 硬编码 echo）与 tt_job 结构化 calls[] 基因不兼容
4. "缺口非空之后怎么办"未定义——循环语义与重入机制缺位

## 决策

### 决策 9：引擎归属——旁路 run_cycle，新建确定性流水线
**T1 探查结论（代码证据，非猜测）**：run_cycle 与 tt_job calls[] 四点不兼容——
1. Reasoning 输出：`reason.reason()` 返回自然语言 String + `contains("tool_call")` 字符串匹配
2. Execution 输入：`action_str = suggested_actions.join(", ")`（来自 memory.query），`command = "echo"` 硬编码占位
3. ToolAdapter 签名：`execute(command: &str, args: &[String])` vs calls 的 `{tool, args: object}`
4. 循环语义：`for _ in 0..7` LLM 驱动的认知状态机 vs 单趟确定性流水线

**决策**：M1 旁路 run_cycle，新建 `src/pipeline/mod.rs` 确定性流水线。run_cycle 是"认知态"载体，pipeline 是"执行态"载体（DNA 原则 3 工作态归 Anaphase）。**接回 run_cycle 为 M1.5 必做项**（防旁路成永久事实）。

**M1.5 接回映射表**（六 stage ↔ 六状态，M1.5 用代码证据精调落点）：

| pipeline stage（M1） | run_cycle 状态（M1.5 落点） |
|---|---|
| stage1 parse calls | Reasoning（输出协议结构化，替换 contains 匹配） |
| stage2 组装 tt_job | Reasoning 尾部 |
| stage3 gRPC 执行 | Execution（替换 echo） |
| stage4 evidence 落盘 | Execution 尾部 |
| stage5 criteria 校验 | Reflection（判据消费点） |
| stage6 ledger 写入 | Reflection 尾部 |

### 决策 10：UNMET + retry_due + 谱系计数——循环第一态
- **方案 A 采纳（单趟闭环）**：缺口非空 → 写 `status: UNMET` 结论并结束；M1 不消费重试队列
- **UNMET 记录必填 `retry_due`**（= `now + retry_policy.base_delay_secs`，来自 fixture-codex.json，不硬编码）
- **`parent_id` 谱系字段**：append-only 不许改写，重试次数靠记录链长（M1 只写不读，M1.5 靠链长计 attempts）
- **循环 = 队列消费**：M1 落账（写 retry_due），M1.5 消费（scan_due → 重入 pipeline）。循环从 M1.5 的"新功能"变成 M1 记录语义的自然延伸
- **两循环物种声明**：run_cycle 的 7 轮循环 = 会话内认知重试（LLM 驱动）；retry_due 队列 = 跨会话证据缺口重入（确定性驱动）。M1.5 接线时两者如何协作是那时的设计题，不得混淆

### 决策 11：pipeline 结构契约——六 stage 独立可调用，禁巨型 run()
- 六 stage 分离：纯函数（stage1/2/5）+ 带可注入 IO 边界的函数（stage3/4/6）
- **禁巨型 `run()`**：每个 stage 独立可测
- **pipeline 直接持有 GrpcTentacleAdapter**（不改旧 ToolAdapter trait——它被 run_cycle 消费，改签名有回归风险；不新造 ToolExecutor 抽象——M1 无第二实现者，勿增实体。ToolExecutor 推迟到 M1.5 出现第二实现者时，届时 ADR 补记）
- **测试缝隙 = T0 的 MockTentacle gRPC server**（复刻 mind_integration 的 TcpListener 模式）

### 执行中确认的工程决策（T0-T8 落地）
| # | 决策 |
|---|---|
| 1 | proto 对齐：vendor Tentacle 权威 proto（tentacle.v1 完整拷贝，含溯源注释），Anaphase 侧适配；build.rs 无需改动 |
| 2 | M1 Tentacle 侧 = mock server（复刻 mind_integration 模式） |
| 3 | M1 e2e = 双 mock（LLM + Tentacle），fixture 数据内联；manifest+js 插件延 M1.5 |
| 4 | ReasoningAdapter 复用 HttpReasoningAdapter（真实 OpenAI 兼容客户端），FlowModus 延 M2 |
| 5 | LLM 输出边界 = calls[]，job_id/created_at 信封由 Anaphase 组装；解析失败上抛不入账 |
| 6 | expect→criteria 映射 + 规则/数据分离（参数来自 fixture-codex.json，数据来自工具） |
| 7 | M1.5 范围 = Tentacle `--transport grpc` + fixture 插件 + 真实连通测试；identity_labels/seen_entropy_bloom 语义 M1.5 定义 |
| 8 | Tuck 深度集成延 M1.5（M1 mock Risk-Level=LOW） |
| 12 | 确定性钳制：trace_id/evidence_id = `{job_id}#{index}` 派生；evidence/ledger 禁记 endpoint/端口；JSON 构造禁 HashMap（struct 或 BTreeMap） |
| 13 | parse_llm_calls() 纯函数归属 contract 模块（三例单测：合法/非法 JSON/枚举越界） |
| 14 | EvidenceRecord 携带 expect 字段（自包含，M1.5 无需从 calls 重推导即可复核） |
| 15 | fixture-data-shapes.md 为 M1 内联 fixture 与 M1.5 插件输出的共享形状契约 |

## 关联
- ADR-0002（DNA 原则 11 零硬编码）
- Anaphase-Helix DNA.md 原则 3/7/9/11
- M1 任务书 v2.3（本 ADR 为其落档）
