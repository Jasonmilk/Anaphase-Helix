//! ADR-0049 — 会话收敛：状态 + 骨架 + 按需下钻。
//!
//! 协议本体是 `lodestone-spec:ADR-0002` D5（收敛即压缩 / 活动区+沉淀区）与 D6（三级下钻），
//! **本模块不重新发明任何一条**，只做映射：文档 → 会话链，磁石（lode）→ period。
//!
//! 三条不变量（ADR-0049 §二 D5）：
//! - **隐性化 ≠ 删除**：本模块**没有任何删除路径**。一轮的全文永远在它的 `.events.jsonl`
//!   里（append-only）；"沉淀"= 从默认视图退出，不是搬文件。
//! - **默认永不丢失**：状态缺省**由龄期派生**（配置的 7 天），不是存下来的事实；
//!   显式覆盖才写侧车 ⇒ 历史一行不回写。
//! - **可审查**：`describe` 回答"第几轮 / 因为什么被隐性 / 现在在哪 / 怎么取回"。

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use super::query::PeriodSummary;

/// `lodestone-spec:ADR-0002` D4 的状态机在本仓只实现前两态（协议第三态 `aligned`
/// 留给 v2.x —— **不预造**，ADR-0049 D2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeriodStatus {
    Draft,
    Converged,
}

impl PeriodStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Converged => "converged",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "draft" => Some(Self::Draft),
            "converged" => Some(Self::Converged),
            _ => None,
        }
    }
}

/// WHERE the status came from — this is the audit answer to "why is it hidden".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusSource {
    /// An explicit `.state` sidecar (a human or an explicit action wrote it).
    Stored,
    /// Not stored: the period is older than `hide_after_days`.
    DerivedFromAge,
    /// Not stored and not old: the default view.
    Default,
}

impl StatusSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stored => "stored",
            Self::DerivedFromAge => "derived: age >= converge_hide_after_days",
            Self::Default => "default: fresh",
        }
    }
}

/// The knobs (ADR-0049 D6). Defaults live in `config.rs` — **no literal here**
/// (DNA 铁律 11).
#[derive(Debug, Clone, Copy)]
pub struct ConvergenceConfig {
    pub enabled: bool,
    pub hide_after_days: u64,
    pub compress_after_days: u64,
    pub skeleton_max_lines: usize,
    pub skeleton_line_chars: usize,
    /// D5: the SUNK region's in-place gist is ONE bounded blob, not one line per round.
    pub gist_max_chars: usize,
}

impl Default for ConvergenceConfig {
    /// Protocol defaults from `config.rs` — **no literal in this module**.
    fn default() -> Self {
        Self {
            enabled: crate::config::DEFAULT_CONVERGE_ENABLED,
            hide_after_days: crate::config::DEFAULT_CONVERGE_HIDE_AFTER_DAYS,
            compress_after_days: crate::config::DEFAULT_CONVERGE_COMPRESS_AFTER_DAYS,
            skeleton_max_lines: crate::config::DEFAULT_CONVERGE_SKELETON_MAX_LINES,
            skeleton_line_chars: crate::config::DEFAULT_CONVERGE_SKELETON_LINE_CHARS,
            gist_max_chars: crate::config::DEFAULT_CONVERGE_GIST_MAX_CHARS,
        }
    }
}

impl ConvergenceConfig {
    /// Read the knobs off the loaded config (ADR-0049 D6 — every one of them is a
    /// config field, none is a literal at a call site).
    pub fn from_config(cfg: &crate::config::AnaphaseConfig) -> Self {
        Self {
            enabled: cfg.converge_enabled,
            hide_after_days: cfg.converge_hide_after_days,
            compress_after_days: cfg.converge_compress_after_days,
            skeleton_max_lines: cfg.converge_skeleton_max_lines,
            skeleton_line_chars: cfg.converge_skeleton_line_chars,
            gist_max_chars: cfg.converge_gist_max_chars,
        }
    }
}

