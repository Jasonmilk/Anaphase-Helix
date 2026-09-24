"""Reading the allowance ledger: `ci/baseline.toml`.

Split out of `check_line_budget.py` when that file passed its own tooling tier for the
fifth time in one work unit. Its seed reason said, in advance, that the signal would be
a split rather than another re-take — and a re-take is exactly the papering-over this
ledger keeps recording. The checker kept growing because every new rule about
allowances was written next to the code that enforces them; this file is where the
reading lives, and the enforcing stays behind.

Nothing here decides anything. Every function turns one section of the ledger into a
dict, and every one of them raises `WaiverError` rather than skipping: a malformed
entry that is ignored reads exactly like an entry that passed.
"""

import os
import subprocess
from datetime import date


class WaiverError(Exception):
    pass


PITS_FILE = "ci/pits.toml"
# A fix may add at most this fraction of a target's own baseline.
FIX_CAP_RATIO = 0.10

# The floor exists because a percentage of a small file is not a usable allowance: 10%
# of a 56-line module is 5 lines, and a correctness fix does not fit in 5 lines.
#
# **The previous value, 5, was a guess and it was wrong by an order of magnitude.** It
# produced three bypasses in one session — a raised seed, a bounded window opened by
# hand, and a class switch — each of which was a way around this number rather than a
# way to fix it. The reviewer's diagnosis: I kept opening escape hatches instead of
# correcting the rule, and a rule that has to be escaped three times is telling you it
# is wrong.
#
# Measured from this repository's own history rather than chosen: every `fix` commit
# that touched `src/`, net lines, all time.
#
#     n = 27   min = 3   Q1 = 36   median = 71   Q3 = 136   max = 430
#
# The floor is the LOWER QUARTILE, not the minimum and not the median. The minimum (3)
# would leave the floor where it was and keep the problem; the median (71) would be
# generous enough that the ratio stopped meaning anything for small files. Q1 covers
# three quarters of real fixes while still bounding the top.
#
# Reproduce with:
#   git log --format=%h --grep='^fix' | while read h; do git show --numstat --format= $h -- 'src/*.rs'; done
FIX_CAP_FLOOR = 36


def load_data_only_causes(path):
    """`{target: (cause, due)}` — why a hot DATA-ONLY file is still DATA-ONLY.

    The churn criterion must not be passable by silence, and it must not be passable
    by deleting the marker either: `config.rs` genuinely has 7 branches, so calling it
    ordinary source would be the wrong correction. What it demands is a STATEMENT —
    that someone looked at why the file changes and wrote it down.
    """
    if not os.path.exists(path):
        return {}
    out, cur, in_sec = {}, None, False

    def flush():
        if cur and cur.get("target") and cur.get("cause"):
            out[cur["target"]] = (cur["cause"], cur.get("due"))

    for raw in open(path, encoding="utf-8"):
        line = raw.strip()
        if line.startswith("["):
            flush()
            in_sec = line == "[[data_only_churn]]"
            cur = {} if in_sec else None
            continue
        if not in_sec or "=" not in line or line.startswith("#"):
            continue
        k, _, v = line.partition("=")
        cur[k.strip()] = v.strip().strip('"')
    flush()
    return out


def measure_churn(root, commits=200):
    """How many of the last `commits` commits touched each path.

    The window is a parameter and is reported wherever the number is used, because a
    churn figure without its window is an unexplained number. FAIL-OPEN: outside a git
    checkout, or without git, this returns empty and the criterion simply does not
    apply — a check that cannot run must not read as a check that found something.
    """
    try:
        r = subprocess.run(["git", "log", "--format=", "--name-only", f"-{commits}"],
                           cwd=root, capture_output=True, text=True, timeout=60)
    except (OSError, subprocess.SubprocessError, subprocess.TimeoutExpired):
        return {}
    if r.returncode != 0:
        return {}
    counts = {}
    for line in r.stdout.splitlines():
        line = line.strip()
        if line:
            counts[line] = counts.get(line, 0) + 1
    return counts


