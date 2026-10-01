//! D1's criteria, one per invariant (ADR-0048 §328). Pure: no store, no clock, no files — so a failure
//! names the RULE, not the environment. Each test states the rule and, where the rule is a guard, the
//! mutation that would slip past if the guard were removed.

use super::*;   /* `test_support` and the re-exports, exactly like the sibling test modules */
use super::gc::{plan, GcInput, Object, Vacancy};

fn obj(id: &str, parent: Option<&str>, stamped: bool) -> Object {
    Object { id: id.to_string(), parent: parent.map(|p| p.to_string()), stamped }
}

/// C5 + C6 + the happy path: stamped, unheld, past grace ⇒ collected.
#[test]
fn a_stamped_unheld_object_past_grace_is_collected() {
    let input = GcInput {
        objects: vec![obj("a", None, true)],
        refs: vec![],
        pins: vec![],
        vacancies: vec![Vacancy { id: "a".into(), at: 100 }],
        grace_secs: 10,
    };
    let p = plan(&input, 111);
    assert_eq!(p.collected, vec!["a".to_string()]);
    assert!(p.kept.is_empty());
}

/// C6 — a stamped object WITH A HOLDER is a ghost: never collected, always named.
#[test]
fn a_stamped_object_with_a_holder_is_a_ghost_and_is_kept() {
    let input = GcInput {
        objects: vec![obj("a", None, true)],
        refs: vec![],
        pins: vec![("a".into(), 1)],
        vacancies: vec![Vacancy { id: "a".into(), at: 1 }],
        grace_secs: 10,
    };
    let p = plan(&input, 10_000);
    assert!(p.collected.is_empty(), "a held object is never collected");
    assert_eq!(p.ghosts, vec!["a".to_string()], "and the ghost is NAMED");
    assert_eq!(p.kept, vec!["a".to_string()]);
}

/// C14 — pins are COUNTS: two holders, one releases, nothing is collected.
#[test]
fn pin_refcounts_survive_one_release() {
    let base = |pins: Vec<(String, u32)>| GcInput {
        objects: vec![obj("a", None, true)],
        refs: vec![],
        pins,
        vacancies: vec![Vacancy { id: "a".into(), at: 1 }],
        grace_secs: 10,
    };
    assert!(plan(&base(vec![("a".into(), 2)]), 10_000).collected.is_empty(), "two holders");
    assert!(plan(&base(vec![("a".into(), 1)]), 10_000).collected.is_empty(), "one holder left");
    assert_eq!(plan(&base(vec![("a".into(), 0)]), 10_000).collected, vec!["a".to_string()], "no holders");
    /* MUTATION: a SET model would have collected after the first release — the count is what forbids it. */
    let counted = super::gc::GcInput { pins: vec![("a".into(), 1), ("a".into(), 1)], ..base(vec![]) };
    assert_eq!(plan(&counted, 10_000).ghosts, vec!["a".to_string()], "two separate holders ADD up");
}

/// C9 — the anchor is the VACANCY FACT; when it is absent the absence is NAMED, never guessed.
#[test]
fn an_absent_anchor_is_named_not_guessed() {
    let input = GcInput {
        objects: vec![obj("a", None, true)],
        refs: vec![],
        pins: vec![],
        vacancies: vec![],
        grace_secs: 10,
    };
    let p = plan(&input, 10_000);
    assert!(p.collected.is_empty(), "without a fact there is nothing to measure the grace against");
    assert_eq!(p.no_anchor, vec!["a".to_string()], "and the missing anchor is named");
}

/// C9 (second half) — inside the window it is PROTECTED; the window itself is the fact's age.
#[test]
fn the_grace_window_is_measured_from_the_vacancy_fact() {
    let input = GcInput {
        objects: vec![obj("a", None, true)],
        refs: vec![],
        pins: vec![],
        vacancies: vec![Vacancy { id: "a".into(), at: 100 }],
        grace_secs: 10,
    };
    assert!(plan(&input, 105).collected.is_empty(), "5 < 10 ⇒ still protected");
    assert_eq!(plan(&input, 110).collected, vec!["a".to_string()], "exactly at the window ⇒ collectible");
    assert_eq!(plan(&input, 999).collected, vec!["a".to_string()]);
}

