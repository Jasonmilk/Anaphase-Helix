# ADR-0037: 术语更名——印痕 → 证轨 / Engram → ProveTrack

- **状态**: Accepted
- **日期**: 2026-09-14
- **决策范围**: 生态级术语（anaphase-helix / Cellrix / helix-mind 的代码与文档）
- **关联**: ADR-0029（印痕链条完整性）、ADR-0026 D6（模型标签与 finalize）、ADR-0015（Cellrix 水之波光 WebUI）
- **取代**: 无（旧名 `Engram` 与「印痕」**一并退役，不保留别名**）

## 1. 背景与问题

「Engram」/「印痕」指同一事物：Helix 全链路审计印记（每轮认知的输入、注入、
判据、行动、交付物，连同防篡改哈希链）。命名须"简短准确一眼懂"（DNA 命名原则）。

两个旧名各有问题：

- `Engram` 是神经科学借词，中文语境既不直观也无法自解释；对外沟通每次都要额外解释。
- 「印痕」偏被动——它描述"留下的痕迹"，**没有表达"可验证的证明轨道"**这一核心语义：
  该对象的本质不是"痕迹"，而是"每条判据都有证据可复核的轨道"。

用户 2026-09-14 拍板更名。

## 2. 决策

### D1: 新名

中文「**证轨**」／英文「**ProveTrack**」。二者并列使用（`ProveTrack（证轨）`）。

选名理由：「证」= 判据/证据/可复核，「轨」= 轨道/时间线/可回放；合起来正是
该对象的语义——一条每条判据都可被证据复核的轨道。英文 ProveTrack 同构。

### D2: 映射（代码按语言惯例，散文按品牌名）

| 层 | 旧 | 新 |
|---|---|---|
| 类型 | `EngramQuery` / `EngramEntry` / `EngramViewState` / `EngramPayload` | `ProveTrackQuery` / `ProveTrackEntry` / `ProveTrackViewState` / `ProveTrackPayload` |
| 函数 | `render_engram` / `set_engram` / `attach_engram` | `render_prove_track` / `set_prove_track` / `attach_prove_track` |
| 变量 | `engram_rx` / `engram_toggle` | `prove_track_rx` / `prove_track_toggle` |
| 模块/文件 | `mod engram` / `engram.rs` | `mod prove_track` / `prove_track.rs` |
| 前端资产 | `engram.html` | `prove_track.html` |
| JS 全局 | `__ENGRAM__` / `__engramLoad` / `__engramClear` / `__engramMeta` | `__PROVE_TRACK__` / `__proveTrackLoad` / `__proveTrackClear` / `__proveTrackMeta` |
| CSS/路由 | `view-engram` / `v-engram` | `view-prove-track` / `v-prove-track` |
| JS 状态键 | `state.engram` | `state.prove_track` |
| 枚举 | `ActiveView::Engram` | `ActiveView::ProveTrack` |
| 散文 | `Engram` / `印痕` | `ProveTrack` / `证轨` |

### D3: 范围（实测 44 文件）

anaphase-helix **19** ／ Cellrix **23** ／ helix-mind **2**；其余 6 仓 0。

### D4: 无兼容风险（逐项物理核对，非推断）

1. **事件流线协议词表不含 `engram`**——实测词表为 `user/message`、`assistant/attempt`、
   `assistant/think`、`assistant/reply`、`context/inject`、`tool/call`、`tool/result`、
   `check/status`、`verdict/status`、`turn/start`、`turn/end`。**更名不动线协议。**
2. **序列化字段名不含 `engram`**——`EngramQuery`/`EngramEntry` 的字段是
   `entries`/`count`/`queried_by`（Tuck `/v1/audit` 的真实响应形状）；结构体名不参与序列化。
3. **无配置键**——本地 `config.toml` 无 `engram` 键；`config.rs` 中 `engram` 仅出现在文档注释。
4. 无外部包依赖该模块名。

因此本次为**纯命名变更**：不动线协议、不动序列化字段、不动配置键、不动判据语义。

### D5: 本次不做（用户裁决）

`Cellrix/web/assets/engram.html` 更名后仍为 **955 行**，是唯一违反"单文件 ≤400 行"
红线的资产（其余资产 351/323/290/212/207/132/69/21/18 全部达标）。
**本轮只更名，解耦另开一轮**——控制改动面、便于回滚。

## 3. 测试

- anaphase-helix / Cellrix 全量 `cargo test` 复验：编译通过，用例数与更名前一致。
- Cellrix UI 测试 `index_html_contains_both_views_and_prove_track_fields`：
  断言 HTML 同时含两个视图与证轨字段（测试名随术语同步）。

## 4. 验证

- 全生态 `grep -rI -e engram -e Engram -e 印痕` 在**代码与前端资产中归零**
  （仅历史 ADR 正文与本 ADR 保留旧名以记录更名事实）。
- WebUI 真实渲染：证轨视图显示 `/api/audit` 链路 + REPLY 交付物 + 模型标签。

## 5. 后果

- 对外沟通不再需要解释借词；中文名自解释。
- **历史 ADR 的正文与文件名一并回改**（用户 2026-09-14 明确裁决：「连文件名一起改」）：
  `ADR-0029-engram-chain-integrity.md` → `ADR-0029-provetrack-chain-integrity.md`；
  其余 7 份 anaphase ADR（0026/0027/0028/0030/0034/0035）+ Cellrix ADR-0015 的正文术语同步替换。
- **这是对 `docs/decisions/README.md`「Active 不可覆写、保留原文永不删除」规范的一次
  有意识例外，理由如下**：更名不是"发现错误"（该规范针对的是决策内容出错），而是
  同一对象的**名称演进**；留旧名会让读者在术语断层处把同一事物误读成两个不同对象，
  与 DNA「命名准确简短一眼懂」冲突。裁决与例外由本 ADR 承载，全程可追溯。
- 若未来需要旧名检索，以本 ADR 的 D2 映射表为唯一换算依据。
