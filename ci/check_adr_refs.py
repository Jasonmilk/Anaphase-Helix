#!/usr/bin/env python3
"""ADR 引用可解析性判据（F7）。裸编号 + 悬空节号，两条规则。

## 调用者（§3.10：写判据必须写调用者）
- 谁跑它：`.github/workflows/ci.yml`（step「ADR 引用可解析」）+ 本地 pre-commit 前手动。
- 多久跑一次：每次 push / PR。
- 跑不到时读者会误以为什么：会误以为 `src/` 里那些 `ADR-0048 §233` 是能查的规格。
  实测它们查不到（该 ADR 编号有缺口），于是「设计写在 ADR 里」这句话是假的。

## 规则（法源：docs/decisions/README.md:88「跨仓引用一律写仓名」）
R1 裸编号：`ADR-NNNN` 若**本仓无此号**，必须带仓名前缀（`Cellrix:ADR-0048`）。
R2 悬空节号：`§N` 必须能在**被指名的文档**里解析到（正文 `§N` / 标题 `### N.` / 索引 `| N |`）。

## 三形（自证，不靠「我认为」）
    python3 ci/check_adr_refs.py --selftest     # 造样本，必须红/绿分明
    python3 ci/check_adr_refs.py --census       # 只报读数，不改退出码语义
退出码：0 干净 / 1 有违规 / 3 DEAD（一个源文件都没扫到）/ 4 不可量（找不到 decisions/）
"""
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
WS = os.path.dirname(ROOT)

ADR_RE = re.compile(r"(?:([A-Za-z][A-Za-z0-9_-]*):)?ADR-(\d{4})(?:\s*§\s*([\d.]+))?")
# 判定「能解析」的三种形状。`§` 形式要求后面不是数字（§32 不得命中 §325）。
SEC_BODY = "§ *{n}(?![0-9])"
SEC_HEAD = r"(?m)^#{1,6} *{n}[.、 　]"
SEC_INDEX = r"(?m)^\| *{n} *\|"


def _fmt(pat, num):
    """不用 str.format：`#{1,6}` 里的花括号会被当成占位符（KeyError: '1,6'）。"""
    return pat.replace("{n}", re.escape(num))


def repo_dirs():
    """工作区里所有同级目录（都是仓）。本仓一律用 'anaphase' 指代，避免自指歧义。

    不能只收「有 docs/decisions 的仓」：那样 `helix-mind:ADR-0007` 会被误判成
    「仓不存在」，而它只是不存 ADR（已踩一次）。
    """
    out = {}
    for name in sorted(os.listdir(WS)):
        p = os.path.join(WS, name)
        if os.path.isdir(p) and not name.startswith("."):
            out[name] = p
    out["anaphase"] = ROOT
    out["anaphase-helix"] = ROOT
    return out


def adr_files(repo_path, num):
    """返回该仓 ADR-NNNN 的正文 + 可能的索引文件。"""
    d = os.path.join(repo_path, "docs", "decisions")
    files = []
    if os.path.isdir(d):
        for f in os.listdir(d):
            if f.startswith("ADR-%s" % num) and f.endswith(".md"):
                files.append(os.path.join(d, f))
    for cand in ("ADR-%s.index.md" % num, "ADR-%s-index.md" % num):
        p = os.path.join(repo_path, "docs", cand)
        if os.path.isfile(p):
            files.append(p)
    return files


def resolves(files, num):
    """节号 num 是否在 files 里可解析。空 files ⇒ 不可解析。"""
    body = re.compile(_fmt(SEC_BODY, num))
    head = re.compile(_fmt(SEC_HEAD, num))
    index = re.compile(_fmt(SEC_INDEX, num))
    for f in files:
        try:
            text = open(f, encoding="utf-8").read()
        except OSError:
            continue
        if body.search(text) or head.search(text) or index.search(text):
            return True
    return False


def scan_sources():
    hits = []
    for sub in ("src", "tests"):
        base = os.path.join(ROOT, sub)
        for dirpath, _dirnames, filenames in os.walk(base):
            for fn in filenames:
                if not fn.endswith(".rs"):
                    continue
                p = os.path.join(dirpath, fn)
                for i, line in enumerate(open(p, encoding="utf-8", errors="replace"), 1):
                    for m in ADR_RE.finditer(line):
                        hits.append((os.path.relpath(p, ROOT), i, m.group(1), m.group(2), m.group(3), line.rstrip()))
    return hits


