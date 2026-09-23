//! A5 (K-118): 审批结局是四值闭集，且**只有** `AllowedOnce` 放行。
//!
//! 放在 `tests/` 而不是 `src/hitl.rs` 的测试模块里：`src/` 的文件受行数棘轮约束、
//! `tests/` 不受 —— 而这条断言的价值不在它住在哪，在于它**能失败**。实测过：
//! 把 `Unavailable` 加进 `allows_execution` 的匹配（fail-open），下面两条同时红
//! （`exactly one outcome may grant: [AllowedOnce, Unavailable]`，计数 2 != 1）。

use anaphase::hitl::{ApprovalOutcome, WaitMode};

/// 闭集是**穷尽**的，且恰好一值放行。
///
/// 枚举进同一个数组，是为了让"新增一个值却忘了想它是否放行"变红：新增值会让
/// 计数仍为 1（若它不放行）或变成 2（若它放行），后者立刻被抓。
#[test]
fn the_grant_set_is_exhaustive_and_singleton() {
    const ALL: [ApprovalOutcome; 4] = [
        ApprovalOutcome::AllowedOnce,
        ApprovalOutcome::Rejected,
        ApprovalOutcome::Cancelled,
        ApprovalOutcome::Unavailable,
    ];
    let grants: Vec<&ApprovalOutcome> = ALL.iter().filter(|o| o.allows_execution()).collect();
    assert_eq!(grants.len(), 1, "exactly one outcome may grant: {grants:?}");
    assert_eq!(*grants[0], ApprovalOutcome::AllowedOnce);
}

/// 控制：这条断言必须**有判别力**。
///
/// 用一个只在测试里存在的"fail-open 版"判据重演同一集合：它把 `Unavailable` 也当放行，
/// 计数必须变成 2。若两个计数相同，上面那条断言就区分不了"只有一个授权"与
/// "失败也被默许"，即它无法失败。
#[test]
fn the_control_would_catch_a_fail_open_change() {
    let buggy = |o: ApprovalOutcome| {
        matches!(o, ApprovalOutcome::AllowedOnce | ApprovalOutcome::Unavailable)
    };
    const ALL: [ApprovalOutcome; 4] = [
        ApprovalOutcome::AllowedOnce,
        ApprovalOutcome::Rejected,
        ApprovalOutcome::Cancelled,
        ApprovalOutcome::Unavailable,
    ];
    let good = ALL.iter().filter(|o| o.allows_execution()).count();
    let bad = ALL.iter().filter(|o| buggy(**o)).count();
    assert_eq!(good, 1);
    assert_eq!(
        bad, 2,
        "the buggy predicate must give a different count, otherwise the assertion \
         above cannot tell the two apart"
    );
    assert_ne!(good, bad);
}

/// 三种等待形态必须彼此**可区分**。
///
/// 这条断言不做"授权与等待正交"的检查 —— 那是**类型系统**保证的：
/// `allows_execution(self)` 只接受 `ApprovalOutcome`，`WaitMode` 在类型上进不去，
/// 写一条"循环三种模式再断言同一事实"的测试只会看起来在测而什么都没测。
/// 这里换成一件真会出错的事：三个变体被写成同一个值时（复制粘贴、或将来误加
/// `#[derive]` 之外的别名），"三种等待"就退化成一种而没人会发现。
#[test]
fn the_three_wait_modes_are_distinct() {
    let modes = [WaitMode::Silent, WaitMode::Announce, WaitMode::AwaitingHuman];
    for (i, a) in modes.iter().enumerate() {
        for (j, b) in modes.iter().enumerate() {
            assert_eq!(
                i == j,
                a == b,
                "WaitMode variants must be pairwise distinct: {a:?} vs {b:?}"
            );
        }
    }
}
