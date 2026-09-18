#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""CI-6 — the line budget, checked rather than declared.

Three things are checked, and the first is the one that makes the other two
trustworthy:

  1. NCLOC per source file against a budget, with waivers read from the shared
     `ci/baseline.toml`.
  2. Files marked `//! DATA-ONLY` really are data-only: a branch budget, so that
     the marker cannot be used as a self-signed pass. A marker nobody verifies
     is the same shape as "AI 无权修改" — the claim is present and the mechanism
     is not.
  3. Lines longer than a ceiling — squashing code onto one line lowers the line
     count without lowering the difficulty. This one WARNS by default and does
     not change the exit code: the tree currently has long string literals and
     data-heavy lines that are legitimately long, and a check that fails on them
     would be routed around rather than obeyed. `--line-mode fail` makes it
     blocking where that is the right call.

Exit codes, matching the shared convention:
    0  every check passed
    1  at least one violation (the check ran and judged)
    3  the checker itself could not run (bad input, missing tool) — never 0

Why 3 matters: a checker that returns 0 when it cannot run reports success for
work it never did. That is the fifth time this family has met a default value
that disguises an absence as a legitimate result.

Counting comments out is deliberate (NCLOC, not physical lines): the budget
exists to bound what a reader must hold in their head, and comments do not add to
that. It also makes writing a file-header contract summary free, which is the
outcome we want.

Usage:
    python3 ci/check_line_budget.py [--root DIR] [--budget N] [--branch-budget N]
                                    [--max-line N] [--json]