fn state_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.state"))
}

/// The stored override, if any. `None` means "derive it" — never "draft".
pub fn read_state(dir: &Path, id: &str) -> Option<PeriodStatus> {
    std::fs::read_to_string(state_path(dir, id))
        .ok()
        .and_then(|s| PeriodStatus::parse(&s))
}

/// Write the explicit override. **The only writer**; nothing derives-and-stores,
/// so a period's status is either what someone decided or what age says.
pub fn write_state(dir: &Path, id: &str, st: PeriodStatus) -> std::io::Result<()> {
    std::fs::write(state_path(dir, id), st.as_str())
}

/// The RFC3339 instant before which a period counts as converged. Comparison is
/// LEXICOGRAPHIC because the store's timestamps are fixed-width RFC3339 — no date
/// parsing, no timezone, no clock arithmetic beyond one subtraction of seconds.
pub fn converge_cutoff(now_secs: u64, hide_after_days: u64) -> String {
    crate::ledger::unix_secs_to_rfc3339(
        now_secs.saturating_sub(hide_after_days.saturating_mul(86_400)),
    )
}

/// D2: stored override wins; otherwise age decides; `enabled = false` disables the
/// whole mechanism (the user's off switch, ADR-0049 D6).
pub fn resolve_status(
    stored: Option<PeriodStatus>,
    first_ts: &str,
    cutoff: &str,
    enabled: bool,
) -> (PeriodStatus, StatusSource) {
    if let Some(st) = stored {
        return (st, StatusSource::Stored);
    }
    if enabled && !first_ts.is_empty() && first_ts < cutoff {
        (PeriodStatus::Converged, StatusSource::DerivedFromAge)
    } else {
        (PeriodStatus::Draft, StatusSource::Default)
    }
}

/// One round as it appears in the L0 skeleton. Plain data: the skeleton builder
/// below is a **pure function** of these (ADR-0049 D3) — no IO, no clock, so it
/// can be unit-tested and mutated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkeletonRow {
    pub id: String,
    pub status: PeriodStatus,
    pub label: String,
    /// `lodestone-spec:ADR-0002` D5 的另一半：**原位摘要**。原轮沉底后，这一行的原文
    /// 不再出现在骨架里，但这段 gist 仍在 —— 它取自该轮自己的字节，**永不替代原文**
    /// （事件流仍是原件的所在地），并带来源 id 与 `!body <id>` 取回路径。
    pub gist: Option<String>,
    /// 一次**具名的用户动作**的落点：`Some(reason)` = 这段被驳回（另开分支）。
    /// **不是模型判断** —— 前沿（AGM 1985）说得明白：冲突时该放弃哪一条，逻辑本身
    /// 决定不了，需要**外部标准**；而让模型自己当裁判的方法都假定强模型，我们是 3B。
    /// 驳回**不删除**任何字节：事件流仍在，`!body` 仍可取回，`!lodes` 仍会列出它。
    pub rejected: Option<String>,
}

/// 驳回的**唯一**落点：`<id>.rejection.log`，**append-only**。
///
/// 每行一次判断：`<when> | <who> | reject|revoke | <why>`。**最后一行决定当前状态**。
/// ⇒ **撤销是叠加一条新事实，不是抹掉旧的**（与 append-only 同向）：反悔本身也是一次判断，
/// 带着自己的 when / who / why，可被后人读到。
pub fn reject_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.rejection.log"))
}

/// Every judgement made about this round, oldest first — the full history, including reversals.
pub fn read_rejection_log(dir: &Path, id: &str) -> Vec<String> {
    std::fs::read_to_string(reject_path(dir, id))
        .map(|t| t.lines().filter(|l| !l.trim().is_empty()).map(|l| l.to_string()).collect())
        .unwrap_or_default()
}

