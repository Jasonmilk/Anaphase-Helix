//! ADR-0049 — 会话收敛的判据（状态 / 骨架 / 下钻）。
//!
//! 判据只有一条：**20 轮的链在第 21 轮应"更轻"而不是"更少"** —— 注入更短（骨架有界），
//! 且第 1 轮的事实**仍可取回**。只有前者没有后者就是截断。

use anaphase::session_events::convergence::*;

fn row(id: &str, st: PeriodStatus) -> SkeletonRow {
    SkeletonRow { id: id.to_string(), status: st, label: format!("label-{id}") }
}

fn chain(n: usize) -> Vec<SkeletonRow> {
    (0..n).map(|i| row(&format!("run-x-p{i:016x}"), PeriodStatus::Draft)).collect()
}

/// ① 有界：行数受 `max_lines` 约束，**且省略必须是具名的**（说得出"还有几条"和怎么取回）。
///
/// /// Mutation: 把省略行去掉（静默截断）⇒ 输出少一行且不含 `!body` ⇒ 红。
#[test]
fn skeleton_is_bounded_and_the_elision_is_named() {
    let out = skeleton(&chain(100), 5);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 6, "5 rows + 1 named elision line: {lines:?}");
    assert!(lines[0].contains("95 earlier round(s) hidden"), "{:?}", lines[0]);
    assert!(lines[0].contains("!body"), "省略必须给出取回路径: {:?}", lines[0]);
    assert!(
        lines.iter().skip(1).all(|l| l.contains("[draft]")),
        "剩下的是轮次行: {lines:?}"
    );
}

/// ② 保留的是**最新**的（活动区在尾部），不是最旧的。
///
/// /// Mutation: `rows[hidden..]` 改成 `rows[..max_lines]` ⇒ 留下的是最旧的 5 条 ⇒ 红。
#[test]
fn skeleton_keeps_the_newest_rounds() {
    let rows = chain(10);
    let out = skeleton(&rows, 3);
    let kept: Vec<&str> = out
        .lines()
        .filter(|l| !l.contains("hidden"))
        .map(|l| l.split_whitespace().nth(1).unwrap())
        .collect();
    let expect: Vec<&str> = rows[7..].iter().map(|r| r.id.as_str()).collect();
    assert_eq!(kept, expect, "must keep the ACTIVE zone (newest), got {kept:?}");
}

/// ③ 单轮链 / 关闭 / 空输入都有**具名**结果，不靠"看起来没问题"。
#[test]
fn skeleton_degenerate_inputs_are_named() {
    assert_eq!(skeleton(&[], 5), "", "空链 -> 空串，不是 panic");
    assert_eq!(skeleton(&chain(3), 0), "", "0 行预算 -> 空串");
    let one = skeleton(&chain(1), 5);
    assert_eq!(one.lines().count(), 1, "单轮不产生省略行: {one:?}");
}

/// 状态：存储值优先；否则龄期；`enabled=false` 整个机制关掉（用户的开关）。
///
/// /// Mutation: 把 `if let Some(st) = stored { return (st, Stored) }` 删掉 ⇒ 存储值被龄期
/// /// 覆盖 ⇒ 第一条断言红。
#[test]
fn status_stored_beats_age_and_the_switch_disables_everything() {
    let old = "2020-01-01T00:00:00Z";
    let cutoff = converge_cutoff(1_700_000_000, 7);
    let (st, src) = resolve_status(Some(PeriodStatus::Draft), old, &cutoff, true);
    assert_eq!((st, src), (PeriodStatus::Draft, StatusSource::Stored), "存储值必须优先");
    let (st, src) = resolve_status(None, old, &cutoff, true);
    assert_eq!((st, src), (PeriodStatus::Converged, StatusSource::DerivedFromAge));
    let (st, src) = resolve_status(None, old, &cutoff, false);
    assert_eq!((st, src), (PeriodStatus::Draft, StatusSource::Default), "关掉 = 不隐性");
    let fresh = "2099-01-01T00:00:00Z";
    let (st, _) = resolve_status(None, fresh, &cutoff, true);
    assert_eq!(st, PeriodStatus::Draft, "近期不隐性");
}

/// 血缘：只走 `parent`，到根为止；环不许死循环。
#[test]
fn chain_walks_lineage_only_and_survives_a_cycle() {
    use anaphase::session_events::PeriodSummary;
    let mk = |id: &str, parent: Option<&str>| PeriodSummary {
        period_id: id.to_string(),
        job_id: "j".into(),
        first_ts: "2026-01-01T00:00:00Z".into(),
        last_ts: "2026-01-01T00:00:00Z".into(),
        count: 1,
        preview: "p".into(),
        reply: "r".into(),
        parent: parent.map(|s| s.to_string()),
        conversation_id: None,
        model: None,
        name: None,
    };
    let list = vec![mk("c", Some("b")), mk("b", Some("a")), mk("a", None)];
    let path: Vec<&str> = chain_of(&list, "c").iter().map(|p| p.period_id.as_str()).collect();
    assert_eq!(path, vec!["a", "b", "c"], "root first, leaf last");
    let cyc = vec![mk("y", Some("z")), mk("z", Some("y"))];
    assert_eq!(chain_of(&cyc, "y").len(), 2, "环必须终止，不做死循环");
}
