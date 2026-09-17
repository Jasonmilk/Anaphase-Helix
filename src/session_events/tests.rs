//! Behaviour tests for the stream, identity and naming surface.
//!
//! `mod.rs` declares this module; temp dirs come from `test_support`.

    use super::*;
    use super::naming::NAME_MAX_CHARS;
    use crate::trace::Redaction;
    use serde_json::json;
    use std::fs;

    fn ts() -> String {
        crate::ledger::unix_secs_to_rfc3339(1_700_000_000)
    }

    fn tmp_dir() -> std::path::PathBuf {
        super::test_support::tmp_dir("tests")
    }

    #[test]
    fn crystallize_distills_unmet_into_rule() {
        let dir = tmp_dir().join("distill");
        let redact = Redaction::default();
        let mut stream = SessionEventStream::open(dir.clone(), "run-abc", "run-abc", redact).unwrap();
        let t = ts();
        stream
            .emit(&t, EventType::UserMessage, json!({ "text": "calc 7^9" }))
            .unwrap();
        stream
            .emit(&t, EventType::ToolCall, json!({ "tool": "calc", "index": 0, "expect": "ok" }))
            .unwrap();
        stream
            .emit(
                &t,
                EventType::ToolResult,
                json!({ "tool": "calc", "ok": true, "duration_ms": 276, "outcome": "40353607", "outcome_sha": "abcd1234" }),
            )
            .unwrap();
        stream
            .emit(
                &t,
                EventType::Check,
                json!({ "check_id": "run-xyz#c0", "check": "exec_ok", "passed": false, "judge": "rule", "gate": "hard", "expect": "ok", "evidence_id": "run-xyz#0", "reason": "ok=false or echo mismatch" }),
            )
            .unwrap();
        stream
            .emit(&t, EventType::Verdict, json!({ "job_id": "run-abc", "status": "Unmet", "reason": "failed: exec_ok" }))
            .unwrap();
        stream
            .emit(&t, EventType::TurnEnd, json!({ "done": true, "success": false, "impasse": false, "verdict": "Unmet" }))
            .unwrap();
        drop(stream);

        let out = crystallize(&dir, 50).unwrap();
        assert_eq!(out.len(), 1, "one unmet period -> one suggestion");
        let s = &out[0];
        assert_eq!(s.job_id, "run-abc");
        assert_eq!(s.tool, "calc");
        assert_eq!(s.expect, "ok");
        assert_eq!(s.failed_checks, vec!["exec_ok".to_string()]);
        assert_eq!(s.outcome_shas, vec!["abcd1234".to_string()]);
        assert!(s.suggested_rule.contains("gate=hard judge=rule"));
        assert!(dir.join("crystallized/rule-run-abc.json").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn crystallize_skips_met_periods() {
        use crate::trace::Redaction;
        let dir = tmp_dir().join("skips");
        let redact = Redaction::default();
        let mut stream = SessionEventStream::open(dir.clone(), "run-31e", "run-31e", redact).unwrap();
        let t = ts();
        stream
            .emit(&t, EventType::Verdict, json!({ "job_id": "run-31e", "status": "Met", "reason": "all checks passed" }))
            .unwrap();
        drop(stream);
        let out = crystallize(&dir, 50).unwrap();
        assert!(out.is_empty(), "Met periods are not ore");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn vocabulary_is_stable() {
        assert_eq!(EventType::TurnStart.as_str(), "turn/start");
        assert_eq!(EventType::UserMessage.as_str(), "user/message");
        assert_eq!(EventType::ContextInject.as_str(), "context/inject");
        assert_eq!(EventType::Attempt.as_str(), "assistant/attempt");
        assert_eq!(EventType::ToolCall.as_str(), "tool/call");
        assert_eq!(EventType::ToolResult.as_str(), "tool/result");
        assert_eq!(EventType::Verdict.as_str(), "verdict/status");
        assert_eq!(EventType::AssistantReply.as_str(), "assistant/reply");
        assert_eq!(EventType::Usage.as_str(), "assistant/usage");
        assert_eq!(EventType::TurnEnd.as_str(), "turn/end");
    }

    #[test]
    fn emits_monotonic_seq_and_roundtrips() {
        let dir = tmp_dir();
        let mut stream = SessionEventStream::open(dir.clone(), "run-a1", "run-a1", Redaction::default()).unwrap();
        let t = ts();
        stream.emit(&t, EventType::TurnStart, json!({})).unwrap();
        stream.emit(&t, EventType::UserMessage, json!({ "text": "hi" })).unwrap();
        let rows: Vec<String> = fs::read_to_string(stream.path())
            .unwrap()
            .lines()
            .map(|l| l.to_string())
            .collect();
        assert_eq!(rows.len(), 2);
        let e0: SessionEvent = serde_json::from_str(&rows[0]).unwrap();
        let e1: SessionEvent = serde_json::from_str(&rows[1]).unwrap();
        assert_eq!(e0.seq, 0);
        assert_eq!(e1.seq, 1);
        assert_eq!(e0.event_type, "turn/start");
        assert_eq!(e1.event_type, "user/message");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn period_start_carries_resume_and_choice_detail() {
        let dir = tmp_dir();
        let mut stream = SessionEventStream::open(dir.clone(), "run-c3", "run-c3", Redaction::default()).unwrap();
        let t = ts();
        let detail = json!({
            "tiers": { "L1": 1, "L2": 2, "L3": 5 },
            "top": [{ "id": "n-1", "tier": "L3", "heat": 0.82, "phase": "liquid" }]
        });
        stream
            .emit_period_start(&t, "hello", 8, 800, Some("run-abc"), Some(&detail))
            .unwrap();
        let rows: Vec<SessionEvent> = fs::read_to_string(stream.path())
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(rows.len(), 3);
        let ctx = &rows[2];
        assert_eq!(ctx.event_type, "context/inject");
        assert_eq!(ctx.data["nodes"], 8);
        assert_eq!(ctx.data["chars"], 800);
        assert_eq!(ctx.data["resume_from"], "run-abc");
        assert_eq!(ctx.data["choice"]["tiers"]["L3"], 5);
        assert_eq!(ctx.data["choice"]["top"][0]["tier"], "L3");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_summary_flattens_last_round_as_history() {
        let dir = tmp_dir();
        let mut stream = SessionEventStream::open(dir.clone(), "run-d4", "run-d4", Redaction::default()).unwrap();
        let t = ts();
        stream.emit(&t, EventType::TurnStart, json!({})).unwrap();
        stream.emit(&t, EventType::UserMessage, json!({ "text": "用计算器算 7 的 9 次方" })).unwrap();
        stream.emit(&t, EventType::Attempt, json!({ "text": "我将用确定性工具计算，而不是口算。" })).unwrap();
        let summary = read_summary(&dir, "run-d4", 400).expect("summary");
        assert!(summary.contains("用计算器算 7 的 9 次方"));
        assert!(summary.contains("我将用确定性工具计算"));
        assert!(summary.contains("human said:"));
        assert!(summary.contains("helix answered:"));
        // Missing period -> None (resuming a vanished episode is a no-op).
        assert!(read_summary(&dir, "run-missing", 400).is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn prose_is_never_a_parent_pointer() {
        // Real ids (the shape Anaphase now mints): an ALLOCATED tail, not a
        // plain derived digest.
        let minted = allocate_period_id("run-8bba24c5ee368a4a", 1_760_000_000);
        assert!(is_period_id(&minted), "a minted period id must be a parent: {minted}");
        // A bare derived digest is a REPLAY HANDLE, not an identity. It must not
        // pass: every period of that input shares it, so accepting it as a
        // parent is how one period's lineage silently points at another's.
        assert!(
            !is_period_id("run-8bba24c5ee368a4a"),
            "a content digest is not an identity"
        );
        assert!(is_job_id("run-8bba24c5ee368a4a"), "but it is a valid job id");
        // The two shapes that actually polluted the live stream.
        assert!(
            !is_period_id("human said: 我最喜欢的数字是 7\nhelix answered: 7"),
            "the human-readable continuation summary must never act as a parent"
        );
        assert!(
            !is_period_id("run-adr0043-t5b-verify"),
            "an arbitrary caller-supplied id must not become a parent just because \
             it carries the run- prefix"
        );
        assert!(!is_period_id(""), "empty is not a parent");
        assert!(!is_period_id("run-"), "the prefix alone is not a parent");
    }

    /// B17': the allocator is the chain's only source of identity, so it must
    /// REJECT what it cannot allocate rather than repair it.
    ///
    /// The failure this guards against is not a wrong id but a
    /// valid-LOOKING one: the previous version filtered the hex characters out of
    /// whatever it was given, so `run-abcd&` and `run-abcd` produced the SAME
    /// period id while both looked well-formed. Nothing downstream could tell.
    #[test]
    fn allocation_rejects_rather_than_repairs() {
        let t = 1_760_000_000u64;
        // Criterion 1: non-hex is an explicit failure, never a filtered digest.
        let err = try_allocate_period_id("run-abcd&", t).expect_err("non-hex must fail");
        assert!(err.contains("not hex"), "the reason must be specific: {err}");
        // Criterion 4 (collision): a repaired digest would equal this one.
        let clean = try_allocate_period_id("run-abcd", t).unwrap();
        let dirty = try_allocate_period_id("run-abcd&", t);
        assert!(dirty.is_err(), "a repaired id would collide with {clean}");
        // Criterion 2: nothing hex-like at all must not yield `run--p...`.
        assert!(try_allocate_period_id("run-日本語", t).is_err());
        assert!(try_allocate_period_id("run-", t).is_err());
        assert!(try_allocate_period_id("plainhex", t).is_err(), "the run- prefix is required");
        // Criterion 3: no panic on non-ASCII (the old code byte-sliced).
        assert!(try_allocate_period_id("run-aaa日本語", t).is_err());
        assert!(try_allocate_period_id("run-9f9f9f9f9f9f9f9f9", t).is_err(), "longer than 16 hex");
        // Criterion 4 (uniqueness): same digest, same second, still distinct --
        // and an allocated id is always shaped like one.
        let a = try_allocate_period_id("run-abcd", t).unwrap();
        let b = try_allocate_period_id("run-abcd", t).unwrap();
        assert_ne!(a, b, "two allocations must not share an id");
        assert!(is_period_id(&a) && is_period_id(&b), "allocated ids must be recognisable");
        assert!(a.starts_with("run-abcd-"), "and must carry their digest as a prefix: {a}");
    }

    /// C9 reverse guard: a `job_id` may never be *used* as an identity, even
    /// though the field is present in every row. This is the mistake the K-006
    /// family is made of (a digest standing in for "this run"), so it gets a
    /// test rather than only a comment.
    #[test]
    fn job_id_is_not_an_identity() {
        let dir = tmp_dir().join("not-an-identity");
        fs::create_dir_all(&dir).unwrap();
        let job = "run-bbbb2222";
        let p1 = allocate_period_id(job, 1_760_000_000);
        let p2 = allocate_period_id(job, 1_760_000_001);
        assert_ne!(p1, p2, "two runs of one input must not share an identity");
        for (id, ts) in [(&p1, "t1"), (&p2, "t2")] {
            let mut s = SessionEventStream::open(
                dir.clone(),
                id,
                job,
                Redaction::default(),
            )
            .unwrap();
            s.emit(ts, EventType::UserMessage, json!({ "text": "same input" })).unwrap();
        }
        // Resolving the DIGEST is ambiguous and must say so, not pick one.
        let r = resolve_period(&dir, &PeriodRef::parse(job)).unwrap();
        match r {
            Resolved::Ambiguous(ids) => {
                assert_eq!(ids.len(), 2, "both runs must be listed: {ids:?}");
            }
            other => panic!("a repeated input must not resolve silently: {other:?}"),
        }
        let _ = fs::remove_dir_all(&dir);
    }

    /// The frozen title is content, collapsed and bounded — never empty, never
    /// derived from a clock.
    #[test]
    fn freeze_name_is_bounded_content_not_a_clock() {
        assert_eq!(freeze_name("  记住:   我最喜欢的\n数字是 7  "), Some("记住: 我最喜欢的 数字是 7".to_string()));
        assert_eq!(freeze_name("   "), None, "nothing to freeze");
        assert_eq!(freeze_name(""), None);
        let long = "甲".repeat(NAME_MAX_CHARS + 10);
        let got = freeze_name(&long).unwrap();
        assert_eq!(got.chars().count(), NAME_MAX_CHARS + 1, "bounded + ellipsis");
        assert!(got.ends_with('…'));
        assert_eq!(freeze_name(&"乙".repeat(NAME_MAX_CHARS)).unwrap().chars().count(), NAME_MAX_CHARS, "exactly at the bound needs no ellipsis");
    }

    #[test]
    fn redacts_strings_recursively() {
        let dir = tmp_dir();
        let mut stream = SessionEventStream::open(dir.clone(), "run-b2", "run-b2", Redaction::new(vec!["sk-secret-key-12345".to_string()]))
        .unwrap();
        let t = ts();
        stream
            .emit(
                &t,
                EventType::Attempt,
                json!({ "text": "call sk-secret-key-12345 now", "nested": { "k": "sk-secret-key-12345" } }),
            )
            .unwrap();
        let row: SessionEvent =
            serde_json::from_str(&fs::read_to_string(stream.path()).unwrap()).unwrap();
        let text = row.data["text"].as_str().unwrap();
        let nested = row.data["nested"]["k"].as_str().unwrap();
        assert!(!text.contains("sk-secret-key-12345"), "raw literal leaked: {text}");
        assert!(!nested.contains("sk-secret-key-12345"), "nested literal leaked: {nested}");
        assert!(text.contains("[REDACTED]"), "no redaction marker: {text}");
        let _ = fs::remove_dir_all(&dir);
    }
