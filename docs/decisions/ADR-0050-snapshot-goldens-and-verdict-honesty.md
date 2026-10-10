# ADR-0050：快照 golden 必须与载荷同步；且「没跑」不得报成 pass

- **状态**：**Accepted（2026-10-09）**
- **日期**：2026-10-09
- **决策范围**：anaphase-helix（测试判据 + 报告纪律）
- **关联**：`KNOWN_ISSUES` K25 · K20（`bypass` 具名）· `helix-mind:tools/adr_head.py`（本文头的解析器）

## 背景（2026-10-09 实测，全部工具产出）

给事件载荷新增一个字段（`bypass`，rail 旁路具名，K20）之后，
`tests/golden/run_cycle_summary.txt` **没有**同步重生成 ⇒ 该文件里的
`a_whole_cycle_matches_the_snapshot` **每次都红**（实测 `R R R R R R`）。

**而我把这个稳定红当成了「既有 flaky」，放过了一整个会话。**
放过它的那条推理是：同一套判据连跑两次失败集合不同 ⇒ 判为 flaky。
**那条推理有一个漏洞**：我的探测器用 `grep -c '^test result: ok'` 判绿，
而 **`test result: ok. 0 passed; N filtered out`** —— **测试根本没跑** —— 也被数成了绿。
⇒ 于是「8/8 绿」是假的，稳定红被误读成随机。**这正是「声明代替检查」。**

## 决定（两条判据，都可执行）

1. **快照类 golden 是载荷的函数。** 任何改变事件载荷 / 可观察输出的改动，
   **必须**在同一次提交里用 `REGEN_SNAPSHOT=1` 重生成 golden，**并在提交信息里写明理由**
   （测试自己就要求这一点：`REGEN_SNAPSHOT=1 and say why in the commit message`）。
   ⇒ 否则 golden 会**把真回归伪装成 flaky** —— 比直接红更坏，因为它教人忽略。
2. **判「绿」时，必须先问「它跑了吗」。** 只看 `FAILED` 或**具体测试名**的结果；
   **不得**用「存在 `test result: ok` 行」判定通过 —— `0 passed; N filtered out` 会污染它。

## 红测（能红的判据）

- 给任一事件载荷加字段而**不**重生成 golden ⇒ 对应快照测试**必红**（实测可行）。
- 用一个把测试过滤掉的命令去「验证」⇒ **不得**被读成 pass（本条正是为此写的）。

## 边界（说清做不到的）

本 ADR **不**声称解释了 `tests/run_cycle_pipeline.rs` 里剩下的那一类：
`run_config_soft_reflex_threshold_blocks` / `run_cycle_deterministic_replay` 等
**隔离通过、一起跑必红**（且同文件内失败数量随运次变化：实测 3 / 5 / 2 failed）。
那是一个**独立的、仍在查的**干扰类，见 `KNOWN_ISSUES` K25（已按本轮证据改写）。
