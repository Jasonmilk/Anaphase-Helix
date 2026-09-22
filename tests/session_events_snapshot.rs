//! A behaviour snapshot for the session event stream.
//!
//! Why: refactors are the one change where "the tests still pass" is weakest.
//! The suite covers what someone thought to assert; a move covers every line,
//! including the ones nobody asserted. This file pins the BYTES the stream
//! writes for a fixed sequence of events, so a pure move has to produce
//! byte-identical output and any difference has to be explained.
//!
//! Determinism comes from the injected clock — `ts()` is a constant, not
//! `SystemTime::now()` — so the same events always serialise to the same bytes.
//! That is what makes a snapshot possible here at all, and it was worth checking
//! before attempting the same thing on a larger file (a 1500-NCLOC state machine
//! has no such luxury).
//!
//! The golden is not restated in this file. A test that hard-codes its own copy
//! of the expected output keeps passing after the golden drifts, which defeats
//! the point.

use std::fs;
use std::path::{Path, PathBuf};

use anaphase::session_events::{EventType, SessionEventStream};
use anaphase::trace::Redaction;
use serde_json::json;

const GOLDEN: &str = "tests/golden/session_events_stream.jsonl";

fn ts() -> String {
    anaphase::ledger::unix_secs_to_rfc3339(1_700_000_000)
}

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("se-snapshot-{}-{}", std::process::id(), tag));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

/// One fixed period: an id, a handle, and a sequence of events that exercises
/// every branch the stream has — the header pair, a context inject with a
/// parent and a choice, a usage row, a tool pair, a verdict and the ending.
fn write_fixed_period(dir: &Path) -> Vec<u8> {
    let mut stream = SessionEventStream::open(
        dir.to_path_buf(),
        "run-aaaa1111-p00000001",
        "run-aaaa1111",
        Redaction::default(),
    )
    .unwrap();
    let t = ts();
    stream.emit_period_start(
        &t,
        "用计算器算 7 的 9 次方",
        2,
        800,
        Some("run-0000beef-p00000001"),
        Some(&json!({ "chosen": "memory", "why": "fixture" })),
        // None, not 0: this fixture does not measure injection, and absent must
        // stay distinct from measured-zero.
        None,
    )
    .unwrap();
    stream
        .emit(&t, EventType::Usage, json!({ "prompt_tokens": 1511, "completion_tokens": 50, "cached_tokens": null }))
        .unwrap();
    stream
        .emit(&t, EventType::ToolCall, json!({ "tool": "numbers", "index": 0, "expect": "numbers" }))
        .unwrap();
    stream
        .emit(
            &t,
            EventType::ToolResult,
            json!({ "tool": "numbers", "index": 0, "ok": true, "duration_ms": 3, "outcome": "40353607" }),
        )
        .unwrap();
    stream
        .emit(&t, EventType::Verdict, json!({ "job_id": "run-aaaa1111", "status": "Met", "reason": "ok" }))
        .unwrap();
    stream
        .emit(&t, EventType::TurnEnd, json!({ "done": true, "success": true }))
        .unwrap();
    fs::read(stream.path()).unwrap()
}

fn golden_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(GOLDEN)
}

#[test]
fn the_stream_writes_the_byte_identical_snapshot() {
    let dir = scratch("fixed");
    let produced = write_fixed_period(&dir);
    let path = golden_path();
    // Regeneration is explicit and never silent: a snapshot that rewrites
    // itself on failure would agree with whatever the code does, which is the
    // opposite of a snapshot.
    if std::env::var("REGEN_SNAPSHOT").is_ok() {
        fs::write(&path, &produced).unwrap();
        eprintln!("regenerated {}", path.display());
        return;
    }
    let golden = fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}\n\
             First run, or a deliberate behaviour change: regenerate with \
             REGEN_SNAPSHOT=1 and say why in the commit message.",
            path.display()
        )
    });
    assert_eq!(
        String::from_utf8_lossy(&produced),
        String::from_utf8_lossy(&golden),
        "the stream's bytes changed; a pure move must not do that"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// The snapshot must be able to fail. A golden nobody can break is decoration —
/// the same defect as a waiver nobody reads or a vector nobody loads.
#[test]
fn the_snapshot_notices_a_changed_byte() {
    let dir = scratch("mutated");
    let mut produced = write_fixed_period(&dir);
    // Flip one byte in the middle of the payload: smallest possible behaviour
    // change that is still a change.
    let mid = produced.len() / 2;
    produced[mid] = if produced[mid] == b'x' { b'y' } else { b'x' };
    // Compared against the UNMUTATED bytes of this same run, so the assertion
    // holds whether or not a golden has been generated yet.
    let clean = write_fixed_period(&scratch("clean"));
    assert_ne!(
        produced, clean,
        "the comparison cannot distinguish a mutated stream from the original, so \
         the snapshot above proves nothing"
    );
    let _ = fs::remove_dir_all(&dir);
}
