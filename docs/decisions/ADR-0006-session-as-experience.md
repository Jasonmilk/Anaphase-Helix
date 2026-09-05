# ADR-0006：会话即经历——Episode 边界与三模式参与度

- **状态**: Proposed → Active（2026-09-05 用户批准）
- **日期**: 2026-09-05
- **决策范围**: Anaphase（episode 生命周期与模式）/ 契约复用（INTENT-7 动词、认知工艺、L3 content）
- **关联**: ADR-0005（候选 E 确定性信封）、ADR-0021（认知工艺，Mind 体系）、INTENT-7 spec v1.0.0-RFC-4（autonomy_level / FINISH / WRITE_NODE）
- **前置事实**（物理核验）：Mind L3 节点 `id: UUID`、`content: JSON` 保留结构化记录、默认 PRIVATE；L3 永不物理删除，请求删除→突触切断+遗忘标记；认知工艺已有"元批判"工序与独立会话隔离；INTENT-7 已有 `FINISH`（认知循环结束→Mind 记录 L3 收尾）与 `autonomy_level = AGENT/OPEN/SURVIVAL`；Anaphase `main.rs` 已有 NoopMemoryAdapter 装配路径（无 Mind 驾驶基础）。

## 1. 背景与问题

Helix 生态无"会话"概念——Anaphase 的 `run_cycle` 是逐轮认知循环，轮与轮之间没有"这段对话属于同一段经历"的边界。传统 harness（DeepSeek/ChatGPT 式）把会话当作隔离的上下文容器；Helix 拥有连续心智、睡眠复盘与代谢循环，沿用该模型等于否定潜意识层的价值。

**核心立场（用户主张）**：会话不是人类的资产，会话是 Helix 的经历——每次对话是它遭遇的一段 L3 情景。Mind 应能"看到"会话（元认知），像人反思"我刚才经历了什么"。

**审查教训**（2026-09-05 严肃审查）：本决策的所有复用点必须来自 spec/代码原文核验，不引用不可复核的版本号、不发明不存在的 spec 章节；DSH（dsh-desktop）只作灵感参考（宿主壳哲学），**零借用其命名**。

## 2. 决策

### D1: 经历边界 = Episode（不新建协议、不新建 crate、不改 Mind schema）

Episode 是 Anaphase 侧的**经历分组标签**：标识"从哪条输入开始的一段对话"。物理落点：
- `contract::derive_episode_id(input)`：与 `derive_job_id` 共用内部 `fnv64`（FNV-1a 64-bit，offset basis `0xcbf29ce484222325`、prime `0x100000001b3`），前缀 `ep-` + 16 hex。**确定性**：同输入同 id，可回放。
- `agent_loop::Episode { id, step }`：`step` 为轮次索引（0 起，每轮 run_cycle 自增）。
- **L3 摄取带 provenance**：Reflection 写入时 content 为结构化 JSON，含 `"episode": "{id}#{step}"`（与 trace_id=`{job_id}#{index}` 同一派生模式）。Mind 侧无需任何改动——L3 `content: JSON` 本就保留结构化记录，episode 是 content 的一个字段。

**为何不落 Mind schema**：Mind 节点 `id: UUID` 已是全局唯一标识；episode 只是"经历分组键"，不需要全局唯一（同首条输入的两个会话可共享 episode_id，靠 step 与内容区分）。尊重 Mind data-contract，不加字段，极致解耦。

### D2: 经历收束 = EpisodeDigest（复用现有 remember 通道）

`end_episode()` 生成 digest（`{episode_id, turns, first_input}`）并写入 L3——**复用既有 `MemoryAdapter::remember`，不发明 RPC**。语义对应 INTENT-7 `FINISH`（认知循环结束→Mind 记录 L3 收尾），但 Anaphase 侧不发协议消息：digest 作为 L3 content 落盘，睡眠复盘（认知工艺，ADR-0021）自然消化。**忘了对话，记得教训**（L3 突触切断语义已由 Mind 保障）。

### D3: 三模式 = 同一 Helix 的三种 Mind 参与度（不建三套心智）

