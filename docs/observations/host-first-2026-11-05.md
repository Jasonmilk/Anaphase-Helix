# 首件真活 · 宿主 anaphase 让 phyt-DNA 引擎做一次外部判断

> 形态：**引擎留母仓，工件在宿主，账本经 `PHYT_LEDGER` 指宿主**（`ci-local.sh:20` 的 env 正门）。
> 不装母子套件副本（复制整套方法论 = 12 份真源，已撤回）。

## 调用了什么（2 次，都是母仓引擎）

| # | 命令 | rc | 结果 |
|---|---|---|---|
| 1 | `PHYT_DECISIONS=<宿主>/docs/decisions bash phyt-DNA/tools/spec-lint.sh` | 0 | 通过（168 个受管文件 · 128 行注释含反引号，具名局限） |
| 2 | `bash phyt-DNA/tools/check-pool.sh <宿主>/docs/decisions` | **2** | **★ BLOCK：零闸门** |

## 五数（按 PLAN ④ 的机检口径；未测的具名，不填 0）

| 数 | 值 | 说明 |
|---|---|---|
| 被拦 | **1** | `check-pool` BLOCK：`decisions` 里**没有一份带 `hard: true` 的 ADR** ⇒ 空集合上的"全部通过"是伪证 |
| 返工 | 0 | 本轮没有返工（这是第一次真跑） |
| 假绿 | 0 | 未观察到（本轮的拦是真拦，不是伪装成绿的） |
| `elapsed_ms` | **UNMEASURABLE** | 本轮没埋计时（不编造读数） |
| 调用数 | 2 | spec-lint + check-pool，均为母仓引擎 |

## 真事撞出来的差分（这是本轮唯一交付物）

> **宿主 52 份 ADR，0 份 `hard: true`。** ⇒ phyt 的"闸门"模型在 anaphase 这边**对象不存在**。
> 不是"我们判据少"，而是**我们的判据没有以 `hard:` 可执行形状声明**（我们的 CI 门在 `.github/workflows` + `ci/*.py`，不在 ADR 头部）。

⇒ 由此产生的口径反哺（一行，只写在撞到的地方，不预写）：

```
adopt ≠ copy：引擎留母仓，工件/账本在宿主（PHYT_LEDGER / PHYT_DECISIONS 两个 env）
anaphase 的闸门住在 CI + ci/*.py，不在 ADR 头部 ⇒ 与 phyt 的 hard:true 模型不是同一层，不能互相当证据
```

## 宿主账本

`"source": "ci-local"` 行 = **0**（本轮走的是 spec-lint / check-pool，不是 ci-local）⇒ PLAN 判据①仍未满足。
这是**副产品缺失**，不是目标；不为此再跑一次 ci-local。
