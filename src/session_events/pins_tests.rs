//! C14b's criteria (ADR-0048 §330): a holder COUNT that survives a restart, an owner that must be
//! declarative, and orphans that are named rather than silently protective or silently collected.

use super::*;

#[test]
fn pin_refcounts_add_and_survive_a_reopen() {
    let dir = test_support::tmp_dir("pins_reopen");
    pins::pin(&dir, "obj", "panel").unwrap();
    pins::pin(&dir, "obj", "tui").unwrap();
    let (by_object, by_holder) = pins::replay(&dir).unwrap();
    assert_eq!(by_object.get("obj").copied(), Some(2), "two holders ⇒ count 2 (C14: a count, not a set)");
    assert_eq!(by_holder.get(&("obj".to_string(), "panel".to_string())).copied(), Some(1));
    /* RE-READING IS THE RESTART: the state is the WAL, so nothing is lost when the process is gone. */
    let (again, _) = pins::replay(&dir).unwrap();
    assert_eq!(again.get("obj").copied(), Some(2), "the count survives a restart (§330 C14b)");
    pins::unpin(&dir, "obj", "panel").unwrap();
    let (after, _) = pins::replay(&dir).unwrap();
    assert_eq!(after.get("obj").copied(), Some(1), "one holder released, the other still holds");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_per_process_looking_owner_is_refused_by_name() {
    for bad in ["", "550e8400-e29b-41d4-a716-446655440000", "0123456789abcdef0123", "owner with spaces"] {
        let err = check_owner(bad).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("invalid pin owner"), "the refusal NAMES itself: {msg}");
    }
    let err = check_owner("0123456789abcdef0123").unwrap_err().to_string();
    assert!(err.contains("restart-stable"), "and explains WHY a UUID-shaped owner is refused: {err}");
    assert!(check_owner("panel").is_ok() && check_owner("m5-shell").is_ok() && check_owner("tui").is_ok(),
            "declarative names are accepted");
}

#[test]
fn an_undeclared_owner_is_named_as_an_orphan_and_still_protects() {
    let dir = test_support::tmp_dir("pins_orphan");
    pins::pin(&dir, "obj", "panel").unwrap();
    std::fs::write(dir.join(".pins").join(".owners"), "tui\n").unwrap();   /* `panel` is NOT declared alive */
    let orphans = pins::orphan_pins(&dir).unwrap();
    assert_eq!(orphans, vec![("obj".to_string(), "panel".to_string())], "the orphan is NAMED");
    let (by_object, _) = pins::replay(&dir).unwrap();
    assert_eq!(by_object.get("obj").copied(), Some(1), "and it still PROTECTS (a named leak beats a silent loss)");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn releasing_an_orphan_is_explicit_and_reported() {
    let dir = test_support::tmp_dir("pins_release");
    pins::pin(&dir, "obj", "panel").unwrap();
    assert_eq!(pins::release_orphan_owner(&dir, "panel").unwrap(), 1, "one holding released");
    let (by_object, _) = pins::replay(&dir).unwrap();
    assert_eq!(by_object.get("obj").copied(), None, "the object is no longer held");
    assert_eq!(pins::release_orphan_owner(&dir, "panel").unwrap(), 0, "a second release is a NAMED absence");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_pin_wal_is_internal_and_never_read_as_a_period() {
    let dir = test_support::tmp_dir("pins_internal");
    let pid = "run-aaaabbbbccccdddd-p0000000040000040";
    {
        let mut s = SessionEventStream::open(dir.clone(), pid, pid, crate::trace::Redaction::default()).unwrap();
        s.emit("2026-10-01T00:00:00Z", EventType::TurnEnd, serde_json::json!({})).unwrap();
    }
    pins::pin(&dir, pid, "panel").unwrap();
    assert!(dir.join(".pins").join("pins.events.jsonl").is_file(), "the pin WAL has its own file, in its own subdirectory");
    let listed: Vec<String> = list_periods(&dir, 10).unwrap().into_iter().map(|p| p.period_id).collect();
    assert_eq!(listed, vec![pid.to_string()], "and it is NOT listed as a period (internal, not protocol)");
    let _ = std::fs::remove_dir_all(&dir);
}