/// The store's own bytes — the only thing a理由 is allowed to point at.
fn store_text(dir: &Path) -> String {
    let mut out = String::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            let is_events = p.extension().and_then(|x| x.to_str()) == Some("jsonl");
            if !is_events {
                continue;
            }
            if let Ok(t) = std::fs::read_to_string(&p) {
                out.push_str(&t);
                out.push('\n');
            }
        }
    }
    out
}

/// `why` 的**形态约束**（人类 2026-10-09）：必须写成「**因为〈可观测条件〉，所以不吸收**」。
///
/// 理由不是洁癖：用户现在替 Helix 摸索，**日后要把学到的交给 ta** ⇒ 每条判断都要能被**学会**。
/// 「因为我觉得不对」只能被**记住**；指向可观测者的理由才能变成 Helix 自己的判据。
///
/// **「可观测」不是关键词表** —— 判据是：理由必须与**存储里真实存在的字节**有公共子串
/// （≥4 字符），或直接点名一个 store 里存在的 period id。
/// 连词齐备但**指向不了任何东西**的理由（"因为我觉得不对，所以不吸收"）⇒ **拒写**。
pub fn validate_reason(dir: &Path, reason: &str) -> Result<(), String> {
    let r = reason.trim();
    if r.is_empty() {
        return Err("理由为空：缺理由与没记必须是两种可分辨的读数".to_string());
    }
    if !(r.starts_with("因为") && r.contains("所以")) {
        return Err("理由必须写成「因为〈可观测条件〉，所以不吸收」".to_string());
    }
    let text = store_text(dir);
    let chars: Vec<char> = r.chars().collect();
    /* WITNESS: a ≥4-char run shared with the store's bytes. The STORE is the arbiter, so this is
     * not a keyword list — a reason that names something real passes, and one that names only the
     * speaker's own feeling fails. (Named residual: a 4-char run that coincidentally occurs in the
     * store also passes; the guard raises the floor, it does not read minds.) */
    for i in 0..chars.len() {
        if i + 4 > chars.len() {
            break;
        }
        let run: String = chars[i..i + 4].iter().collect();
        if text.contains(&run) {
            return Ok(());
        }
    }
    // A period id named in the reason is a witness too.
    for tok in r.split(|c: char| c.is_whitespace() || c == '，' || c == '。' || c == '「' || c == '」') {
        if super::identity::is_period_id(tok) {
            return Ok(());
        }
    }
    Err(format!(
        "理由指向不了任何可观测的东西 —— 它与存储里真实存在的字节没有 4 字以上的公共串。\n  请写成「因为〈可观测条件：引用轮次/事件/工具/事实串〉，所以不吸收」"
    ))
}

/// Append one judgement. **Never rewrites**: a revoke is a NEW line, not a deletion.
pub fn append_rejection(
    dir: &Path,
    id: &str,
    action: &str,
    why: &str,
    who: &str,
    when: &str,
) -> std::io::Result<()> {
    let line = format!("{} | {} | {} | {}\n", when.trim(), who.trim(), action, why.trim());
    use std::io::Write as _;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(reject_path(dir, id))?;
    f.write_all(line.as_bytes())
}

/// 记下驳回（append 一条 `reject`）。**形态不符即拒写**。
pub fn write_rejection(
    dir: &Path,
    id: &str,
    why: &str,
    who: &str,
    when: &str,
) -> std::io::Result<()> {
    validate_reason(dir, why).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    append_rejection(dir, id, "reject", why, who, when)
}

/// 撤销：同样受**形态约束**（反悔也是一次判断），但**不删任何历史行**。
pub fn revoke_rejection(
    dir: &Path,
    id: &str,
    why: &str,
    who: &str,
    when: &str,
) -> std::io::Result<()> {
    validate_reason(dir, why).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    append_rejection(dir, id, "revoke", why, who, when)
}

