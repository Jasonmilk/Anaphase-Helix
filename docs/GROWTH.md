# Anaphase-Helix 生长记录
> **版本**：v1.8
> **日期**：2026-09-06
> **规则**：仅保留最近 3 条记录，超则归档至 `docs/archive/growth/`
> **归档策略**：历史随仓库版本化，永不删除


---

---

---

## 记录 21：O-5 按需认知注入——记忆折叠进推理请求（2026-09-06，ADR-0023）
**健康快照**：✅ 完成（189 tests 全绿 = 183 + 6 注入/折叠/近零增长）
**物理事实**：
- 探查确认记忆检索断裂：memory_nodes 存 context 但从未注入 LLM（检索白做）
- 对话窗口无数据源（HTTP 仅 snapshot/events 端点）——窗口 L0 诚实标注待 UI
  会话层接入，不伪造缓冲
- main.rs 硬编码演示输入（0 硬编码违规）→ `--input` / `smoke_input` / 协议
  默认 const 三级来源
**验证**：
- 注入：prompt 含 `[memory]` + 节点原文（断裂修复）✅
- 折叠：超预算截断 + 显式标记 ✅；预算 0 = 纯无状态（legacy 兼容）✅
- 25 轮：注入段每轮 ≤ 预算恒定（Memory-Efficient 验收）✅
- 冒烟：`--input "hello test"` 生效 ✅；全量 189 passed / 0 failed ✅
**状态**：✅ 完成（commit 见 git log；ECOSYSTEM v1.40 同步）

## 记录 22：O-6 判断点后端可配化——JP-1 复杂度评估 Rules/SmallLlm 双后端（2026-09-06，ADR-0024）
**健康快照**：✅ 完成（195 tests 全绿 = 189 + 6 judge）
**物理事实**：
- 用户务实修正编排哲学（2026-09-06）：0 tokens 是默认通道不是教条，3B 级小 LLM
  判断质量 ROI 足够高时可用——已固化 HANDOFF §1.3
- 探查抓真伤：assess_complexity 硬编码 10/40（O-4 已收 MindConfig 但此函数没用）
- FlowModus 本地存在（Python，5 层确定性路由）——judge-points contract
  v1.0-draft 入其 docs（JP-1/JP-2 规格）
**验证**：
- 零硬编码：assess_complexity 字面量删除，RulesJudge 阈值来自 MindConfig ✅
- SmallLlm 成功路径：本地 mock OpenAI 端点 → complex 标签 → TIER_COMPLEX ✅
- 失败回退：不可达端点 → 回退 Rules（32 字符 → moderate）✅
- 非法标签拒绝 + config 缺参降级（警告 + Rules）✅
- 全量 195 passed / 0 failed ✅
**状态**：✅ 完成（ECOSYSTEM v1.41 同步；judge-points contract 已推 FlowModus）

## 记录 16：P10a 认知工艺触发链路（helix_craft 客户端 + 按需折入）完成（2026-09-06）
**变异类型**：Mind 认知工艺（P10，ADR-0031）的 Anaphase 侧触发实体
**背景**：Mind 侧 helix_craft RPC 完成（8c15a4f）后，Anaphase 需客户端同步 + 按需触发点，闭环 Anaphase→Mind 编排链路
**关键决策与发现**：
1. **默认方法降级（D1）**：MemoryAdapter.craft() 默认 Err（不可用）——Noop/全部既有实现零改动，静默降级（增强非依赖）
2. **按需驱动（D2）**：GrpcMindAdapter.craft() 内部从 MindConfig 取工序集/约束（协议默认，DNA 原则 11）——循环只问"要不要想"，adapter 决定"怎么想"（极致解耦）
3. **触发语义（D3）**：run_cycle MemoryRetrieval 阶段、非结构化输入触发（结构化 `!` 命令计划已存在，跳过思考）；失败静默（craft_note=None）
4. **0 token 折入（D4）**：Reasoning 阶段以 [think-first] 标记折入 synthesis——确定性思考先于 LLM tokens（用户编排哲学：先思考，后花钱）
5. **验证**：tests/craft_trigger.rs 3 例（触发+注入断言 / 结构化跳过 / Noop 降级）+ MockMind craft stub；全套件 198 绿 0 warning
**验收**：Anaphase→Mind 真实 gRPC 编排触发链路打通（触发→helix_craft→CognitiveCraft orchestrate→0 token synthesis→注入 prompt）
---
