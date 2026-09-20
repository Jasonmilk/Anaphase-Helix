# ADR-0044: 授权缝 —— 承诺与许可分离，许可来自人类措辞

- **状态**: Active
- **日期**: 2026-09-20
- **决策范围**: anaphase-helix（`src/adapters/mind.rs`）
- **关联**: `anaphase:ADR-0022`（Mind 适配器零硬编码收口）、`anaphase:ADR-0002`（DNA 原则 11 零硬编码）、`anaphase:ADR-0040`（生态事实单一来源）、`helix-mind:ADR-0044`（决策先于仪器）、K-059 / K-045(b)
- **取代**: 无。修正 `ADR-0022` 中 `allow_imagination` 的推导方式，原决策的其余部分不变

## 1. 决策背景

`src/adapters/mind.rs` 组装 `HelixQueryRequest` 时，三个字段代表**三种不同的权威**，却混在一个结构体里、由同一层产生：

| 字段 | 应当是谁的声音 |
|:---|:---|
| `suggested_mode` | **身体**的建议（按复杂度与查询长度推导） |
| `allow_imagination` | **人类**的许可 |
| `autonomy_level` | **姿态** |

而实际写的是：

```rust
allow_imagination: suggested_mode == CognitiveMode::Imagination
```

**身体自己建议、又自己批准。** 一个门由请求者向自己签发，**永远拒绝不了**——它不是许可，是回声。这违反 VISION 原则 3「身体可建议，不决策」。

**而且它反转了。** `suggested_mode` 无复杂度状态时按查询**长度**回退，于是：

| 查询 | 身体建议 | 旧许可 | 正确 |
|:---|:---|:---|:---|
| 短 +「探索」 | Skilled（按长度） | **false** ✗ | true |
| 长、无探索词 | Imagination（按长度） | **true** ✗ | false |

**人类明说"探索"，反而是唯一拿不到想象的情况**；而一个长的技术性问题，仅仅因为长，就自动获得了想象许可。

**⇒ 同一份 `explore_keywords` 早已被 `derive_budget_tier` 信任**（`src/config.rs`），只是没接到许可上。

**⇒ 为什么这个缺陷能活下来**：许可的字面量内联在 `query()` 里，而 `query()` 需要活的 gRPC 通道——**只能靠读，不能靠跑**。

## 2. 决策

### D1｜许可来自人类措辞，身体只转达

新增纯函数 `exploratory_intent(query, cfg) -> bool`，命中 `explore_keywords` 即视为人类要求探索。`allow_imagination` 取它的值。

**⇒ 词表单一来源**（DNA 原则 11）：`exploratory_intent` 与 `derive_budget_tier` 消费同一份 `explore_keywords`，不另立第二份。

### D2｜请求构造抽为纯函数，使接线可测

`build_query_request(...)` 从 `query()` 内联段抽出，`suggested_mode` 由调用方推导后**显式传入**——以便同一份建议既进请求、也进证轨（`MindProvenance`），**不会出现两个来源**。

**⇒ 这是本 ADR 的重点，不是顺带的**：抽出来之前，"谁决定什么"只能靠阅读检查；抽出来之后它可以被断言。**一个只能靠读来验证的接线，就是会被读漏的接线。**

**⇒ 代价**：`build_query_request` / `exploratory_intent` / `derive_suggested_mode` / `derive_budget_tier` 成为公开 API。断言随之移入 `tests/mind_grant_policy.rs`——`src/` 受行数棘轮约束，`tests/` 不受。

### D3｜不因"查询长"而自我授权

身体的启发式仍可**建议**想象，但**不再据此授予许可**。若 Mind 建议想象而人类未许可，Mind 侧按既有规则回落 Anchor 并给出理由（"Imagination not allowed, falling back to Anchor"）——**这个回落因此第一次真的可能发生**。

## 3. 理由

1. **许可的本质是"可以被拒绝"。** 向自己签发的许可无法被拒绝，因此不是许可。
2. **人类的措辞是现成的、已被信任的信号。** 预算层级已经在用它；许可接同一份，不增概念、不增配置项。
3. **修复的是反转本身。** 短查询说「探索」必须能拿到想象，长的技术问题不得因为长而拿到。

## 4. 影响

| 方面 | 影响 |
|:---|:---|
| 长技术性查询 | 不再自动获得想象许可（**行为变化，有意**） |
| 短探索性查询 | 第一次能获得想象许可 |
| 公开 API | 四个策略函数公开 |
| 行数预算 | `mind.rs` 363 → 395，走 fix_window（k_id `K-045, K-059`） |

## 5. 变异证据

把 `allow_imagination` 改回 `suggested_mode == CognitiveMode::Imagination`，`tests/mind_grant_policy.rs::imagination_grant_comes_from_the_human_not_from_the_bodys_suggestion` **两个方向都红**——短探索查询（期望 true，实得 false）与长技术查询（期望 false，实得 true）。实测红。

**⇒ 记录一次被事实纠正的断言**：该测试第一版假设 `long_query()` 这个既有夹具不授权想象，**实测失败**——夹具含「未知」，而「未知」正是一个 `explore_keywords`，它**合法地**授权了想象。**是断言错了，不是代码错了。** 现改用不含探索词的长技术查询，并把这一条留在注释里。

## 6. 状态

**Active**（2026-09-20）。
