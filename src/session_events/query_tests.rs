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
