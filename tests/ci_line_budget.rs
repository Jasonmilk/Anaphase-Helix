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

// --------------------------------------------------------------- fix windows
//
// The third class. A waiver is a DEBT ("I will repay it") and carries owner+due;
// a split window is a REFACTOR allowance ("the structure changed") and carries a
// cap plus an ADR; a fix is neither — its legitimacy is the pit record it closes.
//
// These run against scratch trees rather than the real repo, because the rules
// are about the checker's own inputs and the real tree is only ever in one of the
// states that matter. `run_checker_at` exists for that.

fn run_checker_at(root: &Path, args: &[&str]) -> (i32, String, String) {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("ci/check_line_budget.py");
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

/// Build a throwaway tree: `src/` files with exactly `n` code lines each, a
/// generated ratchet, and whatever allowance text the test wants to exercise.
fn scratch(name: &str, files: &[(&str, usize)], base: &[(&str, usize)], allowance: &str) -> std::path::PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("ci6-scratch")
        .join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("ci")).expect("scratch ci/");
    for (rel, n) in files {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).expect("scratch parent");
        // Content is never parsed as Rust by the checker, only counted, so the
        // line has to be code and nothing more.
        let body: String = (0..*n).map(|i| format!("let x{i} = 1;\n")).collect();
        std::fs::write(&path, body).expect("scratch file");
    }
    // The version comes from the checker rather than being written here, so a bump
    // cannot leave the fixtures silently measuring under a stale ruler — which is
    // exactly the failure the versioning exists to prevent, and it fired on these
    // fixtures the moment it was added.
    let version = Command::new("python3")
        .arg("-c")
        .arg(format!(
            "import importlib.util;\
             s = importlib.util.spec_from_file_location('clb', {root:?} + '/ci/check_line_budget.py');\
             m = importlib.util.module_from_spec(s); s.loader.exec_module(m);\
             print(m.COUNTER_VERSION)",
            root = env!("CARGO_MANIFEST_DIR")
        ))
        .output()
        .expect("counter version");
    let mut ratchet = format!("counter = {}\n", String::from_utf8_lossy(&version.stdout).trim());
    for (rel, n) in base {
        ratchet.push_str(&format!("[[ratchet]]\ntarget = \"{rel}\"\nncloc  = {n}\n"));
    }
    std::fs::write(root.join("ci/ratchet.gen.toml"), ratchet).expect("scratch ratchet");
    std::fs::write(root.join("ci/baseline.toml"), allowance).expect("scratch baseline");
    // A permissive ledger, so a test about the CAP is not also a test about the
    // citation. Tests that are about the citation call `scratch_with_pits` and
    // supply their own.
    std::fs::write(
        root.join("ci/pits.toml"),
        "[[pit]]\nid = \"K-999\"\nrecorded = \"2026-01-01\"\nsummary = \"scratch\"\n",
    )
    .expect("scratch pits");
    root
}

/// The load-bearing half: a fix window must clear a FILE-level red, which is the
/// one thing a split allowance deliberately cannot do.
///
/// The mutation is inside the test. Between the control and the fix window the
/// only difference is the allowance, so if the last assertion ever passes without
/// the entry having been read, the control above it goes red instead — the test
/// cannot pass by the checker ignoring its inputs.
#[test]
fn a_fix_window_clears_a_file_red_that_a_split_window_cannot() {
    let files = [("src/a.rs", 110)];
    let base = [("src/a.rs", 100)];

    // Control: +10 over the baseline is red, and red as a FILE.
    let root = scratch("fixwin-red", &files, &base, "");
    let (code, out, err) = run_checker_at(&root, &[]);
    assert_eq!(code, 1, "the growth must be caught first:\n{out}{err}");
    assert!(out.contains("file(s) grew"), "and named as a file:\n{out}");

    // A split window must NOT clear it: it loosens a directory aggregate while
    // every single file stays pinned (D5 semantics 2). This is the assertion
    // that makes the third class load-bearing rather than decorative.
    let split = "[[split_window]]\ntarget=\"src/\"\nallowance=100\nreason=\"x\"\ndue=\"2099-01-01\"\n";
    let root = scratch("fixwin-split", &files, &base, split);
    let (code, out, err) = run_checker_at(&root, &[]);
    assert_eq!(code, 1, "a split allowance must not loosen a single file:\n{out}{err}");

    // The fix window does clear it, because a fix adds lines to a named file.
    let fix = "[[fix_window]]\ntarget=\"src/a.rs\"\ncap=10\nk_id=\"K-999\"\nreason=\"x\"\n";
    let root = scratch("fixwin-green", &files, &base, fix);
    let (code, out, err) = run_checker_at(&root, &[]);
    assert_eq!(code, 0, "a fix window with the measured cap must clear it:\n{out}{err}");
}

