//! Reading periods back: one period's rows, its summary, and the ordered list.
//!
//! `read_summary` and `read_period` resolve their key the same way: a
//! `period_id` is unique by construction, while a `job_id` matching several
//! periods is an ERROR carrying the candidates — picking one would answer with
//! confidence about the wrong run.

use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::identity::{is_period_id, resolve_one};
use super::naming::period_name;
use super::types_and_stream::{EventType, SessionEvent};


/// One period's conversation summary for explicit continuation (resume):
/// the human's message plus the assistant's attempt, bounded and flattened
/// into "true history" prose. None when the period has neither, or the
/// stream is missing — resuming a vanished episode is a no-op, never an
/// error.
///
/// Resolves the key the same way `read_period` does. An AMBIGUOUS key also
/// yields None here: a resume cannot continue two periods at once, and
/// inventing a summary from an arbitrary one would write a false parent into
/// the stream (the K-004 fault, in a new place). The caller sees the absence
/// and can re-issue with an explicit `period_id`.
pub fn read_summary(dir: &PathBuf, key: &str, max_chars: usize) -> Option<String> {
    let id = resolve_one(dir, key).ok()?;
    let path = dir.join(format!("{id}.events.jsonl"));
    let Ok(content) = fs::read_to_string(&path) else {
        return None;
    };
    let mut user: Option<String> = None;
    let mut attempt: Option<String> = None;
    for line in content.lines() {
        let Ok(ev) = serde_json::from_str::<SessionEvent>(line) else {
            continue;
        };
        match ev.event_type.as_str() {
            "user/message" => {
                user = ev.data.get("text").and_then(|v| v.as_str()).map(|s| s.to_string())
            }
            "assistant/attempt" => {
                attempt = ev
                    .data
                    .get("text")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            }
            _ => {}
        }
    }
    let user = user?;
    let mut out = format!("human said: {}", user.chars().take(max_chars).collect::<String>());
    if let Some(a) = attempt {
        let cut: String = a.chars().take(max_chars).collect();
        if !cut.is_empty() {
            out.push_str("\nhelix answered: ");
            out.push_str(&cut);
        }
    }
    Some(out)
}

/// Read one period's full event stream (on-demand query).
///
/// `key` may be an allocated `period_id` (unique by construction) or a derived
/// `job_id`. A `job_id` that matches several periods is an ERROR carrying the
/// candidates: picking one would answer with confidence about the wrong run.
pub fn read_period(dir: &std::path::Path, key: &str) -> io::Result<Vec<SessionEvent>> {
    let id = resolve_one(dir, key)?;
    let path = dir.join(format!("{id}.events.jsonl"));
    let body = fs::read_to_string(&path)?;
    let mut events = Vec::new();
    for line in body.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<SessionEvent>(line) {
            Ok(e) => events.push(e),
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("bad event row: {e}"),
                ))
            }
        }
    }
    Ok(events)
}

/// (The period-id shape check now lives at the top of this module, alongside
/// the identity-vs-replay-handle rationale. The doc comment above records why
/// the reader refuses non-ids: `resume_from` is MACHINE-READABLE lineage, and
/// the writer once put the human-readable continuation summary in that slot,
/// orphaning 12 of 139 periods on the live stream. History is append-only, so
/// the reader must refuse rather than trust the field.)
/// }

/// One period's list summary (session-management sidebar, ProveTrack v2).
/// Reads only each file's first row (user/message preview + start time)
/// and last row (end time) — on-demand, never a hot index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PeriodSummary {
    /// This period's ALLOCATED identity. Clients key on this: it is what
    /// `parent` points at, and what distinguishes two runs of one input.
    pub period_id: String,
    /// The DERIVED replay handle of this period's input (join key for the body
    /// trace and the audit chain). Kept because callers still hold it; it may
    /// repeat across rows, so it is NOT usable as a list key.
    pub job_id: String,
    pub first_ts: String,
    pub last_ts: String,
    pub count: u64,
    /// Bounded first-human-prompt preview (protocol default 120 chars).
    pub preview: String,
    /// Bounded assistant reply preview (protocol default 200 chars) — the
    /// deliverable of the period, so a session list is never blind.
    pub reply: String,
    /// Continuation parent (`context/inject.resume_from`): this period is a
    /// direct continuation of that one. Null = a fresh conversation root.
    pub parent: Option<String>,
    /// Physical model that served the period (ADR-0036): from the upstream
    /// response, not the config declaration. Null = adapter saw no model.
    pub model: Option<String>,
    /// Human-chosen experience name (`{job_id}.name` sidecar), if any.
    pub name: Option<String>,
}

