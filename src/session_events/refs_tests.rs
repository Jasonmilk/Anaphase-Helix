//! REFS tests (ADR-0048 §309): the four ENDINGS of the third layer, each paired with the guard that
//! produces it — so removing a guard makes a test fail rather than making the system quietly lenient.
//!
//! NOTE the fixture ids are SHAPE-VALID period ids (`run-<16 hex>-p<16 hex>`): a `resume_from` or ref
//! target that is not period-shaped is treated as a JOB reference, which is a different (and refusable)
//! case — the first version of an earlier test suite deceived itself exactly there.

use super::*;
use super::refs::check_ref_name;   /* the traversal guard needs its own name in scope */
use crate::trace::Redaction;
use serde_json::json;
use std::fs;

fn period(dir: &std::path::Path, pid: &str, job: &str) {
    let mut s = SessionEventStream::open(dir.to_path_buf(), pid, job, Redaction::default()).unwrap();
    s.emit("2026-09-30T00:00:00Z", EventType::UserMessage, json!({ "text": "hello" })).unwrap();
    s.emit("2026-09-30T00:00:01Z", EventType::TurnEnd, json!({})).unwrap();
}

#[test]
fn a_ref_round_trips_lists_and_lives_in_the_repository() {
    let dir = test_support::tmp_dir("ref_round_trip");
    let pid = "run-aaaaaaaaaaaaaaaa-p0000000001000001";
    period(&dir, pid, "job-a");
    let resolved = write_ref(&dir, "main", pid).unwrap();
    assert_eq!(resolved, pid, "the resolved id is returned, so the caller learns what was stored");
    assert_eq!(read_ref(&dir, "main").unwrap().as_deref(), Some(pid));
    assert!(dir.join(".refs").join("main").is_file(),
            "a ref is ONE FILE inside the event store (Git keeps refs in the repository, not in a window)");
    let all = list_refs(&dir).unwrap();
    assert_eq!(all, vec![RefEntry { name: "main".to_string(), period_id: pid.to_string() }]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_unknown_ref_is_a_named_absence() {
    let dir = test_support::tmp_dir("ref_absent");
    assert_eq!(read_ref(&dir, "nope").unwrap(), None, "no such ref is None, never an error");
    assert_eq!(delete_ref(&dir, "nope").unwrap(), false, "deleting nothing reports false, not Ok");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_dangling_target_is_refused_and_stores_nothing() {
    let dir = test_support::tmp_dir("ref_dangling");
    let missing = "run-bbbbbbbbbbbbbbbb-p0000000002000002";
    let err = write_ref(&dir, "main", missing).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("dangling"), "the refusal NAMES the reason: {msg}");
    assert!(!dir.join(".refs").join("main").exists(), "a refused write must not leave a pointer");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_ambiguous_target_is_refused_by_name() {
    let dir = test_support::tmp_dir("ref_ambiguous");
    /* Two periods sharing ONE job id: resolving by that digest is ambiguous, and the store refuses
     * rather than picking one (a confident wrong answer is worse than a refusal). */
    period(&dir, "run-cccccccccccccccc-p0000000003000003", "run-shared-digest");
    period(&dir, "run-dddddddddddddddd-p0000000004000004", "run-shared-digest");
    let err = write_ref(&dir, "main", "run-shared-digest").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("AMBIGUOUS"), "ambiguity is named, not guessed: {msg}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn unsafe_names_are_refused() {
    let dir = test_support::tmp_dir("ref_unsafe_names");
    for bad in ["", ".", "..", "../escape", "a/b", "a\\b", ".hidden"] {
        assert!(check_ref_name(bad).is_err(), "name {bad:?} must be refused (traversal guard)");
        assert!(write_ref(&dir, bad, "run-aaaaaaaaaaaaaaaa-p0000000001000001").is_err(),
                "and the write path must refuse it too: {bad:?}");
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_listing_is_sorted_so_it_is_auditable() {
    let dir = test_support::tmp_dir("ref_sorted");
    let pid = "run-eeeeeeeeeeeeeeee-p0000000005000005";
    period(&dir, pid, "job-e");
    write_ref(&dir, "zeta", pid).unwrap();
    write_ref(&dir, "alpha", pid).unwrap();
    let names: Vec<String> = list_refs(&dir).unwrap().into_iter().map(|r| r.name).collect();
    assert_eq!(names, vec!["alpha".to_string(), "zeta".to_string()]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_empty_target_is_refused() {
    let dir = test_support::tmp_dir("ref_empty_target");
    assert!(write_ref(&dir, "main", "   ").is_err(), "a ref must point at something");
    let _ = fs::remove_dir_all(&dir);
}

/// THE REFLOG (ADR-0048 §315): the value that was REPLACED, recorded at the moment it is replaced.
/// `M2`'s grace anchor is derived from this fact, and a back-filled record has fidelity q<1 per write
/// (`q^n` ≈ 0.37) — so the cheap moment to write it is the only moment worth writing it in.
#[test]
fn the_first_write_records_no_previous_value() {
    let dir = test_support::tmp_dir("reflog_first");
    let pid = "run-8888888888888888-p0000000008000008";
    period(&dir, pid, "job-l");
    write_ref(&dir, "current", pid).unwrap();
    let log = read_ref_log(&dir, "current").unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].old, None, "nothing was there before: absence, not a made-up value");
    assert_eq!(log[0].new.as_deref(), Some(pid));
    assert!(!log[0].ts.is_empty(), "and WHEN it happened is part of the fact");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_second_write_records_the_value_it_replaced() {
    let dir = test_support::tmp_dir("reflog_second");
    let a = "run-9999999999999999-p0000000009000009";
    let b = "run-aaaaaaaaaaaaaaaa-p0000000010000010";
    period(&dir, a, "job-a");
    period(&dir, b, "job-b");
    write_ref(&dir, "current", a).unwrap();
    write_ref(&dir, "current", b).unwrap();
    let log = read_ref_log(&dir, "current").unwrap();
    assert_eq!(log.len(), 2, "one line per change, never a rewrite");
    /* MUTATION: a PUT that recorded only the NEW value would leave `old = None` here — this assertion is
     * the one that fails, which is what makes the debt visible instead of silent. */
    assert_eq!(log[1].old.as_deref(), Some(a), "the replaced value is preserved");
    assert_eq!(log[1].new.as_deref(), Some(b));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn clearing_a_ref_is_recorded_as_a_vacancy() {
    let dir = test_support::tmp_dir("reflog_vacancy");
    let pid = "run-bbbbbbbbbbbbbbbb-p0000000011000011";
    period(&dir, pid, "job-v");
    write_ref(&dir, "current", pid).unwrap();
    assert_eq!(delete_ref(&dir, "current").unwrap(), true);
    let log = read_ref_log(&dir, "current").unwrap();
    assert_eq!(log.len(), 2);
    assert_eq!(log[1].old.as_deref(), Some(pid), "the vacated pointer is named");
    assert_eq!(log[1].new, None, "and the vacancy is a VALUE (null), not a missing line");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_history_is_not_mistaken_for_a_ref() {
    let dir = test_support::tmp_dir("reflog_not_a_ref");
    let pid = "run-cccccccccccccccc-p0000000012000012";
    period(&dir, pid, "job-n");
    write_ref(&dir, "current", pid).unwrap();
    let names: Vec<String> = list_refs(&dir).unwrap().into_iter().map(|r| r.name).collect();
    assert_eq!(names, vec!["current".to_string()], "the reflog lives in a subdirectory and is not a ref");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_unknown_ref_has_an_empty_history() {
    let dir = test_support::tmp_dir("reflog_unknown");
    assert!(read_ref_log(&dir, "never-written").unwrap().is_empty(),
            "no history is an EMPTY history, not an error");
    let _ = fs::remove_dir_all(&dir);
}
