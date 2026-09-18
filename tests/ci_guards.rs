//! CI-7 — **not part of `cargo test`, on purpose.**
//!
//! CI-7 proves each guard test can go red by mutating the source and rebuilding.
//! That is incompatible with running inside the suite it mutates. While it held a
//! mutation in `src/run_cycle/mod.rs`, `cargo test` was free to rebuild
//! `tests/run_cycle_pipeline.rs` from the mutated tree, and a full-suite run failed
//! in `run_cycle_deterministic_replay` for no reason of its own. A second run
//! passed. That is the shape this ledger keeps recording: not a wrong answer, an
//! intermittent one, which is worse because it trains you to re-run until green.
//!
//! So the check is a separate gate:
//!
//! ```text
//! python3 ci/check_guards.py          # all guards
//! python3 ci/check_guards.py --skip-slow
//! ```
//!
//! The test below is `#[ignore]`d rather than deleted so the reason is discoverable
//! from inside the suite, and so it can be run deliberately on a quiet tree:
//!
//! ```text
//! cargo test --test ci_guards -- --ignored --test-threads=1
//! ```
//!
//! **Upgrade path** to bring it back into the default suite: run it against a
//! `git worktree` — mutate the copy, build and test there, leave this tree alone.
//! That costs one full compile per run, which is why it is not done yet, and it is
//! the honest price of "prove the guard can fail" if it must run alongside
//! everything else.

use std::path::Path;
use std::process::Command;

#[test]
#[ignore = "mutates src/ and rebuilds; must not run concurrently with the rest of the suite. Run: python3 ci/check_guards.py"]
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
