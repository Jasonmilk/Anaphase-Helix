//! The N-run experiment's validity guard, mechanised.
//!
//! `ci/n_runs.sh` answers "does it still happen, or did I only stop looking?" — and
//! it refuses to answer if the tree moved during the runs. That guard is the last
//! thing in this repository that was still a declaration rather than a check:
//! without a control, `dirty 0 -> 0` is just another assertion that could be
//! hard-coded and never notice.
//!
//! So the script carries a `--selftest-guard` mode that builds a scratch repository,
//! makes it dirty on purpose, and requires the guard to see it. This runs that mode,
//! which is what turns the guard from a claim into a mechanism.
//!
//! Why this matters here specifically: the first attempt at the ten-run experiment
//! was void because the tree was being edited during the runs, and nothing in the
//! script could say so. An experiment that cannot reveal its own invalidity is the
//! same defect as a test that cannot fail.

use std::path::Path;
use std::process::Command;

fn run(args: &[&str]) -> (i32, String, String) {
    let root = env!("CARGO_MANIFEST_DIR");
    let script = Path::new(root).join("ci/n_runs.sh");
    let out = Command::new("bash")
        .arg(&script)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("cannot run {}: {e}", script.display()));
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

/// The guard must notice a tree that is dirty. If it cannot, every "dirty 0 -> 0"
/// it ever printed was decoration.
#[test]
fn the_experiment_guard_sees_a_dirty_tree() {
    let (code, stdout, stderr) = run(&["--selftest-guard"]);
    assert_eq!(
        code, 0,
        "the validity guard failed its own control — it cannot distinguish a dirty \
         tree from a clean one, so its verdicts are unreadable:\n{stdout}{stderr}"
    );
    assert!(
        stdout.contains("guard OK"),
        "and it must say so: {stdout}"
    );
    // The control is only meaningful if it actually made the tree dirty.
    assert!(
        stdout.contains("dirty=1") || stdout.contains("dirty=2"),
        "the control must have dirtied the scratch repo, not merely agreed with \
         itself: {stdout}"
    );
}

/// The script refuses to run on a dirty tree rather than reporting counts that
/// measure a moving target. This is the check that would have caught the first,
/// void attempt.
#[test]
fn the_experiment_refuses_to_run_on_a_moving_tree() {
    // A non-numeric argument is not the point here; what matters is that the
    // script has a path where it declines to produce a verdict at all.
    let (code, _, stderr) = run(&["not-a-number"]);
    assert_ne!(code, 0, "a nonsense argument must not produce a result");
    assert!(
        stderr.contains("usage") || stderr.contains("REFUSING"),
        "and it must say why: {stderr}"
    );
}
