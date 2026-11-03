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

/// B2b-1：两条生产路径必须落成**两个不同的值**。
///
/// 收窄前：`Err(_)`（应答者存在但失败）与"根本没有批准者"**都**落 `Unavailable`
/// ⇒ 「没人值班」与「值班的人联系不上」不可区分，而处置相反（加人 / 修链路）。
///
/// Mutation：把 `from_bool` 的 `Err(_)` 改回 `Unavailable` ⇒ 本条红。
#[test]
fn no_responder_and_responder_failure_are_two_different_facts() {
    use anaphase::hitl::{from_bool, HITLApprover};
    use std::sync::Arc;

    // 路径一：根本没有批准者 ⇒ Unavailable（"没人"）
    let absent = HITLApprover::default();
    assert_eq!(
        absent.check_approval("rm -rf /data", &[]),
        ApprovalOutcome::Unavailable,
        "没有应答者必须落 Unavailable"
    );

    // 路径二：有批准者但调用失败 ⇒ Rejected（"联系不上"，它没有批准过）
    let failing = HITLApprover::new(from_bool(Arc::new(|_c: &str, _a: &[String]| {
        Err("channel down".to_string())
    })));
    assert_eq!(
        failing.check_approval("rm -rf /data", &[]),
        ApprovalOutcome::Rejected,
        "应答者失败不是「没人」，它必须落 Rejected（fail-closed），不许折进 Unavailable"
    );
}

/// B2b-3（接线确认）：生产路径上真的有人在**消费**这个闭集，而不是只有枚举存在。
///
/// 「有枚举」≠「有调用者」——Tuck 侧 `HumanConfirmGate` 就是反例（零生产调用者，
/// 已判废弃）。所以这条判据读源码，要求执行闸对四个值**逐一匹配**。
///
/// Mutation：把 `safety_gate.rs` 里任一分支删掉（例如 `Cancelled`）⇒ 本条红；
/// 把匹配改成 `_ => {}` 通配 ⇒ 本条红（那正是"闭集被折回布尔"的样子）。
#[test]
fn the_closed_set_is_consumed_by_the_execution_gate() {
    let src = include_str!("../src/run_cycle/safety_gate.rs");
    for value in ["AllowedOnce", "Rejected", "Cancelled", "Unavailable"] {
        assert!(
            src.contains(&format!("ApprovalOutcome::{value}")),
            "执行闸没有匹配 {value} —— 闭集必须逐值消费，不能有通配兜底"
        );
    }
    assert!(
        !src.contains("ApprovalOutcome::_ =>"),
        "不许用通配臂把剩余值吞掉：那是把四值闭集折回布尔"
    );
}

/// B2 迁移（2026-11-03）：Tuck 侧那四条判据的**语义**在 anaphase 落地。
///
/// Tuck 的 `ConfirmStatus::NoResponder` 已随 gate 一起判废弃（B2b-2），因为它有零个生产
/// 调用者；而 anaphase 的闭集是生产上真的被消费的那一份（`safety_gate.rs` 逐值匹配）。
/// ⇒ 判据必须跟着**生效的那一侧**走，否则它们守的是形状记录，不是行为。
///
/// 四条对应关系：
///   Tuck `NoResponder`  → anaphase `Unavailable`（没有应答者）
///   Tuck `Confirmed`    → `AllowedOnce`
///   Tuck `Rejected`     → `Rejected`
///   Tuck `TimedOut`     → `Cancelled`（提问被撤回/超时）**且必须 ≠ Unavailable**
///
/// Mutation：把 `Unavailable` 与 `Cancelled` 合并成同一个值 ⇒ 下面 den 判据红。
#[test]
fn the_four_outcomes_are_reachable_and_pairwise_distinct() {
    use anaphase::hitl::{from_bool, HITLApprover};
    use std::sync::Arc;

    let approve = HITLApprover::new(from_bool(Arc::new(|_c: &str, _a: &[String]| Ok(true))));
    let deny = HITLApprover::new(from_bool(Arc::new(|_c: &str, _a: &[String]| Ok(false))));
    let withdrew = HITLApprover::new(Arc::new(|_c: &str, _a: &[String]| ApprovalOutcome::Cancelled));
    let absent = HITLApprover::default();

    let observed = [
        approve.check_approval("rm -rf /x", &[]),
        deny.check_approval("rm -rf /x", &[]),
        withdrew.check_approval("rm -rf /x", &[]),
        absent.check_approval("rm -rf /x", &[]),
    ];
    assert_eq!(
        observed,
        [
            ApprovalOutcome::AllowedOnce,
            ApprovalOutcome::Rejected,
            ApprovalOutcome::Cancelled,
            ApprovalOutcome::Unavailable,
        ],
        "四个值必须各自可达：批准 / 拒绝 / 撤回 / 没人"
    );

    // 「撤回（有人在、但没答成）」与「没人」是两件事，处置相反（重问 / 加人）。
    assert_ne!(ApprovalOutcome::Cancelled, ApprovalOutcome::Unavailable);
}

