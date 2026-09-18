#!/usr/bin/env bash
# The N-run experiment: does the failure still happen, or did I only stop looking?
#
# "I fixed it" and "it stopped happening" are different claims. A single green run
# proves a world exists where the suite passes; it says nothing about whether the
# failing world is gone. So this runs the suite N times and reports the counts —
# and, more importantly, refuses to report them if the tree moved underneath it.
#
# **Why N = 10.** The observed failure rate was roughly one run in three. Ten
# independent runs all green therefore has probability about (2/3)^10 ≈ 1.7% under
# the hypothesis that nothing changed. That is the whole argument for 10 and it is
# written here because the next person needs to know whether 3 would do (it would
# not: (2/3)^3 ≈ 30%) and whether 20 is needed (only if the rate is lower, which
# would itself be worth knowing). **An unexplained N is the same defect as an
# unexplained number anywhere else in this ledger.**
#
# **The mechanism this was used to confirm** was a parallel race, not merely
# "a test read mutated source". `cargo test` runs test binaries concurrently, so
# while CI-7 held a mutation in `src/`, different binaries could be built from
# different revisions — and only the one landing inside the window went red. The
# general constraint, which is not about `run_cycle_deterministic_replay` at all:
#
#   **Any test that reads a file at runtime, or depends on a build artifact, is
#   racing with anything else that writes the tree, inside a parallel suite.**
#
# So the fix was structural — take the mutator out of the suite — and not a repair
# of that test.
#
# TERMS, fixed here because round 34 caught them drifting:
#   POSITIVE control — construct a case that SHOULD fail, and require it to fail.
#                      If it passes, the check is not checking.
#   NEGATIVE control — run the check against known-good input and require it to
#                      pass. It guards against false alarms.
# Replaying 40 historical commit messages to count false rejections is a NEGATIVE
# control, and calling it a positive one — as an earlier round did — makes a check
# that can only ever pass look effective. Getting the two the wrong way round is
# how a permanently-green check survives review.
#
# Usage:  ci/n_runs.sh [N]            (default 10)
#         ci/n_runs.sh --selftest-guard
#
# Exit: 0 = all green AND the tree held still; 1 = a failure, or an invalid run.
set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

dirty_count() { git -C "$ROOT" status --porcelain 2>/dev/null | wc -l | tr -d ' '; }
head_rev()    { git -C "$ROOT" rev-parse HEAD 2>/dev/null; }

# The guard is a mechanism, so it gets a control. Run it on a scratch repository
# that is deliberately made dirty and require it to notice — otherwise
# "dirty 0 -> 0" is just another declaration, which is the family this ledger has
# been recording for thirty rounds: a claim standing in for a check.
if [ "${1:-}" = "--selftest-guard" ]; then
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  ( cd "$tmp" && git init -q . && : > tracked && git add tracked \
      && git -c user.email=t@t -c user.name=t commit -qm init ) >/dev/null 2>&1
  ROOT="$tmp"
  before="$(dirty_count)"
  : > "$tmp/untracked-file"
  after="$(dirty_count)"
  if [ "$before" = "0" ] && [ "$after" != "0" ]; then
    echo "guard OK: clean=$before dirty=$after"
    exit 0
  fi
  echo "guard BROKEN: clean=$before dirty=$after — the validity guard cannot see a dirty tree"
  exit 1
fi

N="${1:-10}"
case "$N" in ''|*[!0-9]*) echo "usage: $0 [N|--selftest-guard]" >&2; exit 2;; esac

rev0="$(head_rev)"; dirty0="$(dirty_count)"
[ "$dirty0" = "0" ] || { echo "REFUSING: working tree has $dirty0 change(s) before the run; a moving tree invalidates the experiment" >&2; exit 1; }
echo "frozen at $rev0, dirty=$dirty0, N=$N"

green=0; red=0; reds=""
for i in $(seq 1 "$N"); do
  out="$(cd "$ROOT" && cargo test 2>&1)"
  n="$(printf '%s\n' "$out" | grep -cE '^test .* FAILED')"
  p="$(printf '%s\n' "$out" | grep -E '^test result' | awk '{s+=$4} END{print s}')"
  if [ "$n" -eq 0 ]; then green=$((green+1)); else
    red=$((red+1))
    reds="${reds}
  run $i: $n failed -> $(printf '%s\n' "$out" | grep -E '^test .* FAILED' | awk '{print $2}' | tr '\n' ' ')"
  fi
  echo "run $i: passed=$p failed=$n"
done

rev1="$(head_rev)"; dirty1="$(dirty_count)"
valid="yes"
[ "$rev0" = "$rev1" ] || valid="NO (rev moved $rev0 -> $rev1)"
[ "$dirty0" = "$dirty1" ] || valid="NO (dirty $dirty0 -> $dirty1)"

echo "=== validity: rev $rev0 -> $rev1, dirty $dirty0 -> $dirty1 : $valid ==="
echo "=== result: green=$green red=$red of $N ==="
if [ "$valid" != "yes" ]; then
  echo "EXPERIMENT INVALID — the tree changed during the runs, so the counts measure a moving target."
  exit 1
fi
printf '%s\n' "$reds"
[ "$red" -eq 0 ]
