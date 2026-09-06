# ADR-0018: Rails——心智外铁轨（人类知识 DAG · Helix 只读引用）

- **状态**: Accepted
- **日期**: 2026-09-06
- **决策范围**: Anaphase（MemoryRetrieval 分支 / rails 模块）
- **关联**: ADR-0003（确定性 pipeline）、ADR-0005（零硬编码收口）、ADR-0016（编排哲学）、
  ADR-0021（认知工艺，Helix-Mind）、ADR-0022/0023（三模式会话范式）

## 1. 背景与问题

Helix 三模式（ADR-0022/0023）中，驾驶模式是"人类决策、Anaphase 执行、Mind 不在场"。
但驾驶模式存在一个刚需缺口：**人类需要 Helix 引用权威知识（宪法、律法、SOP、操作手册），
且必须原文引用、零改写、零编造**——"律所最爱：不猜测，律文直接 copy，不自己吐"。

缺口具体化：
- 查法典只能驾驶模式自己翻，Helix 无法协助（伙伴模式也用不上）
- 通用 LLM 会幻觉法条（行业实测 19% 引用幻觉率；律师因 AI 幻觉引用被法院制裁是真实失败模式）
- 没有"铁轨"，Anaphase 的行为不受知识边界约束——不可控

**核心立场**：人类提供**心智外铁轨**（只读、版本冻结、DAG 结构的知识资产），
Helix 只能沿铁轨选一条边（检索 → 原文引用 → 验证），永远不能生成边（不编造）。

**与熟练模式的关系（关键澄清）**：Helix-Mind 的熟练模式（sa-core.md）本就是
"贴地飞行，沿 DAG 铁轨，仅沿高权重边"——那是**心智内铁轨**（Helix 自身经历内化的
软策略，EMA 权重，会被新经历修正）。rails 是**心智外铁轨**（人类资产，权威事实，
版本冻结，Helix 无权修正）。**同一张"沿 DAG 导航"的机制，两种铁轨**：
错得起就软（熟练，靠反馈收敛）；错不起就硬（rails，靠验证器锁死）。
硬度 = 错误的代价。法律错误代价不可承受 → rails 必须硬。

## 2. 决策

### D1: rails = 人类资产，只读，版本冻结
- 位置：`knowledge_base/rails/<kb>/`（一个 kb 一个目录，如 statutes / company-rules）
- 载体：markdown 原汁原味（人类直接写，无需转换）
- 版本冻结：每个文件 SHA-256 指纹入索引；`manifest()` 输出确定性 JSON 可存为版本记录
- **只读**：`RailScope` 枚举只有 `Read` 变体——类型层面无写侧，Helix 无法改写铁轨

### D2: mddag 轻量解析（不接完整 lodestone 协议）
- 节点 = 一个 markdown 标题 + 其正文（ATX 标题层级定义父子树）
- 边 = 标题层级（parent/children）+ markdown 链接（`[text](doc.md#heading)`）
- 节点 id = `{doc}#{heading}`（确定性派生，无 UUID；重复标题 `~N` 后缀）
- **断链即构建错误**（链接目标不存在 → Err）——铁轨必须良构（确定性 + 严格）
- 完整 lodestone 协议留 M2，本轮轻量消费其 mddag 格式

### D3: 铁轨导航（确定性检索，无嵌入）
- 检索特征：整句子串 + 词项 + **CJK bigram**（≥2 个不同 bigram 命中才计分，防"法的"这类
  单 bigram 噪声假命中）；评分 = 标题命中权重高 + 正文命中低
- **不用向量/嵌入**：概率性 + 外部依赖，违背确定性优先与极致节能
- 展开：`expand` 沿 parent/children/refs 边把邻居记入 **visited set**（provenance）
- 注入：只注入命中的节点子图，总字节受 `max_inject_bytes` 预算约束（按需加载）

### D4: 引用契约（确定性验证器）
- 涉及铁轨的问题，回答 = **原文引用 + 节点 id**，禁止 paraphrase
- 验证器 `verify_reference`（纯函数，criteria 同族）：
  1. 节点存在；2. 节点 ∈ visited（导航真的访问过）；3. 引用文本是节点内容的**原文子串**
  ——三条全过才通过，任何一条失败即拒绝
- **引不到就答不出**：`NO_RAIL_CONTENT` 常量（graceful refusal）——铁轨没有的内容
  绝不生成句子

