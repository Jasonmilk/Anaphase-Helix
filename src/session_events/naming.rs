//! The human-chosen `.name` sidecar and the frozen card title.
//!
//! The sidecar is keyed by `period_id`, so each run owns its card title. The
//! automatic title is frozen once at period creation from the first user
//! message: stable by construction, meaningful, and timezone-free.

use std::fs;
use std::io;

use super::identity::resolve_one;


/// Persist a human-chosen experience name as a `{job_id}.name` sidecar next
/// to the event stream — same directory, same identity, one source of truth
/// shared by every client. The id is validated against the allocated shape so
/// a hostile value can never escape the directory.
///
/// Takes an `id`, not a `job_id`: the sidecar belongs to the period, and a
/// replay handle is not an identity. Callers holding only a `job_id` must
/// resolve it first (`resolve_period`) and pass the resulting period id — or
/// refuse when the resolution is ambiguous.
pub fn rename_period(dir: &std::path::Path, key: &str, name: &str) -> io::Result<()> {
    // Resolve first: renaming "one of the periods that share this digest"
    // would relabel an arbitrary run. Ambiguity is an error, not a choice.
    let resolved = resolve_one(dir, key)?;
    let id: &str = &resolved;
    let valid = id.len() >= 4
        && id.len() <= 64
        && id.starts_with("run-")
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if !valid {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid period id"));
    }
    let name = name.trim();
    let sidecar = dir.join(format!("{id}.name"));
    if name.is_empty() {
        // Empty name = clear the sidecar (rename back to auto preview).
        match fs::remove_file(&sidecar) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    } else {
        fs::write(&sidecar, name)
    }
}

/// Load the optional human-chosen name for a period (`{job_id}.name`).
/// Characters of the first user message kept as a frozen card title.
pub(crate) const NAME_MAX_CHARS: usize = 40;

/// The title to freeze at period creation: the human's first message, whitespace
/// collapsed and bounded. `None` when there is nothing to freeze.
///
/// Pure and timezone-free on purpose. A time-derived title drifts whenever the
/// timestamp it was computed from is rewritten, and it cannot be read as meaning
/// anything; the first message is both stable once frozen and self-describing.
pub fn freeze_name(user_input: &str) -> Option<String> {
    let collapsed = user_input.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return None;
    }
    let total = collapsed.chars().count();
    let mut out: String = collapsed.chars().take(NAME_MAX_CHARS).collect();
    if total > NAME_MAX_CHARS {
        out.push('…');
    }
    Some(out)
}

pub(crate) fn period_name(dir: &std::path::Path, job_id: &str) -> Option<String> {
    let raw = fs::read_to_string(dir.join(format!("{job_id}.name"))).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