/// 当前状态：**最后一行决定**。`Some(why)` = 现在是驳回态。
pub fn read_rejection(dir: &Path, id: &str) -> Option<String> {
    let last = read_rejection_log(dir, id).pop()?;
    let mut parts = last.splitn(4, " | ");
    let _when = parts.next()?;
    let _who = parts.next()?;
    let action = parts.next()?;
    let why = parts.next().unwrap_or("");
    if action.trim() == "reject" {
        Some(why.to_string())
    } else {
        None
    }
}

/// 最近一次判断的 `when | who`（含撤销）。
pub fn read_rejection_at(dir: &Path, id: &str) -> Option<String> {
    let last = read_rejection_log(dir, id).pop()?;
    let mut parts = last.splitn(3, " | ");
    let when = parts.next()?;
    let who = parts.next()?;
    Some(format!("{when} | {who}"))
}

/// `!reject <id> <因为〈可观测条件〉，所以不吸收>` —— 回答必须说清四件事。
pub fn answer_reject(dir: &Path, arg: &str, reason: &str, who: &str, when: &str) -> String {
    let id = arg.trim();
    if id.is_empty() {
        return "(用法：!reject <period_id> 因为〈可观测条件〉，所以不吸收)".to_string();
    }
    match write_rejection(dir, id, reason, who, when) {
        Ok(()) => format!(
            "已驳回 {id}（子树级）\n  · why ：{}\n  · when：{}\n  · who ：{who}\n  · 字节未动（仍在 {id}.events.jsonl）\n  · 它**及其子孙**不再进入注入\n  · 仍可取回：!body {id}\n  · 可撤销：!revoke {id} 因为〈可观测条件〉，所以撤销",
            read_rejection(dir, id).unwrap_or_default(),
            read_rejection_at(dir, id).unwrap_or_default(),
        ),
        Err(e) => format!(
            "(驳回未写入：{e})\n  正确形态：!reject {id} 因为〈可观测条件〉，所以不吸收"
        ),
    }
}

/// `!revoke <id> <因为〈可观测条件〉，所以撤销>` —— **反悔是一次判断，不是擦除**。
pub fn answer_revoke(dir: &Path, arg: &str, why: &str, who: &str, when: &str) -> String {
    let id = arg.trim();
    if id.is_empty() {
        return "(用法：!revoke <period_id> 因为〈可观测条件〉，所以撤销)".to_string();
    }
    match revoke_rejection(dir, id, why, who, when) {
        Ok(()) => {
            let n = read_rejection_log(dir, id).len();
            format!(
                "已撤销对 {id} 的驳回（子树级）—— 它**及其子孙**重回注入\n  · why：{}\n  · 历史未删：该轮共 {n} 条判断，仍在 {}.rejection.log\n  · 原驳回行仍可读（反悔不是擦除）",
                why.trim(),
                id
            )
        }
        Err(e) => format!("(撤销未写入：{e})\n  正确形态：!revoke {id} 因为〈可观测条件〉，所以撤销"),
    }
}

