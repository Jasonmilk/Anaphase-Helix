//! Period identity vs replay handle (K-006 / B17').
//!
//! `job_id` is DERIVED from the input (`fnv64`) and serves *deterministic
//! replay*: the same input yields the same handle. It is a content digest, NOT
//! an identity. `period_id` is ALLOCATED per run (clock + counter) and serves
//! *identity*: one value per period, never reused, never derived from content.
//! A replay therefore allocates a DIFFERENT period id, so a replay comparison
//! must compare the BODY and exclude `period_id`.
//!
//! A period's `parent` points at a `period_id`. Using `job_id` as a parent, or
//! as a lookup key that silently resolves to one of several periods, is the root
//! error this module refuses: see `resolve_period`, and the allocator below,
//! which REJECTS a job id it cannot digest rather than repairing it.

use std::fs;
use std::io;
use std::sync::atomic::{AtomicU64, Ordering};


// ─── Period identity vs replay handle (K-006) ───────────────────────────────
//
// Two identifiers, two incompatible appetites. One field cannot feed both, and
// trying to make `job_id` do both produced every symptom in the K-006 family:
// the same input overwrote a period that another period still pointed at as
// its parent, and the panel silently collapsed two runs into one row.
//
//   `job_id`    DERIVED from the input (`fnv64`). Serves *deterministic
//               replay*: the same input yields the same handle, so a replay can
//               be compared against the original. It is a content digest, NOT
//               an identity.
//   `period_id` ALLOCATED per run (clock + counter). Serves *identity*: one
//               value per period, never reused, never derived from content.
//               Replaying the same input necessarily allocates a DIFFERENT
//               period id — so a replay comparison must compare the BODY and
//               exclude `period_id`.
//
// A period's `parent` points at a `period_id`. Using `job_id` as a parent (or
// as a lookup key that silently resolves to one of several periods) is the
// root error this module now refuses: see `resolve_period` and the ambiguity
// results, plus `job_id_is_not_an_identity` in the tests.

/// The digest part of a `job_id`: `run-` followed by 1..=16 lowercase hex.
///
/// Rejecting rather than filtering is the whole point. The previous version
/// collected the hex characters out of whatever it was handed and sliced the
/// result, which turned an invalid input into a VALID-LOOKING digest: `run-abcd&`
/// became `abcd`, colliding with `run-abcd`, and an input with no hex at all
/// produced `run--p...`. A silently repaired value is worse than a rejected one,
/// because it is indistinguishable from a correct value downstream — the same
/// shape as a default that disguises a missing key as a legitimate one.
fn job_digest(job_id: &str) -> Result<&str, String> {
    let raw = job_id
        .strip_prefix("run-")
        .ok_or_else(|| format!("job id must be run-<hex>, got {job_id:?}"))?;
    if raw.is_empty() {
        return Err(format!("job id has an empty digest: {job_id:?}"));
    }
    if raw.len() > 16 {
        return Err(format!("job id digest exceeds 16 hex chars: {job_id:?}"));
    }
    if !raw.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("job id digest is not hex: {job_id:?}"));
    }
    Ok(raw)
}

/// Allocate the next period id: `run-{input digest}-p{secs}{counter}`.
///
/// The digest keeps different inputs distinguishable at a glance and preserves
/// the `run-` prefix every existing consumer filters on. The `p`-suffixed
/// allocated tail is what makes it an IDENTITY rather than a digest: two runs
/// of the same input differ here.
///
/// Returns an error for a `job_id` that is not `run-<1..=16 hex>` rather than
/// building an id out of the parts that happen to look right. This function is
/// the only allocator in the chain, so an id it gets wrong is an id that is not
/// unique — and every guarantee built on `period_id` (B15's identity, B18's
/// propagation) rests on that.
pub fn try_allocate_period_id(job_id: &str, unix_secs: u64) -> Result<String, String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let digest = job_digest(job_id)?;
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    Ok(format!("run-{digest}-p{unix_secs:010x}{n:06x}"))
}


/// `try_allocate_period_id`, for callers whose input is already known-good
/// (the production path always derives a 16-hex digest). Panics rather than
/// returning a repaired id when that assumption is broken.
pub fn allocate_period_id(job_id: &str, unix_secs: u64) -> String {
    try_allocate_period_id(job_id, unix_secs)
        .unwrap_or_else(|e| panic!("allocate_period_id: {e}"))
}

/// True when `s` is an allocated period id (not merely a `run-`-shaped digest).
///
/// Strict on purpose: this is the guard that keeps a content digest from being
/// used where an identity is required. A plain derived `job_id` and a
/// hand-written `run-…` string both fail — the latter being how two polluted
/// parents entered the live stream during verification.
pub fn is_period_id(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("run-") else {
        return false;
    };
    // The allocated tail is the last `-`-separated segment: `p` + 16 hex chars.
    let Some((_, tail)) = rest.rsplit_once('-') else {
        return false;
    };
    let Some(hex) = tail.strip_prefix('p') else {
        return false;
    };
    hex.len() == 16 && hex.chars().all(|c| c.is_ascii_hexdigit())
}


