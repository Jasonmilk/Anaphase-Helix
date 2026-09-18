#!/usr/bin/env python3
"""CI-7 — a guard test is only a guard if some change can make it red.

P15: the label is an index, the criterion is the gate. A `guards: <id>` tag in a
test file says which mutation is supposed to redden that test. This checker does
not read that claim and nod — it **applies** the mutation, requires the test to
fail, reverts, and requires it to pass again.

Why it executes instead of trusting a manifest: a file saying "someone wrote a
mutation for this" verifies *reachability*, not *capability*. That distinction has
been the defect five times in this ledger (CI-4 measured that a vector was
reachable and stayed green while the loader read the wrong field). A guard test
that nobody has ever seen fail is a test that has only ever agreed with itself.

**[ENG] Known simplification.** The mutation is a literal substitution chosen by
its author, so this proves "this declared change reddens this test" — not "every
change that breaks this invariant would". Two rules keep it from decaying into
self-certification: the substituted text must occur exactly once, and it must
touch a line that is code rather than a comment. Upgrade trigger: when an entry
starts needing a substitution that is only nominally related to the invariant, or
when two entries in one file cannot be applied independently, move to symbol-level
mutation (parse the item, break its body, re-run).

Exit codes, as everywhere here: 0 = ran and every guard is real; 1 = ran and some
guard is decoration; 3 = could not run, so it judged nothing.
"""

import os
import re
import subprocess
import sys

GUARDS_FILE = "ci/guards.toml"
TAG_RE = re.compile(r"guards:\s*([A-Za-z0-9_,\s\-]+)")
TEST_TIMEOUT = 900


class GuardError(Exception):
    pass


def unescape(v):
    """Resolve the escapes this manifest uses.

    The reader below is hand-rolled rather than a TOML parser because the Python
    in this toolchain is 3.9 and `tomllib` arrived in 3.11. That means basic-string
    escapes are ours to resolve, and forgetting to was a real miss: three `from`
    values carried a literal backslash-n, matched nothing, and were caught only by
    the "must occur exactly once" rule that exists for a different reason. The rule
    earned its place before the checker had ever run clean.
    """
    out = []
    i = 0
    while i < len(v):
        c = v[i]
        if c == "\\" and i + 1 < len(v):
            n = v[i + 1]
            out.append({"n": "\n", "t": "\t", "r": "\r", '"': '"', "\\": "\\"}.get(n, n))
            i += 2
            continue
        out.append(c)
        i += 1
    return "".join(out)


def load_guards(path):
    """Parse the guard manifest. Malformed is an error, never a skip."""
    if not os.path.exists(path):
        raise GuardError(f"{path} does not exist")
    out = []
    cur = None

    def flush():
        if cur is None:
            return
        for field in ("id", "test", "file", "from", "to", "k_id"):
            if not cur.get(field):
                raise GuardError(
                    f"guard {cur.get('id')!r} is missing `{field}`. Every entry needs "
                    "an id, the test it reddens, the file and literal to change, and "
                    "the pit that justifies the guard."
                )
        if cur.get("kind", "lib") not in ("lib", "integration"):
            raise GuardError(f"guard {cur['id']!r} has kind {cur['kind']!r}, expected lib|integration")
        out.append(cur)

    for raw in open(path, encoding="utf-8"):
        line = raw.strip()
        if line == "[[guard]]":
            flush()
            cur = {}
        elif line.startswith("["):
            flush()
            cur = None
        elif "=" in line and not line.startswith("#") and cur is not None:
            k, _, v = line.partition("=")
            cur[k.strip()] = unescape(v.strip().strip('"'))
    flush()
    return out


def run_one(guard, root, expect_failure):
    """Run the guard's test once, returning (failed, output)."""
    args = ["cargo", "test"]
    if guard.get("kind", "lib") == "lib":
        args += ["--lib"]
    else:
        args += ["--test", guard.get("target", "integration_test")]
    args += [guard["test"], "--", "--nocapture"]
    try:
        r = subprocess.run(args, cwd=root, capture_output=True, text=True, timeout=TEST_TIMEOUT)
    except subprocess.TimeoutExpired:
        raise GuardError(f"guard {guard['id']!r}: test timed out after {TEST_TIMEOUT}s")
    return r.returncode != 0, (r.stdout or "") + (r.stderr or "")


