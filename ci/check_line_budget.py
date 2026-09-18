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


def load_fix_windows(path, base_ncloc, pits):
    """Lines a FIX legitimately added, authorized by the pit record it closes.

    This is the third class, and it exists because the first two are both wrong
    for a fix:

      - a **waiver** is a DEBT ("I will repay it later"), which is why it carries
        an owner and a due date. Recording a fix as a waiver announces that the
        fix is a liability. It will come due and be chased for repayment, while
        what actually happened is that the code gained an asset;
      - a **split window** is a REFACTOR allowance ("the structure changed"), so
        it carries a cap and points at an ADR. A fix changes no structure to make
        room for itself; folding fix lines into the split allowance also destroys
        the one number that answers "why split 2093 lines" — estimated 60 against
        an actual that must stay measurable.

    So a fix's legitimacy comes from something else: a **K-ID**. "A change must
    cite the pit it closes" is already the rule in this ledger; this applies it to
    lines. A waiver proves it will vanish; a fix window proves it had to exist.

    **A `due` on a fix window is an error, not an oversight.** A fix carrying a
    deadline is a debt wearing the wrong label, and the entire reason for a third
    class is that those two must not be conflated. Same for a missing `k_id`: it
    is the only evidence this class accepts, so an entry without one is a waiver
    with extra steps.

    Returns {target: (cap, k_id)}. Targets may be files or directories; the
    allowance binds to the exact target, unlike the split window, which spreads
    across directories on purpose to avoid re-opening per subdirectory. A fix
    knows which file and which directory it touched.
    """
    if not os.path.exists(path):
        return {}
    out = {}
    target = k_id = due = reason = None
    cap = 0
    # Which section are we in. Without this the parser reads every `[[waiver]]`
    # as a fix window — it resets on the `[` line, then the waiver's own `target`
    # and `due` lines repopulate the fields and `flush()` reports "a fix window
    # with a due date". That is the same defect this file already documents twice
    # for waivers ("keys that match nothing"), arriving from the other side: a
    # section that matches everything.
    in_fix = False

    def flush():
        nonlocal in_fix
        if not in_fix:
            return
        in_fix = False
        if not target:
            return
        if due is not None:
            raise WaiverError(
                f"fix window for {target!r} carries a due date ({due!r}): a fix is not a debt. "
                "Use [[waiver]] if it will be repaid, or [[split_window]] if it is a refactor."
            )
        if not k_id:
            raise WaiverError(
                f"fix window for {target!r} has no k_id: the pit record is the only thing "
                "that makes a fix legitimate, so an entry without one is a waiver in disguise."
            )
        if not cap:
            raise WaiverError(f"fix window for {target!r} has no cap")
        # A fix reaches files, which the split window deliberately does not — so
        # it is the more abusable class, because any change can be called a fix.
        # The ratio is the symmetric constraint: a cap may not exceed a tenth of
        # the target's own baseline. Without it the class has no upper bound that
        # anyone but its author chose.
        base = base_ncloc.get(target)
        if base is not None:
            ceiling = max(FIX_CAP_FLOOR, int(base * FIX_CAP_RATIO))
            if cap > ceiling:
                raise WaiverError(
                    f"fix window for {target!r} asks for {cap} against a baseline of "
                    f"{base}: the cap must not exceed {ceiling} "
                    f"(max({FIX_CAP_FLOOR}, {int(FIX_CAP_RATIO * 100)}% of baseline)). "
                    "A fix that needs more than that is a rewrite, and a rewrite is "
                    "a split window or an ADR."
                )
        # A cap that cites nothing is a waiver in disguise; a cap that cites an id
        # nobody can resolve is the same thing with more typing.
        unknown = [i for i in k_id.split(",") if i.strip() and i.strip() not in pits]
        if unknown:
            raise WaiverError(
                f"fix window for {target!r} cites {', '.join(unknown)}, which "
                f"{os.path.basename(PITS_FILE)} does not list. Record the pit first: "
                "the id is the only evidence this class accepts."
            )
        out[target] = (cap, k_id)

    for raw in open(path, encoding="utf-8"):
        line = raw.strip()
        if line.startswith("["):
            flush()
            if line == "[[fix_window]]":
                in_fix = True
            target = k_id = due = reason = None
            cap = 0
            continue
        if "=" not in line or line.startswith("#"):
            continue
        if not in_fix:
            continue
        if line.startswith("target"):
            target = line.partition("=")[2].strip().strip('"')
        elif line.startswith("cap"):
            cap = int(line.partition("=")[2].strip())
        elif line.startswith("k_id"):
            k_id = line.partition("=")[2].strip().strip('"')
        elif line.startswith("due"):
            due = line.partition("=")[2].strip().strip('"')
        elif line.startswith("reason"):
            reason = line.partition("=")[2].strip().strip('"')
    flush()
    return out


