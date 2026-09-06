# Anaphase-Helix PLAN — 当前阶段导航

> **DNA 方法论 v1.0** ｜ PLAN.md 是导航牌，不是历史档案（≤150 行）。完成记录进 GROWTH.md。

## 当前阶段：O-1 落地 + Rails + O-2 事件总线 + P10a 认知工艺触发 + P10d 预约制闹钟（ADR-0032）——下一步按生态节奏

**P10d 预约制闹钟（ADR-0032，2026-09-06 完成）**：Mind 侧 ana_wakeup/ana_wakeup_ack RPC（Anaphase 客户端
同步）+ MemoryAdapter wakeup/wakeup_ack/consolidate 默认方法（Noop 静默降级）+ GrpcMindAdapter 实现
（jitter 窗口来自 RunCycleConfig 默认 60）+ run_cycle 入口 check_wakeup（每交互看表一次：白名单 action →
consolidate 链 → ack done；未知 action → ack done 释放；失败 → ack done 记录；不可用 → 跳过）。202 tests 全绿
（+4：触发执行/白名单外/无预约/降级）。

**P10a 认知工艺触发（ADR-0031，2026-09-06 完成）**：Mind 侧 helix_craft RPC（Anaphase 客户端同步）——
MemoryAdapter.craft() 默认方法（Noop 零改动静默降级）+ GrpcMindAdapter 调 helix_craft（工序集/约束来自
MindConfig 协议默认，循环只问"要不要想"、adapter 决定"怎么想"）+ run_cycle MemoryRetrieval 对非结构化输入
按需触发（确定性 job_id）→ Reasoning 以 [think-first] 折入 synthesis（0 token 思考先于 LLM tokens）。
198 tests 全绿（+3：craft 触发/结构化跳过/Noop 降级）+ MockMind craft stub。

**O-2 stage 事件总线（ADR-0019，2026-09-06 完成）**：过程白盒第四层——append-only
事件环（事件=过程，ledger=事实，evidence=支撑）+ 六 stage 边界插桩（begin/end/verdict）
+ trace_id=派生 job_id（一次 cycle 一条 trace）+ `GET /v1/agent/events?after=N` 增量
拉取（记录非控制流）+ events_cap 来自 codex contract（零硬编码）。176 tests 全绿
（+7：3 events 单元 + 4 stage_events 集成）+ 真实二进制端点验证。

**Rails 心智外铁轨（ADR-0018，2026-09-06 完成）**：人类知识 DAG（宪法/律法/SOP）只读引用
铁轨——`knowledge_base/rails/<kb>/` markdown 原汁原味 + 确定性索引（标题树=节点、
链接=边、SHA-256 版本冻结、断链即错）+ 铁轨导航（词项 + CJK bigram，无嵌入）+ 引用契约
（原文引用 + visited check 验证器 + 引不到答 NO_RAIL_CONTENT）。接线 MemoryRetrieval
（注入 + rail_mode），RailScope 类型级只读（无写变体）。169 tests 全绿（+9）+ 真实二进制
挂载验证。与熟练模式（Helix-Mind 心智内软铁轨）同源——心智外硬铁轨，错不起就硬。

**CI-144 传输层（ADR-0017，2026-09-06 完成）**：`--stdio` 从 JSON-lines 临时协议切换为
CIB/1.0 MessagePack + 握手 + LE u32 帧（vendored 类型在 `src/ci144/`，serde 逐字段对齐
Cellrix）。事件流：Manifest 首帧 → 1s 节律 Snapshot 推流 → ActionRequest 响应
（`status`/`send_message` 注入回调，协议层业务无关）。160 tests 全绿（+6）+ live 实测
（真实二进制全链路）。驾驶舱闭环咽喉打通——Cellrix TUI 对真实 Anaphase 可闭环。