### D5: 接线点 = MemoryRetrieval（如无必要勿增实体）
- MemoryRetrieval 状态（既有 7 状态之一）扩展 rails 分支：rails 命中 → 注入节点 +
  置 `rail_mode`（引用契约标志）→ 照常走 memory.query（心智记忆与铁轨是**两条知识线**，
  物理分开：心智记忆会消化会遗忘，铁轨不消化不遗忘）
- 运行参数进 `RailsConfig`（enabled / kb_dir / max_hits / max_inject_bytes），
  默认值文档化（DNA 原则 11 / ADR-0002）——零硬编码

### D6: 三模式接入
| 模式 | rails 参与 | scope |
|---|---|---|
| 驾驶 | ✅ 主用（人开车走自己的铁轨） | `rail_read` |
| 伙伴 | ✅ 按需借用（Helix 协助查法典） | `rail_read` |
| 生存 | ✅ 环境约束（法律/伦理类） | `rail_read` |
写路径全模式不存在。Tuck 物理 scope 发放（CAPABILITY-13）为后续接线项。

## 3. 备选方案与拒绝理由

| 备选 | 拒绝理由 |
|---|---|
| 向量/嵌入检索 | 概率性 + 外部依赖；违背确定性优先 / 极致节能 / 零新依赖 |
| 完整 lodestone 协议接入 | M2 才消费；本轮 mddag 轻量格式足够，如无必要勿增实体 |
| rails 入库 Mind（消化提炼） | 破坏"人类资产"性质——消化会改写权威内容；两线必须物理分开 |
| 让 Helix 能写铁轨 | 权威知识可被污染；类型层面已锁死（RailScope 无写变体） |
| 集中式知识库后端/图数据库 | 重、外部依赖；本地 markdown + 确定性索引足够 |

## 4. 后果

**正面**:
- 律法/宪法/SOP 场景：Helix 可协助查证且**零编造**（验证器物理锁死）
- 行为可控：知识边界 = 铁轨边界——铁轨有什么才能引什么
- 确定性：同目录两次索引字节级一致；同输入导航/验证结果一致（可回放）
- 与熟练模式共享"沿 DAG 导航"哲学——心智内软铁轨 + 心智外硬铁轨，互不冲突

**负面/代价**:
- 铁轨内容需要人类维护（权威来源、版本管理）——这是设计使然（人类资产）
- 中文检索用 bigram 近似（无分词器），长文本命中精度有限——确定性换取精度，可接受
- 多 kb 挂载（同时挂 statutes + company-rules）本轮未做——config 指向单 kb，多 kb 后续

**风险与对策**:
- 铁轨内容过期（法规修订）→ 版本冻结 + 人类换版本；Anaphase 不自动更新（保持确定性）
- bigram 误命中 → 验证器兜底（引用必须原文子串），检索只负责召回入口

## 5. 实现要点与状态

| 项 | 位置 | 状态 |
|---|---|---|
| rails 模块（Index/Node/Navigation/RailCheck/RailScope + build/navigate/expand/verify） | `src/rails.rs` | ✅ 完成 |
| RailsConfig（enabled/kb_dir/max_hits/max_inject_bytes，默认值文档化） | `src/config.rs` | ✅ 完成 |
| MemoryRetrieval rails 分支（注入 + rail_mode） | `src/run_cycle.rs` | ✅ 完成 |
| 启动挂载（缺失/失败 fail-open） | `src/main.rs` | ✅ 完成 |
| 演示 kb（虚构微型法典，跨文档引用） | `knowledge_base/rails/demo/` | ✅ 完成 |
| 测试（索引确定性/断链/导航/验证器/refusal/scope/e2e） | `tests/rails.rs` | ✅ 9 用例 |
| Tuck `rail_read` scope 物理发放 | Tuck（CAPABILITY-13） | ⏳ 后续接线 |
| 多 kb 挂载 / 真实 kb 接入 | — | ⏳ 按需 |

**验证**：169 tests 全绿（160 基线 + 9 新增）；真实二进制启动日志 `Rails mounted:
knowledge_base/rails/demo`；e2e（run_cycle 真实循环）rail 命中注入原文节点 + rail_mode 置位。

## 6. 一句话总结

> 熟练模式是 Helix 长出来的软铁轨，rails 是人类放上去的硬铁轨——
> 同一张 DAG 导航图，左边会进化，右边不冻结不放行；
> 查法典时 Helix 只会说"原文如此"——它没有别的路。