def load_pits(path):
    """The ids a fix window may cite.

    Read from `ci/pits.toml` rather than trusting the cap's own text: an id that
    resolves to nothing is a waiver wearing a fix's label, and a citation nobody
    can resolve is exactly the "key that matches nothing" defect this checker has
    already been caught by twice.
    """
    if not os.path.exists(path):
        return set()
    out = set()
    for raw in open(path, encoding="utf-8"):
        line = raw.strip()
        if line.startswith("id"):
            out.add(line.partition("=")[2].strip().strip('"'))
    return out


def load_seeds(path):
    """Hand-signed seed values for keys that have no baseline yet.

    A new key is the one place the ratchet can move UP: `min(old, current)` never
    raises an existing value, so the only way a baseline grows is by being absent
    and re-seeded. Left automatic, that is the checker raising its own bound -- the
    round-25 defect (the ratchet could write the value it checks) through a
    different door.

    So a seed is a decision, and it is signed here, out of band, citing the pit or
    ADR that justifies it. The automatic write-back may only CONSUME a signature;
    when it meets a key with none, it refuses and says so.
    """
    if not os.path.exists(path):
        return {}
    out = {}
    cur = None

    def flush():
        if cur is None:
            return
        if not cur.get("target"):
            return
        if not (cur.get("k_id") or cur.get("adr")):
            raise WaiverError(
                f"seed for {cur['target']!r} cites neither a k_id nor an adr. A seed "
                "without a pit is just a number someone wanted."
            )
        if not cur.get("ncloc"):
            raise WaiverError(f"seed for {cur['target']!r} has no ncloc")
        out[cur["target"]] = int(cur["ncloc"])

    in_seed = False
    for raw in open(path, encoding="utf-8"):
        line = raw.strip()
        if line.startswith("["):
            flush()
            cur = {} if line == "[[seed]]" else None
            in_seed = line == "[[seed]]"
            continue
        if not in_seed or "=" not in line or line.startswith("#"):
            continue
        k, _, v = line.partition("=")
        cur[k.strip()] = v.strip().strip('"')
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
PITS_FILE = "ci/pits.toml"
# A fix may add at most this fraction of a target's own baseline. The absolute
# floor keeps small files from being unable to cite any fix at all.
FIX_CAP_RATIO = 0.10
FIX_CAP_FLOOR = 5
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
    """Everything this budget governs.

    Rust under `src/`, plus the checkers in `ci/`. The checkers were the only files
    in the repository that nothing counted: CI-7 decides whether every guard in the
    repo is real, and no budget, ratchet or waiver applied to it — nor to CI-6,
    which had the same exemption for the same accidental reason (the walker looked
    for `.rs`). The auditors were the sole unaudited files.

    Meta-level caps here: **there is no CI-8.** If a third checker is ever needed,
    it means CI-7 was built wrong, not that the ladder needs another rung. A
    checker's checker is still a checker, and the recursion has to stop somewhere
    or it is only building upward.
    """
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in DEFAULT_EXCLUDES]
        for name in sorted(filenames):
            rel = os.path.relpath(os.path.join(dirpath, name), root)
            if name.endswith(".rs"):
                yield rel
            elif name.endswith(".py") and rel.replace(os.sep, "/").startswith("ci/"):
                yield rel


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


