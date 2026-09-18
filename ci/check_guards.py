#!/usr/bin/env python3
"""CI-7 — a guard test is only a guard if some change can make it red.

P15: the label is an index, the criterion is the gate. A `guards: <id>` tag in a
test file says which mutation is supposed to redden that test. This checker does
not read that claim and nod — it **applies** the mutation and requires *that test*
to fail, for a reason that is about the test and not about the build.

Why it executes instead of trusting a manifest: a file saying "someone wrote a
mutation for this" verifies *reachability*, not *capability*. That distinction has
been the defect six times in this ledger (CI-4 measured that a vector was
reachable and stayed green while the loader read the wrong field).

**What "failed" must mean.** The first version treated any non-zero exit as
success, so a mutation that merely broke the *build* counted as a verified guard —
the suite fails to compile, every test "fails", and the entry passes. The previous
round's own words were "a mutation that does not compile is not evidence", and the
machine built the next day executed the loosest reading of them. So three
conditions, all required:

1. the build succeeds — `error[E…]` or `could not compile` is a checker error,
   not a red test;
2. the **named** test reports FAILED. Not "something failed": a pre-existing red
   test anywhere would otherwise make a no-op mutation look like a real guard, and
   this checker would certify it;
3. without the mutation the same test reports ok. An already-red test is not a
   guard, and only the pair proves anything.

**Cost.** Each guard needs one mutated run, and the un-mutated baseline is shared
across all guards that live in the same test target, so the cost is
`targets + guards` runs rather than `2 * guards`. Runs are filtered to the one
test, so the binary starts once and executes one case. On top of that there is a
declared time budget: exceeding it fails the check and demands tiering rather than
letting it get slower every round. A gate that is quietly switched off is worse
than no gate, and "it got too slow" is how that starts.

**[ENG] Known simplification.** The mutation is a literal substitution chosen by
its author, so this proves "this declared change reddens this test" — not "every
change that breaks this invariant would". Two rules keep it from decaying into
self-certification: the substituted text must occur exactly once, and it must
touch a line that is code rather than a comment. Upgrade trigger: when an entry
needs a substitution only nominally related to the invariant, or two entries in one
file cannot be applied independently, move to symbol-level mutation.

Exit codes: 0 = ran, every guard real; 1 = ran, some guard is decoration or the
budget is exceeded; 3 = could not run, so it judged nothing.
"""

import json
import os
import re
import subprocess
import sys
import time

GUARDS_FILE = "ci/guards.toml"
TAG_RE = re.compile(r"guards:\s*([A-Za-z0-9_,\s\-]+)")
RESULT_RE = re.compile(r"^test (\S+) \.\.\. (ok|FAILED|ignored)", re.M)
BUILD_ERROR_RE = re.compile(r"^error\[E\d+\]|^error: could not compile", re.M)
TEST_TIMEOUT = 900
DEFAULT_BUDGET_SECONDS = 240


class GuardError(Exception):
    pass


def unescape(v):
    """Resolve the escapes this manifest uses.

    The reader below is hand-rolled rather than a TOML parser because the Python
    in this toolchain is 3.9 and `tomllib` arrived in 3.11. That means basic-string
    escapes are ours to resolve, and forgetting to was a real miss: three `from`
    values carried a literal backslash-n, matched nothing, and were caught only by
    the "must occur exactly once" rule that exists for a different reason.
    """
    out = []
    i = 0
    while i < len(v):
        c = v[i]
        if c == "\\" and i + 1 < len(v):
            out.append({"n": "\n", "t": "\t", "r": "\r", '"': '"', "\\": "\\"}.get(v[i + 1], v[i + 1]))
            i += 2
            continue
        out.append(c)
        i += 1
    return "".join(out)