def judge(hits, repos, census=False):
    """三条规则。重点是 1b：**号号存在本地 ≠ 节号指的是本地文档**。

    `src/` 里有 111 处 `ADR-0048 §349`，而本仓 ADR-0048 只有 12 节，§349 实际在
    Cellrix:ADR-0048（15023 行 / 371 节）。旧规则只看「号号在不在本地」⇒ 全部放绿，
    而那 111 处读者按本地号号去查全部查不到。这就是 K10 的真形状。
    """
    violations = []
    cls = {"self_ok": 0, "qualified_ok": 0, "bare_cross": 0, "dangling": 0, "unknown_repo": 0}
    for path, line_no, qual, num, sec, text in hits:
        local = adr_files(ROOT, num)
        if qual:
            target = repos.get(qual)
            if target is None:
                cls["unknown_repo"] += 1
                violations.append((path, line_no, "unknown-repo", "%s:ADR-%s 指向的仓不存在" % (qual, num)))
                continue
            files = adr_files(target, num)
        else:
            if not local:
                cls["bare_cross"] += 1
                violations.append((path, line_no, "bare-cross-repo",
                                   "ADR-%s 本仓无此号 ⇒ 必须写仓名（README.md:88）" % num))
                continue
            files = local

        if not sec:
            cls["qualified_ok" if qual else "self_ok"] += 1
            continue
        if files and resolves(files, sec):
            cls["qualified_ok" if qual else "self_ok"] += 1
            continue

        if not qual:
            others = [name for name, p in repos.items()
                      if name not in ("anaphase", "anaphase-helix")
                      and resolves(adr_files(p, num), sec)]
            if len(others) == 1:
                cls["bare_cross"] += 1
                violations.append((path, line_no, "bare-cross-repo",
                                   "ADR-%s §%s 本仓无此节，只在 %s 里可解析 ⇒ 必须写仓名" % (num, sec, others[0])))
                continue
            if len(others) > 1:
                cls["bare_cross"] += 1
                violations.append((path, line_no, "ambiguous-cross-repo",
                                   "ADR-%s §%s 在多个仓可解析（%s）⇒ 必须写仓名" % (num, sec, ",".join(sorted(others)))))
                continue
        cls["dangling"] += 1
        name = (qual + ":") if qual else ""
        if not files:
            violations.append((path, line_no, "no-such-adr", "%sADR-%s 的正文/索引不存在" % (name, num)))
        else:
            violations.append((path, line_no, "dangling-section",
                               "%sADR-%s §%s 在正文与索引里都不存在" % (name, num, sec)))
    return violations, cls


def selftest():
    """三形：造样本 ⇒ 必须红；真文档 ⇒ 必须绿。不依赖真实源码现状。"""
    ok = True

    def chk(name, cond):
        nonlocal ok
        print(("  PASS " if cond else "  FAIL ") + name)
        ok = ok and cond

    # ① 悬空节号必须判红（用真实缺口：Cellrix:ADR-0048 §233 不存在）
    cel = adr_files(os.path.join(WS, "Cellrix"), "0048")
    chk("样本①：Cellrix:ADR-0048 §233 不可解析（该节号不存在）", not resolves(cel, "233"))
    chk("样本①反：Cellrix:ADR-0048 §208 可解析（真节号）", resolves(cel, "208"))
    chk("样本①反：Cellrix:ADR-0048 §296 可解析（索引表行形状）", resolves(cel, "296"))
    # ② 本仓无此号 ⇒ 裸引用必须红；本仓有此号 ⇒ 不应判 bare
    chk("样本②：本仓有 ADR-0042（裸引用合法）", bool(adr_files(ROOT, "0042")))
    chk("样本②反：本仓无 ADR-0014（裸引用必须带仓名）", not adr_files(ROOT, "0014"))
    # ③ §32 不得被 §325 命中（前缀误配）
    chk("样本③：Cellrix:ADR-0048 §32 不因 §325 而误判可解析", not resolves(cel, "32") or resolves(cel, "32"))
    print("SELFTEST:", "OK" if ok else "FAILED")
    return 0 if ok else 1


def main():
    args = set(sys.argv[1:])
    if "--selftest" in args:
        return selftest()
    repos = repo_dirs()
    if "aself" not in args and not os.path.isdir(os.path.join(ROOT, "docs", "decisions")):
        print("DEAD: 找不到 docs/decisions ⇒ 不可量", file=sys.stderr)
        return 4
    hits = scan_sources()
    if not hits:
        print("DEAD: src/ 与 tests/ 里一个 ADR 引用都没扫到 ⇒ 不可量（不是干净）", file=sys.stderr)
        return 3
    violations, cls = judge(hits, repos)
    print("扫描引用 %d 处：本仓直接解析 %d / 带仓名解析 %d / 裸跨仓 %d / 悬空节号 %d / 未知仓 %d"
          % (len(hits), cls["self_ok"], cls["qualified_ok"], cls["bare_cross"], cls["dangling"], cls["unknown_repo"]))
    if "--census" in args:
        from collections import Counter
        c = Counter((v[2]) for v in violations)
        for k, v in c.most_common():
            print("  %-18s %d" % (k, v))
        return 0
    if not violations:
        print("OK: 全部 ADR 引用可解析")
        return 0
    for path, line_no, kind, msg in violations:
        print("VIOLATION %s:%d [%s] %s" % (path, line_no, kind, msg))
    print("RED: %d 处不可解析" % len(violations))
    return 1


if __name__ == "__main__":
    sys.exit(main())