/// A cap is a cap: one line past it is still red.
#[test]
fn a_fix_window_does_not_excuse_growth_past_its_cap() {
    let fix = "[[fix_window]]\ntarget=\"src/a.rs\"\ncap=9\nk_id=\"K-999\"\nreason=\"x\"\n";
    let root = scratch("fixwin-over", &[("src/a.rs", 110)], &[("src/a.rs", 100)], fix);
    let (code, out, err) = run_checker_at(&root, &[]);
    assert_eq!(code, 1, "cap 9 against a growth of 10 must still fail:\n{out}{err}");
    assert!(out.contains("+9 fix allowance"), "and must show the cap it applied:\n{out}");
}

/// The allowance binds to the target it names and nothing else. Spreading it
/// would hand slack to files the fix never touched.
#[test]
fn a_fix_window_does_not_spread_to_a_neighbouring_file() {
    let fix = "[[fix_window]]\ntarget=\"src/a.rs\"\ncap=10\nk_id=\"K-999\"\nreason=\"x\"\n";
    let root = scratch(
        "fixwin-spread",
        &[("src/a.rs", 110), ("src/b.rs", 110)],
        &[("src/a.rs", 100), ("src/b.rs", 100)],
        fix,
    );
    let (code, out, err) = run_checker_at(&root, &[]);
    assert_eq!(code, 1, "b.rs was never named and must stay red:\n{out}{err}");
    assert!(out.contains("src/b.rs"), "and the report must name it:\n{out}");
    assert!(!out.contains("src/a.rs  "), "a.rs is covered and must not be listed:\n{out}");
}

/// A fix carrying a deadline is a debt wearing the wrong label, and the entry is
/// refused rather than honoured. Exit 3, not 1: a malformed baseline is "the
/// checker could not run", never "the code has violations".
#[test]
fn a_fix_window_with_a_due_date_is_refused_as_a_mislabeled_debt() {
    let bad = "[[fix_window]]\ntarget=\"src/a.rs\"\ncap=10\nk_id=\"K-999\"\ndue=\"2099-01-01\"\nreason=\"x\"\n";
    let root = scratch("fixwin-due", &[("src/a.rs", 10)], &[("src/a.rs", 10)], bad);
    let (code, _, err) = run_checker_at(&root, &[]);
    assert_eq!(code, 3, "must be refused as unrunnable, not reported as a violation:\n{err}");
    assert!(err.contains("not a debt"), "and must say why:\n{err}");
}

/// Without a pit record the entry is a waiver in disguise, and it is refused.
#[test]
fn a_fix_window_without_a_pit_id_is_refused_as_a_disguised_waiver() {
    let bad = "[[fix_window]]\ntarget=\"src/a.rs\"\ncap=10\nreason=\"x\"\n";
    let root = scratch("fixwin-nokid", &[("src/a.rs", 10)], &[("src/a.rs", 10)], bad);
    let (code, _, err) = run_checker_at(&root, &[]);
    assert_eq!(code, 3, "must be refused as unrunnable:\n{err}");
    assert!(err.contains("k_id"), "and must name what is missing:\n{err}");
}

/// The `src/` tree needs a pit ledger to read. Scratch trees build their own.
fn scratch_with_pits(name: &str, files: &[(&str, usize)], base: &[(&str, usize)], allowance: &str, pits: &str) -> std::path::PathBuf {
    let root = scratch(name, files, base, allowance);
    std::fs::write(root.join("ci/pits.toml"), pits).expect("scratch pits");
    root
}

