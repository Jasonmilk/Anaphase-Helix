//! REFS — the third layer (ADR-0048 §307/§309).
//!
//! Objects (periods) and edges (`parent`) existed; a REF is the small pointer that says *this
//! conversation continues from HERE*. Git keeps refs in the repository, not in the window that happens
//! to be open — so a ref survives a restart and is shared by every reader instead of being guessed.
//!
//! Shape: ONE FILE per ref under `<events>/.refs/<name>`, holding a single period id (a Git ref is,
//! literally, a small file with one identifier). Nothing else is stored: the conversation a ref points
//! into is DERIVED from the lineage (`conversation_id`), never duplicated here.
//!
//! THREE ENDINGS, NAMED (the rule this project keeps meeting):
//!   · no such ref                       -> `Ok(None)`  (a named absence, not an error)
//!   · a target resolving to no period   -> `Err` (refused: a dangling pointer is never stored)
//!   · a target matching several periods -> `Err` (refused: ambiguity is named, never guessed)
//!   · an unsafe name                    -> `Err` (traversal guard: a ref cannot escape `.refs`)

use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use super::identity::{resolve_period, PeriodRef, Resolved};

/// One ref: the NAME a caller uses and the PERIOD it points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefEntry {
    pub name: String,
    pub period_id: String,
}

fn refs_dir(events_dir: &Path) -> PathBuf {
    events_dir.join(".refs")
}

/// A ref name is a PATH of safe segments (`conversations/<root>`), the shape Git uses for
/// `refs/heads/<name>`. EVERY segment is checked: nesting is allowed and traversal is not — the earlier
/// one-segment guard would have refused the very shape this layer needs.
pub fn check_ref_name(name: &str) -> io::Result<()> {
    let refusal = |why: &str| io::Error::new(ErrorKind::InvalidInput, format!("invalid ref name {name:?}: {why}"));
    if name.is_empty() {
        return Err(refusal("a ref must have a name"));
    }
    if name.contains('\\') || name.contains('\u{0}') {
        return Err(refusal("backslashes and NUL are not part of a ref name"));
    }
    for seg in name.split('/') {
        if seg.is_empty() || seg == "." || seg == ".." || seg.starts_with('.') {
            return Err(refusal("every segment must be a plain name (no empty, no dot-prefixed, no traversal)"));
        }
    }
    Ok(())
}

/// One line of a ref's HISTORY: which value was replaced, and when (a Git-style reflog).
///
/// WHY THIS EXISTS BEFORE ANYONE NEEDS IT (ADR-0048 §315): `PUT` overwrites, so the OLD value is gone
/// the moment it is replaced — and the deletion subsystem (`M2`) derives its grace anchor from exactly
/// that fact: `(time, vacated position)`. A record written AFTERWARDS has fidelity q<1 per write, and
/// `q^n` decays to ~0.37 for any n — that is, a back-filled stub is worth about as much as no stub. The
/// only cheap moment to record it is the moment it happens. A SHAPE debt, not a feature debt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefLogEntry {
    pub ts: String,
    pub old: Option<String>,
    pub new: Option<String>,
}


fn now_rfc3339() -> String {
    super::types_and_stream::now_ts()
}


/// Read a ref's history, oldest first. An unreadable line is SKIPPED rather than fatal: a log with one
/// torn line still answers "what was here before", which is the question it exists to answer.
/// The ref's history — DERIVED from the period streams, never a second book (ADR-0048 §321).
///
/// A `ref/move` row lives in the stream of the period the ref pointed at, so the anchor and the object
/// share one stream. Reading the history is therefore a **projection** over the streams; deleting the
/// projection loses nothing.
pub fn read_ref_log(events_dir: &Path, name: &str) -> io::Result<Vec<RefLogEntry>> {
    check_ref_name(name)?;
    let mut out = Vec::new();
    let entries = match fs::read_dir(events_dir) {
        Ok(e) => e,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e),
    };
    for entry in entries {
        let entry = entry?;
        let fname = entry.file_name().to_string_lossy().to_string();
        if !fname.ends_with(".events.jsonl") {
            continue;
        }
        let body = match fs::read_to_string(entry.path()) {
            Ok(b) => b,
            Err(_) => continue,
        };
        for line in body.lines() {
            let v: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if v.get("type").and_then(|t| t.as_str()) != Some("ref/move") {
                continue;
            }
            let d = v.get("data").cloned().unwrap_or(serde_json::Value::Null);
            if d.get("name").and_then(|n| n.as_str()) != Some(name) {
                continue;
            }
            out.push(RefLogEntry {
                ts: v.get("time").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                old: d.get("old").and_then(|o| o.as_str()).map(|x| x.to_string()),
                new: d.get("new").and_then(|o| o.as_str()).map(|x| x.to_string()),
            });
        }
    }
    /* ORDERED BY THE POINTER CHAIN, NOT BY STREAM ORDER (ADR-0048 §321 + §320 P12). The entries live in
     * different streams, and cross-stream timestamps are NOT comparable — so the only honest order is the
     * one the pointers themselves define: `old -> new`. An entry the chain cannot reach keeps its scan
     * position (named as a partial reconstruction rather than silently reordered). */
    let mut chained: Vec<RefLogEntry> = Vec::new();
    let mut cursor: Option<String> = None;
    let mut remaining = out;
    loop {
        let next = remaining
            .iter()
            .position(|e| e.old == cursor && e.new.is_some())
            .or_else(|| {
                if cursor.is_none() {
                    remaining.iter().position(|e| e.old.is_none())
                } else {
                    None
                }
            });
        match next {
            Some(idx) => {
                let e = remaining.remove(idx);
                cursor = e.new.clone();
                chained.push(e);
            }
            None => break,
        }
    }
    chained.extend(remaining); /* unreachable entries stay, in scan order */
    Ok(chained)
}

