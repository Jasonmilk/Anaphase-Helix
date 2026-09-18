//! CI-7 — the guard manifest, run by the same command as everything else.
//!
//! The checker is `ci/check_guards.py`; this exists so `cargo test` is the single
//! entry point and so a checker that CANNOT RUN is not read as a checker that
//! passed. Same reason `tests/ci_line_budget.rs` exists, and the same exit-code
//! split: 0 ran and clean, 1 ran and found decoration, 3 could not run.
//!
//! It is slow on purpose. Proving a guard is real means running its test twice —
//! once mutated, once not — so CI-7 costs seconds per guard. That is the price of
//! not shipping a suite of tests that have only ever agreed with themselves.

use std::path::Path;
use std::process::Command;

#[test]
fn every_guard_is_reddened_by_its_declared_mutation() {
    let root = env!("CARGO_MANIFEST_DIR");
    let script = Path::new(root).join("ci/check_guards.py");
    let out = Command::new("python3")
        .arg(&script)
        .current_dir(root)
        .output()
        .unwrap_or_else(|e| panic!("cannot run {}: {e}", script.display()));
    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    match code {
        0 => {}
        1 => panic!("CI-7: guard(s) are decoration:\n{stdout}{stderr}"),
        3 => panic!("CI-7 CHECKER ERROR — it did not run, so it judged nothing:\n{stdout}{stderr}"),
        other => panic!("CI-7 unexpected exit {other}:\n{stdout}{stderr}"),
    }
}
