//! C14b — PINS WITH A RESTART-STABLE OWNER (ADR-0048 §330).
//!
//! The refcount (C14) answers "how many holders"; it does NOT answer "whose holders" — and a per-process
//! random identity makes every pin an orphan the moment the process restarts, which is a PERMANENT leak
//! (the reviewer measured ~39% of objects). So:
//!   · an owner is a DECLARATIVE name (`panel`, `tui`, `m5-shell`), and a per-process-looking token is
//!     REFUSED BY NAME — the refusal is what keeps the name meaningful across restarts;
//!   · the pins live in an append-only WAL (`<events>/pins.jsonl`, the same `open_append` as every other
//!     append: max+1 numbering, tail-newline guard, torn-tail refusal). It is INTERNAL — not part of the
//!     client protocol — so it does not widen the event vocabulary;
//!   · `<events>/.owners` DECLARES which owners are alive. A pin whose owner is not declared is an ORPHAN:
//!     it is NAMED, it still PROTECTS (silently collecting someone's object is worse than a named leak),
//!     and it is released only by an EXPLICIT act — the same discipline as D0's tombstone.

use std::collections::HashMap;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

/// The pin WAL lives in its OWN SUBDIRECTORY (ADR-0048 §330). `SessionEventStream` names files
/// `<id>.events.jsonl`, and `list_periods` walks the events directory non-recursively — so a subdirectory is
/// the one place an internal stream can live without ever being read as a period. (Measured the hard way:
/// writing to `pins.jsonl` while the stream wrote `pins.events.jsonl` made every count read zero.)
fn pins_dir(events_dir: &Path) -> PathBuf {
    events_dir.join(".pins")
}
fn pins_path(events_dir: &Path) -> PathBuf {
    pins_dir(events_dir).join("pins.events.jsonl")
}
fn owners_path(events_dir: &Path) -> PathBuf {
    pins_dir(events_dir).join(".owners")
}

/// An owner must be a DECLARATIVE name. A per-process token (a UUID, a bare hex blob, a pid-shaped number)
/// is refused: it would look like ownership while guaranteeing the pin becomes an orphan on restart.
pub fn check_owner(owner: &str) -> io::Result<()> {
    let refusal = |why: &str| {
        io::Error::new(ErrorKind::InvalidInput, format!("invalid pin owner {owner:?}: {why}"))
    };
    if owner.is_empty() {
        return Err(refusal("an owner must be named"));
    }
    if owner.len() > 64 || !owner.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.') {
        return Err(refusal("an owner is a plain declarative name (letters, digits, `-`, `_`, `.`)"));
    }
    let degit = owner.replace(['-', '_'], "");
    let looks_like_uuid = degit.len() >= 16
        && degit.chars().all(|c| c.is_ascii_hexdigit())
        && owner.chars().any(|c| c.is_ascii_digit());
    if looks_like_uuid {
        return Err(refusal(
            "a UUID-shaped owner is NOT restart-stable — it makes every pin an orphan on restart (C14b). \
             Name the component instead (`panel`, `tui`, `m5-shell`).",
        ));
    }
    Ok(())
}

/// Record one pin/unpin FACT. Append-only: the history is the state, so a restart cannot lose a holder.
fn append_pin_fact(events_dir: &Path, op: &str, object: &str, owner: &str) -> io::Result<()> {
    check_owner(owner)?;
    let mut stream = super::types_and_stream::SessionEventStream::open_append(
        pins_dir(events_dir),
        "pins",
        "pins",
        crate::trace::Redaction::default(),
    )?;
    stream.emit(
        &super::types_and_stream::now_ts(),
        super::types_and_stream::EventType::TurnStart, /* placeholder type: this stream is INTERNAL and is
                                                        * never read as protocol events; the FACTS are in
                                                        * `data` (`op`/`object`/`owner`) */
        serde_json::json!({ "op": op, "object": object, "owner": owner }),
    )?;
    Ok(())
}

pub fn pin(events_dir: &Path, object: &str, owner: &str) -> io::Result<()> {
    append_pin_fact(events_dir, "pin", object, owner)
}

pub fn unpin(events_dir: &Path, object: &str, owner: &str) -> io::Result<()> {
    append_pin_fact(events_dir, "unpin", object, owner)
}

/// Replay the pin WAL: per-object refcounts (C14), and per-(object, owner) holdings.
pub fn replay(events_dir: &Path) -> io::Result<(HashMap<String, u32>, HashMap<(String, String), u32>)> {
    let mut by_object: HashMap<String, u32> = HashMap::new();
    let mut by_holder: HashMap<(String, String), u32> = HashMap::new();
    let body = match fs::read_to_string(pins_path(events_dir)) {
        Ok(b) => b,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok((by_object, by_holder)),
        Err(e) => return Err(e),
    };
    for line in body.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
        let d = v.get("data").cloned().unwrap_or(serde_json::Value::Null);
        let (Some(op), Some(object), Some(owner)) = (
            d.get("op").and_then(|x| x.as_str()),
            d.get("object").and_then(|x| x.as_str()),
            d.get("owner").and_then(|x| x.as_str()),
        ) else {
            continue;
        };
        let entry = by_holder.entry((object.to_string(), owner.to_string())).or_insert(0);
        match op {
            "pin" => *entry += 1,
            "unpin" => *entry = entry.saturating_sub(1),
            _ => continue,
        }
        let total = by_object.entry(object.to_string()).or_insert(0);
        match op {
            "pin" => *total += 1,
            "unpin" => *total = total.saturating_sub(1),
            _ => {}
        }
    }
    by_object.retain(|_, v| *v > 0);
    by_holder.retain(|_, v| *v > 0);
    Ok((by_object, by_holder))
}

/// The DECLARED live owners. An empty or missing file declares NOTHING, which makes every pin an orphan —
/// and that is named rather than assumed away.
pub fn declared_owners(events_dir: &Path) -> io::Result<Vec<String>> {
    match fs::read_to_string(owners_path(events_dir)) {
        Ok(b) => Ok(b
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

/// Holdings whose owner is NOT declared alive: NAMED, still protective, releasable only explicitly.
pub fn orphan_pins(events_dir: &Path) -> io::Result<Vec<(String, String)>> {
    let (_, holders) = replay(events_dir)?;
    let declared: std::collections::HashSet<String> = declared_owners(events_dir)?.into_iter().collect();
    let mut out: Vec<(String, String)> = holders
        .keys()
        .filter(|(_, owner)| !declared.contains(owner))
        .cloned()
        .collect();
    out.sort();
    Ok(out)
}

/// Release every holding of one orphaned owner, by recording an explicit `unpin` for each. Returns how many
/// holdings were released (0 = a NAMED absence, not a silent success).
pub fn release_orphan_owner(events_dir: &Path, owner: &str) -> io::Result<usize> {
    let (_, holders) = replay(events_dir)?;
    let mut released = 0usize;
    for ((object, o), n) in holders.iter() {
        if o != owner {
            continue;
        }
        for _ in 0..*n {
            append_pin_fact(events_dir, "unpin", object, owner)?;
            released += 1;
        }
    }
    Ok(released)
}
