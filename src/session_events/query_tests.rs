//! Query-surface tests: listing, resolution, ambiguity, replay equivalence.

    use super::*;
    use super::naming::period_name;
    use crate::trace::Redaction;
    use serde_json::json;
    use std::fs;

    #[test]
    fn lists_periods_newest_first() {
        let dir = test_support::tmp_dir("lists_periods_newest_first");
        // Two periods written out of time order (b first, a second).
        let mut b = SessionEventStream::open(dir.clone(), "run-b2", "run-b2", Redaction::default()).unwrap();
        let mut a = SessionEventStream::open(dir.clone(), "run-a1", "run-a1", Redaction::default()).unwrap();
        b.emit("2026-09-07T00:00:00Z", EventType::UserMessage, json!({ "text": "second" })).unwrap();
        b.emit("2026-09-07T00:00:02Z", EventType::TurnEnd, json!({})).unwrap();
        a.emit("2026-09-07T00:00:10Z", EventType::UserMessage, json!({ "text": "first message" })).unwrap();
        a.emit("2026-09-07T00:00:12Z", EventType::TurnEnd, json!({})).unwrap();
        let list = list_periods(&dir, 10).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].job_id, "run-a1", "newest first");
        assert_eq!(list[1].job_id, "run-b2");
        assert_eq!(list[0].preview, "first message");
        assert_eq!(list[0].count, 2);
        let _ = fs::remove_dir_all(&dir);
    }

    /// A parent that names no existing period must be reported as absent, and a
    /// parent that exists must survive `limit` truncation.
    ///
    /// The dangling value is deliberately SHAPE-VALID (`run-0badc0de` passes
    /// `is_period_id`) so this covers what that check cannot: the writer was right
    /// about the format and the period is simply gone. A consumer receiving it
    /// either reconstructs a thread that is not there or silently treats the period
    /// as a root; `None` is the honest answer.
    ///
    /// The second half pins the ORDER of the two steps. Normalisation runs before
    /// `truncate`, because a parent that merely fell outside the requested window
    /// still exists — nulling it would be a lie about the data rather than a
    /// convenience for the caller.
    #[test]
    fn a_parent_that_does_not_exist_is_reported_as_absent() {
        let dir = test_support::tmp_dir("dangling_parent_is_absent");
        // Identity is allocated, so the parent pointers below are period ids.
        let root_id = allocate_period_id("run-aaaa1111", 1_760_000_000);
        let orphan_id = allocate_period_id("run-cccc3333", 1_760_000_001);
        let child_id = allocate_period_id("run-bbbb2222", 1_760_000_002);
        // Oldest: the parent of the child, and the one limit=1 will truncate.
        let mut root = SessionEventStream::open(dir.clone(), &root_id, "run-aaaa1111", Redaction::default()).unwrap();
        root.emit("2026-09-07T00:00:00Z", EventType::UserMessage, json!({ "text": "root" })).unwrap();
        root.emit("2026-09-07T00:00:01Z", EventType::TurnEnd, json!({})).unwrap();
        // Middle: continues a period that does not exist.
        let mut orphan = SessionEventStream::open(dir.clone(), &orphan_id, "run-cccc3333", Redaction::default()).unwrap();
        orphan.emit("2026-09-07T00:00:05Z", EventType::ContextInject, json!({ "resume_from": "run-0badc0de" })).unwrap();
        orphan.emit("2026-09-07T00:00:06Z", EventType::TurnEnd, json!({})).unwrap();
        // Newest: continues the root, which exists.
        let mut child = SessionEventStream::open(dir.clone(), &child_id, "run-bbbb2222", Redaction::default()).unwrap();
        child.emit("2026-09-07T00:00:10Z", EventType::ContextInject, json!({ "resume_from": root_id })).unwrap();
        child.emit("2026-09-07T00:00:11Z", EventType::TurnEnd, json!({})).unwrap();

        let all = list_periods(&dir, 10).unwrap();
        let by = |id: &str| {
            all.iter()
                .find(|p| p.period_id == id)
                .unwrap()
                .parent
                .clone()
        };
        assert_eq!(by(&child_id), Some(root_id.clone()));
        assert_eq!(by(&orphan_id), None, "a parent that exists nowhere is not a parent");

        let one = list_periods(&dir, 1).unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].period_id, child_id, "newest first");
        assert_eq!(
            one[0].parent,
            Some(root_id),
            "a parent truncated out of the window still EXISTS and must not be nulled"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// A parent written as a JOB id is refused, and the SAME period resolves once
    /// the parent is the canonical period id.
    ///
    /// This is the measured shape of a live defect, not a hypothetical. The panel
    /// sends its current selection in a field called `job_id`, and the server used
    /// to store whatever arrived as the parent pointer — so a chain created from a
    /// job id records `run-<hex>` where the reader requires `run-<hex>-p<16hex>`.
    /// Observed in this workspace's own history: 166 periods, two with a
    /// resolvable parent, and a four-period chain whose every `resume_from` is a
    /// job id. The sidebar groups by period identity, so all four read as roots
    /// and each card stood alone.
    ///
    /// The reader is right to refuse: `is_period_id` is what keeps prose out (the
    /// K-004 fault). The fix belongs at the writer, which now resolves the key
    /// before recording it.
    ///
    /// What this assertion can and cannot catch, measured rather than assumed:
    /// removing EITHER reader guard (`is_period_id`, or the later "the parent must
    /// be one of the known period ids" pass) still yields `None`, because the two
    /// are independent defences over the same value. So this is an end-to-end
    /// assertion and it will NOT go red for a single-layer regression — verified by
    /// mutating each in turn. It is written down because a check believed to be
    /// stronger than it is is worse than one that is known to be weak; the writer
    /// half below is the part that is asserted for a reason.
    #[test]
    fn a_job_id_parent_is_refused_while_its_period_id_resolves() {
        let dir = test_support::tmp_dir("job_id_parent_refused");
        let parent_id = allocate_period_id("run-aaaa1111", 1_760_000_000);
        let job_form = "run-aaaa1111";
        let child_id = allocate_period_id("run-bbbb2222", 1_760_000_002);

        let mut parent = SessionEventStream::open(dir.clone(), &parent_id, job_form, Redaction::default()).unwrap();
        parent.emit("2026-09-07T00:00:00Z", EventType::UserMessage, json!({ "text": "root" })).unwrap();
        parent.emit("2026-09-07T00:00:01Z", EventType::TurnEnd, json!({})).unwrap();

        // Exactly what the old writer produced: the job id, not the period.
        let mut child = SessionEventStream::open(dir.clone(), &child_id, "run-bbbb2222", Redaction::default()).unwrap();
        child.emit("2026-09-07T00:00:10Z", EventType::ContextInject, json!({ "resume_from": job_form })).unwrap();
        child.emit("2026-09-07T00:00:11Z", EventType::TurnEnd, json!({})).unwrap();

        let list = list_periods(&dir, 10).unwrap();
        let child_row = list.iter().find(|p| p.period_id == child_id).unwrap();
        assert_eq!(
            child_row.parent, None,
            "a job id is not a period id and must not be read as one — the sidebar groups by period identity"
        );

        // The other half, and the reason the fix is at the writer: the key the
        // client sent DOES resolve to exactly one period, so a writer that
        // resolves it first produces a parent the reader accepts.
        let resolved = resolve_one(&dir, job_form).expect("one period carries this job id");
        assert_eq!(resolved, parent_id, "the job id resolves to the period that owns it");
        assert!(is_period_id(&resolved), "and what it resolves to satisfies the reader's check");

        // Re-open as the fixed writer would: the canonical period id in the same
        // slot makes the child a child.
        let child2_id = allocate_period_id("run-bbbb2222", 1_760_000_003);
        let mut child2 = SessionEventStream::open(dir.clone(), &child2_id, "run-bbbb2222", Redaction::default()).unwrap();
        child2.emit("2026-09-07T00:00:20Z", EventType::ContextInject, json!({ "resume_from": resolved })).unwrap();
        child2.emit("2026-09-07T00:00:21Z", EventType::TurnEnd, json!({})).unwrap();
        let after = list_periods(&dir, 10).unwrap();
        let fixed = after.iter().find(|p| p.period_id == child2_id).unwrap();
        assert_eq!(
            fixed.parent,
            Some(parent_id),
            "with the canonical id in the slot the parent is found — this is what the writer now records"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// T1 + T8: the same input twice. Both periods survive, both are listed,
    /// and the shared `job_id` refuses to resolve to either one.
    #[test]
    fn repeated_input_keeps_both_periods_and_refuses_to_guess() {
        let dir = test_support::tmp_dir("repeated_input_two_periods");
        let job = "run-aaaa1111";
        let first = allocate_period_id(job, 1_760_000_000);
        let second = allocate_period_id(job, 1_760_000_100);

        for (id, ts, text) in [
            (&first, "2026-09-07T00:00:00Z", "first run"),
            (&second, "2026-09-07T00:10:00Z", "second run"),
        ] {
            let mut s = SessionEventStream::open(dir.clone(), id, job, Redaction::default()).unwrap();
            s.emit(ts, EventType::TurnStart, json!({})).unwrap();
            s.emit(ts, EventType::UserMessage, json!({ "text": text })).unwrap();
        }

        // T1: the first run is still readable, by its own id.
        let events = read_period(&dir, &first).unwrap();
        assert_eq!(events.len(), 2, "the earlier run must not have been overwritten");
        assert_eq!(events[1].data["text"], "first run");
        let second_events = read_period(&dir, &second).unwrap();
        assert_eq!(second_events[1].data["text"], "second run");
        assert_ne!(first, second, "two runs must own distinct identities");

        // Both rows reach the list — the truth, not a collapsed single row.
        let all = list_periods(&dir, 10).unwrap();
        assert_eq!(all.len(), 2, "both runs must be listed");
        assert!(all.iter().all(|p| p.job_id == job));
        assert!(all.iter().any(|p| p.period_id == first));
        assert!(all.iter().any(|p| p.period_id == second));

        // T8: opening by the shared digest must ERROR and name the candidates.
        let err = read_period(&dir, job).expect_err("an ambiguous digest must not resolve");
        let msg = err.to_string();
        assert!(msg.contains(&first) && msg.contains(&second), "candidates must be named: {msg}");
        // ...and nothing was mutated by the refusal.
        assert_eq!(read_period(&dir, &first).unwrap().len(), 2);
        assert_eq!(read_period(&dir, &second).unwrap().len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    /// T2 + T9: the positive control. A digest that matches exactly ONE period
    /// still resolves, so the refusal above is about ambiguity rather than a
    /// break in the ordinary path.
    #[test]
    fn a_unique_digest_still_resolves_by_job_id() {
        let dir = test_support::tmp_dir("unique_digest_resolves");
        let job = "run-1234abcd";
        let only = allocate_period_id(job, 1_760_000_000);
        let mut s = SessionEventStream::open(dir.clone(), &only, job, Redaction::default()).unwrap();
        s.emit("2026-09-07T00:00:00Z", EventType::UserMessage, json!({ "text": "hi" })).unwrap();

        let events = read_period(&dir, job).unwrap();
        assert_eq!(events.len(), 1, "a single match resolves without ceremony");
        assert_eq!(events[0].period_id, only);
        let _ = fs::remove_dir_all(&dir);
    }

    /// A rename by an ambiguous digest must change NOTHING. Silently picking
    /// one run to relabel is the same guess as silently reading one.
    #[test]
    fn rename_refuses_an_ambiguous_digest() {
        let dir = test_support::tmp_dir("rename_refuses_ambiguous");
        let job = "run-cafe1234";
        let a = allocate_period_id(job, 1_760_000_000);
        let b = allocate_period_id(job, 1_760_000_001);
        for id in [&a, &b] {
            let mut s = SessionEventStream::open(dir.clone(), id, job, Redaction::default()).unwrap();
            s.emit("2026-09-07T00:00:00Z", EventType::TurnStart, json!({})).unwrap();
        }
        let err = rename_period(&dir, job, "chosen").expect_err("an ambiguous key must not rename");
        assert!(
            err.to_string().contains("ambiguous period reference"),
            "the refusal must say why: {err}"
        );
        // Neither run acquired a sidecar.
        assert!(period_name(&dir, &a).is_none());
        assert!(period_name(&dir, &b).is_none());
        // The unique key still renames normally (positive control).
        rename_period(&dir, &a, "only this one").unwrap();
        assert_eq!(period_name(&dir, &a), Some("only this one".to_string()));
        assert!(period_name(&dir, &b).is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    /// T10: replay still works — the same input produces the same BODY, and
    /// the comparison deliberately EXCLUDES the allocated identity.
    ///
    /// This test exists to protect the allocation decision, not merely to
    /// check it. If a future replay comparison included `period_id`, every
    /// replay would differ and the obvious "fix" would be to derive the id
    /// from the content again — silently undoing what B15 did and restoring
    /// the K-006 collision. So the exclusion is asserted, at the one place a
    /// comparison naturally happens.
    #[test]
    fn replay_compares_body_and_excludes_allocated_identity() {
        let dir = test_support::tmp_dir("replay_excludes_identity");
        let job = "run-1e91a0";
        let first = allocate_period_id(job, 1_760_000_000);
        let replay_id = allocate_period_id(job, 1_760_000_500);
        assert_ne!(first, replay_id, "an identity is allocated, so a replay differs here");

        // Identical logical content, replayed under the same handle.
        for id in [&first, &replay_id] {
            let mut s = SessionEventStream::open(dir.clone(), id, job, Redaction::default()).unwrap();
            s.emit("2026-09-07T00:00:00Z", EventType::TurnStart, json!({})).unwrap();
            s.emit(
                "2026-09-07T00:00:01Z",
                EventType::UserMessage,
                json!({ "text": "same question" }),
            )
            .unwrap();
        }

        let a = read_period(&dir, &first).unwrap();
        let b = read_period(&dir, &replay_id).unwrap();
        assert_eq!(a.len(), b.len(), "a replay must produce the same row count");
        for (x, y) in a.iter().zip(b.iter()) {
            // The ONLY field allowed to differ between a period and its replay.
            assert_ne!(x.period_id, y.period_id, "identity is per-run by design");
            assert_eq!(x.event_type, y.event_type);
            assert_eq!(x.job_id, y.job_id, "the replay handle is stable");
            assert_eq!(x.seq, y.seq);
            assert_eq!(x.time, y.time);
            assert_eq!(x.data, y.data, "the body is what a replay comparison reads");
        }
        // Both runs still resolvable: the replay did not displace the original.
        assert_eq!(read_period(&dir, &first).unwrap().len(), 2);
        assert_eq!(read_period(&dir, job).unwrap_err().to_string().contains("ambiguous"), true);
        let _ = fs::remove_dir_all(&dir);
    }

    /// T5: `seq` restarts every period, so the unique row key is the PAIR
    /// `(period_id, seq)`. Two periods' row 0 are not duplicates of each other.
    #[test]
    fn seq_is_scoped_to_its_period() {
        let dir = test_support::tmp_dir("seq_scoped_to_period");
        let job = "run-9999beef";
        let a = allocate_period_id(job, 1_760_000_000);
        let b = allocate_period_id(job, 1_760_000_001);
        for id in [&a, &b] {
            let mut s = SessionEventStream::open(dir.clone(), id, job, Redaction::default()).unwrap();
            s.emit("2026-09-07T00:00:00Z", EventType::TurnStart, json!({})).unwrap();
        }
        let ea = read_period(&dir, &a).unwrap();
        let eb = read_period(&dir, &b).unwrap();
        assert_eq!(ea[0].seq, 0);
        assert_eq!(eb[0].seq, 0, "seq restarts in the second period");
        // Same seq, different period id => distinct rows, not a duplicate.
        let key_a = (ea[0].period_id.clone(), ea[0].seq);
        let key_b = (eb[0].period_id.clone(), eb[0].seq);
        assert_ne!(key_a, key_b, "(period_id, seq) is the unique row key");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn reads_one_period_by_id() {
        let dir = test_support::tmp_dir("read_one_period_by_id");
        let mut s = SessionEventStream::open(dir.clone(), "run-e5", "run-e5", Redaction::default()).unwrap();
        s.emit("2026-09-07T00:00:00Z", EventType::TurnStart, json!({})).unwrap();
        s.emit("2026-09-07T00:00:01Z", EventType::UserMessage, json!({ "text": "hi" })).unwrap();
        let events = read_period(&dir, "run-e5").unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "turn/start");
        assert_eq!(events[1].event_type, "user/message");
        assert!(read_period(&dir, "missing").is_err(), "unknown id must error");
        let _ = fs::remove_dir_all(&dir);
    }

/// THE CONVERSATION IS THE LINEAGE ROOT, DECLARED BY THE READER (ADR-0048 §297).
///
/// `job_id` is a content digest: two conversations that open with the same words share it, so it can
/// never name a conversation (measured on the live store: 22 job ids stood in for 79 chains).
/// NOTE the fixture ids are SHAPE-VALID period ids (`run-<16 hex>-p<16 hex>`) — a `resume_from` that is
/// not period-shaped is treated as a JOB reference and REFUSED (see `job_id_parent_refused`), which is
/// exactly how the first version of these tests deceived itself.
#[test]
fn conversation_root_names_itself_and_continuations_inherit_it() {
    let dir = test_support::tmp_dir("conversation_root_names_itself");
    let root_id = "run-aaaaaaaaaaaaaaaa-p0000000001000001";
    let child_id = "run-bbbbbbbbbbbbbbbb-p0000000002000002";
    let mut root = SessionEventStream::open(dir.clone(), root_id, "job-r", Redaction::default()).unwrap();
    root.emit("2026-09-30T00:00:00Z", EventType::UserMessage, json!({ "text": "hello" })).unwrap();
    root.emit("2026-09-30T00:00:01Z", EventType::TurnEnd, json!({})).unwrap();
    let mut child = SessionEventStream::open(dir.clone(), child_id, "job-c", Redaction::default()).unwrap();
    child.emit("2026-09-30T00:00:05Z", EventType::ContextInject, json!({ "resume_from": root_id })).unwrap();
    child.emit("2026-09-30T00:00:06Z", EventType::UserMessage, json!({ "text": "again" })).unwrap();
    child.emit("2026-09-30T00:00:07Z", EventType::TurnEnd, json!({})).unwrap();

    let list = list_periods(&dir, 10).unwrap();
    let by = |id: &str| list.iter().find(|p| p.period_id == id).cloned().unwrap();
    assert_eq!(by(root_id).parent, None, "the root has no parent");
    assert_eq!(by(root_id).conversation_id.as_deref(), Some(root_id), "a root names itself");
    assert_eq!(by(child_id).parent.as_deref(), Some(root_id), "the continuation points at its root");
    assert_eq!(by(child_id).conversation_id.as_deref(), Some(root_id), "a continuation inherits its root");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn two_conversations_sharing_a_job_id_stay_two() {
    let dir = test_support::tmp_dir("same_job_two_conversations");
    // SAME job id on both chains: the digest repeats by construction.
    for (pid, ts) in [("run-cccccccccccccccc-p0000000003000003", "2026-09-30T01:00:00Z"),
                      ("run-dddddddddddddddd-p0000000004000004", "2026-09-30T02:00:00Z")] {
        let mut p = SessionEventStream::open(dir.clone(), pid, "run-same-digest", Redaction::default()).unwrap();
        p.emit(ts, EventType::UserMessage, json!({ "text": "same opening words" })).unwrap();
        p.emit(ts, EventType::TurnEnd, json!({})).unwrap();
    }
    let list = list_periods(&dir, 10).unwrap();
    let conv: Vec<String> = list.iter().filter_map(|p| p.conversation_id.clone()).collect();
    assert_eq!(list.len(), 2);
    assert_eq!(conv.len(), 2, "both roots declare a conversation");
    assert_ne!(conv[0], conv[1], "sharing a job_id must NOT fuse two conversations");
    assert_eq!(list[0].job_id, list[1].job_id, "and the digest really does repeat here");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_root_outside_the_window_is_named_absent() {
    let dir = test_support::tmp_dir("conversation_root_outside_window");
    let r1 = "run-1111111111111111-p0000000001000001";
    let r2 = "run-2222222222222222-p0000000002000002";
    let r3 = "run-3333333333333333-p0000000003000003";
    let mut p1 = SessionEventStream::open(dir.clone(), r1, "job-1", Redaction::default()).unwrap();
    p1.emit("2026-09-30T00:00:00Z", EventType::UserMessage, json!({ "text": "one" })).unwrap();
    p1.emit("2026-09-30T00:00:01Z", EventType::TurnEnd, json!({})).unwrap();
    let mut p2 = SessionEventStream::open(dir.clone(), r2, "job-2", Redaction::default()).unwrap();
    p2.emit("2026-09-30T00:00:10Z", EventType::ContextInject, json!({ "resume_from": r1 })).unwrap();
    p2.emit("2026-09-30T00:00:11Z", EventType::TurnEnd, json!({})).unwrap();
    let mut p3 = SessionEventStream::open(dir.clone(), r3, "job-3", Redaction::default()).unwrap();
    p3.emit("2026-09-30T00:00:20Z", EventType::ContextInject, json!({ "resume_from": r2 })).unwrap();
    p3.emit("2026-09-30T00:00:21Z", EventType::TurnEnd, json!({})).unwrap();

    let full = list_periods(&dir, 10).unwrap();
    assert_eq!(full.iter().find(|p| p.period_id == r3).unwrap().conversation_id.as_deref(),
               Some(r1), "with the root in the window the answer IS the root");

    let cut = list_periods(&dir, 2).unwrap();   /* the root is now OUTSIDE the window */
    assert_eq!(cut.len(), 2);
    for p in &cut {
        assert_eq!(p.conversation_id, None,
                   "a window-dependent root is not a fact: {} must report ABSENCE, not the oldest visible ancestor",
                   p.period_id);
    }
    let _ = fs::remove_dir_all(&dir);
}

/// D0 — THE TOMBSTONE, READ SIDE (ADR-0048 §311). The WRITER is not landed (see `query.rs`: a second
/// stream handle starts at offset 0 and overwrote history — caught by the "bytes stay" criterion), so
/// these fixtures append a tombstone ROW by hand. What they pin is the read side and its preconditions.
fn append_tombstone(dir: &std::path::Path, pid: &str, seq: u64) {
    use std::io::Write;
    let path = dir.join(format!("{pid}.events.jsonl"));
    let mut f = fs::OpenOptions::new().append(true).open(&path).unwrap();
    writeln!(f, "{}", json!({
        "type": "period/tombstone", "period_id": pid, "job_id": pid, "seq": seq,
        "time": "2026-10-01T00:00:00Z", "data": { "reason": "fixture" }
    })).unwrap();
}

#[test]
fn a_tombstoned_period_disappears_from_the_list_but_its_bytes_stay() {
    let dir = test_support::tmp_dir("tombstone_hides_not_deletes");
    let pid = "run-1111111111111111-p0000000001000001";
    let mut s = SessionEventStream::open(dir.clone(), pid, pid, Redaction::default()).unwrap();
    s.emit("2026-09-30T00:00:00Z", EventType::UserMessage, json!({ "text": "hi" })).unwrap();
    s.emit("2026-09-30T00:00:01Z", EventType::TurnEnd, json!({})).unwrap();
    drop(s);
    assert_eq!(list_periods(&dir, 10).unwrap().len(), 1, "listed before the tombstone");

    append_tombstone(&dir, pid, 2);
    assert_eq!(list_periods(&dir, 10).unwrap().len(), 0, "readers no longer see it");
    assert_eq!(count_tombstoned(&dir).unwrap(), 1, "and the hide is COUNTED, not silent");
    let raw = fs::read_to_string(dir.join(format!("{pid}.events.jsonl"))).unwrap();
    assert!(raw.lines().count() >= 3 && raw.contains("turn/end") && raw.contains("period/tombstone"),
            "every byte stays on disk — D0 destroys nothing (that is D2): {raw}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_child_of_a_tombstoned_period_is_still_listed() {
    let dir = test_support::tmp_dir("tombstone_keeps_the_chain");
    let root = "run-3333333333333333-p0000000003000003";
    let child = "run-4444444444444444-p0000000004000004";
    let mut r = SessionEventStream::open(dir.clone(), root, root, Redaction::default()).unwrap();
    r.emit("2026-09-30T00:00:00Z", EventType::TurnEnd, json!({})).unwrap();
    drop(r);
    let mut c = SessionEventStream::open(dir.clone(), child, child, Redaction::default()).unwrap();
    c.emit("2026-09-30T00:00:10Z", EventType::ContextInject, json!({ "resume_from": root })).unwrap();
    c.emit("2026-09-30T00:00:11Z", EventType::TurnEnd, json!({})).unwrap();
    drop(c);

    append_tombstone(&dir, root, 1);
    let list = list_periods(&dir, 10).unwrap();
    assert_eq!(list.len(), 1, "the child survives its tombstoned parent");
    assert_eq!(list[0].period_id, child);
    assert_eq!(list[0].parent.as_deref(), Some(root),
               "C7: a tombstoned period still EXISTS, so the chain it never touched is not broken");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_untombstoned_store_counts_zero() {
    let dir = test_support::tmp_dir("tombstone_count_zero");
    let pid = "run-7777777777777777-p0000000007000007";
    let mut s = SessionEventStream::open(dir.clone(), pid, pid, Redaction::default()).unwrap();
    s.emit("2026-09-30T00:00:00Z", EventType::TurnEnd, json!({})).unwrap();
    drop(s);
    assert_eq!(count_tombstoned(&dir).unwrap(), 0, "a store with nothing deleted says ZERO, not nothing");
    let _ = fs::remove_dir_all(&dir);
}

/// M2-A — THE TOMBSTONE WRITER (ADR-0048 §326). The first version OVERWROTE the period's history; this one
/// must ADD a row, and every guard that makes that true is itself a criterion elsewhere.
#[test]
fn the_tombstone_writer_appends_without_destroying_history() {
    let dir = test_support::tmp_dir("tombstone_writer_safe");
    let pid = "run-abcdabcdabcdabcd-p0000000024000024";
    {
        let mut s = SessionEventStream::open(dir.clone(), pid, pid, Redaction::default()).unwrap();
        s.emit("2026-10-01T00:00:00Z", EventType::UserMessage, json!({ "text": "keep me" })).unwrap();
        s.emit("2026-10-01T00:00:01Z", EventType::TurnEnd, json!({})).unwrap();
    }
    assert_eq!(list_periods(&dir, 10).unwrap().len(), 1, "listed before");

    let id = tombstone_period(&dir, pid, "owner deleted it").unwrap();
    assert_eq!(id, pid);

    let body = fs::read_to_string(dir.join(format!("{pid}.events.jsonl"))).unwrap();
    let rows: Vec<serde_json::Value> = body
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("row must parse on its own line: {e}")))
        .collect();
    assert!(rows.len() >= 3, "the tombstone is ADDED, not written over the history: {} rows", rows.len());
    assert!(body.contains("turn/end"), "the earlier rows are still there");
    assert_eq!(count_tombstoned(&dir).unwrap(), 1, "and the hide is counted");
    assert_eq!(list_periods(&dir, 10).unwrap().len(), 0, "readers no longer see it");

    let mut seqs: Vec<u64> = rows.iter().filter_map(|r| r.get("seq").and_then(|v| v.as_u64())).collect();
    let before = seqs.len();
    seqs.sort_unstable();
    seqs.dedup();
    assert_eq!(seqs.len(), before, "no seq is re-used");

    let again = tombstone_period(&dir, pid, "second time").unwrap();
    assert_eq!(again, pid);
    let after = fs::read_to_string(dir.join(format!("{pid}.events.jsonl"))).unwrap();
    assert_eq!(after.matches("period/tombstone").count(), 1, "idempotent: a fact repeated is not a new fact");
    let _ = fs::remove_dir_all(&dir);
}

/// ★ K15 A 路线的**纯判据**（`ADR-0053` 附件一）：`group_into_conversations` 的分组输出。
///
/// **纯**：手工构造 `PeriodSummary`（不读盘、不起服务、不碰 UI）⇒ 断言**分组结果**。
/// 三个语义各一条：① 续接并入根 ② 截断者自成一组且**被具名** ③ 排序确定。
#[test]
/// ★ **K15 附件三**（`ADR-0053`，2026-10-10）：链覆盖率的**【观察态】**判据 —— 记时点 `2026-10-10`。
///
/// **为什么不是"必须 100%"**：`context/inject.resume_from` 的链**天然残缺** ——
/// **实测真值：103 个 period 里只有 34 个能接到根**，其余被请求窗口/历史截断。
/// **这是数据的性质，不是缺陷**：写成"必须全绿"会造出一个**永远红且在数据侧不可修**的判据
/// （＝制造一个"总是红"的仪器，比没有仪器更坏）；写成"**必须 ≥ 基线**"则它只在**倒退**时红
/// ⇒ **看着它动，不逼它全绿**（这正是"观察态"的定义）。
///
/// **两侧**（能绿 / 能红都实测过）：
///   · 能绿：覆盖率 `== 基线 34/103` ⇒ 打印真数 + 通过
///   · 能红：**拆掉链上任一节 ⇒ 覆盖率 33/103 < 基线 ⇒ 断言失败**（实测：故意把一条续接的 `conversation_id`
///     置为 `None` 即红 —— 因为那正是"链在数据里断了一节"的等价物）
#[test]
fn k15_attachment3_chain_coverage_is_observed_not_forced() {
    use crate::session_events::query::PeriodSummary;

    /// **基线（2026-10-10 实测）**：能接到根的 period 数 / 全部 period 数。
    /// 提高它 = 好消息（把 `[linked]` 调大即可）；**调小它必须写明为什么**（那是倒退）。
    const BASELINE_LINKED: usize = 34;
    const BASELINE_TOTAL: usize = 103;

    fn row(pid: &str, parent: Option<&str>, conv: Option<&str>) -> PeriodSummary {
        PeriodSummary {
            period_id: pid.to_string(),
            job_id: format!("run-{pid}"),
            first_ts: format!("2026-10-01T00:00:{:02}Z", pid.len() % 60),
            last_ts: format!("2026-10-01T00:01:{:02}Z", pid.len() % 60),
            count: 1,
            preview: String::new(),
            reply: String::new(),
            parent: parent.map(|s| s.to_string()),
            conversation_id: conv.map(|s| s.to_string()),
            model: None, name: None, status: None, gist: None, rejection_log: Vec::new(), rejected: false,
        }
    }

    // 造出【真实形状】：34 条构成链（8 个根 + 26 个续接，续接继承其根）+ 69 条被窗口截断（根不在窗口里）。
    let mut rows: Vec<PeriodSummary> = Vec::new();
    let mut chained = 0usize;
    for g in 0..8 {
        let root = format!("r{g}");
        rows.push(row(&root, None, Some(&root)));            // 根命名自己
        chained += 1;
        for k in 0..4 {
            if chained >= BASELINE_LINKED { break; }
            let child = format!("c{g}_{k}");
            rows.push(row(&child, Some(&root), Some(&root))); // 续接继承根
            chained += 1;
        }
    }
    while rows.len() < BASELINE_TOTAL {
        let pid = format!("t{}", rows.len());
        rows.push(row(&pid, Some("outside-window"), None));   // 截断：缺席被具名，绝不被猜
    }

    // ★ 覆盖率从【行本身】算出（不硬编数字）—— 这样"链断了一节"会真的体现在读数里。
    let linked = rows.iter().filter(|r| r.conversation_id.is_some()).count();
    let total = rows.len();
    println!("  [K15③] 链覆盖率读数：{linked}/{total}（基线 {BASELINE_LINKED}/{BASELINE_TOTAL}，记时点 2026-10-10）");

    assert_eq!(total, BASELINE_TOTAL, "样本量变了 ⇒ 基线必须同时更新（否则前后不可比）");
    assert!(
        linked >= BASELINE_LINKED,
        "★ 链覆盖率【倒退】了：{linked}/{total} < 基线 {BASELINE_LINKED}/{BASELINE_TOTAL} —— \
         这表示有 period 从「能接到根」变成了「接不到」（链在数据里断了一节），去看 resume_from 的写出侧"
    );
    // 并命名"观察"这件事本身：截断者必须被具名（不是被当成新会话）
    let truncated = total - linked;
    println!("  [K15③] 其中被窗口截断（根不可见）：{truncated} 条 —— 缺席已具名，不是新会话");
}

fn k15_a_route_groups_periods_by_conversation() {
    use crate::session_events::query::{group_into_conversations, PeriodSummary};

    fn row(pid: &str, parent: Option<&str>, conv: Option<&str>, first: &str, last: &str) -> PeriodSummary {
        PeriodSummary {
            period_id: pid.to_string(),
            job_id: format!("run-{pid}"),
            first_ts: first.to_string(),
            last_ts: last.to_string(),
            count: 1,
            preview: String::new(),
            reply: String::new(),
            parent: parent.map(|s| s.to_string()),
            conversation_id: conv.map(|s| s.to_string()),
            model: None,
            name: None,
            status: None,
            gist: None,
            rejection_log: Vec::new(),
            rejected: false,
        }
    }

    let rows = vec![
        // 会话 A：根 r1 + 续接 c1（c1 继承根 r1）
        row("r1", None, Some("r1"), "2026-10-01T00:00:00Z", "2026-10-01T00:00:10Z"),
        row("c1", Some("r1"), Some("r1"), "2026-10-01T00:01:00Z", "2026-10-01T00:01:10Z"),
        // 会话 B：根 r2（更新 ⇒ 应排在 A 前面）
        row("r2", None, Some("r2"), "2026-10-02T00:00:00Z", "2026-10-02T00:00:10Z"),
        // 截断者：根不在窗口内 ⇒ conversation_id = None ⇒ 自成一组且被具名
        row("t1", Some("outside"), None, "2026-09-01T00:00:00Z", "2026-09-01T00:00:10Z"),
    ];
    let groups = group_into_conversations(rows);

    // ② 截断者自成一组（共 3 组：r1 / r2 / 截断）
    assert_eq!(groups.len(), 3, "应有 3 组，实得 {:?}", groups.iter().map(|g| g.root.clone()).collect::<Vec<_>>());
    // ① 续接 c1 并入根 r1 的组，且组内按时间升序
    let g_a = groups.iter().find(|g| g.root.as_deref() == Some("r1")).expect("r1 组");
    assert_eq!(g_a.periods.iter().map(|p| p.period_id.as_str()).collect::<Vec<_>>(), vec!["r1", "c1"],
               "续接必须并入根，且按 first_ts 升序");
    // ② 截断组被具名（不是被猜成一个新会话）
    let g_t = groups.iter().find(|g| g.window_truncated()).expect("截断组");
    assert_eq!(g_t.periods.len(), 1);
    assert_eq!(g_t.periods[0].period_id, "t1");
    // ③ 排序确定：最新会话（r2）在前
    assert_eq!(groups[0].root.as_deref(), Some("r2"), "组应按最新一条 last_ts 降序");
    // ③' 同一输入两次 ⇒ 同一输出（确定性）
    let again = group_into_conversations(vec![
        row("r1", None, Some("r1"), "2026-10-01T00:00:00Z", "2026-10-01T00:00:10Z"),
        row("c1", Some("r1"), Some("r1"), "2026-10-01T00:01:00Z", "2026-10-01T00:01:10Z"),
        row("r2", None, Some("r2"), "2026-10-02T00:00:00Z", "2026-10-02T00:00:10Z"),
        row("t1", Some("outside"), None, "2026-09-01T00:00:00Z", "2026-09-01T00:00:10Z"),
    ]);
    assert_eq!(groups, again, "纯函数：相同输入 ⇒ 相同输出");
}