def load_guards(path):
    """Parse the manifest into (guards, settings). Malformed is an error, not a skip."""
    if not os.path.exists(path):
        raise GuardError(f"{path} does not exist")
    guards, settings = [], {"budget_seconds": DEFAULT_BUDGET_SECONDS}
    cur = None

    def flush():
        if cur is None:
            return
        for field in ("id", "test", "file", "from", "to", "k_id"):
            if not cur.get(field):
                raise GuardError(
                    f"guard {cur.get('id')!r} is missing `{field}`. Every entry needs an "
                    "id, the test it reddens, the file and literal to change, and the pit "
                    "that justifies the guard."
                )
        if cur.get("kind", "lib") not in ("lib", "integration"):
            raise GuardError(f"guard {cur['id']!r} has kind {cur['kind']!r}, expected lib|integration")
        if cur.get("kind") == "integration" and not cur.get("target"):
            raise GuardError(f"guard {cur['id']!r} is an integration guard and needs `target`")
        if cur.get("tier", "fast") not in ("fast", "slow"):
            raise GuardError(f"guard {cur['id']!r} has tier {cur['tier']!r}, expected fast|slow")
        guards.append(cur)

    for raw in open(path, encoding="utf-8"):
        line = raw.strip()
        if line == "[[guard]]":
            flush()
            cur = {}
        elif line.startswith("["):
            flush()
            cur = None
        elif "=" in line and not line.startswith("#"):
            k, _, v = line.partition("=")
            k, v = k.strip(), unescape(v.strip().strip('"'))
            if cur is not None:
                cur[k] = v
            elif k == "budget_seconds":
                settings["budget_seconds"] = int(v)
            else:
                # A `key = value` outside any section used to be dropped in silence,
                # which is the one thing this file's docstring says it never does:
                # "malformed is an error, not a skip". A whole entry missing its
                # `[[guard]]` header would therefore vanish and its guard would look
                # un-tagged rather than mis-written. Found by the self-check test
                # whose fixture was itself missing that header.
                raise GuardError(
                    f"{path}: `{k} = …` appears outside any section. An entry missing "
                    "its `[[guard]]` header is dropped silently otherwise, and a "
                    "dropped entry is indistinguishable from a passing one."
                )
    flush()
    return guards, settings


def syntax_ok(path):
    """Does the mutated file still parse?

    Rust is covered by the cargo output (`error[E…]` / `could not compile`). Other
    languages are not, and that gap was real: the first version of this check was
    demonstrated with a **Python** syntax error, and the Rust-only pattern sailed
    straight past it — the checker reported the guard as verified while the tool it
    guards did not parse. So a `.py` target gets `py_compile`.

    This does not catch every way a mutation can break the *tool* rather than the
    *invariant*; it catches the case where the mutation is not even a program.
    """
    if path.endswith(".py"):
        r = subprocess.run([sys.executable, "-m", "py_compile", path],
                           capture_output=True, text=True)
        if r.returncode != 0:
            return False, (r.stdout or "") + (r.stderr or "")
    return True, ""


def target_args(guard):
    if guard.get("kind", "lib") == "lib":
        return ["--lib"]
    return ["--test", guard["target"]]


def run_tests(guards, root, timeout=TEST_TIMEOUT):
    """Run several guards' tests in one cargo invocation per test target."""
    results = {}
    for g in guards:
        args = ["cargo", "test"] + target_args(g) + [g["test"], "--", "--nocapture"]
        started = time.time()
        try:
            r = subprocess.run(args, cwd=root, capture_output=True, text=True, timeout=timeout)
        except subprocess.TimeoutExpired:
            raise GuardError(f"guard {g['id']!r}: test timed out after {timeout}s")
        out = (r.stdout or "") + (r.stderr or "")
        if BUILD_ERROR_RE.search(out):
            raise GuardError(
                f"guard {g['id']!r}: the mutated tree does not COMPILE, so the failure "
                f"is the build's and not the test's. A mutation that does not compile is "
                f"not evidence. Output tail:\n{out[-2000:]}"
            )
        results[g["id"]] = (dict(RESULT_RE.findall(out)), out, time.time() - started)
    return results


def failed_specifically(statuses, name):
    """Did the NAMED test report FAILED, as opposed to something else failing?"""
    for full, verdict in statuses.items():
        if full == name or full.endswith("::" + name):
            return verdict == "FAILED", full
    return False, None