**编排哲学（ADR-0016，2026-09-06，修订版）**：确定性优先分诊（0 tokens > 少/小 LLM > 多/大 LLM）+ 认知工艺触发点（四拍/五工序归 Mind，Anaphase 只触发）+ 按需感知 = 设置 budget_tier（ADR-0010）+ 三层递进边界（执行层分诊→System 0 门控→五工序）+ 依赖边界（并行/窗口感知→FlowModus；前缀稳定→Callosum）+ 轨迹三层。详见 `docs/decisions/ADR-0016-orchestration-philosophy.md`。

### 候选 O 系列（编排哲学落地，按依赖序）

| # | 任务 | 验收 | 依赖 |
|---|---|---|---|
| O-1 | 想 stage 接 Mind 契约：感知（口袋/资源/生态点亮）→ 设置 budget_tier（ADR-0010）→ 触发 helixQuery → 消费 effective_mode/suggested_actions 编排执行 | ✅ 已完成：结构化输入零 LLM（`!tool` 分诊）+ 生态点亮探测 + 感知点（ecc1924，152 tests 全绿） | ADR-0016 D1/D2.5/D3 |
| O-2 | stage 事件总线：六 stage 边界发确定性事件（stage_begin/stage_end/verdict） | ✅ 已完成（ADR-0019：事件环 + ?after=seq 增量拉取 + /v1/agent/events 端点；176 tests 全绿，9c0e60c 起） | ADR-0016 D5 |
| O-3 | stage 事件轨迹持久化：跨重启可回放的过程白盒（`EventRing::from_jsonl` + 实时逐轮追加）+ 模式无关黑匣子（ADR-0021：事件环提升 AgentLoop 级，驾驶模式无 pipeline 也记录 cycle 轨迹） | ✅ 已完成（ADR-0020/0021：默认 events.jsonl，seq 接续 + 坏行失败关闭 + cap 强制；181 tests 全绿） | O-2 |
| O-4 | 认知工艺触发接线验证（伙伴模式）：想 stage 触发 Mind → Mind 走四拍/五工序 → Anaphase 按建议编排 | ✅ 已完成（ADR-0022：复用既有 mock Mind 验证触发链；T3 驾驶模式不触发回归守卫；MindConfig 零硬编码收口 mind.rs 12 处字面量；183 tests 全绿） | O-1/O-2 |
| O-5 | 按需加载落点：请求只带本轮所需（原 O-3 定义） | ✅ 已完成（ADR-0023：记忆折叠注入 Reasoning prompt——修复检索断裂；`memory_inject_chars` 预算封顶，25 轮近零增长验证；`--input`/`smoke_input` 演示输入来源化；窗口 L0 诚实标注待对话入口；189 tests 全绿） | 候选 G 快照 |
| O-6 | 判断点后端可配化（JP-1 复杂度评估）：Rules 默认 / SmallLlm 3B 可选，失败回退 | ✅ 已完成（ADR-0024：`src/judge.rs` Judge trait + RulesJudge（阈值来自 MindConfig，零字面量）+ SmallLlmJudge（OpenAI 兼容 3B 端点，失败回退 Rules）；`judge_backend`/`judge_endpoint`/`judge_model` config；修复 assess_complexity 10/40 字面量残留；judge-points contract v1.0-draft 入 FlowModus docs；195 tests 全绿） | FlowModus 模型池（JP-2 待 Mind 对接） |
| ⏳ | 并行调度 + 上下文窗口感知 | 等待 FlowModus | FlowModus 未完成 |
| ⏳ | 前缀稳定/KV 缓存复用 | 等待 Callosum，不勉强 | Callosum |

### 候选 D' 完成状态（本轮）

| 任务 | 内容 | 状态 |
|---|---|---|
| D'-1 | `contract::derive_seen_bloom`（bl- 前缀，fnv64 共享原语）替换 execute_calls 空串占位 | ✅ ADR-0007 |
| D'-3 | `pipeline::resolve_pipeline`（fail-open）+ main.rs tentacle_endpoint 接线 | ✅ ADR-0007 |
| D'-2 | Tuck 深度集成：`SecurityGate` 接线点 + ledger `blocked` 记录 + 真实 TuckSecurityGate 连通 | ✅ ADR-0008 |
| D'-4 | 真实场景插件（非 fixture，接入 MCP-Learner stable/ 工具）：`Expect::Ok` 结构判据 + live e2e（真实 Tentacle + 学习产物 3/3 全绿） | ✅ ADR-0009 |

