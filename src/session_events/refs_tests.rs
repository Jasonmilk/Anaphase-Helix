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
    /* A SEGMENT-BASED RULE (M1d): `a/b` is now a LEGITIMATE nested name, so the list that must be refused
     * is the one whose SEGMENTS are unsafe — the earlier list encoded the one-segment design and became a
     * false red the moment nesting landed (the test was right for its time, wrong for this one). */
    for bad in ["", ".", "..", "../escape", "a//b", "a/./b", "a\\b", ".hidden", "a/.hidden"] {
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

/// M1d — MULTIPLE REFS (ADR-0048 §317, ledger P2). Git keeps ONE ref per branch; a single fixed name
/// cannot express "which conversation", so the conversation set was unaddressable. Nesting is the shape
/// Git uses (`refs/heads/<name>`), and every SEGMENT is still guarded.
#[test]
fn nested_ref_names_are_addressable() {
    let dir = test_support::tmp_dir("refs_nested");
    let a = "run-1111111111111111-p0000000013000013";
    let b = "run-2222222222222222-p0000000014000014";
    period(&dir, a, "job-na");
    period(&dir, b, "job-nb");
    write_ref(&dir, "current", a).unwrap();
    write_ref(&dir, &format!("conversations/{a}"), a).unwrap();
    write_ref(&dir, &format!("conversations/{b}"), b).unwrap();

    let names: Vec<String> = list_refs(&dir).unwrap().into_iter().map(|r| r.name).collect();
    assert_eq!(names, vec![
        "conversations/run-1111111111111111-p0000000013000013".to_string(),
        "conversations/run-2222222222222222-p0000000014000014".to_string(),
        "current".to_string(),
    ], "each conversation is separately addressable, in a deterministic order");
    assert_eq!(read_ref(&dir, &format!("conversations/{b}")).unwrap().as_deref(), Some(b),
               "and each one is readable by its own name");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_traversing_segment_is_still_refused() {
    let dir = test_support::tmp_dir("refs_nested_guard");
    for bad in ["conversations/../escape", "conversations//x", "conversations/.hidden", "a/./b"] {
        assert!(check_ref_name(bad).is_err(), "segment guard must refuse {bad:?}");
    }
    assert!(check_ref_name("conversations/run-abc").is_ok(), "and must ACCEPT a nested name");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_nested_ref_keeps_its_own_history() {
    let dir = test_support::tmp_dir("refs_nested_reflog");
    let a = "run-3333333333333333-p0000000015000015";
    let b = "run-4444444444444444-p0000000016000016";
    period(&dir, a, "job-ha");
    period(&dir, b, "job-hb");
    let name = "conversations/c1";
    write_ref(&dir, name, a).unwrap();
    write_ref(&dir, name, b).unwrap();
    let log = read_ref_log(&dir, name).unwrap();
    assert_eq!(log.len(), 2, "the history belongs to the NAMED ref, not to a global log");
    assert_eq!(log[1].old.as_deref(), Some(a));
    let _ = fs::remove_dir_all(&dir);
}

/// The numbering wall (ADR-0048 §319): appending to an existing stream must CONTINUE the numbering.
/// Reusing the last `seq` would make two events indistinguishable in order — deterministically, not
/// probabilistically. This is the criterion the reviewer asked for, stated as the failure it forbids.
#[test]
fn appending_continues_the_numbering_and_never_repeats_a_seq() {
    let dir = test_support::tmp_dir("append_seq_continues");
    let pid = "run-1234123412341234-p0000000020000020";
    {
        let mut s = SessionEventStream::open(dir.clone(), pid, pid, Redaction::default()).unwrap();
        s.emit("2026-10-01T00:00:00Z", EventType::UserMessage, json!({ "text": "a" })).unwrap();
        s.emit("2026-10-01T00:00:01Z", EventType::TurnEnd, json!({})).unwrap();
    }
    let before: Vec<u64> = read_period(&dir, pid).unwrap().iter().map(|e| e.seq).collect();
    let max_before = *before.iter().max().unwrap();
    {
        /* THE SECOND HANDLE APPENDS — the case that used to overwrite from offset 0. */
        let mut s = SessionEventStream::open_append(dir.clone(), pid, pid, Redaction::default()).unwrap();
        s.emit("2026-10-01T00:00:02Z", EventType::Check, json!({ "after": true })).unwrap();
    }
    let after: Vec<u64> = read_period(&dir, pid).unwrap().iter().map(|e| e.seq).collect();
    assert_eq!(after.len(), 3, "nothing was overwritten (append, not rewrite)");
    assert_eq!(after[2], max_before + 1, "the new event continues from max+1, never reusing a number");
    let mut sorted = after.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), after.len(), "no duplicate seq ⇒ order stays distinguishable");
    let _ = fs::remove_dir_all(&dir);
}

/// P13 — A TORN TAIL MUST BE NAMED, NOT SKIPPED (ADR-0048 §320).
/// Skipping a half-written last line makes `max+1` collide with the number that row already claimed:
/// two events, one `seq`, order indistinguishable. The reader refuses by name, and the explicit repair
/// is a separate, deliberate act.
#[test]
fn a_torn_tail_is_refused_by_name_and_can_be_repaired_explicitly() {
    use std::io::Write;
    let dir = test_support::tmp_dir("torn_tail");
    let pid = "run-5678567856785678-p0000000021000021";
    {
        let mut s = SessionEventStream::open(dir.clone(), pid, pid, Redaction::default()).unwrap();
        s.emit("2026-10-01T00:00:00Z", EventType::UserMessage, json!({ "text": "a" })).unwrap();
    }
    let path = dir.join(format!("{pid}.events.jsonl"));
    {
        /* A process killed mid-write: half a line, no newline. */
        let mut f = fs::OpenOptions::new().append(true).open(&path).unwrap();
        write!(f, "{{\"type\":\"assistant/reply\",\"seq\":1,\"da").unwrap();
    }
    let err = match SessionEventStream::open_append(dir.clone(), pid, pid, Redaction::default()) {
        Ok(_) => panic!("a torn tail must be REFUSED by name, not opened silently"),
        Err(e) => e,
    };
    assert!(err.to_string().contains("torn"), "the refusal NAMES the corruption: {err}");
    assert!(err.to_string().contains("RE-USE"), "and names the danger it avoids: {err}");

    let dropped = repair_torn_tail(&path).unwrap();
    assert!(dropped > 0, "the explicit repair reports how many bytes it removed");
    let mut s = SessionEventStream::open_append(dir.clone(), pid, pid, Redaction::default()).unwrap();
    s.emit("2026-10-01T00:00:02Z", EventType::TurnEnd, json!({})).unwrap();
    let seqs: Vec<u64> = read_period(&dir, pid).unwrap().iter().map(|e| e.seq).collect();
    let mut uniq = seqs.clone();
    uniq.sort_unstable();
    uniq.dedup();
    assert_eq!(uniq.len(), seqs.len(), "after the repair, no seq is reused: {seqs:?}");
    let _ = fs::remove_dir_all(&dir);
}

/// ⑤ THE WRITER LOCK (ADR-0048 §322). "Single writer" as a comment is worth 0 bits, and M5 (the shell) is
/// a known future second writer: the second writer must be REFUSED BY NAME, and breaking a lock must be a
/// deliberate act rather than a timeout.
#[test]
fn a_second_writer_is_refused_by_name_and_released_on_drop() {
    let dir = test_support::tmp_dir("writer_lock");
    let first = WriterLock::acquire(&dir).unwrap();
    let err = WriterLock::acquire(&dir).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("another writer holds"), "the refusal NAMES the holder: {msg}");
    assert!(msg.contains("release_stale_writer"), "and names the deliberate way out: {msg}");
    drop(first);
    assert!(WriterLock::acquire(&dir).is_ok(), "dropping the lock releases it");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_ref_write_takes_and_releases_the_lock() {
    let dir = test_support::tmp_dir("writer_lock_around_write");
    let pid = "run-aaaabbbbccccdddd-p0000000022000022";
    period(&dir, pid, "job-w");
    write_ref(&dir, "current", pid).unwrap();
    assert!(!dir.join(".refs").join(".writer").exists(),
            "the write released the lock (a stale file would block the NEXT writer)");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_stale_lock_is_named_and_only_broken_explicitly() {
    let dir = test_support::tmp_dir("writer_lock_stale");
    fs::create_dir_all(dir.join(".refs")).unwrap();
    fs::write(dir.join(".refs").join(".writer"), "{\"pid\":1,\"since\":1}\n").unwrap();
    let err = WriterLock::acquire(&dir).unwrap_err();
    assert!(err.to_string().contains("STALE"), "an old lock is NAMED stale, not silently stolen: {err}");
    assert_eq!(release_stale_writer(&dir).unwrap(), true, "the explicit break reports that there was one");
    assert_eq!(release_stale_writer(&dir).unwrap(), false, "and a second break is a NAMED absence");
    let _ = fs::remove_dir_all(&dir);
}

/// ⑧ RETENTION ≥ GRACE (ADR-0048 §323). The vacancy fact must outlive the window that depends on it; the
/// two numbers live in ONE place and their relation is a criterion, not a sentence.
#[test]
fn the_declared_retention_covers_the_grace_window() {
    let (retention, grace) = check_retention_covers_grace()
        .expect("the declared pair must satisfy retention >= grace");
    assert!(retention >= grace, "retention {retention}s vs grace {grace}s");
    assert!(grace > 0 && retention > 0, "both windows are real quantities, not zeros");
}

#[test]
fn the_retention_check_can_go_red() {
    /* MUTATION: the pure comparison must refuse the under-protecting case — otherwise the criterion above
     * would be a tautology that passes for any pair. */
    assert_eq!(retention_covers_grace(10, 20), false, "shorter retention ⇒ under-protection");
    assert_eq!(retention_covers_grace(20, 10), true);
    assert_eq!(retention_covers_grace(20, 20), true, "equality is enough: the anchor survives the window");
}

/// A MISSING TRAILING NEWLINE MUST NOT CONCATENATE TWO ROWS (ADR-0048 §325). Measured the hard way: the
/// ref writer appended to a file whose last row had no `\n`, two objects landed on one line, and three
/// unrelated suites crashed parsing that line — reds that looked like someone else's bug.
#[test]
fn appending_to_a_file_without_a_trailing_newline_does_not_concatenate_rows() {
    let dir = test_support::tmp_dir("append_newline_guard");
    let pid = "run-9999888877776666-p0000000023000023";
    let path = dir.join(format!("{pid}.events.jsonl"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(&path, "{\"type\":\"turn/end\",\"period_id\":\"x\",\"job_id\":\"x\",\"seq\":1,\"time\":\"t\",\"data\":{}}").unwrap();
    {
        let mut s = SessionEventStream::open_append(dir.clone(), pid, pid, Redaction::default()).unwrap();
        s.emit("2026-10-01T00:00:03Z", EventType::Check, json!({ "after": true })).unwrap();
    }
    let body = fs::read_to_string(&path).unwrap();
    for line in body.lines() {
        if line.trim().is_empty() {
            continue;
        }
        serde_json::from_str::<serde_json::Value>(line)
            .unwrap_or_else(|e| panic!("every row must parse on its own line: {e} — body: {body}"));
    }
    assert_eq!(body.lines().count(), 2, "one row per line, never concatenated");
    let _ = fs::remove_dir_all(&dir);
}
