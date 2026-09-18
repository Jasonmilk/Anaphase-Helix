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

/// M10 — the checker's own inputs have to be guarded too.
///
/// The failure this exists for: `ratchet.gen.toml` held a directory baseline of
/// 3499 while the directory totalled 3499, so `baseline + allowance` was a bound
/// that could never be reached and the allowance was bookkeeping rather than a
/// gate. It went unnoticed for two commits and was found by measuring, not by the
/// checker — which means nothing guarded the checker's own inputs.
///
/// Exit 3 covers "the checker crashed". This covers "the checker computed the
/// wrong thing and said PASS", which is strictly more dangerous because it is
/// green.
#[test]
fn a_wrong_baseline_is_caught_rather_than_trusted() {
    // Force a directory baseline far below reality. If the check only ever
    // compares reality against whatever the file says, this passes and the
    // ratchet is decorative.
    let (code, stdout, stderr) = run_checker(&["--ratchet-override", "src/=1"]);
    assert_eq!(
        code, 1,
        "a baseline of 1 against a real directory cannot be a pass; the check \
         must fail on its own input being wrong.\n{stdout}{stderr}"
    );
    assert!(
        stdout.contains("directory total") || stderr.contains("directory total"),
        "and it must say which bound was exceeded, not fail generically:\n{stdout}{stderr}"
    );
    assert_ne!(code, 3, "the override must not break the checker itself");
}

/// The override is a test affordance and must not change normal behaviour.
#[test]
fn the_override_does_not_affect_the_real_run() {
    let (code, _, stderr) = run_checker(&[]);
    assert_eq!(code, 0, "the tree is clean: {stderr}");
}

/// The counter itself has a known-answer fixture.
///
/// The reconciliation against tokei (350/350, 168/168, 291/291) was a one-time
/// event. If the counter drifts later — a regex tweak, a new comment form — the
/// baselines would drift with it, because the same script measures, writes the
/// baselines, and checks them. Nothing would report that.
///
/// So `ci/canary/ncloc_known.rs` holds a file whose NCLOC is known by
/// construction: 50 code lines, 60 comment lines, and three lines containing
/// `//` inside string literals — the case a hand-rolled counter gets wrong first.
/// This asserts the checker still reports 52 for it.
#[test]
fn the_counter_reports_a_known_value_for_the_canary() {
    let root = env!("CARGO_MANIFEST_DIR");
    let script = Path::new(root).join("ci/check_line_budget.py");
    let out = Command::new("python3")
        .arg("-c")
        .arg(format!(
            "import sys; sys.path.insert(0, {root:?} + '/ci');\
             from check_line_budget import ncloc;\
             print(ncloc(open({root:?} + '/ci/canary/ncloc_known.rs', encoding='utf-8').read()))"
        ))
        .output()
        .unwrap_or_else(|e| panic!("cannot run the counter: {e}"));
    let got: i64 = String::from_utf8_lossy(&out.stdout).trim().parse().unwrap_or(-1);
    assert_eq!(
        got, 52,
        "the NCLOC counter no longer agrees with the canary. The baselines are \
         written by this same counter, so a drift here silently moves every \
         ratchet with it.\nstdout: {} stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = script;
}
