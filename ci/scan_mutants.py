#!/usr/bin/env python3
"""An OFFLINE mutation scanner — the reviewer's round-38 proposal, built.

`cargo-mutants` cannot be installed here (no network: crates.io answers 403, there is
no `.cargo/config.toml`, no `vendor/`, nothing in the registry cache — verified, not
assumed). And even if it could be run on another machine, a one-off number is not a
mechanism: it is stale the next day. So the mutation operators are applied here, in
the repository, by a script that runs in CI.

This is deliberately the crude version. The published tooling does the same thing at
a larger scale: most mutation analysis applies *simple pattern-based operators over
program text* rather than parsing the language — see Pilz, "Improving Rust Mutation
Testing using Static and Dynamic Analysis" (TU Wien, 2023). Pattern substitution is
not a shortcut someone took here; it is what the field does.

WHAT IT ANSWERS, and why it is worth the build cost: "does any test notice if I break
this?" A mutation that no test kills is a place where the code can be wrong and the
suite stays green — which is the same question K-036 asks by hand ("tests whose
message claims X while asserting Y"), asked exhaustively and without a human.

TWO THINGS IT MUST NOT GET WRONG, both learned the hard way in this ledger:

  1. **A mutation that does not compile is not evidence.** CI-7 once counted a broken
     build as a red test and certified a guard that did not exist. Here such sites are
     `unviable` and excluded from every count — the same three-way split the mature
     tools use (caught / missed / unviable), arrived at again the slow way.
  2. **Each iteration costs a rebuild.** Measured here: `cargo test --lib` is ~17s and
     the full suite ~35s, so the scanner uses `--lib` and SAMPLES rather than sweeping.
     A census of this repository would run for hours; the sample size is printed with
     every result, because an unexplained N is the defect this ledger keeps finding.
 3. **It does not mutate your working tree at all.** Every mutation is applied inside
     a `git worktree` copy. The first version mutated in place, and paid for it twice:
     a killed run left a mutation behind, and a background run silently broke five
     `run_cycle` tests — K-041 reproduced live. A tool that can quietly corrupt the
     tree it is measuring gets worse the longer it runs, so "run it for hours,
     exclusively" was a bet against being killed. **The tool may be slow; it may not
     be holding the source.**

Usage:
    ci/scan_mutants.py --limit 12            # sample, and say so
    ci/scan_mutants.py --scope src/run_cycle
    ci/scan_mutants.py --operators ok-err     # one family only

Exit: 0 = ran, nothing missed in the sample; 1 = ran, missed mutations found;
      3 = could not run, so it judged nothing.
"""

import argparse
import os
import re
import subprocess
import sys
import time

# Simple text operators. Each is a regex applied to source, with a name that says what
# breaking it means. Kept few and blunt on purpose: a subtle operator produces a
# subtle answer, and the point here is to find code no test defends.
OPERATORS = [
    ("bool-flip", r"\btrue\b", "false", "a branch or assertion that is now inverted"),
    ("bool-flip-rev", r"\bfalse\b", "true", "a fallback that now fires by default"),
    ("cmp-flip", r"==", "!=", "a comparison that is now its opposite"),
    ("cmp-flip-lt", r"<=", ">", "a bound that no longer bounds"),
    ("and-or", r"&&", "||", "a guard that is now permissive"),
    ("ok-err", r"\bOk\(", "Err(", "a success path turned into a failure (or a fail-open)"),
    ("some-none", r"\bSome\(", "None", "a present value reported as absent"),
]

BUILD_ERROR_RE = re.compile(r"^error\[E\d+\]|^error: could not compile", re.M)
RESULT_RE = re.compile(r"^test (\S+) \.\.\. (ok|FAILED|ignored)", re.M)


class ScanError(Exception):
    pass


def run(cmd, root, timeout=1800):
    return subprocess.run(cmd, cwd=root, capture_output=True, text=True, timeout=timeout)