def load_tooling(path):
    """The tooling tier: `{path_prefix: (budget, adr)}`.

    A CLASSIFICATION rather than an exemption. An exemption says "this file may break
    the rule" and carries a due date, because it is a debt. A classification says
    "this is a different kind of file and the rule was never about it" and carries no
    date, because nothing is owed.

    The 300-line budget approximates "how much a reader must hold in their head" for a
    Rust module. A checker is not that shape: it carries several allowance classes, a
    ratchet, a seed gate and a counter, and its size tracks the number of rules it
    enforces. Judging it by the module limit produces a file permanently over budget
    and kept alive by a waiver that keeps coming due — a rule generating paperwork
    rather than a bound doing work. See ADR-0042 D9.
    """
    if not os.path.exists(path):
        return {}
    out, cur = {}, None

    def flush():
        if cur and cur.get("target") and cur.get("budget"):
            out[cur["target"]] = (int(cur["budget"]), cur.get("adr") or cur.get("reason") or "?")

    in_tool = False
    for raw in open(path, encoding="utf-8"):
        line = raw.strip()
        if line.startswith("["):
            flush()
            in_tool = line == "[tooling]"
            cur = {} if in_tool else None
            continue
        if not in_tool or "=" not in line or line.startswith("#"):
            continue
        k, _, v = line.partition("=")
        cur[k.strip()] = v.strip().strip('"')
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


def load_seeds(path, today=None, counter_version=None):
    """Hand-signed seed values for keys that have no baseline yet.

    A new key is the one place the ratchet can move UP: `min(old, current)` never
    raises an existing value, so the only way a baseline grows is by being absent
    and re-seeded. Left automatic, that is the checker raising its own bound -- the
    round-25 defect (the ratchet could write the value it checks) through a
    different door.

    So a seed is a decision, and it is signed here, out of band, citing the pit or
    ADR that justifies it. The automatic write-back may only CONSUME a signature;
    when it meets a key with none, it refuses and says so.

    **`grandfathered` is a migration window, not a permanent category.** Three keys
    were seeded automatically before this gate existed and were signed afterwards.
    Signing them after the fact is honest but it dilutes the rule: if any past value
    can be signed retroactively, "hand-signed" stops meaning "decided in advance".
    So a grandfathered seed carries a deadline, and past it the checker refuses —
    the seed must be re-taken deliberately, or the file split, or the entry removed.
    Same shape as `[[split_window]]`: bounded, dated, and it expires on its own.
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
        if cur.get("counter") is None:
            raise WaiverError(
                f"seed for {cur['target']!r} does not record which counter version it was "
                "signed under. A hand signature binds who may change a value; it says "
                "nothing about whether the value is right, so the version of the thing "
                "that produced it has to travel with it."
            )
        if int(cur["counter"]) != counter_version:
            raise WaiverError(
                f"seed for {cur['target']!r} was signed under counter version "
                f"{cur['counter']}, and the counter is now version {counter_version}. "
                "The recorded value is a correct measurement of a DIFFERENT rule, so it "
                "must be re-taken deliberately — this is the check that would have caught "
                "343 and 811 without anyone deciding to scan `ci/`."
            )
        out[cur["target"]] = (int(cur["ncloc"]), cur.get("grandfathered"), cur.get("counter"), cur.get("k_id") or cur.get("adr"))

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
        # Entries are keyed by target, so a second one REPLACES the first —
        # TOML-legal, semantically not. Measured 2026-09-24: adding a fix window
        # for `src/adapters/` silently replaced the K-044 window's 104 lines with
        # 69 — the allowance SHRANK and the check got strictly worse, with nothing
        # said. Existence is not semantics; the loader refuses rather than picking
        # one, because which of the two the author meant is not knowable here.
        if target in out:
            raise WaiverError(
                f"two fix windows target {target!r}: they share a key, so the second "
                f"would silently replace the first ({out[target][0]} -> {cap}). "
                "Merge them into one entry — a target has one allowance, not several."
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

# Bump whenever the COUNTING LOGIC changes — what a language's comments are, what
# counts as a blank line, which extensions are in scope. Every seed and every
# ratchet baseline is derived from this counter, so a change here silently
# invalidates all of them: the old numbers were correct measurements of a different
# rule.
#
# Round 36's point, and it is the reason this exists: the seeds are hand-signed,
# which binds WHO may change a value and says nothing about whether the value is
# RIGHT. The only source of a seed's number is this counter, so a counter bug signs
# a wrong number in good faith. That happened — 343 and 811 were signed, and were
# wrong by 32 and 144 — and it was found by a human deciding to scan `ci/`, not by
# anything in this file. Next time nobody will scan.
#
#   1 — Rust only: `//`, `/* */`, everything else fell through as code
#   2 — language-aware: added the `hash` path (# comments, quote-aware, docstring
#       state carried across lines) and a language whitelist. Existing seeds were
#       re-signed under version 2; their old values were measurements of version 1.