/// **只读**取 gist（列表投影用；不产生副作用 —— 生产仍在 `gist_of` 的按需写一次）。
pub fn read_gist(dir: &Path, id: &str) -> Option<String> {
    std::fs::read_to_string(dir.join(format!("{id}.gist")))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// D5: the sunk round's in-place gist. **Write-once** per period (`<id>.gist`), derived from that
/// round's own frozen name/preview — so it is versioned by content, traceable to its source id,
/// and it can never replace the original: the event file stays exactly as written. Produced
/// **on demand** (when the skeleton first needs it), which is the user's own "按需存储和按需激活".
pub fn gist_of(dir: &Path, p: &PeriodSummary, max_chars: usize) -> String {
    let path = dir.join(format!("{}.gist", p.period_id));
    if let Ok(existing) = std::fs::read_to_string(&path) {
        let t = existing.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    let src = p
        .name
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(p.preview.as_str());
    let collapsed = src.split_whitespace().collect::<Vec<_>>().join(" ");
    let total = collapsed.chars().count();
    let mut g: String = collapsed.chars().take(max_chars).collect();
    if total > max_chars {
        g.push('…');
    }
    let _ = std::fs::write(&path, &g);
    g
}

/// Build the rows for a chain (oldest first). The label is the period's frozen
/// name when it has one, else its first-message preview — both already bounded,
/// so a row cannot grow with the content.
pub fn skeleton_rows(
    dir: &Path,
    path_oldest_first: &[&PeriodSummary],
    cfg: &ConvergenceConfig,
    now_secs: u64,
) -> Vec<SkeletonRow> {
    let cutoff = converge_cutoff(now_secs, cfg.hide_after_days);
    let mut rows = path_oldest_first
        .iter()
        .map(|p| {
            let stored = read_state(dir, &p.period_id);
            let (status, _src) = resolve_status(stored, &p.first_ts, &cutoff, cfg.enabled);
            let label = p
                .name
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(p.preview.as_str());
            let label: String = label
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(cfg.skeleton_line_chars)
                .collect();
            SkeletonRow {
                id: p.period_id.clone(),
                status,
                label,
                gist: Some(gist_of(dir, p, cfg.gist_max_chars)),
                rejected: read_rejection(dir, &p.period_id),
            }
        })
        .collect::<Vec<_>>();

    /* **子树级**（人类 2026-10-09 裁决）：`reject` 标记的是**一段**（B 及其后续），不是单点。
     * 沿路径自最旧向最新折叠：一旦某轮被驳回，**其后所有轮**都继承该驳回状态。
     * ⇒ 「另开分支」的语义 = 这条支线整体退出主线注入，而不是只挖掉中间一个洞。 */
    let mut inherited = false;
    for r in rows.iter_mut() {
        if r.rejected.is_some() {
            inherited = true;
        } else if inherited {
            r.rejected = Some(format!("继承自其祖先的驳回（子树级）"));
        }
    }
    rows
}

/// List the store and build the L0 rows for the chain ending at `leaf`.
pub fn skeleton_rows_for(
    dir: &Path,
    leaf: &str,
    cfg: &ConvergenceConfig,
    now_secs: u64,
) -> Vec<SkeletonRow> {
    let Ok(list) = super::query::list_periods(dir, usize::MAX) else {
        return Vec::new();
    };
    let path = chain_of(&list, leaf);
    skeleton_rows(dir, &path, cfg, now_secs)
}

/// `lodestone-spec:ADR-0002` D6 **L0** — one line per round, and the line count is
/// the ONLY thing that bounds the injection. When the chain is longer than the
/// budget the **newest** rows survive (the active zone) and the elision is NAMED
/// with the command that gets the rest back — a silent cut would make "converged"
/// and "lost" the same reading.
pub fn skeleton(rows: &[SkeletonRow], max_lines: usize, gist_chars: usize) -> String {
    /* 一次具名驳回的**唯一效果**：这段内容不再进入**注入**。它不进 gist，也不占最近 N 行。
     * 字节不动、`!body` 可达、`!lodes` 仍列出 —— 驳回是"从默认视图退出"，不是删除
     * （隐性化 ≠ 删除）。 */
    let kept: Vec<&SkeletonRow> = rows.iter().filter(|r| r.rejected.is_none()).collect();
    if kept.is_empty() || max_lines == 0 {
        return String::new();
    }
    let mut out = String::new();
    let hidden = kept.len().saturating_sub(max_lines);
    if hidden > 0 {
        /* D5's other half: the sunk region keeps an IN-PLACE GIST. It is ONE bounded blob —
         * "one line per round" would just move the old linear growth into the gist — and it
         * covers the sunk rounds oldest-first, so the earliest content is the last to be cut.
         * When the character budget cannot hold them all, the REMAINDER IS NAMED: a silent cut
         * would make "converged" and "lost" the same reading. */
        let mut gist = String::new();
        let mut used = 0usize;
        let mut included = 0usize;
        for r in &kept[..hidden] {
            let piece = r.gist.as_deref().unwrap_or(r.label.as_str());
            let cost = piece.chars().count() + 3;
            if used + cost > gist_chars {
                break;
            }
            if !gist.is_empty() {
                gist.push_str(" / ");
            }
            gist.push_str(piece);
            used += cost;
            included += 1;
        }
        let _ = writeln!(
            out,
            "- [gist of {hidden} earlier round(s) — bounded; originals intact, `!body <id>` retrieves any]"
        );
        let _ = write!(out, "  {gist}");
        if included < hidden {
            let _ = write!(out, " … (+{} more, `!lodes`)", hidden - included);
        }
        out.push('\n');
    }
    for r in &kept[hidden..] {
        let _ = writeln!(out, "- {} [{}] {}", r.id, r.status.as_str(), r.label);
    }
    out
}

/// D7: the four questions, answerable from the store alone. No new state.
pub fn describe(dir: &Path, p: &PeriodSummary, cfg: &ConvergenceConfig, now_secs: u64) -> String {
    let cutoff = converge_cutoff(now_secs, cfg.hide_after_days);
    let stored = read_state(dir, &p.period_id);
    let (status, source) = resolve_status(stored, &p.first_ts, &cutoff, cfg.enabled);
    let mut out = String::new();
    let _ = writeln!(out, "round:     {}", p.period_id);
    let _ = writeln!(out, "status:    {}", status.as_str());
    let _ = writeln!(out, "why:       {}", source.as_str());
    let _ = writeln!(
        out,
        "where:     {}",
        dir.join(format!("{}.events.jsonl", p.period_id)).display()
    );
    let _ = writeln!(out, "retrieve:  !body {}", p.period_id);
    let _ = writeln!(
        out,
        "lineage:   parent={}",
        p.parent.as_deref().unwrap_or("(root)")
    );
    out
}

/// `lodestone-spec:ADR-0002` D6 **L2** — the round's FULL body, straight from the
/// append-only event file. This is the retrieval path criterion 2 rests on: it
/// does not "recall", it reads.
pub fn retrieve_body(dir: &Path, key: &str) -> Option<String> {
    super::query::read_summary(&dir.to_path_buf(), key, usize::MAX)
}

/// `lodes` (L0) as a command answer: the chain's skeleton.
pub fn answer_lodes(
    dir: &Path,
    leaf: Option<&str>,
    cfg: &ConvergenceConfig,
    now_secs: u64,
) -> String {
    let Some(leaf) = leaf else {
        return "(no conversation selected — `!lodes <period_id>`)".to_string();
    };
    let rows = skeleton_rows_for(dir, leaf, cfg, now_secs);
    if rows.is_empty() {
        return format!("(unknown round: {leaf})");
    }
    /* `!lodes` 是**读数**，不是注入：它**列出全部**轮次，含被驳回的 —— 否则"驳回"
     * 就等于"看不见"，而那正是删除。被驳回的行标出理由与取回路径。 */
    let mut out = String::new();
    let rejected: Vec<&SkeletonRow> = rows.iter().filter(|r| r.rejected.is_some()).collect();
    if !rejected.is_empty() {
        let _ = writeln!(out, "[{} rejected branch(es) — kept, not injected]", rejected.len());
        for r in rejected {
            let _ = writeln!(
                out,
                "- {} [rejected] {} — 理由：{}  · !body {}",
                r.id, r.label, r.rejected.as_deref().unwrap_or("未说明"), r.id
            );
        }
    }
    out.push_str(&skeleton(&rows, cfg.skeleton_max_lines, cfg.gist_max_chars));
    out
}

/// D6 **L1** — one round expanded: status, why, where, and how to get the body.
pub fn answer_lode(dir: &Path, slug: &str, cfg: &ConvergenceConfig, now_secs: u64) -> String {
    let Ok(list) = super::query::list_periods(dir, usize::MAX) else {
        return "(store unreadable)".to_string();
    };
    match list.iter().find(|p| p.period_id == slug) {
        Some(p) => describe(dir, p, cfg, now_secs),
        None => format!("(unknown round: {slug})"),
    }
}

/// `sediment` — the converged rounds, named. The index of what left the default
/// view; it is a VIEW, never a file move (ADR-0049 D5).
pub fn answer_sediment(dir: &Path, leaf: &str, cfg: &ConvergenceConfig, now_secs: u64) -> String {
    let rows = skeleton_rows_for(dir, leaf, cfg, now_secs);
    let converged: Vec<&SkeletonRow> = rows
        .iter()
        .filter(|r| r.status == PeriodStatus::Converged)
        .collect();
    if converged.is_empty() {
        return "(沉淀区为空 — no round has converged yet)".to_string();
    }
    let mut out = String::new();
    let _ = writeln!(
        out,
        "[沉淀区 — {} round(s); full bytes retained, retrieve with `!body <id>`]",
        converged.len()
    );
    for r in converged {
        let _ = writeln!(out, "- {} {}", r.id, r.label);
    }
    out
}

/// The L0 skeleton for the chain ending at `leaf`, or `None` when the mechanism is
/// off or the chain is a single round. One call site, one branch: the caller keeps
/// its old history line instead of getting an empty skeleton.
pub fn skeleton_for_prompt(
    dir: &Path,
    leaf: &str,
    cfg: &ConvergenceConfig,
    now_secs: u64,
) -> Option<String> {
    if !cfg.enabled {
        return None;
    }
    let rows = skeleton_rows_for(dir, leaf, cfg, now_secs);
    if rows.len() < 2 {
        return None;
    }
    Some(skeleton(&rows, cfg.skeleton_max_lines, cfg.gist_max_chars))
}

/// **I5 切片**（`Cellrix:ADR-0047` §33）：从选中根出发的**可达子图**，沿 `parent` 边做闭包。
///
/// 与两种"看起来像"的东西严格区分：
/// * `chain_of` = 从根到某个叶子的**一条祖先链**（`lineage_path`，上下文加载专用）；
/// * 整棵子树 BFS = A4 的旧错误（`chainJobIds` 从根把**所有分支**扫进同一窗口）。
///
/// 本函数只回答一个问题：**这些 period 里，哪些的祖先链真的连到 `root`**。
/// 返回 `(切片成员, 截断说明)`：成员按 (first_ts, period_id) 升序；
/// 若窗口里有行的 `parent` 指向未加载的 id（`conversation_id == None` 是同一事实的既有表达），
/// 第二个元素给出**具名截断说明**——切片可能不完整，但不许假装完整（I6 同族）。
pub fn slice_of<'a>(
    list: &'a [PeriodSummary],
    root: &str,
) -> (Vec<&'a PeriodSummary>, Option<String>) {
    use std::collections::{HashMap, HashSet};
    let by_id: HashMap<&str, &PeriodSummary> =
        list.iter().map(|p| (p.period_id.as_str(), p)).collect();
    if !by_id.contains_key(root) {
        return (Vec::new(), Some(format!("root {root} 不在窗口内 ⇒ 切片不可量（不猜）")));
    }
    // 子边：parent -> [child]（只认 id 父指针；人类可读摘要不可能出现在这里）
    let mut children: HashMap<&str, Vec<&PeriodSummary>> = HashMap::new();
    let mut dangling_parents: HashSet<&str> = HashSet::new();
    for p in list {
        match p.parent.as_deref() {
            Some(par) if by_id.contains_key(par) => children.entry(par).or_default().push(p),
            Some(par) => {
                dangling_parents.insert(par);
            }
            None => {}
        }
    }
    let mut seen: HashSet<&str> = HashSet::new();
    let mut stack = vec![root];
    let mut out: Vec<&PeriodSummary> = Vec::new();
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        if let Some(p) = by_id.get(id) {
            out.push(*p);
        }
        if let Some(kids) = children.get(id) {
            for k in kids {
                stack.push(k.period_id.as_str());
            }
        }
    }
    out.sort_by(|a, b| (a.first_ts.as_str(), a.period_id.as_str()).cmp(&(b.first_ts.as_str(), b.period_id.as_str())));
    let trunc = if dangling_parents.is_empty() {
        None
    } else {
        Some(format!(
            "窗口里有 {} 个父 id 未被加载 ⇒ 切片可能不完整（具名，不假装完整）",
            dangling_parents.len()
        ))
    };
    (out, trunc)
}

