# ADR-0024: 判断点后端可配化——复杂度评估的 Rules / SmallLlm 双后端（O-6）

- **状态**: Accepted
- **日期**: 2026-09-06
- **决策范围**: Anaphase（judge 模块 / config / run_cycle 复杂度评估链）
- **关联**: ADR-0002（DNA 原则 11）、ADR-0001（Mind 触发链）、ADR-0022（认知工艺触发）、
  judge-points contract v1.0-draft（FlowModus docs，JP-1/JP-2）

## 1. 背景与问题

用户务实修正编排哲学（2026-09-06）：**0 tokens 是默认通道，不是教条**——
没把握/实现不了时，小 LLM（尤其 3B 级）判断完全合法，判断质量 ROI 高于省
tokens 时选 LLM。为 FlowModus 铺路：判断点显式选择后端（Rules / SmallLlm /
LLM），不做系统级 Auto Router。

物理探查发现真伤：
1. **0 硬编码违规残留**：`assess_complexity`（复杂度评估）硬编码阈值 10/40，
   O-4 已把阈值收进 `MindConfig`（skilled_len/anchor_len）但此函数没用——无来源
   字面量；
2. **JP-1 无后端抽象**：复杂度评估只有长度启发式一条路径，无法按需切换。

## 2. 决策

### D1: judge 模块（极致解耦）
新建 `src/judge.rs`：
- `Judge` trait：`assess_complexity(query) -> u8`（**全后端必返回 1/2/3**，total）；
- `RulesJudge`：长度启发式，阈值来自 `MindConfig`（skilled_len/anchor_len），
  零字面量；
- `SmallLlmJudge`：OpenAI 兼容 chat completion（3B 级端点，如 Tuck 的本地
  llama base URL）；**任何失败（连接/HTTP/非法标签）回退 RulesJudge**
  （fail-safe，确定性优先）；
- `resolve_judge`：config 装配；`small_llm` 缺 endpoint/model → 降级 Rules +
  启动警告（fail-safe）。

### D2: config 单一来源
`AnaphaseConfig` 增 `judge_backend`（`rules` 默认 / `small_llm`，serde
snake_case）、`judge_endpoint`、`judge_model`。Rules 阈值复用 `MindConfig`
（不新建重复字段）。

### D3: 组装点唯一
main 装配 `resolve_judge` → `agent.judge`；`AgentLoop::new` 默认
RulesJudge（阈值取自 `MindConfig::default()`，非字面量）。调用点：
PreAssessment → `judge.assess_complexity(input).await` → `memory.set_complexity`
（原 `assess_complexity` 自由函数删除）。

### D4: JP-2（budget_tier）保持 Rules
`derive_budget_tier` 已 config-sourced（规范），SmallLlm 可换性在
judge-points contract（JP-2）标注，待 Mind 侧对接（EnergyContext 构造链）时
落地——如无必要勿增实体，不铺摊子。

## 3. 验收结果（物理验证）

| 判据 | 结果 |
|---|---|
| 零硬编码修复 | ✅ `assess_complexity` 自由函数（字面量 10/40）删除，RulesJudge 阈值来自 MindConfig |
| Rules 默认行为不变 | ✅ 195 全绿（189 基线 + 6 judge 新测试），默认后端零回归 |
| SmallLlm 成功路径 | ✅ 本地 mock OpenAI 端点返回 `complex` → 映射 TIER_COMPLEX（真实 HTTP 端到端） |
| SmallLlm 失败回退 | ✅ 不可达端点 → 回退 Rules 判定（fail-safe） |
| 非法标签拒绝 | ✅ `parse_tier_label` 拒绝自由文本（确定性优先） |
| config 缺参降级 | ✅ `small_llm` 无 endpoint → 降级 Rules + 警告 |
| 阈值来源 | ✅ `rules_judge_uses_config_thresholds_not_literals`（5/8 阈值验证 1/2/3） |

## 4. 备选方案与拒绝理由

| 备选 | 拒绝理由 |
|---|---|
| 只在 Rules 加 config 阈值（不做 SmallLlm） | 用户明确 3B 判断质量 ROI 高；为 FlowModus 铺路需要判断点后端抽象 |
| 系统级 Auto Router（按 query 自动选后端） | M3 边界——显式选择，不做无脑自动路由 |
| JP-2 同步可配化 | EnergyContext 构造是同步链，SmallLlm 需 async 网络——改动面大、收益低；契约已标注待 Mind 对接 |
| SmallLlm 失败返回 Err 由调用方处理 | 违约——`Judge` 全后端 total（必返回 1/2/3），回退内聚在实现内，调用方零分支 |

## 5. 后果

**正面**：
- 复杂度评估按需选后端：默认 0 tokens（Rules），配置一行切 3B 分类（质量提升）；
- 0 硬编码在 JP-1 收口（阈值单一来源 MindConfig）；
- FlowModus 铺路落地：判断点后端选择范式 + 契约（JP 清单）已就位；
- fail-safe 链完整：任何 SmallLlm 异常都不会污染编排（回退 Rules 确定性判定）。

**负面/代价**：
- SmallLlm 每轮复杂度评估多一次 3B 调用（有界小成本，换取语义判断质量）；
- JP-2 尚未可配化（契约标注，待 Mind 侧对接）。

**风险与对策**：
- 3B 端点延迟/抖动 → 超时回退 Rules（reqwest 默认超时）；judge 请求无
  retry（确定性优先，宁可回退不重试）；
- 模型输出格式漂移 → `parse_tier_label` 只认三个裸标签，其余一律回退。