/// Write the ref act INTO the target period's stream (ADR-0048 §321).
///
/// Uses `open_append`, so numbering continues from that stream's last `seq` (never re-used) and nothing
/// is overwritten. If the stream is torn, this FAILS BY NAME rather than guessing a number.
fn record_ref_move(
    events_dir: &Path,
    target_period: &str,
    name: &str,
    old: Option<&str>,
    new: Option<&str>,
) -> io::Result<()> {
    let mut stream = super::types_and_stream::SessionEventStream::open_append(
        events_dir.to_path_buf(),
        target_period,
        target_period,
        crate::trace::Redaction::default(),
    )?;
    stream.emit(
        &super::types_and_stream::now_ts(),
        super::types_and_stream::EventType::RefMove,
        serde_json::json!({ "name": name, "old": old, "new": new }),
    )?;
    Ok(())
}

/// Read one ref. `Ok(None)` means "there is no such ref" — an absence, not a failure.
pub fn read_ref(events_dir: &Path, name: &str) -> io::Result<Option<String>> {
    check_ref_name(name)?;
    match fs::read_to_string(refs_dir(events_dir).join(name)) {
        Ok(s) => {
            let t = s.trim().to_string();
            Ok(if t.is_empty() { None } else { Some(t) })
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Point `name` at `period_ref`, REFUSING a target that does not name exactly one existing period.
/// Returns the RESOLVED period id (so the caller learns which period the pointer actually names).
/// SINGLE-WRITER ASSUMPTION (ADR-0048 §318): this reads the previous value and then writes the new one —
/// a check-then-write window. Today exactly ONE process writes `.refs` (the panel), and the reflog records
/// what that process replaced. A second concurrent writer could record a stale `old`, so multi-writer
/// support must close (or explicitly acknowledge) that window with a CONCURRENCY criterion, not by luck.
pub fn write_ref(events_dir: &Path, name: &str, period_ref: &str) -> io::Result<String> {
    check_ref_name(name)?;
    let target = period_ref.trim();
    if target.is_empty() {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "a ref must point at a period: empty target refused",
        ));
    }
    let id = match resolve_period(events_dir, &PeriodRef::parse(target))? {
        Resolved::One(id) => id,
        Resolved::Ambiguous(ids) => {
            return Err(io::Error::new(
                ErrorKind::InvalidInput,
                format!("ref target {target:?} is AMBIGUOUS ({} periods match) — name one period explicitly", ids.len()),
            ))
        }
        Resolved::None => {
            return Err(io::Error::new(
                ErrorKind::NotFound,
                format!("ref target {target:?} does not resolve to any period — refusing to store a dangling pointer"),
            ))
        }
    };
    let dir = refs_dir(events_dir);
    fs::create_dir_all(&dir)?;
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let old = read_ref(events_dir, name)?;
    fs::write(&path, format!("{id}\n"))?;
    /* THE OLD VALUE IS RECORDED AT THE MOMENT IT IS REPLACED, IN THE TARGET'S OWN STREAM
     * (ADR-0048 §321): a second book beside the streams would leave cross-stream order
     * undefined, and the grace anchor compares against the object's history. */
    record_ref_move(events_dir, &id, name, old.as_deref(), Some(&id))?;
    Ok(id)
}

/// Remove a ref. `Ok(false)` = there was nothing to remove (again a NAMED absence).
pub fn delete_ref(events_dir: &Path, name: &str) -> io::Result<bool> {
    check_ref_name(name)?;
    let old = read_ref(events_dir, name)?;
    match fs::remove_file(refs_dir(events_dir).join(name)) {
        Ok(()) => {
            /* A VACANCY is recorded in the stream of the period that was vacated (ADR-0048 §321). */
            if let Some(prev) = old.as_deref() {
                record_ref_move(events_dir, prev, name, old.as_deref(), None)?;
            }
            Ok(true)
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

/// Every ref, sorted by name. Deterministic on purpose: a listing that shuffles is not auditable.
pub fn list_refs(events_dir: &Path) -> io::Result<Vec<RefEntry>> {
    let mut out = Vec::new();
    let root = refs_dir(events_dir);
    collect_refs(&root, &root, &mut out)?;
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Walk the ref tree. A directory starting with `.` holds HISTORY (`.log`), never a ref — the same
/// distinction Git makes between `refs/` and `logs/`.
fn collect_refs(root: &Path, dir: &Path, out: &mut Vec<RefEntry>) -> io::Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            collect_refs(root, &path, out)?;
            continue;
        }
        let period_id = match fs::read_to_string(&path) {
            Ok(s) => s.trim().to_string(),
            Err(_) => continue,
        };
        if period_id.is_empty() {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| name.clone());
        out.push(RefEntry { name: rel, period_id });
    }
    Ok(())
}
