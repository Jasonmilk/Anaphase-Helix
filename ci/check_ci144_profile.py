#!/usr/bin/env python3
"""CI-144 profile 的"声明-证据"判据。

为什么需要它：profile 里每一句「已采纳」「不适用」如果没有可解析的落点，就与
「在一个我们不使用的帧格式里 grep 一个字段」同族 —— 恒绿，且看起来在严密防守。
本判据只做一件事：把 profile 的每条声明换成一个可失败的检查。

调用者（§3.10）：`anaphase-helix/.github/workflows/ci.yml` step（每次 push/PR）
+ 本地 `python3 ci/check_ci144_profile.py`；`--selftest` 自带正反样本。

规则：
  R1 status ∈ {adopted, absent, deviated}，否则红。
  R2 status=adopted ⇒ evidence 必须解析：`path` 存在；`path::symbol` 还能 grep 到。
  R3 status=absent  ⇒ evidence 必须含 `revisit:`（缺席不许静默；触发条件必写）。
  R4 status=deviated ⇒ evidence 必须含 `why:`（偏离必须具名）。
  空表 ⇒ exit 3（读到空 ≠ 干净）。

退出码：0 全绿 / 1 有红 / 2 表格格式坏 / 3 一条声明都没读到
"""
import io
import os
import re
import sys

STATUSES = {"adopted", "absent", "deviated"}
START = "<!-- profile-claims:start -->"
END = "<!-- profile-claims:end -->"


def parse(text):
    if START not in text or END not in text:
        raise SystemExit("CHECKER ERROR: profile 缺少声明表标记 %s … %s" % (START, END))
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
            bad.append("%s: status=%r 不在 %s" % (cid, status, sorted(STATUSES)))
            continue
        if status == "adopted":
            path, _, sym = evidence.partition("::")
            path = path.strip("` ")
            full = os.path.join(root, path)
            if not os.path.isfile(full):
                bad.append("%s: adopted 但 evidence 路径不存在: %s" % (cid, path))
                continue
            if sym:
                sym = sym.strip("` ").strip()
                body = io.open(full, encoding="utf-8", errors="replace").read()
                if sym not in body:
                    bad.append(
                        "%s: adopted 但 evidence 符号解析不到: %s::%s" % (cid, path, sym)
                    )
        elif status == "absent":
            if "revisit:" not in evidence:
                bad.append("%s: absent 但没写 revisit: 触发条件" % cid)
        elif status == "deviated":
            if "why:" not in evidence:
                bad.append("%s: deviated 但没写 why:" % cid)
    return bad


def run(root, quiet=False):
    text = io.open(os.path.join(root, "docs/ci144-profile.md"), encoding="utf-8").read()
    rows = parse(text)
    if not rows:
        print("CHECKER DEAD: 读到 0 条声明 ⇒ 不是干净，是没读到")
        return 3
    bad = check(root, rows)
    if not quiet:
        print("声明 %d 条：%s" % (
            len(rows),
            ", ".join("%s=%d" % (s, sum(1 for r in rows if r[2] == s)) for s in sorted(STATUSES)),
        ))
    if bad:
        for b in bad:
            print("RED: " + b)
        return 1
    if not quiet:
        print("OK: 每条声明都有可解析落点或具名的缺席/偏离理由")
    return 0


def selftest():
    import tempfile

    ok = [
        "| c1 | 帧 | adopted | `src/main.rs::fn main` |",
        "| c2 | 注册表 | absent | revisit: 有物理执行面时 |",
        "| c3 | 硬覆盖 | deviated | why: break-glass 已三次被拒 |",
    ]
    cases = [
        ("valid", ok, 0),
        ("bad-status", ["| c1 | 帧 | yes | `src/main.rs` |"], 1),
        ("missing-path", ["| c1 | 帧 | adopted | `no/such.rs` |"], 1),
        ("missing-symbol", ["| c1 | 帧 | adopted | `src/main.rs::no_such_symbol_xyz` |"], 1),
        ("absent-no-revisit", ["| c1 | 帧 | absent | 我们没接 |"], 1),
        ("deviated-no-why", ["| c1 | 帧 | deviated | 不用 |"], 1),
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
    if "--selftest" in sys.argv:
        return selftest()
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    return run(root)


if __name__ == "__main__":
    sys.exit(main())
