#!/usr/bin/env python3
"""Declaration-with-evidence checker for the CI-144 profile.

Why this exists: every "adopted" / "absent" statement in the profile that has no
resolvable anchor is the same pathology as grepping a field inside a frame
format we do not carry -- always green, and it looks like a defence.

Caller (see GLOSSARY 3.10): `.github/workflows/ci.yml` step 4 on every push/PR,
plus local `python3 ci/check_ci144_profile.py`.

Rules:
  R1 status must be one of {adopted, absent, deviated}.
  R2 status=adopted  -> evidence must resolve: `path` exists, and `path::symbol`
                        must be greppable when the symbol is given.
  R3 status=absent   -> evidence must carry `revisit:` (absence may not be silent).
  R4 status=deviated -> evidence must carry `why:` (a deviation must be named).
  Reading zero declarations is not clean -> exit 3.

Exit codes: 0 all green / 1 red / 2 malformed table / 3 read nothing
"""
import io
import os
import sys

STATUSES = {"adopted", "absent", "deviated"}
START = "<!-- profile-claims:start -->"
END = "<!-- profile-claims:end -->"


def parse(text):
    if START not in text or END not in text:
        raise SystemExit("CHECKER ERROR: profile is missing the claim-table markers")
    body = text.split(START, 1)[1].split(END, 1)[0]
    rows = []
    for line in body.strip().splitlines():
        line = line.strip()
        if not line.startswith("|") or set(line) <= set("|-: "):
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if len(cells) != 4 or cells[0].lower() == "id":
            continue
        rows.append(cells)
    return rows


def check(root, rows):
    bad = []
    for cid, claim, status, evidence in rows:
        if status not in STATUSES:
            bad.append("%s: status=%r not in %s" % (cid, status, sorted(STATUSES)))
            continue
        if status == "adopted":
            path, _, sym = evidence.partition("::")
            path = path.strip("` ")
            full = os.path.join(root, path)
            if not os.path.isfile(full):
                bad.append("%s: adopted but evidence path is missing: %s" % (cid, path))
                continue
            if sym:
                sym = sym.strip("` ").strip()
                body = io.open(full, encoding="utf-8", errors="replace").read()
                if sym not in body:
                    bad.append("%s: adopted but evidence symbol not found: %s::%s"
                               % (cid, path, sym))
        elif status == "absent":
            if "revisit:" not in evidence:
                bad.append("%s: absent but no `revisit:` trigger" % cid)
        elif status == "deviated":
            if "why:" not in evidence:
                bad.append("%s: deviated but no `why:`" % cid)
    return bad


DEFAULT_FILE = "docs/ci144-profile.md"


def run(root, quiet=False, rel=DEFAULT_FILE):
    text = io.open(os.path.join(root, rel), encoding="utf-8").read()
    rows = parse(text)
    if not rows:
        print("CHECKER DEAD: read 0 declarations -> not clean, read nothing")
        return 3
    bad = check(root, rows)
    if not quiet:
        print("declarations %d: %s" % (
            len(rows),
            ", ".join("%s=%d" % (s, sum(1 for r in rows if r[2] == s)) for s in sorted(STATUSES)),
        ))
    if bad:
        for b in bad:
            print("RED: " + b)
        return 1
    if not quiet:
        print("OK: every declaration resolves, or names its absence/deviation")
    return 0


def selftest():
    import tempfile

    ok = [
        "| c1 | framing | adopted | `src/main.rs::fn main` |",
        "| c2 | registry | absent | revisit: when a physical surface appears |",
        "| c3 | hard override | deviated | why: break-glass refused three times |",
    ]
    cases = [
        ("valid", ok, 0),
        ("bad-status", ["| c1 | framing | yes | `src/main.rs` |"], 1),
        ("missing-path", ["| c1 | framing | adopted | `no/such.rs` |"], 1),
        ("missing-symbol", ["| c1 | framing | adopted | `src/main.rs::no_such_symbol_xyz` |"], 1),
        ("absent-no-revisit", ["| c1 | framing | absent | we never adopted it |"], 1),
        ("deviated-no-why", ["| c1 | framing | deviated | unused |"], 1),
        # Symmetry (§3.4): every exemption mechanism needs a "must be exempted"
        # positive sample, or the exemption rule is untested and can rot to
        # "always red" -- which is indistinguishable from a broken checker.
        ("exempt-positive", [
            "| c1 | framing | absent | revisit: event-triggered when X ships |",
            "| c2 | framing | deviated | why: our transport has no such frame |",
        ], 0),
        ("empty", [], 3),
    ]
    fail = 0
    for name, rows, want in cases:
        with tempfile.TemporaryDirectory() as d:
            os.makedirs(os.path.join(d, "docs"))
            os.makedirs(os.path.join(d, "src"))
            io.open(os.path.join(d, "src/main.rs"), "w").write("fn main() {}\n")
            body = "\n".join(rows)
            io.open(os.path.join(d, "docs/ci144-profile.md"), "w", encoding="utf-8").write(
                "x\n%s\n%s\n%s\n" % (START, body, END)
            )
            got = run(d, quiet=True)
        tag = "PASS" if got == want else "FAIL"
        if got != want:
            fail += 1
        print("%s %-20s want=%d got=%d" % (tag, name, want, got))
    print("selftest: %d/%d" % (len(cases) - fail, len(cases)))
    return 0 if fail == 0 else 1


def main():
    """--root DIR and --file REL let another repo reuse this shape unchanged
    (helix-mind/docs/helixECO/consensus-ledger.md). Same rules, one tool."""
    if "--selftest" in sys.argv:
        return selftest()
    argv = sys.argv[1:]
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    rel = DEFAULT_FILE
    if "--root" in argv:
        root = argv[argv.index("--root") + 1]
    if "--file" in argv:
        rel = argv[argv.index("--file") + 1]
    return run(root, rel=rel)


if __name__ == "__main__":
    sys.exit(main())