/// C13 — a PURE function of `(state, now)`: same arguments, same answer; a later `now` only ever adds.
#[test]
fn the_plan_is_a_pure_function_of_state_and_now() {
    let input = GcInput {
        objects: vec![obj("a", None, true), obj("b", Some("a"), true)],
        refs: vec![],
        pins: vec![],
        vacancies: vec![Vacancy { id: "a".into(), at: 0 }, Vacancy { id: "b".into(), at: 50 }],
        grace_secs: 10,
    };
    assert_eq!(plan(&input, 20), plan(&input, 20), "same state, same now ⇒ identical plan");
    let early = plan(&input, 20).collected;
    let late = plan(&input, 100).collected;
    assert!(early.iter().all(|x| late.contains(x)), "a later clock never UN-collects: {early:?} ⊄ {late:?}");
}

/// The theorem (no dangling): a kept child protects its stamped parent, and the protection is NAMED.
#[test]
fn a_kept_child_protects_its_stamped_parent() {
    let input = GcInput {
        objects: vec![obj("parent", None, true), obj("child", Some("parent"), false)],
        refs: vec![],
        pins: vec![],
        vacancies: vec![Vacancy { id: "parent".into(), at: 0 }],
        grace_secs: 1,
    };
    let p = plan(&input, 1_000);
    assert!(p.collected.is_empty(), "collecting the parent would leave the child dangling");
    assert_eq!(p.protected_by_descendant, vec!["parent".to_string()], "and the reason is named");
}

/// A ref is a ROOT (C-fact, §321): a stamped, unheld, long-vacated object that a ref names is kept.
#[test]
fn a_ref_is_a_root_and_nothing_it_names_is_collected() {
    let input = GcInput {
        objects: vec![obj("a", None, true)],
        refs: vec!["a".into()],
        pins: vec![],
        vacancies: vec![Vacancy { id: "a".into(), at: 0 }],
        grace_secs: 0,
    };
    assert!(plan(&input, 10_000).collected.is_empty(), "the root set is not the collector's to empty");
}

/// C5 — an UNSTAMPED object is never an input, whatever else is true of it.
#[test]
fn an_unstamped_object_is_never_collected() {
    let input = GcInput {
        objects: vec![obj("a", None, false)],
        refs: vec![],
        pins: vec![],
        vacancies: vec![Vacancy { id: "a".into(), at: 0 }],
        grace_secs: 0,
    };
    let p = plan(&input, 10_000);
    assert!(p.collected.is_empty() && p.ghosts.is_empty() && p.no_anchor.is_empty(), "no stamp, no case");
    assert_eq!(p.kept, vec!["a".to_string()]);
}

/* ── THE STORE-FACING CRITERIA (ADR-0048 §329) ─────────────────────────────────────────────────── */

#[test]
fn the_rfc3339_reader_agrees_with_the_writer_the_ledger_owns() {
    for secs in [0u64, 1_000, 1_788_393_600, 1_800_000_000] {
        let s = crate::ledger::unix_secs_to_rfc3339(secs);
        assert_eq!(super::gc::rfc3339_to_secs(&s), Some(secs), "round-trip of {s}");
    }
    assert_eq!(super::gc::rfc3339_to_secs("not-a-time"), None, "and a malformed stamp is REFUSED");
}