/// The chain from the ROOT down to `leaf`, oldest first. Walks `parent` links only
/// (ADR-0047's `lineage_path`) — never expands into branches.
pub fn chain_of<'a>(list: &'a [PeriodSummary], leaf: &str) -> Vec<&'a PeriodSummary> {
    let by_id: std::collections::HashMap<&str, &PeriodSummary> =
        list.iter().map(|p| (p.period_id.as_str(), p)).collect();
    let mut path: Vec<&PeriodSummary> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut cur = leaf;
    while let Some(p) = by_id.get(cur) {
        if !seen.insert(p.period_id.as_str()) {
            break; // a cycle is not a lineage
        }
        path.push(*p);
        match p.parent.as_deref() {
            Some(par) => cur = par,
            None => break,
        }
    }
    path.reverse();
    path
}

#[cfg(test)]
mod slice_tests {
    use super::*;

    fn row(pid: &str, parent: Option<&str>, first: &str) -> PeriodSummary {
        PeriodSummary {
            period_id: pid.to_string(),
            job_id: format!("job-{pid}"),
            first_ts: first.to_string(),
            last_ts: first.to_string(),
            count: 1,
            preview: String::new(),
            reply: String::new(),
            parent: parent.map(|s| s.to_string()),
            conversation_id: parent.map(|_| "root").map(|s: &str| s.to_string()),
            model: None,
            name: None,
            status: None,
            gist: None,
            rejection_log: Vec::new(),
            rejected: false,
        }
    }

