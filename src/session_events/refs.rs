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

/// A ref name must be ONE safe path segment: no separators, no traversal, no leading dot, no empty.
pub fn check_ref_name(name: &str) -> io::Result<()> {
    let bad = name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
        || name.starts_with('.');
    if bad {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            format!("invalid ref name {name:?}: a ref is ONE path segment, with no separator and no leading dot"),
        ));
    }
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
    fs::write(dir.join(name), format!("{id}\n"))?;
    Ok(id)
}

/// Remove a ref. `Ok(false)` = there was nothing to remove (again a NAMED absence).
pub fn delete_ref(events_dir: &Path, name: &str) -> io::Result<bool> {
    check_ref_name(name)?;
    match fs::remove_file(refs_dir(events_dir).join(name)) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

/// Every ref, sorted by name. Deterministic on purpose: a listing that shuffles is not auditable.
pub fn list_refs(events_dir: &Path) -> io::Result<Vec<RefEntry>> {
    let mut out = Vec::new();
    let entries = match fs::read_dir(refs_dir(events_dir)) {
        Ok(e) => e,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e),
    };
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let period_id = match fs::read_to_string(entry.path()) {
            Ok(s) => s.trim().to_string(),
            Err(_) => continue,
        };
        if period_id.is_empty() {
            continue;
        }
        out.push(RefEntry { name, period_id });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}
