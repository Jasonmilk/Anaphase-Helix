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
  2. **Anything that mutates the tree and rebuilds races with anything else reading
     it.** K-041: CI-7 inside `cargo test` made a full run fail intermittently because
     cargo could rebuild a test binary from mutated source. This scanner must run
     alone, and it refuses to start if the working tree is dirty.

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
        for i, line in enumerate(lines):
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
    args = ap.parse_args()
    root = os.path.abspath(args.root)

    # Refuse to run on a dirty tree: this mutates sources, and K-041 is the record of
    # what happens when something does that while other things read the tree.
    d = dirty(root)
    if d:
        print(f"REFUSING: working tree has {len(d)} change(s); this scanner mutates sources "
              f"and rebuilds, so it must run alone on a clean tree.", file=sys.stderr)
        return 3

    files = []
    for part in args.scope.split(","):
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
    for path, name, lineno, original, mutated in sites:
        full = os.path.join(root, path)
        text = open(full, encoding="utf-8").read()
        try:
            open(full, "w", encoding="utf-8").write(
                text.replace(original, mutated, 1))
            r = run(["cargo", "test", "--quiet"], root)
        finally:
            open(full, "w", encoding="utf-8").write(text)
        out = (r.stdout or "") + (r.stderr or "")
        label = f"{os.path.relpath(full, root)}:{lineno} [{name}]"
        if BUILD_ERROR_RE.search(out):
            unviable.append(label)
        elif r.returncode != 0:
            caught.append(label)
        else:
            missed.append(label)
        print(f"  {'caught  ' if label in caught else 'MISSED  ' if label in missed else 'unviable'} {label}")

    elapsed = time.time() - started
    print(f"\n=== result: caught={len(caught)} missed={len(missed)} unviable={len(unviable)} "
          f"of {len(sites)} sampled, {elapsed:.0f}s ===")
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