/// True when `s` has the derived-digest shape (`run-` + hex), i.e. a possible
/// `job_id`. Used only to tell "this is a replay handle" from "this is a
/// period id" at the API boundary — never to resolve a period by itself.
pub fn is_job_id(s: &str) -> bool {
    match s.strip_prefix("run-") {
        Some(rest) => !rest.is_empty() && rest.chars().all(|c| c.is_ascii_hexdigit()),
        None => false,
    }
}

/// How a caller named a period at the API boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeriodRef {
    /// An allocated identity: resolves to exactly one period, or to nothing.
    Id(String),
    /// A derived replay handle: may match zero, one, or MANY periods.
    Job(String),
}

impl PeriodRef {
    /// Classify a caller-supplied key. A period id is checked first because its
    /// shape is strictly narrower.
    pub fn parse(key: &str) -> Self {
        if is_period_id(key) {
            Self::Id(key.to_string())
        } else {
            Self::Job(key.to_string())
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Id(s) | Self::Job(s) => s,
        }
    }
}

/// Resolution outcome. AMBIGUOUS is a first-class result, not an error to be
/// papered over: a caller that guesses which period was meant produces a
/// confident wrong answer, which is worse than a refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    One(String),
    Ambiguous(Vec<String>),
    None,
}

/// Error kind used when a replay handle matches more than one period. Carried
/// as an `io::ErrorKind::Other` message at present so the public signatures do
/// not change shape; the MESSAGE carries the candidates, because a refusal that
/// does not say what to use instead just moves the guess to the caller.
pub fn ambiguous_error(key: &str, candidates: &[String]) -> io::Error {
    io::Error::new(
        io::ErrorKind::Other,
        format!(
            "ambiguous period reference {key:?}: matches {} periods ({}); use a period_id",
            candidates.len(),
            candidates.join(", ")
        ),
    )
}

/// Error kind used when nothing matches the reference.
pub fn not_found_error(key: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!("no period matches {key:?}"),
    )
}

/// Resolve a caller key to exactly one period id, refusing ambiguity.
///
/// This is the single place where "which period did the caller mean?" is
/// answered, and it never guesses: an ambiguous key is an error carrying the
/// candidates, not a silent pick of the newest or the first.
pub fn resolve_one(dir: &std::path::Path, key: &str) -> io::Result<String> {
    if key.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "period key required",
        ));
    }
    match resolve_period(dir, &PeriodRef::parse(key))? {
        Resolved::One(id) => Ok(id),
        Resolved::Ambiguous(ids) => Err(ambiguous_error(key, &ids)),
        Resolved::None => Err(not_found_error(key)),
    }
}

/// Resolve a `PeriodRef` against the event directory.
///
/// `Id` ⇒ at most one file. `Job` ⇒ every period whose rows carry that job id,
/// which is zero, one, or many. Never picks one.
pub fn resolve_period(dir: &std::path::Path, r: &PeriodRef) -> io::Result<Resolved> {
    let suffix = ".events.jsonl";
    let mut hits: Vec<String> = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(p) = name.strip_suffix(suffix) else {
            continue;
        };
        match r {
            PeriodRef::Id(id) => {
                if p == id {
                    hits.push(p.to_string());
                }
            }
            PeriodRef::Job(job) => {
                // Identity comes from the row, never from the file name: a name
                // is a locator, and reading identity out of a path is how this
                // module got into trouble. A legacy file (written before
                // `period_id` existed) has rows without the field, so fall back
                // to the name for those only.
                let id = period_id_of_file(&entry.path())?.unwrap_or_else(|| p.to_string());
                let carries = period_job_id(&entry.path())?;
                if carries.as_deref() == Some(job) || id == *job || p == job {
                    hits.push(id);
                }
            }
        }
    }
    hits.sort();
    hits.dedup();
    Ok(match hits.len() {
        0 => Resolved::None,
        1 => Resolved::One(hits.remove(0)),
        _ => Resolved::Ambiguous(hits),
    })
}

/// The `period_id` carried by a stream's first parseable row. A legacy stream
/// (written before the field existed) yields `None`.
fn period_id_of_file(path: &std::path::Path) -> io::Result<Option<String>> {
    let body = match fs::read_to_string(path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    for line in body.lines() {
        if line.trim().is_empty() {
            continue;
        }
        // Read the RAW value, not the wire type: identity must not depend on
        // `SessionEvent`, or the two modules would need each other.
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            return Ok(v
                .get("period_id")
                .and_then(|x| x.as_str())
                .filter(|x| !x.is_empty())
                .map(|x| x.to_string()));
        }
    }
    Ok(None)
}

/// The `job_id` carried by a stream's first parseable row (legacy files omit
/// it; those are resolved by the file name instead).
fn period_job_id(path: &std::path::Path) -> io::Result<Option<String>> {
    let body = match fs::read_to_string(path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    for line in body.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            return Ok(v.get("job_id").and_then(|x| x.as_str()).map(|x| x.to_string()));
        }
    }
    Ok(None)
}