    /// I5 正样本：从根出发只收**祖先链连到根**的节点（含分叉），不含别的树。
    #[test]
    fn slice_is_the_reachable_subgraph_not_the_whole_subtree() {
        let rows = vec![
            row("r", None, "t1"),
            row("a", Some("r"), "t2"),
            row("b", Some("r"), "t3"),
            row("a1", Some("a"), "t4"),
            row("other", None, "t5"),
            row("other1", Some("other"), "t6"),
        ];
        let (slice, trunc) = slice_of(&rows, "r");
        let ids: Vec<&str> = slice.iter().map(|p| p.period_id.as_str()).collect();
        assert_eq!(ids, vec!["r", "a", "b", "a1"], "只收 r 的可达闭包，按时间升序");
        assert!(trunc.is_none(), "没有悬空父 ⇒ 不报截断");
    }

    /// 反样本①：根不在窗口 ⇒ 空切片 + 具名（不许猜成"根没有子节点"）。
    #[test]
    fn a_root_outside_the_window_is_named_not_empty_silently() {
        let rows = vec![row("a", Some("r"), "t1")];
        let (slice, trunc) = slice_of(&rows, "r");
        assert!(slice.is_empty());
        assert!(trunc.unwrap_or_default().contains("不在窗口内"));
    }

    /// 反样本②：窗口截断了父 ⇒ 切片照给，但必须带具名截断说明（I6 同族）。
    #[test]
    fn a_truncated_window_is_reported_by_name() {
        let rows = vec![row("b", Some("missing-parent"), "t1"), row("a", None, "t2")];
        let (slice, trunc) = slice_of(&rows, "a");
        assert_eq!(slice.len(), 1);
        assert!(trunc.unwrap_or_default().contains("未被加载"));
    }
}