/// A fix window is the class that reaches FILES, so it is the more abusable one:
/// any change can be called a fix. The cap is bounded by the target's own
/// baseline — at most 10% of it — so the bound is not the author's own number.
///
/// The fixtures stay under the 300-line per-file budget on purpose: at 1000 the
/// `over_budget` path fires first and the ratio rule is never reached, so the
/// test would pass for the wrong reason.
// guards: fix-cap-ratio
#[test]
fn a_fix_window_cap_may_not_exceed_a_tenth_of_the_baseline() {
    let pits = "[[pit]]\nid = \"K-999\"\n";
    // 100 into 1000 is exactly a tenth: allowed.
    let ok = "[[fix_window]]\ntarget=\"src/a.rs\"\ncap=10\nk_id=\"K-999\"\nreason=\"x\"\n";
    let root = scratch_with_pits("fixcap-ok", &[("src/a.rs", 110)], &[("src/a.rs", 100)], ok, pits);
    let (code, _, err) = run_checker_at(&root, &[]);
    assert_eq!(code, 0, "a tenth is the bound, so it must pass:\n{err}");

    // 101 is over it: refused as unrunnable, not reported as a violation.
    let over = "[[fix_window]]\ntarget=\"src/a.rs\"\ncap=11\nk_id=\"K-999\"\nreason=\"x\"\n";
    let root = scratch_with_pits("fixcap-over", &[("src/a.rs", 110)], &[("src/a.rs", 100)], over, pits);
    let (code, _, err) = run_checker_at(&root, &[]);
    assert_eq!(code, 3, "over the ratio must be refused:\n{err}");
    assert!(err.contains("must not exceed 10"), "and must name the bound:\n{err}");
}

/// The floor exists so a small file is not barred from citing any fix at all:
/// 10% of a 30-line file is 3, which is not a usable allowance.
#[test]
fn a_fix_window_cap_has_a_floor_for_small_files() {
    let pits = "[[pit]]\nid = \"K-999\"\n";
    let maxed = "[[fix_window]]\ntarget=\"src/a.rs\"\ncap=5\nk_id=\"K-999\"\nreason=\"x\"\n";
    let root = scratch_with_pits("fixcap-floor", &[("src/a.rs", 35)], &[("src/a.rs", 30)], maxed, pits);
    let (code, _, err) = run_checker_at(&root, &[]);
    assert_eq!(code, 0, "the floor is 5 for a 30-line file:\n{err}");
}

/// An id nobody can resolve is a waiver wearing a fix's label.
#[test]
fn a_fix_window_citing_an_unrecorded_pit_is_refused() {
    let pits = "[[pit]]\nid = \"K-111\"\n";
    let bad = "[[fix_window]]\ntarget=\"src/a.rs\"\ncap=10\nk_id=\"K-999\"\nreason=\"x\"\n";
    let root = scratch_with_pits("fixpit-unknown", &[("src/a.rs", 110)], &[("src/a.rs", 100)], bad, pits);
    let (code, _, err) = run_checker_at(&root, &[]);
    assert_eq!(code, 3, "an unresolvable citation must be refused:\n{err}");
    assert!(err.contains("K-999"), "and must name the id:\n{err}");
    assert!(err.contains("does not list"), "and say where it looked:\n{err}");
}

/// A target with no baseline has no ratio to compute, so the ratio rule cannot
/// bar it — but the id rule still applies. Asserted so the `base is None` path is
/// not silently a way past the citation requirement.
#[test]
fn an_unknown_target_still_has_to_cite_a_recorded_pit() {
    let pits = "[[pit]]\nid = \"K-111\"\n";
    let bad = "[[fix_window]]\ntarget=\"src/new.rs\"\ncap=9999\nk_id=\"K-999\"\nreason=\"x\"\n";
    let root = scratch_with_pits(
        "fixpit-newtarget",
        &[("src/a.rs", 10)],
        &[("src/a.rs", 10)],
        bad,
        pits,
    );
    let (code, _, err) = run_checker_at(&root, &[]);
    assert_eq!(code, 3, "no baseline does not waive the citation:\n{err}");
    assert!(err.contains("does not list"), "{err}");
}

/// The parser must not read the sections before it. Writing a `[[waiver]]` above
/// a `[[fix_window]]` first read the waiver as a fix window — it reset on the
/// `[` line, then the waiver's own `target` and `due` lines refilled the fields,
/// so a legitimate waiver was reported as "a fix window with a due date". That is
/// the mirror of the defect already recorded twice in this checker: not a key
/// that matches nothing, but a section that matches everything.
#[test]
fn a_waiver_above_a_fix_window_is_not_read_as_one() {
    let both = "[[waiver]]\ncheck=\"CI-6\"\ntarget=\"src/a.rs\"\nreason=\"x\"\nowner=\"o\"\ndue=\"2099-01-01\"\n\n\
                [[fix_window]]\ntarget=\"src/b.rs\"\ncap=10\nk_id=\"K-999\"\nreason=\"y\"\n";
    let root = scratch(
        "fixwin-order",
        &[("src/a.rs", 100), ("src/b.rs", 110)],
        &[("src/a.rs", 100), ("src/b.rs", 100)],
        both,
    );
    let (code, out, err) = run_checker_at(&root, &[]);
    assert_eq!(
        code, 0,
        "the waiver must be read as a waiver and the fix window as a fix window:\n{out}{err}"
    );
}