**关键成果**：`seen_entropy_bloom` 从 `""` 占位升级为真实确定性指纹（`bl-` + FNV-1a(`{tool}#{params}`)）；配置 `tentacle_endpoint` 后启动即走六 stage 流水线（fail-open，未配置/失败保持 echo fallback）；**D'-2 管控闭环咽喉落地**——pipeline 执行路径可被 Tuck 闸门拦截（`src/security.rs` SecurityGate trait + `with_security_gate` + ledger `Blocked` 记录，Reject/HITL 阻塞 call 且不进 Tentacle；真实连通测试经 dev-only tuck-core 验证 Low→Pass 执行 / Catastrophic→Reject / Critical→HitlRequired）；**候选 G-T2（ADR-0010）**——`AgentLoop::capture()` 共享快照投影（mode/state/episode/ledger），`/v1/agent/snapshot` 输出真实状态（消除 `token_consumed: 1234` 硬编码），HTTP 端点不触碰 agent 内部（极致解耦）；126 passed + 6 live（#[ignore]，含 m1_e2e_live 3 + m1_5_d4_live 3）。

### M1.5 / 候选 E / 候选 F 剩余项（已消项）

- ~~Reasoning 输出结构化~~（E-T2 完成）
- ~~suggested_actions 结构化 + pipeline 完整 merge~~（E-T3..T5 完成）
- ~~Helix 无会话概念~~（候选 F 完成）
- ~~seen_entropy_bloom 空串占位~~（D'-1 完成）
- ~~main.rs 未消费 tentacle_endpoint~~（D'-3 完成）

### 下一阶段候选

- **候选 D' 剩余**：无（四项全部完成；真实 MCP 执行体升级属 ECOSYSTEM 第二优先级 #4）
- **候选 G：Anaphase 驾驶舱**（完成：G-T2 ✅ / G-T3..T5 ✅ / G-T6 ✅ / G-3 transport 契约修复 ✅ / G-4 bootstrap ✅）——Cellrix 白盒驾驶舱（模式栏 + 经历时间线 + Ledger 审查视图 + 生态状态板）；TUI 先行，Web 面板（G2）后续；**bootstrap `up`（ADR-0011）：一条命令起全栈（tentacle→anaphase→探测→可选 --cockpit）**；**G-5 易用引导 UX 完成**（ADR-0012：欢迎/前置检查/启动/下一步四段式，缺失项带构建提示，首跑零困惑）；**G-6 交互菜单**（ADR-0013：一条命令之后只有选择题——开驾驶舱/看状态/配置说明/退出）
- **候选 A：Tentacle Rust 重构**（P10b 后自然启动）——凭证标签流转（Tuck 注入）/ 异步协程沙箱（ARM 端侧）/ 动态共识适配层 / 多传输层扩展
- **候选 B：生态手套协议渐进**（P10c 预留扩展位）——Cellrix 原生手套协议接入
- **候选 C：保持 P11c/P11d 暂缓**，等 Mind 侧认知工艺显式化

## 认知工艺双向复用轨道状态（备忘录重编号后）

| 阶段 | 状态 | 说明 |
|---|---|---|
| **P11a** CraftAdapter | ✅ 完成（裁决：不建） | 间接触发已覆盖，意志优先，勿增实体 |
| **P11b** 编排链路 | ✅ 完成（验证闭环） | OrchestrationAdapter 不建，链路已通 |
| **P11c** OrchestrationCore | ⏸️ 暂缓 | trait 归属待 Mind 认知工艺显式化后裁决 |
| **P11d** 双向复用 | ⏸️ 暂缓 | 依赖 P11c；模式同构+接口复用，职责不合并 |

---

*Anaphase-Helix PLAN v2.6（候选 G + G-4..G-7，2026-09-06）*
