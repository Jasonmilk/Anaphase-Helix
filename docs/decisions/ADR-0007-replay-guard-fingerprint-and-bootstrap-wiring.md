# ADR-0007：重放守卫指纹 + 启动接线（候选 D'-1 / D'-3）

- **状态**: Proposed → Active（2026-09-05 用户批准）
- **日期**: 2026-09-05
- **决策范围**: Anaphase（熵指纹派生 + pipeline 启动装配）
- **关联**: ADR-0004（seen_entropy_bloom 语义定义）、ADR-0005（pipeline 信封）、ADR-0006（fnv64 共享派生原语）
- **前置事实**（物理核验）：Tentacle proto 有 `seen_entropy_bloom = 5` 字段（optional）；Tentacle grpc 服务仅透传 `Option<String>`（`req.seen_entropy_bloom.is_empty() → None`），**无消费逻辑**；`execute_calls` 当前传 `String::new()` 占位；`SystemClock`（production）与 `FakeClock`（测试）已存在；`PipelineConfig::from_codex` 读 `knowledge_base/fixture-codex.json`；`config.toml` 已有 `tentacle_endpoint` 但 main.rs 未消费；`resolve_memory_adapter` 已示范 fail-open 装配模式。

## 1. 背景与问题

候选 D'（PLAN v2.0）四项中，D'-2（Tuck 深度集成）依赖 Tuck 侧接口、D'-4（真实场景插件）依赖 MCP-Learner 升级——均阻塞。**不阻塞的两项先行**：

- **D'-1 重放守卫**：`seen_entropy_bloom` 从空串占位升级为真实确定性熵指纹。物理事实：Tentacle 字段存在但未消费——Anaphase 侧把"这个 call 的特征"（工具+参数）确定性地指纹化并透传，Tentacle 侧消费点留相邻债（跨仓库，勿越界）。
- **D'-3 启动接线**：`tentacle_endpoint` 配置至今未被 main.rs 消费——确定性执行通道（pipeline）在运行时从未被装配。接线后 `cargo run`（配置了 endpoint 时）直接走六 stage 流水线而非 echo fallback。

## 2. 决策

### D1: 熵指纹 = fnv64 派生（D'-1，不改 Tentacle、不依赖 Callosum）

`contract::derive_seen_bloom(tool, params) -> String`：`bl-` + 16 hex（`fnv64(format!("{tool}#{params}"))`）。复用 ADR-0006 提取的共享原语——**不新建哈希、不新建 bloom 实现**。同 tool+params 同指纹（确定性回放）；不同 call 不同指纹。

`execute_calls` 的占位 `String::new()` 替换为 `derive_seen_bloom(&call.tool, &params)`。行为零变化（Tentacle 未消费），但语义真实有效——指纹从"占位空串"升级为"call 特征"。

**为何不做 Anaphase 内部 bloom filter**：bloom 的检测语义属于执行体（Tentacle 侧判断"这个副作用执行过没有"）；Anaphase 是信封层，只应携带特征。内部 bloom 会引入位图状态，破坏 pipeline 的无状态确定性（同输入两次运行结果需字节级一致）。**Callosum 不参与**：Callosum 是上下文内存分配器（左右脑互通），重放守卫是执行特征携带，职责不同（勿增实体、极致解耦）。

### D2: 启动接线 = fail-open 装配（D'-3）

`pipeline::resolve_pipeline(endpoint: Option<String>, config: PipelineConfig) -> Option<Pipeline>`（async）：endpoint 空 → None；`GrpcTentacleAdapter::new` 失败 → warn + None（fail-open，DNA 铁律 6，与 `resolve_memory_adapter` 同模式）；成功 → `Pipeline::new(adapter, SystemClock, config)`。

main.rs：`tentacle_endpoint` 非空 → `resolve_pipeline` → `agent.with_pipeline(pipeline)`。未配置/失败 → 保持 legacy echo fallback（向后兼容 105 测试）。

## 3. 备选方案与拒绝理由

| 备选 | 拒绝理由 |
|---|---|
| Anaphase 内部 bloom filter（位图 + 跳过执行） | 破坏 pipeline 无状态确定性（同输入两次执行字节级一致是 ADR-0003 验收判据）；检测语义属于执行体 |
| 接 Callosum 布隆过滤器 | Callosum 是上下文内存分配器（GetStaticPrefix/UpdateStaticPrefix），职责不同；且无实现（勿增实体、勿依赖幽灵组件） |
| 改 Tentacle 增加消费逻辑 | 跨仓库协调（阻塞项）；Tentacle 字面零改动是生态约束先例；消费点留相邻债 |
| main.rs 内联装配 | 与 resolve_memory_adapter 的既有 fail-open 模式不一致；pipeline 装配逻辑应可单测 |

## 4. 后果

**正面**：
- `seen_entropy_bloom` 从占位升级为真实确定性指纹（节能语义立即可见、可审计）；
- 运行时确定性执行通道落地：配置 `tentacle_endpoint` 即走六 stage 流水线；
- 零破坏：未配置/失败保持 echo fallback；Tentacle 零改动。

**负面/代价**：
- 指纹暂不被 Tentacle 消费（行为无变化，仅语义正确）——相邻债：Tentacle 侧消费点待生态裁决；
- 启动接线依赖 fixture-codex.json 文件（配置来源，与测试一致）。

**风险与对策**：
- `GrpcTentacleAdapter::new` 启动时连接失败 → fail-open 降级 None + warn（不阻塞启动）；
- 指纹格式未来被 Tentacle 消费时需跨仓库对齐 → `bl-` 前缀与 run-/ep- 同族，格式文档化于本 ADR。

## 5. 实现要点

| 项 | 位置 | 状态 |
|---|---|---|
| `derive_seen_bloom` + golden 测试 | `src/contract/mod.rs` | 待实现 |
| `execute_calls` 指纹透传 | `src/pipeline/mod.rs` | 待实现 |
| `resolve_pipeline`（fail-open）+ 单测 | `src/pipeline/mod.rs` | 待实现 |
| main.rs 启动接线 | `src/main.rs` | 待实现 |
| mock 捕获 bloom + 透传集成测试 | `tests/common/mod.rs` + `tests/mock_tentacle.rs` | 待实现 |
| 文档五件套 | ADR-0007 → PLAN → GROWTH → README → ECOSYSTEM | 待实现 |

**相邻债**：Tentacle 侧 `seen_entropy_bloom` 消费逻辑（生态裁决后）；D'-2（Tuck 深度集成）/ D'-4（真实场景插件）仍阻塞。