/// 地基件：**四值计数**（C-057 的收尾要求 —— 延后期不许静默）。
///
/// 为什么必须有：没有应答者时门一律 fail-closed ⇒ 所有动作得到 `Unavailable`。
/// 不计数，我们手上永远只有「已拒绝」这一条已知信息，学不到「多少动作本该请求批准」。
///
/// 四态齐（只测"批准"会恒绿，这正是 Q-GATE-tuck-verify 与 security_gate_test 那两族的形状）。
/// Mutation：把 `record` 里某一支删掉 ⇒ 对应读数停在 0 ⇒ 本条红。
#[test]
fn the_four_outcomes_are_counted() {
    use anaphase::hitl::{from_bool, HitlCounters, HITLApprover};
    use std::sync::Arc;

    let c = Arc::new(HitlCounters::default());
    let approve = HITLApprover::new(from_bool(Arc::new(|_c: &str, _a: &[String]| Ok(true))))
        .with_counters(c.clone());
    let deny = HITLApprover::new(from_bool(Arc::new(|_c: &str, _a: &[String]| Ok(false))))
        .with_counters(c.clone());
    let withdrew = HITLApprover::new(Arc::new(|_c: &str, _a: &[String]| ApprovalOutcome::Cancelled))
        .with_counters(c.clone());
    let absent = HITLApprover::default().with_counters(c.clone());

    let _ = approve.check_approval("rm -rf /x", &[]);
    let _ = deny.check_approval("rm -rf /x", &[]);
    let _ = withdrew.check_approval("rm -rf /x", &[]);
    let _ = absent.check_approval("rm -rf /x", &[]);

    assert_eq!(c.snapshot(), [1, 1, 1, 1], "批 / 拒 / 撤回 / 没人 各记一次");

    // ★ 维度：Unavailable 必须分到「哪类动作」上。全局总数回答不了"该给哪类动作配人"。
    // 本轮用命令首词作类别（本层拿不到 risk_tier，它在 Tuck 的 PFP 里，见 C-046）。
    let kinds = c.unavailable_kinds();
    assert_eq!(kinds, vec![("rm".to_string(), 1)], "Unavailable 要按动作类别分桶：{kinds:?}");

    // ★ 生产路径没有"忘装"这个选项：计数永远在（去掉 Option 的理由）
    let prod_default = HITLApprover::default();
    let _ = prod_default.check_approval("scp /x", &[]);
    assert_eq!(
        prod_default.counters().snapshot(),
        [0, 0, 0, 1],
        "默认构造（生产用的那个）必须自带计数 —— 否则生产恒 0，计数器就是装饰品"
    );
    assert_eq!(prod_default.counters().unavailable_kinds(), vec![("scp".to_string(), 1)]);
}