def run(root, budget, branch_budget, max_line, check, line_mode='warn', ratchet=True, emit=False, override=None):
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
    # The ratchet lives in its own GENERATED file. It is a derived
    # measurement, not an exemption, so the checker may write it — but it
    # must not live somewhere a human signs. Loaded before the allowances
    # because a fix cap is bounded by the baseline it applies to.
    ratchet_path = os.path.join(root, RATCHET_FILE)
    ratchet_base = load_ratchet(ratchet_path) if ratchet else {}
    # Captured BEFORE any override: an override is a test affordance, and letting
    # it inflate a fix cap would mean a forced baseline could widen an allowance.
    cap_base = dict(ratchet_base)
    pits_path = os.path.join(root, PITS_FILE)
    try:
        waived, expired = load_waivers(waivers_path, check, today)
        # Both window loaders raise WaiverError too, and they must be inside this
        # try for the same reason the waivers are: exit 3 means "the checker could
        # not run", exit 1 means "it ran and found violations". A malformed
        # window escaping as an uncaught traceback exits 1, which reports a
        # typo in the baseline as a violation in the code — the two outcomes this
        # whole file exists to keep apart. The split window had this gap from the
        # start; it was found while adding the third class next to it.
        split_windows = load_split_windows(waivers_path, today) if ratchet else {}
        fix_windows = (
            load_fix_windows(waivers_path, cap_base, load_pits(pits_path)) if ratchet else {}
        )
        seeds = load_seeds(waivers_path) if ratchet else {}
    except WaiverError as e:
        print(f"CHECKER ERROR: {waivers_path}: {e}", file=sys.stderr)
        return 3

    if override:
        # Passed per call rather than held in module state: the tests run
        # concurrently in one process, and a global leaked one test's forced
        # baseline into another's run.
        ratchet_base[override[0]] = override[1]
    grown = []
    unseeded = []
    oversigned = []
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
        # A fix window is the one allowance that reaches FILES. The split window
        # deliberately does not: it loosens a directory aggregate while every
        # single file stays pinned, because a refactor moves lines and does not
        # add them to one file. A fix does add them to one file, and it says
        # which one.
        fix_allow = fix_windows.get(key, (0, None))[0]
        if key in ratchet_base and n > ratchet_base[key] + fix_allow:
            grown.append((rel, ratchet_base[key], n, fix_allow))
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
        # Exact target only. The split window spreads its cap across every
        # directory because a split creates directories as it goes; a fix names
        # the directory it grew, and spreading its cap would hand the same slack
        # to directories the fix never touched.
        fix_allow = fix_windows.get(dk, (0, None))[0]
        limit = ratchet_base[dk] + allow + fix_allow
        if v > limit:
            dir_grown.append((k, ratchet_base[dk], v, split_windows.get(dk, 0), fix_allow))
    blocking = marker_failures or expired_note or grown or dir_grown or unseeded or oversigned
    if not split_window_open:
        blocking = blocking or bool(over_budget)
    if line_mode == "fail":
        blocking = blocking or long_lines
    # Monotone write-back: the new baseline is min(old, current). Auto-taking the
    # current value would let the ratchet follow the code upward and never fire;
    # a hand-written value would rot and would also let a file grow back after
    # shrinking. min() is the only form that keeps the property one-directional,
    # and it is derived, so nobody types a number (P8).
    # A forced baseline is a test affordance; writing it back would persist the
    # fabricated value into the real baseline and break every later run.
    # Seed validation is a PRE-PASS, not a report after the fact. Two reasons, and
    # the first version got both wrong: `blocking` is computed above, so a check
    # placed after the write-back sat behind `return 0` and could never fire -- it
    # passed its own positive control by doing nothing. And more importantly, an
    # invalid seed must stop the write-back rather than be reported alongside it,
    # or the checker has already taken the upward step it is complaining about.
    if ratchet and not emit:
        for k, v in current.items():
            if k in ratchet_base:
                continue
            signed = seeds.get(k)
            if signed is None:
                unseeded.append(k)
            elif v > signed:
                oversigned.append((k, signed, v))
        if unseeded or oversigned:
            print(f"CHECKER ERROR: {len(unseeded) + len(oversigned)} key(s) have no valid signed seed:", file=sys.stderr)
            for k in unseeded:
                print(f"        {k}: no [[seed]] entry, so the automatic write-back would be "
                      f"setting a baseline for the first time. A seed is a decision: add "
                      f"[[seed]] target = \"{k}\" ncloc = <current> k_id = \"<pit>\".", file=sys.stderr)
            for k, signed, v in oversigned:
                print(f"        {k}: signed for {signed} but measures {v}. Re-take the seed by "
                      f"editing baseline.toml deliberately; letting the checker follow the code "
                      f"upward is the one direction a ratchet must not move.", file=sys.stderr)
            return 3

    if ratchet and not emit and not override:
        merged = dict(ratchet_base)
        for k, v in current.items():
            if k in merged:
                merged[k] = min(merged[k], v)
                continue
            signed = seeds.get(k)
            if signed is None:
                unseeded.append(k)
                continue
            if v > signed:
                oversigned.append((k, signed, v))
                continue
            merged[k] = v
        for k, v in dir_totals.items():
            dk = k + "/"
            if split_window_open:
                # Inside the window the directory baseline is FROZEN at its
                # opening value. It must not be lowered either: the checker runs
                # many times during a split, and a monotone min() would walk the
                # baseline down the current total on every run, spending the
                # allowance a little at a time until the gate stopped existing.
                # That is exactly what happened — a baseline of 3499 against a
                # total of 3499 means "the next 60 lines are free", and repeated
                # runs kept it there.
                #
                # A frozen value plus the allowance is a real bound: growth past
                # baseline+allowance fails. On close, the window re-seeds once
                # from the actual total.
                if dk not in merged:
                    merged[dk] = v
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
    # Printed BEFORE the early PASS return. It used to sit after it, so the
    # advisory appeared only when something else had already failed -- a file 757
    # lines over a 300-line budget was reported to nobody on a clean run.
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
        for k, was, now, allow, fix_allow in dir_grown:
            extra = f" (with a +{allow} split allowance)" if allow else ""
            if fix_allow:
                extra += f" (with a +{fix_allow} fix allowance)"
            print(f"        {k}/  {was} -> {now}{extra}")
    if grown:
        print(f"FAIL  {len(grown)} file(s) grew past their ratchet:")
        for rel, was, now, fix_allow in grown:
            extra = f" (with a +{fix_allow} fix allowance)" if fix_allow else ""
            print(f"        {rel}  {was} -> {now}{extra}")
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
    # Test-only: force a baseline value, so a test can prove that a WRONG
    # baseline is caught rather than silently trusted. A checker that can
    # only fail when reality is wrong is blind to its own inputs being
    # wrong, and this one had exactly that failure: a directory baseline of
    # 3499 against a total of 3499 went unnoticed for two commits.
    ap.add_argument("--ratchet-override", default=None)
    ap.add_argument("--emit-ratchet", action="store_true")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()
    override = None
    if args.ratchet_override:
        target, _, value = args.ratchet_override.partition("=")
        override = (target, int(value))
    code = run(args.root, args.budget, args.branch_budget, args.max_line, args.check,
               args.line_mode, not args.no_ratchet, args.emit_ratchet, override)
    if args.json:
        print(json.dumps({"exit": code}))
    return code


if __name__ == "__main__":
    sys.exit(main())
