//! ADR-0049 — 会话收敛的判据（状态 / 骨架 / **gist** / 下钻）。
//!
//! 骨架的形状是 `lodestone-spec:ADR-0002` D5：**最近 N 行原文 + 覆盖全部沉底轮次的 gist**。
//! 判据只有一条：**21 轮链，第 1 轮埋独有事实，第 21 轮追问 —— 人不做任何事，AI 自己答出。**
//! 所以下面必须证明事实来自 **gist**，而不是"恰好还在最近 N 行里"。

use anaphase::session_events::convergence::*;

fn row(id: &str, st: PeriodStatus, gist: Option<&str>) -> SkeletonRow {
    SkeletonRow {
        id: id.to_string(),
        status: st,
        label: format!("label-{id}"),
        gist: gist.map(|g| g.to_string()),
        rejected: None,
    }
}

fn chain(n: usize) -> Vec<SkeletonRow> {
    (0..n)
        .map(|i| row(&format!("run-x-p{i:016x}"), PeriodStatus::Draft, None))
        .collect()
}

/// ① 有界：行数受 `max_lines` 约束，**且沉底区必须给出去处**（具名，不静默丢）。
///
/// /// Mutation: 把 gist 行去掉（静默截断）⇒ 输出少一行 ⇒ 红。
#[test]
fn skeleton_is_bounded_and_the_gist_is_named() {
    let out = skeleton(&chain(100), 5, 400);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 7, "gist 行 + gist 正文 + 5 行原文: {lines:?}");
    assert!(lines[0].contains("gist of 95 earlier round(s)"), "{:?}", lines[0]);
    assert!(lines[0].contains("!body"), "必须给出取回路径: {:?}", lines[0]);
    assert!(
        lines.iter().skip(2).all(|l| l.contains("[draft]")),
        "剩下的是最近 N 行原文: {lines:?}"
    );
}

/// ② 保留的是**最新**的（活动区在尾部），不是最旧的。
///
/// /// Mutation: `rows[hidden..]` 改成 `rows[..max_lines]` ⇒ 留下最旧的 ⇒ 红。
#[test]
fn skeleton_keeps_the_newest_rounds() {
    let rows = chain(10);
    let out = skeleton(&rows, 3, 400);
    let kept: Vec<&str> = out
        .lines()
        .filter(|l| l.starts_with("- run-"))
        .map(|l| l.split_whitespace().nth(1).unwrap())
        .collect();
    let expect: Vec<&str> = rows[7..].iter().map(|r| r.id.as_str()).collect();
    assert_eq!(kept, expect, "must keep the ACTIVE zone (newest), got {kept:?}");
}

/// ★ ③ **本轮的判据**：沉底轮次的事实必须**从 gist 取出**，不是"恰好还在最近 N 行里"。
///
/// 构造：10 轮，`max_lines = 3` ⇒ 前 7 轮**必然沉底**；第 1 轮的 gist 里埋 `ZEPHYR-7421`。
/// 断言：该事实出现在骨架里，**且第 1 轮的那行原文不在骨架里** —— 两条同时成立才说明来自 gist。
///
/// /// Mutation: `skeleton` 里不写 gist 正文（只留标题行）⇒ 事实消失 ⇒ 红。
#[test]
fn a_sunk_round_is_still_known_through_its_gist() {
    let mut rows = chain(10);
    rows[0] = row(&rows[0].id.clone(), PeriodStatus::Converged, Some("我的狗叫 ZEPHYR-7421"));
    let sunk_id = rows[0].id.clone();
    let out = skeleton(&rows, 3, 400);

    assert!(
        out.contains("ZEPHYR-7421"),
        "沉底轮次的事实必须仍在骨架里（经 gist）: {out:?}"
    );
    assert!(
        !out.contains(&format!("- {sunk_id} [")),
        "第 1 轮必须真的沉底（否则测的是原文可见，不是 gist）: {out:?}"
    );
}

/// ④ gist 是**一块有界文本**，不是"每轮一行"（那只会把线性增长搬进 gist）。
/// 覆盖不下时，**余量必须具名**。
#[test]
fn gist_is_one_bounded_blob_and_the_remainder_is_named() {
    let rows: Vec<SkeletonRow> = (0..50)
        .map(|i| row(&format!("run-x-p{i:016x}"), PeriodStatus::Draft, Some(&"x".repeat(40))))
        .collect();
    let out = skeleton(&rows, 2, 200);
    assert!(out.contains("more, `!lodes`"), "余量必须具名: {out:?}");
    let gist_line = out.lines().nth(1).unwrap();
    assert!(
        gist_line.chars().count() <= 200 + 32,
        "gist 正文必须受字符预算约束，实际 {}",
        gist_line.chars().count()
    );
}

/// ⑤ 状态：存储值优先；否则龄期；`enabled=false` 整个机制关掉。
///
/// /// Mutation: 删掉 `if let Some(st) = stored { return (st, Stored) }` ⇒ 存储值被龄期覆盖 ⇒ 红。
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
    assert_eq!(resolve_status(None, fresh, &cutoff, true).0, PeriodStatus::Draft);
}

/// ⑥ 退化输入都有具名结果，不靠"看起来没问题"。
#[test]
fn degenerate_inputs_are_named() {
    assert_eq!(skeleton(&[], 5, 400), "", "空链 -> 空串");
    assert_eq!(skeleton(&chain(3), 0, 400), "", "0 行预算 -> 空串");
    assert_eq!(skeleton(&chain(1), 5, 400).lines().count(), 1, "单轮无 gist 行");
}

/// ⑦ 血缘：只走 `parent`，到根为止；环不许死循环。
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
    assert_eq!(chain_of(&cyc, "y").len(), 2, "环必须终止");
}