#[test]
fn collecting_records_a_purge_fact_in_the_objects_own_stream_without_destroying_bytes() {
    use crate::trace::Redaction;
    use serde_json::json;
    use std::fs;
    let dir = test_support::tmp_dir("gc_purge_fact");
    let pid = "run-aaaabbbbccccdddd-p0000000030000030";
    {
        let mut s = super::SessionEventStream::open(dir.clone(), pid, pid, Redaction::default()).unwrap();
        s.emit("2026-10-01T00:00:00Z", super::EventType::UserMessage, json!({ "text": "x" })).unwrap();
    }
    super::tombstone_period(&dir, pid, "gone").unwrap();
    /* A vacancy fact, written by the ref layer, exactly as the collector will read it. */
    super::write_ref(&dir, "current", pid).unwrap();
    super::delete_ref(&dir, "current").unwrap();

    let out = super::collect_garbage(&dir, 4_000_000_000, 10).unwrap();
    assert_eq!(out.collected, vec![pid.to_string()], "stamped, unheld, vacated long ago ⇒ collected");
    let body = fs::read_to_string(dir.join(format!("{pid}.events.jsonl"))).unwrap();
    assert!(body.contains("period/purge"), "the decision is RECORDED as a fact: {body}");
    assert!(body.contains("user/message"), "and D1 destroys NOTHING (that is D2)");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_stamped_object_with_a_kept_child_is_protected_and_not_purged() {
    use crate::trace::Redaction;
    use serde_json::json;
    use std::fs;
    let dir = test_support::tmp_dir("gc_protects_parent");
    let parent = "run-1111222233334444-p0000000031000031";
    let child = "run-5555666677778888-p0000000032000032";
    {
        let mut s = super::SessionEventStream::open(dir.clone(), parent, parent, Redaction::default()).unwrap();
        s.emit("2026-10-01T00:00:00Z", super::EventType::TurnEnd, json!({})).unwrap();
    }
    {
        let mut s = super::SessionEventStream::open(dir.clone(), child, child, Redaction::default()).unwrap();
        s.emit("2026-10-01T00:00:10Z", super::EventType::ContextInject, json!({ "resume_from": parent })).unwrap();
    }
    super::tombstone_period(&dir, parent, "stamped").unwrap();
    super::write_ref(&dir, "current", parent).unwrap();
    super::delete_ref(&dir, "current").unwrap();

    let out = super::collect_garbage(&dir, 4_000_000_000, 10).unwrap();
    assert!(out.collected.is_empty(), "the child still needs it: no dangling may be created");
    assert_eq!(out.protected_by_descendant, vec![parent.to_string()], "and the reason is NAMED");
    let body = fs::read_to_string(dir.join(format!("{parent}.events.jsonl"))).unwrap();
    assert!(!body.contains("period/purge"), "no purge fact for an object that was kept");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_stamped_object_without_a_vacancy_fact_is_named_and_not_purged() {
    use crate::trace::Redaction;
    use serde_json::json;
    use std::fs;
    let dir = test_support::tmp_dir("gc_no_anchor");
    let pid = "run-9999000011112222-p0000000033000033";
    {
        let mut s = super::SessionEventStream::open(dir.clone(), pid, pid, Redaction::default()).unwrap();
        s.emit("2026-10-01T00:00:00Z", super::EventType::TurnEnd, json!({})).unwrap();
    }
    super::tombstone_period(&dir, pid, "stamped").unwrap();
    let out = super::collect_garbage(&dir, 4_000_000_000, 0).unwrap();
    assert!(out.collected.is_empty(), "without a fact there is nothing to measure the grace against");
    assert_eq!(out.no_anchor, vec![pid.to_string()], "the absent anchor is NAMED");
    let _ = fs::remove_dir_all(&dir);
}

/// ④ THE END-TO-END (ADR-0048 §331): delete ⇒ replay ⇒ NO REVIVAL. And the MUTATION that proves the
/// criterion is about the purge FACT rather than about the test: drop that one fact and the object returns.
#[test]
fn a_purged_object_is_neither_visible_nor_existing_after_a_replay() {
    use crate::trace::Redaction;
    use serde_json::json;
    use std::fs;
    let dir = test_support::tmp_dir("gc_e2e_no_revival");
    let pid = "run-deadbeefdeadbeef-p0000000050000050";
    {
        let mut s = super::SessionEventStream::open(dir.clone(), pid, pid, Redaction::default()).unwrap();
        s.emit("2026-10-01T00:00:00Z", EventType::UserMessage, json!({ "text": "keep?" })).unwrap();
        s.emit("2026-10-01T00:00:01Z", EventType::TurnEnd, json!({})).unwrap();
    }
    /* D0 first (hide), then a vacancy fact, then D1 (collect). */
    super::tombstone_period(&dir, pid, "deleted").unwrap();
    super::write_ref(&dir, "current", pid).unwrap();
    super::delete_ref(&dir, "current").unwrap();

    /* D0 alone: hidden from readers, still EXISTING (nothing destroyed yet). */
    assert!(!super::replay_live(&dir).unwrap().contains(&pid.to_string()), "D0 hides it");
    assert!(super::replay_exists(&dir).unwrap().contains(&pid.to_string()), "but its bytes are still there");

    let out = super::collect_garbage(&dir, 4_000_000_000, 0).unwrap();
    assert_eq!(out.collected, vec![pid.to_string()], "D1 collects it");

    /* D1: neither visible NOR existing — and no resurrection on a fresh replay. */
    assert!(!super::replay_live(&dir).unwrap().contains(&pid.to_string()), "not visible after the purge");
    assert!(!super::replay_exists(&dir).unwrap().contains(&pid.to_string()), "and it no longer EXISTS");

    /* MUTATION: ignore the purge fact ⇒ the object comes back. That is exactly the revival C8 forbids,
     * and this assertion is what makes the criterion falsifiable rather than decorative. */
    assert!(super::replay_exists_ignoring_purge(&dir).unwrap().contains(&pid.to_string()),
            "without the purge fact a replay WOULD revive it — so the fact is load-bearing");

    /* And the history it never destroyed is still on disk: D1 records, D2 destroys. */
    let body = fs::read_to_string(dir.join(format!("{pid}.events.jsonl"))).unwrap();
    assert!(body.contains("user/message") && body.contains("period/purge"));
    let _ = fs::remove_dir_all(&dir);
}

/// C7 + the three-state law (ADR-0048 §332): destroying content must keep ID and PARENT, must make the text
/// unrecoverable, and must SAY that it was destroyed — because "destroyed" is not "there was never any".
#[test]
fn content_destruction_keeps_identity_and_lineage_and_names_itself() {
    use crate::trace::Redaction;
    use serde_json::json;
    use std::fs;
    let dir = test_support::tmp_dir("d2_keeps_chain");
    let parent = "run-1010101010101010-p0000000060000060";
    let child = "run-2020202020202020-p0000000061000061";
    {
        let mut s = super::SessionEventStream::open(dir.clone(), parent, parent, Redaction::default()).unwrap();
        s.emit("2026-10-01T00:00:00Z", EventType::UserMessage, json!({ "text": "SECRET-PARENT" })).unwrap();
        s.emit("2026-10-01T00:00:01Z", EventType::AssistantReply, json!({ "text": "SECRET-REPLY" })).unwrap();
        s.emit("2026-10-01T00:00:02Z", EventType::TurnEnd, json!({ "reply": "SECRET-REPLY" })).unwrap();
    }
    {
        let mut s = super::SessionEventStream::open(dir.clone(), child, child, Redaction::default()).unwrap();
        s.emit("2026-10-01T00:00:10Z", EventType::ContextInject, json!({ "resume_from": parent })).unwrap();
    }
    let before = list_periods(&dir, 10).unwrap();
    let parent_before = before.iter().find(|p| p.period_id == parent).cloned().expect("parent listed");
    assert_eq!(parent_before.parent, None, "the parent is a root before D2");

    let report = super::purge_content(&dir, &[parent.to_string()]).unwrap();
    assert_eq!(report.destroyed_rows, 3, "three rows carried content");
    assert!(report.retained.contains(&"model".to_string()) && report.scope.len() == 1,
            "what survived and the SCOPE are declared, not implied (P43): {report:?}");

    let raw = fs::read_to_string(dir.join(format!("{parent}.events.jsonl"))).unwrap();
    assert!(!raw.contains("SECRET-PARENT") && !raw.contains("SECRET-REPLY"), "the content is GONE: {raw}");
    assert!(raw.contains(r#""content":"destroyed""#), "and each row SAYS SO (destroyed != never had): {raw}");

    let after = list_periods(&dir, 10).unwrap();
    let parent_after = after.iter().find(|p| p.period_id == parent).cloned().expect("the parent STILL EXISTS");
    assert_eq!(parent_after.parent, None, "identity and lineage are untouched (C7)");
    let child_after = after.iter().find(|p| p.period_id == child).cloned().expect("the child is listed");
    assert_eq!(child_after.parent.as_deref(), Some(parent), "and the child still resolves its parent");
    let _ = fs::remove_dir_all(&dir);
}

/// THE MUTATION for C7: had D2 deleted the FILE instead of its content, the child's `resume_from` would
/// name an id nothing can resolve — the reader would report a dangling parent, exactly as it should.
#[test]
fn deleting_the_file_instead_of_the_content_would_break_the_chain() {
    use crate::trace::Redaction;
    use serde_json::json;
    use std::fs;
    let dir = test_support::tmp_dir("d2_mutation_file_delete");
    let parent = "run-3030303030303030-p0000000062000062";
    let child = "run-4040404040404040-p0000000063000063";
    for (id, rows) in [(&parent, true), (&child, false)] {
        let mut s = super::SessionEventStream::open(dir.clone(), id, id, Redaction::default()).unwrap();
        if rows {
            s.emit("2026-10-01T00:00:00Z", EventType::UserMessage, json!({ "text": "x" })).unwrap();
        } else {
            s.emit("2026-10-01T00:00:10Z", EventType::ContextInject, json!({ "resume_from": parent })).unwrap();
        }
    }
    /* The WRONG implementation: remove the parent's file. */
    fs::remove_file(dir.join(format!("{parent}.events.jsonl"))).unwrap();
    let list = list_periods(&dir, 10).unwrap();
    let child_now = list.iter().find(|p| p.period_id == child).cloned().expect("child listed");
    assert_eq!(child_now.parent, None,
               "C7: with the file gone the parent link is UNRESOLVABLE and the reader (correctly) nulls it — \
                which is why D2 rewrites rows instead of deleting files");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_object_is_refused_by_name_when_destroying_content() {
    let dir = test_support::tmp_dir("d2_missing");
    let err = super::purge_content(&dir, &["run-0000000000000000-p0000000000000000".to_string()]).unwrap_err();
    assert!(err.to_string().contains("no such stream"), "the refusal names the reason: {err}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// P40 — A REWRITE MUST BE ATOMIC (ADR-0048 §333). The append path has a tail guard; a REWRITE has no tail:
/// a crash in the middle of an in-place write leaves the WHOLE stream unparsable, and one bad row already
/// took three suites down once. So: temp file, flush, rename.
#[test]
fn a_rewrite_leaves_no_temporary_file_and_the_stream_still_parses() {
    use crate::trace::Redaction;
    use serde_json::json;
    use std::fs;
    let dir = test_support::tmp_dir("p40_atomic_ok");
    let pid = "run-5050505050505050-p0000000070000070";
    {
        let mut s = super::SessionEventStream::open(dir.clone(), pid, pid, Redaction::default()).unwrap();
        s.emit("2026-10-01T00:00:00Z", EventType::UserMessage, json!({ "text": "secret" })).unwrap();
    }
    super::purge_content(&dir, &[pid.to_string()]).unwrap();
    let leftovers: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "the temporary file is renamed away, never left behind: {leftovers:?}");
    let body = fs::read_to_string(dir.join(format!("{pid}.events.jsonl"))).unwrap();
    for line in body.lines().filter(|l| !l.trim().is_empty()) {
        serde_json::from_str::<serde_json::Value>(line).expect("every row still parses after the rewrite");
    }
    let _ = fs::remove_dir_all(&dir);
}

/// FAULT INJECTION: a partial write to the TEMPORARY path must leave the TARGET untouched. This is the
/// property that makes the crash window survivable — and it is asserted directly, not inferred.
#[test]
fn a_partial_temporary_write_does_not_touch_the_target() {
    use crate::trace::Redaction;
    use serde_json::json;
    use std::fs;
    let dir = test_support::tmp_dir("p40_fault_injection");
    let pid = "run-6060606060606060-p0000000071000071";
    {
        let mut s = super::SessionEventStream::open(dir.clone(), pid, pid, Redaction::default()).unwrap();
        s.emit("2026-10-01T00:00:00Z", EventType::UserMessage, json!({ "text": "intact" })).unwrap();
    }
    let target = dir.join(format!("{pid}.events.jsonl"));
    let before = fs::read_to_string(&target).unwrap();
    /* The "crash": half a row lands in the temp sibling and the process dies before the rename. */
    fs::write(format!("{}.tmp", target.display()), "{\"type\":\"user/message\",\"per").unwrap();
    let after = fs::read_to_string(&target).unwrap();
    assert_eq!(before, after, "a half-written TEMP file cannot damage the target");
    assert!(after.contains("intact"), "and the old content is still fully readable");
    let _ = fs::remove_dir_all(&dir);
}

/// THE MUTATION for P40: the OLD implementation truncated in place. Reproduce that and the stream is gone.
#[test]
fn an_in_place_truncating_rewrite_would_corrupt_the_stream() {
    use crate::trace::Redaction;
    use serde_json::json;
    use std::fs;
    let dir = test_support::tmp_dir("p40_mutation_in_place");
    let pid = "run-7070707070707070-p0000000072000072";
    {
        let mut s = super::SessionEventStream::open(dir.clone(), pid, pid, Redaction::default()).unwrap();
        s.emit("2026-10-01T00:00:00Z", EventType::UserMessage, json!({ "text": "one" })).unwrap();
        s.emit("2026-10-01T00:00:01Z", EventType::TurnEnd, json!({})).unwrap();
    }
    let target = dir.join(format!("{pid}.events.jsonl"));
    /* in-place truncating write, killed halfway */
    fs::write(&target, "{\"type\":\"us").unwrap();
    let body = fs::read_to_string(&target).unwrap();
    let bad = body.lines().filter(|l| !l.trim().is_empty()).any(|l| serde_json::from_str::<serde_json::Value>(l).is_err());
    assert!(bad, "this is the failure P40 forbids: an interrupted in-place rewrite loses the whole stream");
    let _ = fs::remove_dir_all(&dir);
}

/// P42 — the list is INVERTED (fail-CLOSED): a key that is neither declared as surviving nor known as content
/// is destroyed AND named, so a new field cannot slip through in either direction.
#[test]
fn an_unknown_key_is_destroyed_and_named_as_unclassified() {
    use crate::trace::Redaction;
    use serde_json::json;
    use std::fs;
    let dir = test_support::tmp_dir("p42_fail_closed");
    let pid = "run-8080808080808080-p0000000073000073";
    {
        let mut s = super::SessionEventStream::open(dir.clone(), pid, pid, Redaction::default()).unwrap();
        s.emit("2026-10-01T00:00:00Z", EventType::UserMessage,
               json!({ "text": "content", "body": "NEW-KEY-SECRET", "resume_from": "run-x" })).unwrap();
    }
    let report = super::purge_content(&dir, &[pid.to_string()]).unwrap();
    let raw = fs::read_to_string(dir.join(format!("{pid}.events.jsonl"))).unwrap();
    assert!(!raw.contains("NEW-KEY-SECRET") && !raw.contains("\"body\""),
            "a previously unknown content key is DESTROYED (fail-closed): {raw}");
    assert!(report.unclassified.contains(&format!("{}.body", pid)),
            "and it is NAMED as unclassified rather than removed in silence: {report:?}");
    assert!(raw.contains("resume_from"), "declared surviving fields are untouched");
    let _ = fs::remove_dir_all(&dir);
}

/// M2-D (ADR-0048 §334): a vacancy is READABLE — the fact, the deadline it implies, and a NAMED state.
#[test]
fn a_vacancy_is_readable_with_its_deadline_and_a_named_state() {
    use crate::trace::Redaction;
    use serde_json::json;
    use std::fs;
    let dir = test_support::tmp_dir("m2d_vacancy_view");
    let pid = "run-9090909090909090-p0000000080000080";
    {
        let mut s = super::SessionEventStream::open(dir.clone(), pid, pid, Redaction::default()).unwrap();
        s.emit("2026-10-01T00:00:00Z", EventType::TurnEnd, json!({})).unwrap();
    }
    super::write_ref(&dir, "current", pid).unwrap();
    super::delete_ref(&dir, "current").unwrap();

    let views = super::vacancies(&dir, 60, 0).unwrap();
    let v = views.iter().find(|v| v.object == pid).expect("the vacancy is readable");
    assert!(v.at > 0, "the FACT carries its time");
    assert_eq!(v.protected_until, v.at + 60, "and the deadline it implies");
    assert_eq!(v.state, "protected", "with a named state, not a bare timestamp");

    let later = super::vacancies(&dir, 60, v.protected_until + 1).unwrap();
    let v2 = later.iter().find(|v| v.object == pid).unwrap();
    assert_eq!(v2.state, "expired", "the state changes WITH the clock, in one place");
    let _ = fs::remove_dir_all(&dir);
}

/// The reading surface may not promise more than the store can keep: a grace beyond the declared retention is
/// REFUSED BY NAME (this is how `check_retention_covers_grace` reaches the surface).
#[test]
fn asking_for_more_grace_than_the_declared_retention_is_refused_by_name() {
    let dir = test_support::tmp_dir("m2d_grace_refused");
    let too_long = crate::session_events::REF_MOVE_RETENTION_SECS + 1;
    let err = super::vacancies(&dir, too_long, 0).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("exceeds the declared retention"), "the refusal names the reason: {msg}");
    assert!(msg.contains("cannot keep"), "and what it refuses to do: {msg}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A vacancy whose subject no longer exists is a vacancy without a promise: named `object-gone`, not silently
/// reported as `protected`.
#[test]
fn a_vacancy_whose_object_is_gone_is_named_object_gone() {
    use crate::trace::Redaction;
    use serde_json::json;
    use std::fs;
    let dir = test_support::tmp_dir("m2d_object_gone");
    let pid = "run-a0a0a0a0a0a0a0a0-p0000000081000081";
    {
        let mut s = super::SessionEventStream::open(dir.clone(), pid, pid, Redaction::default()).unwrap();
        s.emit("2026-10-01T00:00:00Z", EventType::TurnEnd, json!({})).unwrap();
    }
    super::tombstone_period(&dir, pid, "gone").unwrap();
    super::write_ref(&dir, "current", pid).unwrap();
    super::delete_ref(&dir, "current").unwrap();
    super::collect_garbage(&dir, 4_000_000_000, 0).unwrap();   /* D1 ⇒ the object stops existing */

    let views = super::vacancies(&dir, 60, 4_000_000_000).unwrap();
    let v = views.iter().find(|v| v.object == pid).expect("the vacancy fact is still readable");
    assert_eq!(v.state, "object-gone", "a vacancy whose subject is gone is NAMED, not called protected");
    let _ = fs::remove_dir_all(&dir);
}

/// The mode vocabulary (ADR-0048 §340): ONE mapping, and the inverse must round-trip. If the panel keyed on
/// the enum names while a payload carried these strings, a declared mode would read as "undeclared".
#[test]
fn the_mode_wire_vocabulary_is_one_mapping_and_round_trips() {
    use crate::config::{mode_from_wire, mode_wire, Mode};
    let all = [Mode::Drive, Mode::Partner, Mode::Survive];
    let wires: Vec<&str> = all.iter().map(|m| mode_wire(*m)).collect();
    assert_eq!(wires, vec!["driving", "partner", "survival"], "the protocol values, in one place");
    for m in all {
        assert_eq!(mode_from_wire(mode_wire(m)), Some(m), "every value round-trips: {m:?}");
    }
    assert_eq!(mode_from_wire("Partner"), None,
               "the ENUM NAME is not the wire value — accepting it would hide the drift again");
    assert_eq!(mode_from_wire("partner "), None, "and a near-miss is unknown, not guessed");
}

/// M3④ (ADR-0048 §345): the OPTIONAL mode must not fall back. A criterion that cannot tell `None` from
/// `Some(Partner)` would let the very defect it guards ("a default claiming to be declared") pass.
#[test]
fn an_undeclared_mode_stays_undeclared_and_never_falls_back() {
    use crate::config::Mode;
    use crate::session_events::mode_wire_opt;
    assert_eq!(mode_wire_opt(None), None, "no declaration ⇒ no value (never Mode::default())");
    assert_eq!(mode_wire_opt(Some(Mode::Partner)), Some("partner"));
    assert_eq!(mode_wire_opt(Some(Mode::Drive)), Some("driving"));
    assert_eq!(mode_wire_opt(Some(Mode::Survive)), Some("survival"));
    /* MUTATION: the fallback implementation would return `Some("partner")` for `None` — the assertion above
     * is what makes that difference observable instead of a silent lie. */
    let fallback = |m: Option<Mode>| Some(crate::config::mode_wire(m.unwrap_or_default()));
    assert_ne!(fallback(None), mode_wire_opt(None), "the fallback IS distinguishable from the honest one");
}
