//! CI-7's own invariants, checked by ordinary tests.
//!
//! **Meta-level caps here.** Round 33's point: CI-7 decides whether every guard in
//! this repository is real, and nothing decided whether CI-7 was. Three levels had
//! already stacked — CI-4 checks vectors, CI-7 checks guards, and then CI-7 itself
//! had to be fixed — and the ladder has to stop. So:
//!
//! - CI-7's invariants are checked **here**, by plain tests;
//! - these tests are **not** mutation-guarded, because guarding them would mean a
//!   CI-8, and needing a CI-8 would mean CI-7 is built wrong;
//! - CI-7's *size* is now governed instead: `ci/check_guards.py` is counted by CI-6
//!   and carries a signed seed, so its growth is visible and bounded.
//!
//! That asymmetry is the point. A checker cannot audit itself with the same rigour
//! it audits others without an infinite regress, so its invariants get direct
//! tests and its budget gets the machinery. Pretending otherwise would be the
//! recursion the round-33 review warns against.
//!
//! The checks run `check_guards` in-process through `python3 -c`, the same way the
//! NCLOC canary test reaches `check_line_budget`. No scratch cargo project is
//! needed because the invariants live in the *parsers and predicates*, not in the
//! subprocess plumbing.

use std::process::Command;

/// Run a snippet against the checker module and return its stdout.
fn py(code: &str) -> String {
    let root = env!("CARGO_MANIFEST_DIR");
    let out = Command::new("python3")
        .arg("-c")
        .arg(format!(
            "import sys, importlib.util\n\
             spec = importlib.util.spec_from_file_location('cg', {root:?} + '/ci/check_guards.py')\n\
             cg = importlib.util.module_from_spec(spec); spec.loader.exec_module(cg)\n\
             {code}"
        ))
        .output()
        .unwrap_or_else(|e| panic!("cannot run python3: {e}"));
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        out.status.success(),
        "the snippet failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    stdout.trim().to_string()
}

/// **Invariant 1: a broken build is not a red test.**
///
/// The predicate has to fire on a Rust compile error, and must NOT fire on the
/// line cargo prints when a test merely fails — `error: test failed, to rerun
/// pass ...` begins with `error:` too, and matching that would make every honest
/// red guard look like a build failure.
#[test]
fn a_build_failure_is_told_apart_from_a_test_failure() {
    let hits = py("print(bool(cg.BUILD_ERROR_RE.search('error[E0308]: mismatched types')))");
    assert_eq!(hits, "True", "a compile error must be recognised");

    let hits = py("print(bool(cg.BUILD_ERROR_RE.search('error: could not compile `anaphase`')))");
    assert_eq!(hits, "True", "a failed build must be recognised");

    // The one that matters: cargo's own failure line starts with `error:` and
    // reports a TEST failure, not a build failure.
    let hits = py("print(bool(cg.BUILD_ERROR_RE.search('error: test failed, to rerun pass `--lib`')))");
    assert_eq!(
        hits, "False",
        "a failing TEST must not be read as a failing BUILD, or every honest red \
         guard is rejected as 'did not compile'"
    );
}

/// **Invariant 2: the NAMED test must be the one that went red.**
///
/// "Something failed" is not enough. A pre-existing red test anywhere would let a
/// no-op mutation look like a real guard, and the checker would certify it.
#[test]
fn only_the_named_test_counts_as_red() {
    let base = "s = {'run_cycle::tests::mine': 'ok', 'other::tests::theirs': 'FAILED'}\n";

    let out = py(&format!(
        "{base}red, full = cg.failed_specifically(s, 'mine')\nprint(red, full)"
    ));
    assert_eq!(
        out, "False run_cycle::tests::mine",
        "the named test is ok, so another test failing must not count"
    );

    let out = py(&format!(
        "{base}red, full = cg.failed_specifically(s, 'theirs')\nprint(red, full)"
    ));
    assert_eq!(out, "True other::tests::theirs", "the named test IS the red one");

    // A test that did not run at all is neither ok nor red, and must be reported
    // as absent rather than silently treated as a pass.
    let out = py("red, full = cg.failed_specifically({}, 'mine')\nprint(red, full)");
    assert_eq!(out, "False None", "a test that never ran must be distinguishable from ok");
}

/// **Invariant 3: the substitution has to be usable at all** — unique, code, and
/// parseable. Each of these was a real miss while CI-7 was being built.
#[test]
fn a_substitution_must_be_unique_code_that_parses() {
    // Escapes: three `from` values once carried a literal backslash-n and matched
    // nothing, because the reader was hand-rolled.
    let out = py("print(repr(cg.unescape('a\\\\nb')))\nprint(repr(cg.unescape('q\\\\\\\"w')))");
    assert_eq!(
        out, "'a\\nb'\n'q\"w'",
        "basic-string escapes must be resolved, or a multi-line substitution silently \
         matches nothing"
    );

    // A non-parsing mutation is not evidence, and Rust's pattern cannot see it.
    let out = py(
        "import tempfile, os\n\
         d = tempfile.mkdtemp(); p = os.path.join(d, 'broken.py')\n\
         open(p, 'w').write('if x > 1  # missing colon\\n')\n\
         print(cg.syntax_ok(p)[0])\n\
         q = os.path.join(d, 'fine.py')\n\
         open(q, 'w').write('if x > 1:\\n    pass\\n')\n\
         print(cg.syntax_ok(q)[0])",
    );
    assert_eq!(
        out, "False\nTrue",
        "a Python mutation that does not parse must be refused; a Rust-only build \
         check sails straight past it"
    );
}

/// The manifest reader refuses entries it cannot act on. A guard with no pit, an
/// unknown kind, or an integration guard with no target is a claim nobody can
/// check, and it must be an error rather than a skip.
#[test]
fn a_malformed_guard_entry_is_refused_rather_than_skipped() {
    for (label, entry, expect) in [
        // Each fixture carries the `[[guard]]` header. The first version of this
        // test omitted it, and the parser accepted the whole file — which is how the
        // "key outside any section is dropped in silence" gap below was found.
        ("no k_id", "[[guard]]\nid = \"g\"\ntest = \"t\"\nfile = \"f\"\nfrom = \"a\"\nto = \"b\"\n", "k_id"),
        ("bad kind", "[[guard]]\nid = \"g\"\ntest = \"t\"\nfile = \"f\"\nfrom = \"a\"\nto = \"b\"\nk_id = \"K\"\nkind = \"banana\"\n", "kind"),
        ("integration without target", "[[guard]]\nid = \"g\"\ntest = \"t\"\nfile = \"f\"\nfrom = \"a\"\nto = \"b\"\nk_id = \"K\"\nkind = \"integration\"\n", "target"),
        ("entry with no section header", "id = \"g\"\ntest = \"t\"\nfile = \"f\"\nfrom = \"a\"\nto = \"b\"\nk_id = \"K\"\n", "outside any section"),
    ] {
        let out = py(&format!(
            "import tempfile, os\n\
             d = tempfile.mkdtemp(); p = os.path.join(d, 'g.toml')\n\
             open(p, 'w').write({entry:?})\n\
             try:\n\
             \x20   cg.load_guards(p); print('ACCEPTED')\n\
             except cg.GuardError as e:\n\
             \x20   print('REFUSED', e)",
        ));
        assert!(
            out.starts_with("REFUSED"),
            "{label}: a malformed entry must be refused, not skipped — a skipped guard is \
             indistinguishable from a passing one. Got: {out}"
        );
        assert!(
            out.contains(expect),
            "{label}: the refusal must name what is missing, expected {expect:?} in: {out}"
        );
    }
}