def main():
    root = os.getcwd()
    path = os.path.join(root, GUARDS_FILE)
    try:
        guards = load_guards(path)
    except GuardError as e:
        print(f"CHECKER ERROR: {e}", file=sys.stderr)
        return 3

    if not guards:
        print(f"CHECKER ERROR: {GUARDS_FILE} lists no guards, so CI-7 judged nothing", file=sys.stderr)
        return 3

    # 1. The tags must point at entries that exist. A tag naming nothing is a claim
    #    nobody can act on.
    by_id = {g["id"]: g for g in guards}
    tagged = set()
    for dirpath, dirnames, filenames in os.walk(os.path.join(root, "src")):
        dirnames[:] = [d for d in dirnames if d != "target"]
        for fn in filenames:
            if not fn.endswith(".rs"):
                continue
            text = open(os.path.join(dirpath, fn), encoding="utf-8", errors="replace").read()
            for m in TAG_RE.finditer(text):
                for name in m.group(1).split(","):
                    name = name.strip()
                    if name:
                        tagged.add(name)
    for dirpath, dirnames, filenames in os.walk(os.path.join(root, "tests")):
        for fn in filenames:
            if not fn.endswith(".rs"):
                continue
            text = open(os.path.join(dirpath, fn), encoding="utf-8", errors="replace").read()
            for m in TAG_RE.finditer(text):
                for name in m.group(1).split(","):
                    name = name.strip()
                    if name:
                        tagged.add(name)

    problems = []
    for name in sorted(tagged):
        if name not in by_id:
            problems.append(f"tag `guards: {name}` names no entry in {GUARDS_FILE}: it is a claim nobody can act on")
    for gid in sorted(by_id):
        if gid not in tagged:
            problems.append(f"guard {gid!r} has an entry in {GUARDS_FILE} but no `guards: {gid}` tag beside its test, so a reader of the test cannot find it")

    if problems:
        print(f"FAIL  {len(problems)} guard tag(s) do not line up with the manifest:")
        for p in problems:
            print(f"        {p}")
        return 1

    # 2. Every mutation must actually redden its test, and the test must be green
    #    without it. The second half is what makes the first mean anything.
    decoration = []
    broken = []
    for g in sorted(guards, key=lambda g: g["id"]):
        target = os.path.join(root, g["file"])
        if not os.path.exists(target):
            broken.append(f"{g['id']}: {g['file']} does not exist")
            continue
        original = open(target, encoding="utf-8").read()
        occurrences = original.count(g["from"])
        if occurrences != 1:
            broken.append(
                f"{g['id']}: the `from` text occurs {occurrences} time(s) in {g['file']}, "
                "so the mutation is ambiguous. It must match exactly once."
            )
            continue
        mutated_line = g["from"].strip()
        if mutated_line.startswith("//") or mutated_line.startswith("#"):
            broken.append(
                f"{g['id']}: the mutation only touches a comment. A substitution that "
                "cannot change behaviour cannot prove a test detects behaviour."
            )
            continue

        try:
            open(target, "w", encoding="utf-8").write(original.replace(g["from"], g["to"], 1))
            failed, out = run_one(g, root, True)
        finally:
            open(target, "w", encoding="utf-8").write(original)

        if not failed:
            decoration.append((g, out))
            continue

        still_failed, out2 = run_one(g, root, False)
        if still_failed:
            broken.append(
                f"{g['id']}: the test fails even WITHOUT the mutation, so the mutation "
                f"proves nothing about it. Output tail:\n{out2[-1500:]}"
            )

    if broken:
        print(f"CHECKER ERROR: {len(broken)} guard(s) could not be evaluated:", file=sys.stderr)
        for b in broken:
            print(f"        {b}", file=sys.stderr)
        return 3

    if decoration:
        print(f"FAIL  {len(decoration)} guard(s) are decoration — the declared mutation does not redden the test:")
        for g, _out in decoration:
            print(f"        {g['id']}  guards {g['test']} in {g['file']}  (pit {g['k_id']})")
        print(
            "        Each test above passed WITH its mutation applied, so it does not\n"
            "        detect what the manifest claims it guards. Either the mutation is\n"
            "        a no-op, or the test is asserting something else."
        )
        return 1

    print(f"OK    {len(guards)} guard(s) verified: each declared mutation reddens its test, and each test is green without it")
    return 0


if __name__ == "__main__":
    sys.exit(main())