/// List periods from the event directory, newest first. `limit` bounds the
/// returned window (protocol default lives at the caller, not here).
pub fn list_periods(dir: &std::path::Path, limit: usize) -> io::Result<Vec<PeriodSummary>> {
    let mut out = Vec::new();
    let entries = fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        // The NAME is only a filter: whether this is a period stream. Identity
        // is read from the rows below, never from the path.
        let Some(file_stem) = name.strip_suffix(".events.jsonl") else {
            continue;
        };
        let body = match fs::read_to_string(entry.path()) {
            Ok(b) => b,
            Err(_) => continue, // torn file mid-write: skip, never fail the list
        };
        let mut first_ts = String::new();
        let mut last_ts = String::new();
        let mut preview = String::new();
        let mut reply = String::new();
        let mut parent = None;
        let mut model = None;
        let mut count: u64 = 0;
        // Identity from the data. Legacy rows carry no `period_id`, so the file
        // stem stands in for them — the one place a name is allowed to speak,
        // and only because those rows predate the field.
        let mut period_id = String::new();
        let mut job_id = file_stem.to_string();
        for line in body.lines() {
            if line.trim().is_empty() {
                continue;
            }
            count += 1;
            let row: SessionEvent = match serde_json::from_str(line) {
                Ok(r) => r,
                Err(_) => continue,
            };
            if count == 1 {
                first_ts = row.time.clone();
                if !row.period_id.is_empty() {
                    period_id = row.period_id.clone();
                }
                if !row.job_id.is_empty() {
                    job_id = row.job_id.clone();
                }
            }
            // Preview = the FIRST human message of the period, wherever it
            // sits in the stream (turn/start leads the file, so a first-row
            // check alone would always miss it — that was the "无用户输入"
            // bug). Bounded by construction (protocol default 120 chars).
            if preview.is_empty() && row.event_type == EventType::UserMessage.as_str() {
                if let Some(t) = row.data.get("text").and_then(|v| v.as_str()) {
                    preview = t.chars().take(120).collect();
                }
            }
            // Reply preview = the first assistant/reply deliverable. Bounded
            // (protocol default 200 chars) — the list answers "what did it
            // say?", not a full transcript.
            if reply.is_empty() && row.event_type == EventType::AssistantReply.as_str() {
                if let Some(t) = row.data.get("text").and_then(|v| v.as_str()) {
                    reply = t.chars().take(200).collect();
                }
                if model.is_none() {
                    model = row
                        .data
                        .get("model")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
            }
            // Continuation parent: context/inject carries resume_from when
            // this period was resumed from a previous one (session thread).
            if parent.is_none() && row.event_type == EventType::ContextInject.as_str() {
                if let Some(r) = row.data.get("resume_from").and_then(|v| v.as_str()) {
                    // Only a real period id is a parent. See `is_period_id`: the
                    // writer used to put the human-readable summary here, and
                    // history is append-only, so the reader must refuse prose
                    // rather than inherit the confusion.
                    if is_period_id(r) {
                        parent = Some(r.to_string());
                    }
                }
            }
            last_ts = row.time;
        }
        if count == 0 {
            continue;
        }
        out.push(PeriodSummary {
            // Identity first, replay handle second: a client keys on
            // `period_id`, and `job_id` is kept because callers still hold it.
            period_id: if period_id.is_empty() {
                job_id.clone()
            } else {
                period_id
            },
            name: period_name(dir, &job_id),
            job_id,
            first_ts,
            last_ts,
            count,
            preview,
            reply,
            parent,
            model,
        });
    }
    // A parent pointer must point at a period that EXISTS. A value that does not
    // resolve is not a parent, and handing it to a consumer invites it to
    // reconstruct a thread that is not there. Two ways a dangling value arises:
    // the period file was removed, or the writer once put the human-readable
    // summary here (`is_period_id` now rejects that on the way in).
    //
    // Normalised against the FULL set, deliberately BEFORE `truncate`: a parent
    // that merely fell outside the requested limit still exists, so nulling it
    // would be a lie about the data rather than a convenience for the caller.
    //
    // Membership is checked against PERIOD IDS, because that is what a parent
    // now points at. Checking `job_id` here would let a digest satisfy the
    // lookup and quietly accept the wrong lineage.
    let known: Vec<String> = out.iter().map(|p| p.period_id.clone()).collect();
    for p in out.iter_mut() {
        if let Some(par) = p.parent.as_ref() {
            if !known.iter().any(|k| k == par) {
                p.parent = None;
            }
        }
    }

    // Newest first by first event timestamp; tie-break by PERIOD ID for
    // determinism (same data, same order). The tie-break must not be `job_id`:
    // two runs of one input share it, so the order would depend on scan order
    // and the same data could render differently between calls.
    out.sort_by(|a, b| {
        b.first_ts
            .cmp(&a.first_ts)
            .then_with(|| a.period_id.cmp(&b.period_id))
    });
    out.truncate(limit);
    Ok(out)
}
