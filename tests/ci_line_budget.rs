//! CI-6 — the line budget, run by the same command as everything else.
//!
//! The checker itself is `ci/check_line_budget.py`; this exists so that
//! `cargo test` is the single entry point locally and in CI, and so that a
//! checker that CANNOT RUN is not mistaken for a check that PASSED.
//!
//! That distinction is the whole reason this file exists rather than the script
//! being called from a shell one-liner. The script uses exit code 3 for "I could
//! not run" and 0 for "I ran and the tree is clean"; a wrapper that only tests
//! for non-zero, or only for zero, collapses the two back together. Five
//! separate defects in this family have the same shape — a default value that
//! disguises an absence as a legitimate result.

use std::path::Path;
use std::process::Command;

fn run_checker(args: &[&str]) -> (i32, String, String) {
    let root = env!("CARGO_MANIFEST_DIR");
    let script = Path::new(root).join("ci/check_line_budget.py");
    let out = Command::new("python3")
        .arg(&script)
        .arg("--root")
        .arg(root)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("cannot run {}: {e}", script.display()));
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

#[test]
fn the_line_budget_check_runs_and_passes() {
    let (code, stdout, stderr) = run_checker(&[]);
    match code {
        0 => {}
        1 => panic!("line budget violated:\n{stdout}{stderr}"),
        // Explicitly separated from 1: "the checker could not run" must never be
        // read as "the checker found nothing".
        3 => panic!("CHECKER ERROR — the check did not run, so it judged nothing:\n{stderr}"),
        other => panic!("unexpected exit {other}:\n{stdout}{stderr}"),
    }
}

#[test]
fn the_marker_cannot_be_used_to_sign_its_own_pass() {
    // A DATA-ONLY marker is a claim; the branch budget is what verifies it. If
    // this ever returns 3 the marker lost its verifier, which is the failure
    // worth shouting about — a marker nobody checks is the "AI 无权修改" shape.
    let (code, _, stderr) = run_checker(&["--branch-budget", "0"]);
    assert_ne!(
        code, 0,
        "with a branch budget of 0, a file marked DATA-ONLY that contains branches \
         must fail; a clean pass means the marker is unverified"
    );
    assert_ne!(code, 3, "the DATA-ONLY verifier did not run: {stderr}");
}