"""

import argparse
import json
import os
import re
import sys
from datetime import date

# ── counting ────────────────────────────────────────────────────────────────

# A line is code unless it is blank or starts a comment. Block comments are
# tracked with a nesting-free state machine; Rust block comments do nest, so the
# depth is counted.
_BLOCK_OPEN = "/*"
_BLOCK_CLOSE = "*/"


def ncloc(text):
    """Non-comment, non-blank physical lines."""
    count = 0
    depth = 0
    for raw in text.splitlines():
        line = raw
        out = []
        i = 0
        while i < len(line):
            if depth == 0 and line.startswith(_BLOCK_OPEN, i):
                depth += 1
                i += 2
                continue
            if depth > 0:
                if line.startswith(_BLOCK_CLOSE, i):
                    depth -= 1
                    i += 2
                    continue
                i += 1
                continue
            if line.startswith("//", i):
                break
            out.append(line[i])
            i += 1
        if "".join(out).strip():
            count += 1
    return count


# Branch keywords at the start of a statement. Deliberately conservative: a
# DATA-ONLY file with ten branches should be read as logic, and a marker that
# survives ten branches is not doing any work.
_BRANCH = re.compile(r"^\s*(if|match|for|while|loop)\b")


def branch_count(text):
    """Branches in the PRODUCTION part only.

    Test modules are excluded because counting them would let a file with a
    branch-free body fail the marker test for having thorough tests — which would
    push people to delete the tests or the marker, and the marker is the cheaper
    thing to lose.
    """
    cut = text.find("#[cfg(test)]")
    prod = text[:cut] if cut > 0 else text
    return sum(1 for line in prod.splitlines() if _BRANCH.match(line))


# ── waivers ─────────────────────────────────────────────────────────────────

class WaiverError(Exception):
    """The waiver file is unreadable or malformed. Never silently ignored."""


def load_ratchet(path):
    """file -> the NCLOC it had when the ratchet was set.

    A ratchet asks a question that needs no calibration: is this file longer
    than it was? That is decidable, cannot misjudge "how big is too big", and
    cannot be dodged by arguing about the threshold. It also makes the budget
    self-tightening — every later edit must fit in the space a previous edit
    left behind.
    """
    if not os.path.exists(path):
        return {}
    out = {}
    target = None
    for raw in open(path, encoding="utf-8"):
        line = raw.strip()
        if line == "[[ratchet]]":
            target = None
        elif line.startswith("target"):
            target = line.partition("=")[2].strip().strip('"')
        elif line.startswith("ncloc") and target:
            out[target] = int(line.partition("=")[2].strip())
    return out


def load_split_windows(path, today):
    """dir -> headroom allowed while a refactor is in flight.

    A pure-move split cannot be zero-delta on NCLOC, and the measurement says why:
    `criteria/mod.rs` went from 291 to 295 when split into three files, because
    `mod.rs` gains a `pub use` line per part that did not exist before. That part
    of the increase is permanent, and the rest is the ordinary weight of files
    naming what they use.

    That matters because the directory ratchet would otherwise go red on the
    first day of a split, and the red would be the checker working correctly. The
    wrong fixes are to make the ratchet report-only or to waive it — both teach
    that refactoring means switching the gate off. Instead the increase is
    authorized IN ADVANCE, with a number, a reason and a date, so it is a bounded
    and auditable allowance rather than a hole.
    """
    if not os.path.exists(path):
        return {}
    out = {}
    target = reason = due = None
    allow = 0

    def flush():
        if not target or not due:
            return
        try:
            y, m, d = (int(x) for x in due.split("-"))
            when = date(y, m, d)
        except Exception:
            raise WaiverError(f"split window for {target!r} has an unparseable due date: {due!r}")
        if when >= today and allow:
            out[target] = allow

    for raw in open(path, encoding="utf-8"):
        line = raw.strip()
        if line == "[[split_window]]":
            flush()
            target = reason = due = None
            allow = 0
        elif line.startswith("["):
            flush()
            target = reason = due = None
            allow = 0
        elif line.startswith("target"):
            target = line.partition("=")[2].strip().strip('"')
        elif line.startswith("allowance"):
            allow = int(line.partition("=")[2].strip())
        elif line.startswith("due"):
            due = line.partition("=")[2].strip().strip('"')
    flush()
    return out


def load_waivers(path, check, today):
    """Returns (active_targets, expired_targets).

    A malformed waiver is an error rather than a skip. If the key spelling ever
    drifts, every waiver silently stops matching and the check reports a clean
    tree it never examined — which is exactly what happened to the first version
    of the vector check.
    """
    if not os.path.exists(path):
        return set(), []
    active, expired = set(), []
    check_field = target = due = owner = None
    in_other_table = False

    def flush():
        if check_field != check or not target:
            return
        if not owner:
            raise WaiverError(f"waiver for {target!r} has no owner")
        try:
            y, m, d = (int(x) for x in due.split("-"))
            when = date(y, m, d)
        except Exception:
            raise WaiverError(f"waiver for {target!r} has an unparseable due date: {due!r}")
        (active if when >= today else expired).add(target)

    for raw in open(path, encoding="utf-8"):
        line = raw.strip()
        if line.startswith("["):
            if line == "[[waiver]]":
                flush()
                check_field = target = due = owner = None
                in_other_table = False
            else:
                # Any other table ends the waiver section. Without this, a
                # `[[ratchet]]` block appended below is parsed as part of the
                # waiver above it, and every waiver after it silently stops
                # matching — the same "keys that match nothing" defect this
                # checker was already caught by once.
                flush()
                check_field = target = due = owner = None
                in_other_table = True
            continue
        if "=" not in line or line.startswith("#"):
            continue
        if in_other_table and not line.startswith("["):
            continue
        key, _, value = line.partition("=")
        value = value.strip().strip('"')
        key = key.strip()
        if key == "check":
            check_field = value
        elif key == "target":
            target = value
        elif key == "due":
            due = value
        elif key == "owner":
            owner = value
    flush()
    return active, sorted(expired)


# ── the check ───────────────────────────────────────────────────────────────

DATA_ONLY_MARK = "//! DATA-ONLY"
RATCHET_FILE = "ci/ratchet.gen.toml"
RATCHET_HEADER = """# GENERATED — do not edit. Regenerated by ci/check_line_budget.py.
#
# The ratchet is a derived measurement, not an exemption, which is why the
# checker writes it while waivers (ci/baseline.toml) stay human-signed.
#
# Each value is min(previous, current): a file may shrink and its ceiling
# follows it down, and it can never grow back. Directory entries end in `/` and
# bound the sum, which is what stops a large file being split into several small
# ones to reset the per-file number.
"""
DEFAULT_EXCLUDES = ("target", "node_modules", ".git")


def iter_sources(root):
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in DEFAULT_EXCLUDES]
        for name in sorted(filenames):
            if name.endswith(".rs"):
                yield os.path.relpath(os.path.join(dirpath, name), root)


def is_test_path(rel):
    """Test code is out of scope for the line budget (spec §3).

    Two shapes count, and the second was missing until a refactor surfaced it:
    the crate's `tests/` directory, and a test module that lives beside the code
    it tests. `src/run_cycle/tests.rs` is named by `mod tests;` from
    `src/run_cycle/mod.rs`, so it is test code in every sense except its path —
    and without this it was budgeted as production.
    """
    parts = rel.split(os.sep)
    if "tests" in parts[:-1] or rel.endswith("_test.rs"):
        return True
    base = os.path.basename(rel)
    # A file whose stem is `tests` inside a module directory.
    return base == "tests.rs" or base.endswith("_tests.rs")


def run(root, budget, branch_budget, max_line, check, line_mode='warn', ratchet=True, emit=False):
    today = date.today()
    iso = os.environ.get("CI_TODAY")
    if iso:
        y, m, d = (int(x) for x in iso.split("-"))
        today = date(y, m, d)

    waivers_path = os.path.join(root, "ci", "baseline.toml")
    # A waiver whose target no longer exists exempts nothing while looking
    # present. The check does fail for it, but it reported the symptom ("file over
    # budget") rather than the cause ("this key is stale"), so whoever read the red
    # went looking at line counts. Same family as everything else here: an object
    # that appears to work and does not.
    all_waiver_targets = set()
    try:
        with open(waivers_path, encoding="utf-8") as fh:
            _chk = None
            for line in fh:
                t = line.strip()
                if t == "[[waiver]]":
                    _chk = None
                elif t.startswith("check"):
                    _chk = t.partition("=")[2].strip().strip('"')
                elif t.startswith("target") and _chk == check:
                    all_waiver_targets.add(t.partition("=")[2].strip().strip('"'))
    except OSError:
        pass
    try:
        waived, expired = load_waivers(waivers_path, check, today)
    except WaiverError as e:
        print(f"CHECKER ERROR: {waivers_path}: {e}", file=sys.stderr)
        return 3

    # The ratchet lives in its own GENERATED file. It is a derived
    # measurement, not an exemption, so the checker may write it — but it
    # must not live somewhere a human signs.
    ratchet_path = os.path.join(root, RATCHET_FILE)
    ratchet_base = load_ratchet(ratchet_path) if ratchet else {}
    split_windows = load_split_windows(waivers_path, date.today()) if ratchet else {}
    grown = []
    current = {}
    dir_totals = {}
    # A split renames files, so every segment produces a file with no waiver key
    # and a stale key for the old name. Both would fire on each of the seven
    # segments, and both are cleared by editing ci/baseline.toml — which is easier
    # than editing code, and therefore a route around the check. So inside an
    # authorized window these are advisory, and the window closes with one
    # alignment pass.
    split_window_open = bool(split_windows)

    violations, marker_failures, long_lines, over_budget = [], [], [], []
    for rel in iter_sources(root):
        if is_test_path(rel):
            continue
        path = os.path.join(root, rel)
        try:
            text = open(path, encoding="utf-8").read()
        except OSError as e:
            print(f"CHECKER ERROR: cannot read {rel}: {e}", file=sys.stderr)
            return 3

        # A DATA-ONLY file is out of scope for the line budget, but ONLY if it
        # passes the branch budget below. The marker is a claim, and the claim
        # gets checked — that is the difference between this and a file signing
        # its own pass.
        data_only = DATA_ONLY_MARK in text
        n = ncloc(text)
        current[rel.replace(os.sep, "/")] = n
        key = rel.replace(os.sep, "/")
        if key in ratchet_base and n > ratchet_base[key]:
            grown.append((rel, ratchet_base[key], n))
        # Directory totals too: a file-level ratchet alone can be dodged by
        # splitting one file into two, because new files have no baseline. The
        # directory total cannot be dodged that way — moving lines around inside
        # a directory leaves the sum unchanged.
        direc = os.path.dirname(key) or "."
        dir_totals[direc] = dir_totals.get(direc, 0) + n
        if not data_only and n > budget and rel not in waived and rel.replace(os.sep, "/") not in waived:
            over_budget.append((rel, n))

        if data_only:
            b = branch_count(text)
            if b > branch_budget:
                marker_failures.append((rel, b))

        for i, line in enumerate(text.splitlines(), 1):
            if len(line) > max_line:
                long_lines.append((rel, i, len(line)))

    expired_note = []
    if expired:
        expired_note.append(
            f"{len(expired)} waiver(s) are past due and exempt nothing: {', '.join(expired)}"
        )

    dir_grown = []
    for k, v in dir_totals.items():
        dk = k + "/"
        if dk not in ratchet_base:
            continue
        # The allowance is granted for the SPLIT, not for one directory path: a
        # segment that creates a subdirectory produces a directory with no
        # baseline at all, and without this the window would have to be re-opened
        # per directory — seven more key edits, which is the route the window
        # exists to close.
        allow = split_windows.get(dk, 0)
        if dk in split_windows or split_window_open:
            allow = max(allow, max(split_windows.values(), default=0))
        limit = ratchet_base[dk] + allow
        if v > limit:
            dir_grown.append((k, ratchet_base[dk], v, split_windows.get(dk, 0)))
    blocking = marker_failures or expired_note or grown or dir_grown
    if not split_window_open:
        blocking = blocking or bool(over_budget)
    if line_mode == "fail":
        blocking = blocking or long_lines
    # Monotone write-back: the new baseline is min(old, current). Auto-taking the
    # current value would let the ratchet follow the code upward and never fire;
    # a hand-written value would rot and would also let a file grow back after
    # shrinking. min() is the only form that keeps the property one-directional,
    # and it is derived, so nobody types a number (P8).
    if ratchet and not emit:
        merged = dict(ratchet_base)
        for k, v in current.items():
            merged[k] = min(merged[k], v) if k in merged else v
        for k, v in dir_totals.items():
            dk = k + "/"
            if dk in split_windows or split_window_open:
                # Inside an authorized window the number moves for reasons that
                # are not shrink-or-grow signal; re-seeded when the window closes.
                continue
            merged[dk] = min(merged[dk], v) if dk in merged else v
        for k in list(merged):
            # A source that vanished stops being a baseline; leaving it would
            # make a re-added file inherit a stale, possibly generous, number.
            if not k.endswith("/") and k not in current:
                del merged[k]
        try:
            with open(ratchet_path, "w", encoding="utf-8") as fh:
                fh.write(RATCHET_HEADER)
                for k in sorted(merged):
                    fh.write(f'[[ratchet]]\ntarget = "{k}"\nncloc  = {merged[k]}\n')
        except OSError as e:
            print(f"CHECKER ERROR: cannot write {ratchet_path}: {e}", file=sys.stderr)
            return 3

    # Targets that are neither an existing file nor a directory entry.
    seen_paths = set(current)
    stale = sorted(
        t for t in all_waiver_targets
        if t not in seen_paths and not t.endswith("/")
    )
    # Inside an authorized split window, a stale key is EXPECTED: a split renames
    # a file on every segment, so its waiver key expires on every segment. Failing
    # there would fire seven times for seven renames and train the habit of
    # editing the key to clear the red — and editing a key is easier than editing
    # code, which makes it a route around the check. So it warns inside the window
    # and blocks when the window closes, where the keys are aligned once.
    window_open = split_window_open
    if stale and not emit:
        tag = "WARN" if window_open else "STALE_EXEMPTION"
        print(f"{tag}  {len(stale)} waiver(s) name a path that no longer exists:")
        for t in stale:
            extra = (
                "  (expected during a split window: align it when the window closes)"
                if window_open
                else "  (this key exempts nothing; update it or delete it)"
            )
            print(f"        {t}{extra}")
        if not window_open:
            blocking = True

    if emit:
        print("[[ratchet]] entries for ci/baseline.toml:")
        for rel in sorted(current):
            print(f'[[ratchet]]\ntarget = "{rel}"\nncloc  = {current[rel]}\n')
        return 0

    ok = not blocking
    if ok:
        extra = ""
        if long_lines:
            extra = f"  ({len(long_lines)} long line(s), advisory)"
        print(f"PASS  NCLOC <= {budget}  DATA-ONLY branches <= {branch_budget}"
              f"  lines <= {max_line}{extra}")
        return 0

    if dir_grown:
        print(f"FAIL  {len(dir_grown)} directory total(s) grew past their ratchet:")
        for k, was, now, allow in dir_grown:
            extra = f" (with a +{allow} split allowance)" if allow else ""
            print(f"        {k}/  {was} -> {now}{extra}")
    if grown:
        print(f"FAIL  {len(grown)} file(s) grew past their ratchet:")
        for rel, was, now in grown:
            print(f"        {rel}  {was} -> {now}")
    if over_budget:
        tag = "WARN" if split_window_open else "FAIL"
        note = (
            "  (advisory during a split window: these are the segments being carved out)"
            if split_window_open
            else ""
        )
        print(f"{tag}  {len(over_budget)} file(s) over {budget} NOCL with no waiver:{note}")
        for rel, n in over_budget:
            print(f"        {rel}  {n}")
    if marker_failures:
        print(
            f"FAIL  {len(marker_failures)} file(s) marked DATA-ONLY exceed {branch_budget} branches "
            f"— the marker is not a pass:"
        )
        for rel, b in marker_failures:
            print(f"        {rel}  {b} branches")
    if long_lines:
        tag = "FAIL" if line_mode == "fail" else "WARN"
        print(f"{tag}  {len(long_lines)} line(s) over {max_line} characters"
              f"{'' if line_mode == 'fail' else ' (advisory; does not change the exit code)'}:")
        for rel, i, n in long_lines[:20]:
            print(f"        {rel}:{i}  {n}")
    for note in expired_note:
        print(f"FAIL  {note}")
    return 1


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default=".")
    ap.add_argument("--budget", type=int, default=300)
    ap.add_argument("--branch-budget", type=int, default=10)
    ap.add_argument("--max-line", type=int, default=120)
    ap.add_argument("--check", default="CI-6")
    ap.add_argument("--line-mode", choices=("warn", "fail"), default="warn")
    ap.add_argument("--no-ratchet", action="store_true")
    ap.add_argument("--emit-ratchet", action="store_true")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()
    code = run(args.root, args.budget, args.branch_budget, args.max_line, args.check,
               args.line_mode, not args.no_ratchet, args.emit_ratchet)
    if args.json:
        print(json.dumps({"exit": code}))
    return code


if __name__ == "__main__":
    sys.exit(main())
