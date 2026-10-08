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
    path_oldest_first
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
            }
        })
        .collect()
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
pub fn skeleton(rows: &[SkeletonRow], max_lines: usize) -> String {
    if rows.is_empty() || max_lines == 0 {
        return String::new();
    }
    let mut out = String::new();
    let hidden = rows.len().saturating_sub(max_lines);
    if hidden > 0 {
        let _ = writeln!(
            out,
            "- [{hidden} earlier round(s) hidden — retrieve with `!body <id>`]"
        );
    }
    for r in &rows[hidden..] {
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
    skeleton(&rows, cfg.skeleton_max_lines)
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
    Some(skeleton(&rows, cfg.skeleton_max_lines))
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