def dirty(root):
    r = run(["git", "status", "--porcelain"], root)
    return [l for l in r.stdout.splitlines() if l.strip()]


def candidate_sites(files, operators, limit):
    """(file, operator, line number, original line, mutated line) — sampled, in order."""
    sites = []
    for path in files:
        try:
            lines = open(path, encoding="utf-8").read().splitlines()
        except OSError:
            continue
        # Everything from the first `#[cfg(test)]` onward is test code. Mutating a test
        # proves nothing about whether a test notices anything — and the first run of
        # this scanner mutated `assert_eq!(deny..., false)` into `true` inside
        # `src/hitl.rs`'s inline test module, which is both meaningless and confusing.
        # The file-level filter below only knows `*_tests.rs`; inline modules need this.
        cut = len(lines)
        for i, line in enumerate(lines):
            if line.strip().startswith("#[cfg(test)]"):
                cut = i
                break
        for i, line in enumerate(lines[:cut]):
            stripped = line.strip()
            # Comments and doc-comments cannot change behaviour, so mutating them
            # proves nothing about a test. CI-7 has this rule for the same reason.
            if stripped.startswith("//") or stripped.startswith("#"):
                continue
            for name, pat, rep, _why in operators:
                if re.search(pat, line):
                    mutated = re.sub(pat, rep, line, count=1)
                    if mutated != line:
                        sites.append((path, name, i + 1, line, mutated))
    # Deterministic order, then a prefix: a sample whose size is not reported is an
    # unexplained number, so `limit` is echoed in the summary.
    sites.sort(key=lambda s: (s[0], s[2], s[1]))
    return sites[:limit] if limit else sites


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default=".")
    ap.add_argument("--scope", default="src/run_cycle,src/reflex.rs,src/security.rs,src/hitl.rs",
                    help="comma-separated files or directories")
    ap.add_argument("--limit", type=int, default=12,
                    help="how many sites to sample; the number is reported with the result")
    ap.add_argument("--operators", default="", help="comma-separated operator names; empty = all")
    ap.add_argument("--selftest", action="store_true",
                    help="run one mutation that a test MUST catch, and require it to be caught")
    ap.add_argument("--files", default="", help="explicit comma-separated files (incremental)")
    ap.add_argument("--changed-since", default="", help="only files changed since this rev, "
                    "e.g. HEAD~1 — the mode that makes this cheap enough for CI")
    args = ap.parse_args()
    root = os.path.abspath(args.root)

    # Refuse to run on a dirty tree: this mutates sources, and K-041 is the record of
    # what happens when something does that while other things read the tree.
    # Crash-safe, not just exception-safe. The first version restored in a `finally`,
    # which SIGTERM does not run: the scan was killed mid-iteration and left
    # `Ok(false)` mutated to `Ok(true)` inside src/hitl.rs. A backup on disk plus a
    # restore-on-start covers SIGKILL too, and a signal handler cannot.
    backup_dir = os.path.join(root, "target", "mutants-backup")
    if os.path.isdir(backup_dir):
        restored = 0
        for name in sorted(os.listdir(backup_dir)):
            # Against ROOT, not the backup dir. The first version joined the backup
            # prefix onto the relative path, so restore-on-start looked for
            # target/mutants-backup/src/run_cycle/mod.rs and silently restored
            # nothing — a recovery path that cannot recover, which is the exact shape
            # this ledger keeps recording.
            src = os.path.join(root, name.replace("__", "/"))
            if os.path.exists(src):
                open(src, "w", encoding="utf-8").write(
                    open(os.path.join(backup_dir, name), encoding="utf-8").read())
                restored += 1
        if restored:
            print(f"recovered {restored} file(s) left mutated by a previous run "
                  f"(a killed scan does not run its cleanup)")
        os.rmdir(backup_dir) if not os.listdir(backup_dir) else None

    d = dirty(root)
    if d:
        print(f"REFUSING: working tree has {len(d)} change(s). The sandbox is a worktree at "
              f"HEAD, so it would not contain your uncommitted edits — and a scan of code "
              f"that is not the code you are working on is a measurement of something else. "
              f"Commit or stash first.", file=sys.stderr)
        return 3

    # THE POSITIVE CONTROL. A zero in the numerator is only evidence if the instrument
    # can produce a non-zero — the rule this ledger has had to apply three times now
    # (a rule without a control, a grep whose zero could have been the wrong path, a
    # counter reporting 0). A `missed = 0` from a scanner that cannot catch anything
    # reads as "our tests are good", which is the opposite of what it would mean.
    #
    # `is_high_risk("rm -rf /data")` is asserted true by `high_risk_detection`, so
    # emptying the write-token list MUST be caught. If it is not, the instrument is
    # broken and every other verdict from this run is void.
    if args.selftest:
        rel = "src/hitl.rs"
        full = os.path.join(root, rel)
        marker = 'const WRITE: &[&str] = &['
        if marker not in open(full, encoding="utf-8").read():
            print(f"CHECKER ERROR: selftest anchor not found in {rel}; the control has "
                  f"outlived its subject", file=sys.stderr)
            return 3
        sandbox = os.path.join(root, "target", "mutants-selftest")
        if os.path.exists(sandbox):
            run(["git", "worktree", "remove", "--force", sandbox], root)
        r = run(["git", "worktree", "add", "--detach", sandbox, "HEAD"], root)
        if r.returncode != 0:
            print(f"CHECKER ERROR: cannot create the selftest sandbox: {r.stderr.strip()}",
                  file=sys.stderr)
            return 3
        try:
            sf = os.path.join(sandbox, rel)
            text = open(sf, encoding="utf-8").read()
            control_from = "            if WRITE.contains(&t.as_str())"
            control_to = "            if false && WRITE.contains(&t.as_str())"
            if control_from not in text:
                print(f"CHECKER ERROR: selftest anchor moved in {rel}; the control has "
                      f"outlived its subject", file=sys.stderr)
                return 3
            open(sf, "w", encoding="utf-8").write(text.replace(control_from, control_to, 1))
            r = run(["cargo", "test", "--lib", "--quiet"], sandbox)
        finally:
            run(["git", "worktree", "remove", "--force", sandbox], root)
        out = (r.stdout or "") + (r.stderr or "")
        if BUILD_ERROR_RE.search(out):
            print("CHECKER ERROR: the control mutation did not compile, so it is not "
                  "evidence either way", file=sys.stderr)
            return 3
        if r.returncode != 0:
            print("OK    positive control: a known-catchable mutation was CAUGHT, so the "
                  "instrument can produce a non-zero")
            return 0
        print("CHECKER ERROR: the positive control was NOT caught. `is_high_risk(\"rm "
              "\")` is asserted true by high_risk_detection, so emptying the write-token "
              "list must fail. A scanner that cannot catch this cannot be trusted to "
              "report a zero.", file=sys.stderr)
        return 3

    files = []
    if args.files:
        files = [os.path.join(root, f.strip()) for f in args.files.split(",") if f.strip()]
    elif args.changed_since:
        r = run(["git", "diff", "--name-only", args.changed_since, "--", "*.rs"], root)
        files = [os.path.join(root, f.strip()) for f in r.stdout.splitlines()
                 if f.strip().endswith(".rs")]
        if not files:
            print(f"no .rs files changed since {args.changed_since}; nothing to scan")
            return 0
    for part in ([] if (args.files or args.changed_since) else args.scope.split(",")):
        part = part.strip()
        p = os.path.join(root, part)
        if os.path.isdir(p):
            for dp, dn, fn in os.walk(p):
                dn[:] = [x for x in dn if x != "target"]
                files += [os.path.join(dp, f) for f in sorted(fn) if f.endswith(".rs")]
        elif os.path.isfile(p):
            files.append(p)
    files = [f for f in files if not f.endswith("_tests.rs") and not f.endswith("/tests.rs")]

    operators = OPERATORS
    if args.operators:
        want = {x.strip() for x in args.operators.split(",")}
        operators = [o for o in OPERATORS if o[0] in want]
        if not operators:
            print(f"CHECKER ERROR: no operator matched {args.operators!r}", file=sys.stderr)
            return 3

    sites = candidate_sites(files, operators, args.limit)
    if not sites:
        print(f"CHECKER ERROR: no candidate sites found under {args.scope}; the scan judged "
              f"nothing", file=sys.stderr)
        return 3

    print(f"scanning {len(sites)} sampled site(s) across {len(files)} file(s) "
          f"(scope {args.scope}, limit {args.limit}, operators "
          f"{','.join(o[0] for o in operators)})")
    print("  NOTE: a sample, not a census. The count is printed with every result for "
          "that reason.")

    caught, missed, unviable = [], [], []
    started = time.time()

    # A `git worktree` at HEAD, used as the sandbox. The scanner edits THAT copy, so
    # the working tree is never in a mutated state — which removes three problems at
    # once rather than patching each: a killed run leaves nothing behind, a concurrent
    # reader never sees a mutation, and no restore path is needed. The cost is one
    # full compile here, which incremental mode (few files) makes acceptable.
    sandbox = os.path.join(root, "target", "mutants-worktree")
    if os.path.exists(sandbox):
        run(["git", "worktree", "remove", "--force", sandbox], root)
    r = run(["git", "worktree", "add", "--detach", sandbox, "HEAD"], root)
    if r.returncode != 0:
        print(f"CHECKER ERROR: cannot create a worktree sandbox ({r.stderr.strip()}). "
              f"Refusing to mutate the working tree instead: this tool has already been "
              f"caught corrupting it twice.", file=sys.stderr)
        return 3
    print(f"  sandbox: {os.path.relpath(sandbox, root)} (a worktree at HEAD; the working "
          f"tree is never mutated)")

    try:
        for path, name, lineno, original, mutated in sites:
            rel = os.path.relpath(os.path.abspath(path), root)
            full = os.path.join(sandbox, rel)
            if not os.path.exists(full):
                unviable.append(f"{rel}:{lineno} [{name}] (not in the sandbox)")
                continue
            text = open(full, encoding="utf-8").read()
            try:
                open(full, "w", encoding="utf-8").write(text.replace(original, mutated, 1))
                r = run(["cargo", "test", "--lib", "--quiet"], sandbox)
            finally:
                open(full, "w", encoding="utf-8").write(text)
            out = (r.stdout or "") + (r.stderr or "")
            label = f"{rel}:{lineno} [{name}]"
            if BUILD_ERROR_RE.search(out):
                unviable.append(label)
                verdict = "unviable"
            elif r.returncode != 0:
                caught.append(label)
                verdict = "caught  "
            else:
                missed.append(label)
                verdict = "MISSED  "
            print(f"  {verdict} {label}")
    finally:
        run(["git", "worktree", "remove", "--force", sandbox], root)

    elapsed = time.time() - started
    print(f"\n=== result: caught={len(caught)} missed={len(missed)} unviable={len(unviable)} "
          f"of {len(sites)} sampled, {elapsed:.0f}s ===")
    if missed == 0 and not caught:
        print("CHECKER ERROR: nothing was caught, so `missed=0` cannot be distinguished "
              "from an instrument that finds nothing. Run --selftest first.", file=sys.stderr)
        return 3
    if missed:
        print("MISSED means: this line can be broken and the whole suite stays green.")
        print("That is the question K-036 asks by hand, answered without a human.")
        for m in missed:
            print(f"        {m}")
        return 1
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ScanError, subprocess.TimeoutExpired) as e:
        print(f"CHECKER ERROR: {e}", file=sys.stderr)
        sys.exit(3)