/// The counter's SECOND path has its own fixture.
///
/// The Rust canary proved the counter correct on the path it tested, and the
/// Python path went untested — so every `#` comment in a Python file was counted as
/// code. `ci/check_guards.py` read 343 when it was 311 and
/// `ci/check_line_budget.py` 811 when it was 667. The conclusion "CI-7 is over the
/// 300-line budget" survived; the numbers did not, and they had been written into
/// signed seeds.
///
/// Round 35's framing, which is the right one: a canary safeguards the path it
/// exercises, and the count went down a different one. Same family as a numerator
/// that is right and a denominator that is not.
///
/// The fixture carries the trap on purpose: a line that is only a string containing
/// `#`, and a multi-line docstring whose continuation lines contain neither `#` nor
/// a triple quote. A naive scan gets both wrong.
#[test]
fn the_counter_reports_a_known_value_for_the_python_canary() {
    let root = env!("CARGO_MANIFEST_DIR");
    let out = Command::new("python3")
        .arg("-c")
        .arg(format!(
            "import importlib.util\n\
             s = importlib.util.spec_from_file_location('clb', {root:?} + '/ci/check_line_budget.py')\n\
             m = importlib.util.module_from_spec(s); s.loader.exec_module(m)\n\
             t = open({root:?} + '/ci/canary/ncloc_known.py', encoding='utf-8').read()\n\
             print(m.ncloc(t, 'hash'), m.ncloc(t, 'rust'))"
        ))
        .output()
        .unwrap_or_else(|e| panic!("cannot run the counter: {e}"));
    let got = String::from_utf8_lossy(&out.stdout);
    let mut parts = got.split_whitespace();
    let py: i64 = parts.next().and_then(|x| x.parse().ok()).unwrap_or(-1);
    let rust: i64 = parts.next().and_then(|x| x.parse().ok()).unwrap_or(-1);
    assert_eq!(
        py, 45,
        "the PYTHON path no longer agrees with its canary. 45 = 40 code lines + 1 \
         inline-comment line + 1 bare-string line + 3 docstring lines; the 15 `#` \
         lines do not count. Got {py}."
    );
    assert_eq!(
        rust, 65,
        "reading the Python fixture with the RUST rules must give a visibly different \
         number — that gap is the defect this canary was built for, and if it has \
         closed, the two paths have merged and one of them is now untested. Got {rust}."
    );
}

/// The DATA-ONLY marker is a declaration, not a mention.
///
/// Scanning `ci/` for the first time made the checker mark ITSELF data-only: the
/// marker string appears in `ci/check_line_budget.py` inside the definition of the
/// marker. It then failed itself for having too many branches. The marker is a Rust
/// inner doc attribute, so a Python file cannot carry one, and it must start a line.
#[test]
fn the_data_only_marker_requires_a_declaration_not_a_mention() {
    let root = env!("CARGO_MANIFEST_DIR");
    let out = Command::new("python3")
        .arg("-c")
        .arg(format!(
            "import importlib.util\n\
             s = importlib.util.spec_from_file_location('clb', {root:?} + '/ci/check_line_budget.py')\n\
             m = importlib.util.module_from_spec(s); s.loader.exec_module(m)\n\
             print(m.declares_data_only(open({root:?} + '/src/config.rs', encoding='utf-8').read(), 'rust'),\n\
             \x20     m.declares_data_only(open({root:?} + '/ci/check_line_budget.py', encoding='utf-8').read(), 'py'),\n\
             \x20     m.declares_data_only('let x = \\\"//! DATA-ONLY\\\";', 'rust'))"
        ))
        .output()
        .unwrap_or_else(|e| panic!("cannot run the checker: {e}"));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "True False False",
        "expected: config.rs declares it (True), a Python file cannot (False), and a \
         Rust string that merely MENTIONS the marker must not (False)"
    );
}