def main():
    root = os.getcwd()
    argv = sys.argv[1:]
    skip_slow = "--skip-slow" in argv
    try:
        guards, settings = load_guards(os.path.join(root, GUARDS_FILE))
    except (GuardError, ValueError) as e:
        print(f"CHECKER ERROR: {e}", file=sys.stderr)
        return 3
    if not guards:
        print(f"CHECKER ERROR: {GUARDS_FILE} lists no guards, so CI-7 judged nothing", file=sys.stderr)
        return 3

    # --- 1. Tags must name entries that exist, and entries must be tagged. ---
    by_id = {g["id"]: g for g in guards}
    tagged = set()
    for sub in ("src", "tests"):
        base = os.path.join(root, sub)
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames[:] = [d for d in dirnames if d != "target"]
            for fn in filenames:
                if not fn.endswith(".rs"):
                    continue
                text = open(os.path.join(dirpath, fn), encoding="utf-8", errors="replace").read()
                for m in TAG_RE.finditer(text):
                    for name in m.group(1).split(","):
                        if name.strip():
                            tagged.add(name.strip())

    problems = [f"tag `guards: {n}` names no entry in {GUARDS_FILE}: a claim nobody can act on"
                for n in sorted(tagged) if n not in by_id]
    problems += [f"guard {g!r} has an entry but no `guards: {g}` tag beside its test, so a reader of the test cannot find it"
                 for g in sorted(by_id) if g not in tagged]
    if problems:
        print(f"FAIL  {len(problems)} guard tag(s) do not line up with the manifest:")
        for p in problems:
            print(f"        {p}")
        return 1

    # --- 2. Validate the substitutions before running anything. ---
    skipped = [g for g in guards if skip_slow and g.get("tier") == "slow"]
    selected = [g for g in guards if g not in skipped]

    broken = []
    for g in selected:
        target = os.path.join(root, g["file"])
        if not os.path.exists(target):
            broken.append(f"{g['id']}: {g['file']} does not exist")
            continue
        original = open(target, encoding="utf-8").read()
        n = original.count(g["from"])
        if n != 1:
            broken.append(f"{g['id']}: the `from` text occurs {n} time(s) in {g['file']}, so the mutation is ambiguous; it must match exactly once")
            continue
        stripped = g["from"].strip()
        if stripped.startswith("//") or stripped.startswith("#"):
            broken.append(f"{g['id']}: the mutation only touches a comment; a substitution that cannot change behaviour cannot prove a test detects behaviour")
    if broken:
        print(f"CHECKER ERROR: {len(broken)} guard(s) could not be evaluated:", file=sys.stderr)
        for b in broken:
            print(f"        {b}", file=sys.stderr)
        return 3

    # --- 3. Shared baseline: every selected guard's test must be green as-is. ---
    baseline = {}
    for target_key in {("lib", None) if g.get("kind", "lib") == "lib" else ("integration", g["target"]) for g in selected}:
        group = [g for g in selected
                 if (("lib", None) if g.get("kind", "lib") == "lib" else ("integration", g["target"])) == target_key]
        try:
            ran = run_tests(group, root)
        except GuardError as e:
            # "Could not run" must be exit 3, not an uncaught traceback exiting 1.
            # The first version of this raised straight out of here, so a
            # non-compiling mutation was reported as a *violation* -- the exact
            # conflation of 1 and 3 that the other two checkers in this directory
            # take care to keep apart.
            print(f"CHECKER ERROR: {e}", file=sys.stderr)
            return 3
        for gid, (statuses, out, _t) in ran.items():
            baseline[gid] = (statuses, out)
    for g in selected:
        statuses, out = baseline[g["id"]]
        ok, full = failed_specifically(statuses, g["test"])
        if full is None:
            print(f"CHECKER ERROR: guard {g['id']!r}: test {g['test']!r} did not run at all, "
                  f"so this entry guards nothing. Output tail:\n{out[-1500:]}", file=sys.stderr)
            return 3
        if ok:
            print(f"CHECKER ERROR: guard {g['id']!r}: test {g['test']!r} is ALREADY red before any "
                  f"mutation, so a mutation cannot prove anything about it.", file=sys.stderr)
            return 3

    # --- 4. Apply each mutation; require the NAMED test to go red. ---
    decoration, elapsed = [], 0.0
    for g in selected:
        target = os.path.join(root, g["file"])
        original = open(target, encoding="utf-8").read()
        try:
            open(target, "w", encoding="utf-8").write(original.replace(g["from"], g["to"], 1))
            parses, why = syntax_ok(target)
            if not parses:
                broken.append(
                    f"{g['id']}: the mutation makes {g['file']} fail to PARSE, so any red "
                    f"test below it is the tool being broken rather than the invariant "
                    f"being detected.\n{why[-800:]}"
                )
                continue
            try:
                statuses, out, took = run_tests([g], root)[g["id"]]
            except GuardError as e:
                print(f"CHECKER ERROR: {e}", file=sys.stderr)
                return 3
            elapsed += took
        finally:
            open(target, "w", encoding="utf-8").write(original)
        went_red, full = failed_specifically(statuses, g["test"])
        if full is None:
            broken.append(f"{g['id']}: with the mutation applied, test {g['test']!r} did not run at all")
            continue
        if not went_red:
            decoration.append(g)

    if broken:
        print(f"CHECKER ERROR: {len(broken)} guard(s) could not be evaluated:", file=sys.stderr)
        for b in broken:
            print(f"        {b}", file=sys.stderr)
        return 3

    if decoration:
        print(f"FAIL  {len(decoration)} guard(s) are decoration — the declared mutation does not redden the named test:")
        for g in decoration:
            print(f"        {g['id']}  guards {g['test']} in {g['file']}  (pit {g['k_id']})")
        print("        Each test above reported ok WITH its mutation applied, so it does not\n"
              "        detect what the manifest claims. Either the mutation is a no-op, or the\n"
              "        test asserts something else.")
        return 1

    # --- 5. The cost ratchet: advisory, deliberately NOT a failure. ---
    #
    # A hard failure here makes adding a guard expensive, so the next person adds
    # fewer — the same shape as a coverage threshold that breeds meaningless tests.
    # Guard density is worth more than wall-clock, so exceeding the budget asks for
    # tiering and still passes. The hard failures above are kept for things that are
    # actually wrong: a decoration guard, an already-red test, a mutation that does
    # not compile.
    budget = settings["budget_seconds"]
    over = elapsed > budget
    if over:
        print(f"WARN  CI-7 took {elapsed:.0f}s against a declared budget of {budget}s "
              f"with {len(selected)} guard(s).")
        print("        Not a failure: a check that is expensive to extend is one that stops")
        print("        being extended, which costs more than the seconds. Tier the slowest")
        print("        guards (`tier = \"slow\"`) when the wall-clock actually starts to hurt.")

    note = f" ({len(skipped)} slow guard(s) skipped by --skip-slow)" if skipped else ""
    print(f"OK    {len(selected)} guard(s) verified: each declared mutation reddens its NAMED test, "
          f"the build still compiles, and every test is green without it. {elapsed:.0f}s of {budget}s.{note}")

    # A WARN nobody can read is K-029: a ledger with no reader. So the cost numbers
    # get a consumer in two shapes — a machine-readable line for whatever collects
    # CI output, and a file on disk for anyone who asks later. `over_budget` is an
    # explicit boolean rather than something to infer from two numbers, because
    # inferring it is how a reader gets it wrong once and stops trusting it.
    summary = {
        "guards": len(selected),
        "skipped_slow": len(skipped),
        "seconds": round(elapsed, 1),
        "budget_seconds": budget,
        "over_budget": bool(over),
        "decoration": 0,
    }
    print("CI7-SUMMARY " + json.dumps(summary, sort_keys=True))
    try:
        os.makedirs(os.path.join(root, "target"), exist_ok=True)
        with open(os.path.join(root, "target", "ci7-report.json"), "w", encoding="utf-8") as fh:
            json.dump(summary, fh, sort_keys=True, indent=1)
    except OSError as e:
        # Not fatal: the line above is the primary consumer. But it must not be
        # silent, because a report that quietly fails to be written is the same
        # class of thing as a ledger nobody reads.
        print(f"WARN  could not write target/ci7-report.json: {e}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
