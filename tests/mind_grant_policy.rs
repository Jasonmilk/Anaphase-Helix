//! P0① 授权缝的政策测试 —— 谁决定 `suggested_mode`，谁决定 `allow_imagination`。
//!
//! 这些断言住在 `tests/` 而不是 `src/adapters/mind.rs` 的 `#[cfg(test)]` 里：`src/`
//! 受行数棘轮约束、`tests/` 不在受治理目录内，所以同样的断言不必用一次额度调整来换。
//! `build_query_request` 从 `query()` 里抽出来，正是为了让这条接线**可跑**而不只是
//! **可读** —— 内联时那句自我签发的 `allow_imagination` 只能靠读才能发现。

use anaphase::adapters::mind::{
    build_query_request, derive_budget_tier, derive_suggested_mode, exploratory_intent,
};
use anaphase::config::MindConfig;
use anaphase::helix_mind_api::{BudgetTier, CognitiveMode};

/// The grant must come from the human's wording, never from the body's own
/// suggestion — and both directions are asserted, because the old expression
/// was wrong in both:
///
/// | query | body suggests | old grant | correct |
/// |---|---|---|---|
/// | short + 「探索」 | Skilled (by length) | **false** ✗ | true |
/// | long, no explore word | Imagination (by length) | **true** ✗ | false |
///
/// Mutation probe: put `allow_imagination: suggested_mode ==
/// CognitiveMode::Imagination` back into `build_query_request` and both
/// assertions fail. That is exactly why the request builder was extracted out
/// of `query()`: inline, it could only be checked by reading.
#[test]
fn imagination_grant_comes_from_the_human_not_from_the_bodys_suggestion() {
    let cfg = MindConfig::default();
    let tp = || "00-00000000000000000000000000000000-0000000000000000-01".to_string();

    // Short + exploratory: the length heuristic lands on Skilled, so under the
    // old rule the human's own request to explore was the one case that could
    // never be granted.
    let short = "探索一下";
    let short_suggestion = derive_suggested_mode(short, 0, &cfg);
    assert_eq!(
        short_suggestion,
        CognitiveMode::Skilled,
        "premise: a short query falls back to Skilled by length"
    );
    let req = build_query_request(short, false, short_suggestion, 0.2, tp(), &cfg);
    assert_eq!(req.suggested_mode, CognitiveMode::Skilled as i32);
    assert!(
        req.allow_imagination,
        "人类明说「探索」，身体必须转达这份授权，哪怕它自己建议的是 Skilled"
    );

    // Long, no exploratory wording: the body's heuristic reaches Imagination by
    // length, and must not thereby authorise itself.
    //
    // Note this cannot use `long_query()`: that fixture contains 「未知」, which
    // IS one of `explore_keywords`, so it legitimately grants the permission.
    // The first version of this test assumed otherwise and failed — the
    // assertion was wrong, not the code.
    let technical = "请检查这个模块的并发安全性以及锁的粒度是否合理并指出超时重试与幂等性方面的缺陷以及错误处理路径上的问题";
    assert!(
        !exploratory_intent(technical, &cfg),
        "premise: this query must contain none of the explore vocabulary"
    );
    let long_suggestion = derive_suggested_mode(technical, 0, &cfg);
    assert_eq!(
        long_suggestion,
        CognitiveMode::Imagination,
        "premise: a long query falls back to Imagination by length"
    );
    let req = build_query_request(technical, false, long_suggestion, 0.2, tp(), &cfg);
    assert_eq!(req.suggested_mode, CognitiveMode::Imagination as i32);
    assert!(
        !req.allow_imagination,
        "查询长不等于人类要求探索；身体不得自我授权想象"
    );
}

/// The explore vocabulary has one source (DNA 原则 11): `explore_keywords`.
#[test]
fn exploratory_intent_is_the_single_source_of_the_explore_vocabulary() {
    let cfg = MindConfig::default();
    assert!(!exploratory_intent("继续上次的部署", &cfg));
    assert!(!exploratory_intent("", &cfg));
    for q in ["探索这个方向", "research the tradeoffs", "imagine another layout", "头脑风暴"] {
        assert!(exploratory_intent(q, &cfg), "{q} must read as exploratory");
    }
    // The predicate is shared, but `derive_budget_tier` checks *length* first,
    // so a short exploratory query still comes back Endogenous: 「头脑风暴」 is
    // 4 chars ≤ `short_query`, and the explore branch is never reached. This is
    // a real divergence from the grant above (which has no length
    // precondition), recorded rather than papered over — raising the tier for
    // short exploratory queries is a separate decision with its own cost.
    assert_eq!(derive_budget_tier("头脑风暴", 0.2, &cfg), BudgetTier::Endogenous);
    assert!(exploratory_intent("头脑风暴", &cfg));
    assert_eq!(
        derive_budget_tier("探索这个方向的全部可能性", 0.2, &cfg),
        BudgetTier::ExogenousRequired
    );
}