// ───────────────────────── 一次具名驳回（不是模型判断）─────────────────────────

fn tmp(tag: &str) -> std::path::PathBuf {
    // pid + 原子序号：同进程内两次同参调用也必须不同（教训：裸 temp_dir 会撞）
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let d = std::env::temp_dir().join(format!(
        "convg-{}-{}-{}",
        tag,
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn sum(id: &str, parent: Option<&str>, preview: &str) -> anaphase::session_events::PeriodSummary {
    anaphase::session_events::PeriodSummary {
        period_id: id.into(),
        job_id: "j".into(),
        first_ts: "2026-01-01T00:00:00Z".into(),
        last_ts: "2026-01-01T00:00:00Z".into(),
        count: 1,
        preview: preview.into(),
        reply: "r".into(),
        parent: parent.map(|s| s.to_string()),
        conversation_id: None,
        model: None,
        name: None,
    }
}

/// 写一个**最小但真实**的 period 事件文件 —— `answer_lodes` 读的是 store，
/// 不是内存列表；用夹具替代它就会测到另一个东西。
fn write_period(dir: &std::path::Path, id: &str, parent: Option<&str>, text: &str) {
    let t = "2026-01-01T00:00:00Z";
    let mut rows = vec![
        format!(r#"{{"type":"turn/start","period_id":"{id}","job_id":"j","seq":0,"time":"{t}","data":{{}}}}"#),
        format!(r#"{{"type":"user/message","period_id":"{id}","job_id":"j","seq":1,"time":"{t}","data":{{"text":"{text}"}}}}"#),
    ];
    if let Some(par) = parent {
        rows.push(format!(r#"{{"type":"context/inject","period_id":"{id}","job_id":"j","seq":2,"time":"{t}","data":{{"resume_from":"{par}"}}}}"#));
    }
    std::fs::write(
        dir.join(format!("{id}.events.jsonl")),
        rows.join("\n") + "\n",
    )
    .unwrap();
}

/// ★ 驳回的**唯一效果**：这段内容不进注入。四条同时断言 ——
/// ① 注入里没有它 ② 它的行仍在 `!lodes` 的读数里（未被删）③ 理由被记下 ④ `!body` 仍可达。
///
/// /// Mutation: `skeleton` 里去掉 `filter(|r| r.rejected.is_none())` ⇒ 被驳回的内容回到注入 ⇒ 红。
#[test]
fn a_rejection_leaves_the_injection_but_stays_readable() {
    let dir = tmp("reject");
    let a = sum("run-x-p0000000000000001", None, "A 段：已定型");
    let b = sum("run-x-p0000000000000002", Some("run-x-p0000000000000001"), "B 段：另开分支");

    write_period(&dir, &a.period_id, None, "A 段：已定型");
    write_period(&dir, &b.period_id, Some(&a.period_id), "B 段：另开分支");
    write_rejection(&dir, &a.period_id, "无").unwrap();
    write_rejection(&dir, &b.period_id, "方向不对，另开一支").unwrap();
    // A 已定型：显式状态（本 ADR §6.2.2 一直留着的那个入口）
    write_state(&dir, &a.period_id, PeriodStatus::Converged).unwrap();
    // A 应当留在注入里，B 不应当。
    std::fs::remove_file(reject_path(&dir, &a.period_id)).unwrap();

    let list = vec![a.clone(), b.clone()];
    let refs: Vec<&anaphase::session_events::PeriodSummary> = list.iter().collect();
    let cfg = ConvergenceConfig::default();
    let rows = skeleton_rows(&dir, &refs, &cfg, 1_700_000_000);
    assert_eq!(rows.len(), 2);
    assert!(rows[0].rejected.is_none(), "A 未被驳回");
    assert_eq!(rows[1].rejected.as_deref(), Some("方向不对，另开一支"), "③ 理由必须记下");

    let injected = skeleton(&rows, 12, 400);
    assert!(injected.contains("A 段：已定型"), "① A 必须在注入里: {injected:?}");
    assert!(
        !injected.contains("B 段：另开分支") && !injected.contains(&b.period_id),
        "① B 的**内容与行**都不得进注入: {injected:?}"
    );

    // ② 读数（!lodes）仍列出它，且带理由与取回路径 —— 否则"驳回"等于"看不见"，就是删除。
    let listed = answer_lodes(&dir, Some(&b.period_id), &cfg, 1_700_000_000);
    assert!(listed.contains("rejected branch"), "② 读数必须列出被驳回的分支: {listed:?}");
    assert!(listed.contains("方向不对，另开一支"), "② 理由必须可读: {listed:?}");
    assert!(listed.contains(&format!("!body {}", b.period_id)), "④ 必须给取回路径: {listed:?}");

    let _ = std::fs::remove_dir_all(&dir);
}

/// 空理由写**具名默认**：「没有理由」与「没记」必须是两种可分辨的读数。
#[test]
fn an_empty_reason_is_recorded_as_a_named_absence() {
    let dir = tmp("reject-empty");
    write_rejection(&dir, "run-x-p0000000000000003", "   ").unwrap();
    let r = read_rejection(&dir, "run-x-p0000000000000003").unwrap();
    assert!(r.contains("未说明"), "空理由不得变成空文件: {r:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// `!reject` 的回答必须说清"字节未动 + 不进注入 + 仍可取回"（可审查四问的落地）。
#[test]
fn the_reject_answer_states_all_three_facts() {
    let dir = tmp("reject-answer");
    let out = answer_reject(&dir, "run-x-p0000000000000004", "重复了");
    assert!(out.contains("字节未动"), "{out:?}");
    assert!(out.contains("不再包含"), "{out:?}");
    assert!(out.contains("!body run-x-p0000000000000004"), "{out:?}");
    let _ = std::fs::remove_dir_all(&dir);
}
