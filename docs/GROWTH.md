# Anaphase-Helix 生长记录
> **版本**：v1.2
> **日期**：2026-08-28
> **规则**：仅保留最近 3 条记录，超则归档至 `docs/archive/growth/`
> **归档策略**：历史随仓库版本化，永不删除
## 记录 1：P10a T3 完成（2026-08-28）
**变异类型**：契约对齐落地 + 测试闭环
**背景**：
- T1（proto 契约同步）与 T2（mind.rs 补全：系统探针/trace/去硬编码/降级钩子）已完成并提交
- T3 目标：mind_endpoint 接线 + 集成测试闭环
**关键决策与发现**：
1. **build.rs `build_server(true)`**：生成 server trait，测试内 mock Mind（不依赖真实 Helix-Mind 二进制）
2. **mock 优雅停机**：`serve_with_incoming_shutdown` + oneshot（`abort()` 不停已接受连接）
3. **`resolve_memory_adapter` 抽取**为可测装配（空→Noop / 失败→warn+Noop fail-open / 成功→Grpc）
4. **发现并修复 3 个问题**：
   - axum 依赖错位（加 tokio-stream 时把 `[dev-dependencies]` 头插在 axum 前，axum 掉进 dev-deps）→ 归位 `[dependencies]`
   - proto `FederatedDAGShareRequest` 生成名为 `FederatedDagShareRequest`（camelCase）→ 修正测试
   - server 下线测试失败 → 改优雅停机机制
5. **方法论闭环补全**：ADR-0001 追加 T3 决议、DNA 原则 1 补 Mind 不可用 fail-open、spec/contract.md 补测试契约
**状态**：✅ 完成（31 测试全绿，T1/T2/T3 契约对齐落地）
---
## 记录 2：方法论体系补全（2026-08-28）
**变异类型**：方法论补全（对齐 Helix-Mind docs 结构）
**背景**：
- 参照 Helix-Mind docs 结构，Anaphase 缺 SPEC.md / RNA.md / DEPRECATE.md / spec/ 分卷
- VISION.md 为一体式（卷 A-E 内嵌），分卷导航指向 spec/ 但目录未建（断链）
**关键决策**：
1. **SPEC.md 新建**：一粒种子的自白（完整叙事）+ 分卷导航，与 Helix-Mind SPEC 角色对齐
2. **spec/ 分卷 6 个**：position / principles / architecture / contract / ci-144 / lifecycle，从 VISION 卷 A-E 拆分 + 架构蓝图 v11.0 提炼
3. **RNA.md 新建**：三层加载协议（活跃档案/分卷/考古）+ AI 协作铁律 + 大版本 SOP
4. **DEPRECATE.md 新建**：凋亡清单（DEP-001 Python legacy / DEP-002 FlowModus / DEP-003 REST 遗留）
5. **VISION.md 重构为根索引**（v2.0）：原子原则表补全至 10 条（对齐 DNA 实际 10 条），生态位置/组件仓库索引
6. **DNA.md 文档生态 SOP 补全**：加入 VISION/SPEC/RNA/DEPRECATE/spec 职责
7. **GROWTH 归档**：最旧"初始化"记录移入 `docs/archive/growth/2026-08-28-initialization.md`
**状态**：✅ 方法论体系补全完成
---
## 记录 3：P10b 认知工艺触发链路完成（2026-08-28）
**变异类型**：认知工艺触发链路 + HITL 审批通道 + 跨项目裁决
**背景**：
- P10b 目标：打通 Anaphase → Mind 认知工艺触发链路
**关键决策与发现**：
1. **T1 System 0 门控经 budget_tier 前置路由验证**：确认 Mind `layer3.rs` 接收映射 `budget_tier` → core `BudgetTier` → `retrieval.query`（链路已通）；划界：预算路由（外部前置）决定扫描范围 / System 0（Mind 内）决定思考深度，二者正交
2. **T2 状态机驱动 suggested_mode**：`MemoryAdapter::set_complexity` 默认钩子 + `GrpcMindAdapter` AtomicU8 复杂度 + `derive_suggested_mode(query, complexity)` 状态优先（1→Skilled/2→Anchor/3→Imagination），0 兜底长度启发式
3. **T3 HITL 审批通道**：`src/hitl.rs`（is_high_risk 写/网络/凭证判定 + check_approval，默认 fail-closed）；Execution 接入执行闸；三层闸门串联：工具审计（入库门）→ HITL（执行闸）→ Tuck（边缘物理闸）
4. **跨项目裁决（认知工艺双向复用备忘录审查）**：方向采纳 + 4 条修正——重编号 P11a→P11d（避免与主线 P10c 撞号）、CraftQuery 暂缓（P10b 间接触发已覆盖第一层，勿增实体）、措辞"模式同构+接口复用"（脑手分离铁律不变）、trait 归属推迟
5. **生态对齐**：Cellrix = 原生生态手套（优先级最高）；CI-144 v2.0 等待冻结不阻塞；Tentacle Rust 重构硬性对齐（凭证标签/布隆过滤器/异步沙箱/动态共识/多传输）
**状态**：✅ 完成（37 测试全绿，P10b 验收通过）
---
## 记录 4：预留
*（按 DNA v2.0 SOP，新记录追加至此，旧记录自动归档）*