| 模式 | Mind 参与 | 物理载体 | 对齐 |
|---|---|---|---|
| `Drive` | 不在场 | main.rs 装配 NoopMemoryAdapter（已有）+ 不写经历（Noop 天然隔离） | 人类直开 Anaphase，类似 dsh 直用 |
| `Partner` | 参与 | GrpcMindAdapter + Episode 生命周期 | INTENT-7 `OPEN`（读写协作） |
| `Survive` | 全权 | Mind 自主循环（P10c 域）；Anaphase 侧枚举占位，反向驱动待 P10a | INTENT-7 `SURVIVAL` |

模式枚举 `Mode { Drive, Partner, Survive }` 入 `RunCycleConfig`（config.toml `[anaphase.run_cycle] mode`），**默认 `Partner`**（Helix 本体是有记忆的伙伴）。运行期 Episode 写入**不按 mode 分支**——Noop adapter 天然不落盘，Drive 自动达成；Survive 不给不存在的反向通道写死逻辑（按需加载）。

### D4: 命名空间声明（DSH 零借用）

DSH 只提供宿主壳哲学灵感（宿主不重实现运行时、门面借众人习惯）。Helix 命名：`Episode`（对齐 L3 episodic memory）、`Mode::Drive/Partner/Survive`、前缀 `ep-`（与 `run-` 同派生模式）。**不采用 DSH/Harness 任何命名**。

### D5: 元认知 = 认知工艺（勿增实体）

"Mind 看到会话"的引擎是既有认知工艺（ADR-0021：元批判工序 + 睡眠复盘 + 价值评估器）。Episode 提供的"经历边界"让认知工艺获得**整段经历的累积对象**（跨轮次差距评估），而非新引擎、新 crate。

## 3. 备选方案与拒绝理由

| 备选 | 拒绝理由 |
|---|---|
| 新建会话协议/RPC（helix_query/helix_write 等） | 重复建设——INTENT-7 FETCH/WRITE_NODE + Mind HelixQuery/HelixConsolidate 已存在（api.md 单一真相源） |
| 改 Mind L3 schema 加 provenance 字段 | 不必——L3 content 本就结构化，episode 是其字段；尊重 data-contract，解耦 |
| 新建 session crate/引擎 | 如无必要勿增实体——episode 是 agent_loop 的轻量 struct |
| 三模式建三套心智 | 破坏心智连续性；正确变量是 Mind 参与度（adapter 装配）与模式枚举 |
| 采用 DSH 命名/会话模型 | 否定 Helix 哲学；门面借习惯，房间全属自己 |

## 4. 后果

**正面**：
- 逐轮 L3 摄取升级为带经历边界的结构化经历，Mind 可"看到"整段会话；
- 元认知获得跨轮次的差距评估对象（认知工艺消费 digest）；
- 三模式物理落地：Drive=Noop 装配（已有）、Partner=episode 生命周期、Survive=P10 域占位；
- 全部复用既有要素：fnv64 派生模式、remember 通道、L3 content、认知工艺、autonomy_level 语义。

**负面/代价**：
- episode 唯一性不保证全局唯一（同首条输入两会话共享 id）——接受：episode 是分组键非主键，Mind 节点 id 才是唯一标识；
- Survive 完整运行依赖 Mind P10a 反向通道（本 ADR 只落枚举与文档）。

**风险与对策**：
- 既有 94 测试破坏风险 → 无 episode 时 Reflection 写入**原样不包装**（严格向后兼容），episode 仅显式 begin 后生效；
- 命名争议 → 简短准确、可协商（用户预设权）。

## 5. 实现要点

| 项 | 位置 | 状态 |
|---|---|---|
| `fnv64` 提取共用 + `derive_episode_id` | `src/contract/mod.rs` | 待实现 |
| `Episode` + `Mode` + begin/end API | `src/agent_loop.rs` | 待实现 |
| Mode 入 config（serde，默认 Partner） | `src/config.rs` + config.toml | 待实现 |
| Reflection 写入带 episode provenance（无 episode 原样） | `src/agent_loop.rs` | 待实现 |
| 测试（golden/生命周期/provenance/兼容/mode serde） | `tests/episode_lifecycle.rs` | 待实现 |
| 文档五件套 | ADR-0006 → PLAN → GROWTH → README → ECOSYSTEM | 待实现 |

**相邻债**：Cellrix 门面（会话列表=经历时间线、状态栏=生活视图）为独立候选（Cellrix 仓库，ADR-0031+ 或 Cellrix 体系）；Survive 反向通道待 Mind P10a。
