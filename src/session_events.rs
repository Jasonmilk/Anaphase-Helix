// Session event stream — the "session as experience" body (ADR-0023).
// One cognitive period appends its structured events to a single JSONL
// file keyed by the derived job id:
//
//   {"type":"turn/start","seq":0,"time":"<rfc3339>","data":{...}}
//
// The job id is the same join key the body trace and the Tuck audit chain
// carry (ProveTrack), so a client renders one period as a turn timeline
// (USER / CONTEXT / ATTEMPT / TOOL / VERDICT badges) from this one source.
//
// Events carry summaries only: the full prompt/response bodies live in the
// reasoning trace. Credentials are redacted on write (recursive string
// pass), so the stream never becomes a sensitive data lake.

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::trace::Redaction;

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

/// Allocate the next period id: `run-{input digest}-p{secs}{counter}`.
///
/// The digest keeps different inputs distinguishable at a glance and preserves
/// the `run-` prefix every existing consumer filters on. The `p`-suffixed
/// allocated tail is what makes it an IDENTITY rather than a digest: two runs
/// of the same input differ here.
pub fn allocate_period_id(job_id: &str, unix_secs: u64) -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let raw = job_id.strip_prefix("run-").unwrap_or(job_id);
    let digest: String = raw.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    let digest = &digest[..digest.len().min(16)];
    format!("run-{digest}-p{unix_secs:010x}{n:06x}")
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
        if let Ok(row) = serde_json::from_str::<SessionEvent>(line) {
            if row.period_id.is_empty() {
                return Ok(None);
            }
            return Ok(Some(row.period_id));
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
        if let Ok(row) = serde_json::from_str::<SessionEvent>(line) {
            return Ok(Some(row.job_id));
        }
    }
    Ok(None)
}

/// Structured event vocabulary of one cognitive period. The `as_str`
/// values are the protocol wire names (contract, not implementation
/// detail); consumers match on these exact strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// A cognitive period started (run_cycle entered the state machine).
    TurnStart,
    /// The human's raw input, redacted on write.
    UserMessage,
    /// Memory/craft injection folded into the prompt (summary only).
    ContextInject,
    /// The Reasoning adapter's output (attempt), redacted on write.
    Attempt,
    /// Private reasoning (thinking) streamed by the model, redacted on
    /// write. Display-only — never participates in criteria.
    Think,
    /// A tool call assembled into the deterministic job (args summary).
    ToolCall,
    /// A tool execution result (evidence row summary, with outcome).
    ToolResult,
    /// One deterministic criteria check (judge/gate/expect/actual/reason).
    Check,
    /// The criteria verdict (MET / UNMET / blocked).
    Verdict,
    /// The assistant's final answer of the period (the deliverable that
    /// closes the ProveTrack chain: user → think → attempt → tools → verdict →
    /// reply → end). Empty reply = the honest zero-length answer, emitted
    /// anyway so the chain never silently loses the deliverable.
    AssistantReply,
    /// Token accounting of ONE upstream round trip (ADR-0038). A period has
    /// three call sites — the retry loop (streamed or buffered, N times) and
    /// the tool-evidence finalize — so this is emitted once per round trip,
    /// not once per period; the per-period total is derived on read, never
    /// accumulated on write.
    ///
    /// Display-only, exactly like `Think`: it is disclosure, never evidence
    /// for criteria. The token-budget circuit breaker (DNA principle 7) reads
    /// `EnergyContext.token_budget` — a different data path, not this event.
    Usage,
    /// The period ended and returned to Perception.
    TurnEnd,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::TurnStart => "turn/start",
            EventType::UserMessage => "user/message",
            EventType::ContextInject => "context/inject",
            EventType::Attempt => "assistant/attempt",
            EventType::Think => "assistant/think",
            EventType::ToolCall => "tool/call",
            EventType::ToolResult => "tool/result",
            EventType::Check => "check/status",
            EventType::Verdict => "verdict/status",
            EventType::AssistantReply => "assistant/reply",
            EventType::Usage => "assistant/usage",
            EventType::TurnEnd => "turn/end",
        }
    }
}

/// One append-only stream row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    /// This period's ALLOCATED identity (`period_id`). The row carries it so
    /// consumers never have to read identity out of a file path, and so a
    /// merged multi-period view can tell two runs of the same input apart.
    /// Empty only on rows written before this field existed (legacy files).
    #[serde(default)]
    pub period_id: String,
    /// The DERIVED replay handle (`fnv64` of the input). Same join key as the
    /// body trace and the Tuck audit chain. This is a content digest, NOT an
    /// identity: two runs of one input share it. Never use it as a parent
    /// pointer or as a lookup key that expects a single period.
    #[serde(default)]
    pub job_id: String,
    /// Row number WITHIN this period. It restarts at 0 for every period, so it
    /// is NOT comparable across periods — the unique row key is the pair
    /// `(period_id, seq)`. A reader that treats bare `seq` as global will
    /// mistake two periods' rows for duplicates of each other.
    pub seq: u64,
    /// RFC3339 timestamp from the injected clock (replayable).
    pub time: String,
    pub data: Value,
}

/// Append-only per-period event sink. Writes are non-fatal at the call
/// site: the cognitive loop must not die on a stream write, mirroring the
/// reasoning trace contract.
pub struct SessionEventStream {
    seq: u64,
    path: PathBuf,
    file: fs::File,
    redact: Redaction,
    /// The period's own identity. The stream knows where it lives and which
    /// period it is, so features that need the `.name` sidecar do not have to
    /// re-derive them from the file path.
    dir: PathBuf,
    /// Allocated identity of this period — the file's name and the `parent`
    /// target. Distinct per run, even for a repeated input.
    period_id: String,
    /// Derived replay handle of this period's input. Carried for the join with
    /// the body trace and the audit chain; never used as a lookup key.
    job_id: String,
}

impl SessionEventStream {
    /// Open (create) the stream for one period under `dir`.
    ///
    /// The file is named after `period_id`, so a repeated input lands in a NEW
    /// file rather than overwriting the earlier run. That is what removed the
    /// old `.truncate(true)`: a fresh name needs no clearing, and clearing was
    /// what discarded history a still-referenced parent depended on (K-006).
    pub fn open(
        dir: PathBuf,
        period_id: &str,
        job_id: &str,
        redact: Redaction,
    ) -> io::Result<Self> {
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{period_id}.events.jsonl"));
        let file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            // Deliberately NOT truncating. Reaching an existing name here would
            // mean two periods share an identity, which is a bug to surface
            // rather than to paper over by clearing the file.
            .open(&path)?;
        Ok(SessionEventStream {
            seq: 0,
            path,
            file,
            redact,
            dir,
            period_id: period_id.to_string(),
            job_id: job_id.to_string(),
        })
    }

    /// Append one event. `time` comes from the caller (injected clock).
    /// The data payload is redacted recursively before it touches disk.
    pub fn emit(&mut self, time: &str, event_type: EventType, data: Value) -> io::Result<()> {
        let row = SessionEvent {
            event_type: event_type.as_str().to_string(),
            period_id: self.period_id.clone(),
            job_id: self.job_id.clone(),
            seq: self.seq,
            time: time.to_string(),
            data: self.redact_value(data),
        };
        self.seq += 1;
        let line = serde_json::to_string(&row)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        writeln!(self.file, "{line}")?;
        Ok(())
    }

    /// Path the stream writes to (for status/diagnostics).
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Recursive string redaction: credentials never reach the stream.
    fn redact_value(&self, value: Value) -> Value {
        match value {
            Value::String(s) => Value::String(self.redact.apply(&s)),
            Value::Object(mut map) => {
                for (_, v) in map.iter_mut() {
                    *v = self.redact_value(v.take());
                }
                Value::Object(map)
            }
            Value::Array(mut arr) => {
                for v in arr.iter_mut() {
                    *v = self.redact_value(v.take());
                }
                Value::Array(arr)
            }
            other => other,
        }
    }
}

/// Convenience helpers for the common payload shapes (keeps call sites
/// terse and the vocabulary single-sourced).
impl SessionEventStream {
    /// turn/start + user/message + context/inject — the period header.
    /// `detail` (SA-Core choice: tiers distribution + top nodes, provenance
    /// only) and `resume_from` (explicit continuation of a previous
    /// experience) ride on context/inject when present.
    pub fn emit_period_start(
        &mut self,
        time: &str,
        user_input: &str,
        nodes: usize,
        inject_chars: usize,
        resume_from: Option<&str>,
        detail: Option<&Value>,
    ) -> io::Result<()> {
        self.emit(time, EventType::TurnStart, json!({}))?;
        self.emit(
            time,
            EventType::UserMessage,
            json!({ "text": user_input }),
        )?;
        // Freeze the card title at creation.
        //
        // The title must NOT drift. It used to be recomputed on every render from
        // `first_ts`, so rewriting a timestamp renamed the card — and ADR-0041
        // records that timestamps DO get rewritten when a job id is reused. The
        // `.name` sidecar already exists for human renames, so the automatic title
        // simply joins that slot and is written ONCE, here, from the first user
        // message: stable by construction, meaningful, and timezone-free (the panel
        // formats local time by design, so the backend must never bake a zone into
        // a stored string).
        //
        // An empty input writes nothing, leaving those periods to the panel's
        // time-based fallback: tolerant degradation, never an empty card title.
        //
        // The sidecar is keyed by `period_id` so the title belongs to the run
        // that froze it; two runs of one input each own their own card title.
        //
        // `rename_period` validates the id and requires the allocated form, so
        // this is a no-op for the short ids tests use. Ignoring that error is
        // deliberate: a missing title must never fail a period write.
        if period_name(&self.dir, &self.period_id).is_none() {
            if let Some(name) = freeze_name(user_input) {
                let _ = rename_period(&self.dir, &self.period_id, &name);
            }
        }
        let mut data = json!({ "nodes": nodes, "chars": inject_chars });
        if let Some(r) = resume_from {
            data["resume_from"] = json!(r);
        }
        if let Some(d) = detail {
            data["choice"] = d.clone();
        }
        self.emit(time, EventType::ContextInject, data)
    }
}

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

/// Scratch directory for tests.
///
/// A fixed name is a race: `cargo test` runs tests in parallel, and a test that
/// removes its directory at the end will remove a *sibling's* files mid-run if
/// both picked the same name. The name is therefore derived from the process id
/// and a per-process sequence number, so every call owns its own directory and
/// no two tests can disturb each other.
#[cfg(test)]
mod test_support {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static SEQ: AtomicUsize = AtomicUsize::new(0);

    pub fn tmp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "se-test-{}-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed),
            tag
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn ts() -> String {
        crate::ledger::unix_secs_to_rfc3339(1_700_000_000)
    }

    fn tmp_dir() -> std::path::PathBuf {
        super::test_support::tmp_dir("tests")
    }

    #[test]
    fn crystallize_distills_unmet_into_rule() {
        use crate::trace::Redaction;
        let dir = tmp_dir().join("distill");
        let redact = Redaction::default();
        let mut stream = SessionEventStream::open(dir.clone(), "run-xyz", "run-xyz", redact).unwrap();
        let t = ts();
        stream
            .emit(&t, EventType::UserMessage, json!({ "text": "calc 7^9" }))
            .unwrap();
        stream
            .emit(&t, EventType::ToolCall, json!({ "tool": "calc", "index": 0, "expect": "ok" }))
            .unwrap();
        stream
            .emit(
                &t,
                EventType::ToolResult,
                json!({ "tool": "calc", "ok": true, "duration_ms": 276, "outcome": "40353607", "outcome_sha": "abcd1234" }),
            )
            .unwrap();
        stream
            .emit(
                &t,
                EventType::Check,
                json!({ "check_id": "run-xyz#c0", "check": "exec_ok", "passed": false, "judge": "rule", "gate": "hard", "expect": "ok", "evidence_id": "run-xyz#0", "reason": "ok=false or echo mismatch" }),
            )
            .unwrap();
        stream
            .emit(&t, EventType::Verdict, json!({ "job_id": "run-xyz", "status": "Unmet", "reason": "failed: exec_ok" }))
            .unwrap();
        stream
            .emit(&t, EventType::TurnEnd, json!({ "done": true, "success": false, "impasse": false, "verdict": "Unmet" }))
            .unwrap();
        drop(stream);

        let out = crystallize(&dir, 50).unwrap();
        assert_eq!(out.len(), 1, "one unmet period -> one suggestion");
        let s = &out[0];
        assert_eq!(s.job_id, "run-xyz");
        assert_eq!(s.tool, "calc");
        assert_eq!(s.expect, "ok");
        assert_eq!(s.failed_checks, vec!["exec_ok".to_string()]);
        assert_eq!(s.outcome_shas, vec!["abcd1234".to_string()]);
        assert!(s.suggested_rule.contains("gate=hard judge=rule"));
        assert!(dir.join("crystallized/rule-run-xyz.json").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn crystallize_skips_met_periods() {
        use crate::trace::Redaction;
        let dir = tmp_dir().join("skips");
        let redact = Redaction::default();
        let mut stream = SessionEventStream::open(dir.clone(), "run-met", "run-met", redact).unwrap();
        let t = ts();
        stream
            .emit(&t, EventType::Verdict, json!({ "job_id": "run-met", "status": "Met", "reason": "all checks passed" }))
            .unwrap();
        drop(stream);
        let out = crystallize(&dir, 50).unwrap();
        assert!(out.is_empty(), "Met periods are not ore");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn vocabulary_is_stable() {
        assert_eq!(EventType::TurnStart.as_str(), "turn/start");
        assert_eq!(EventType::UserMessage.as_str(), "user/message");
        assert_eq!(EventType::ContextInject.as_str(), "context/inject");
        assert_eq!(EventType::Attempt.as_str(), "assistant/attempt");
        assert_eq!(EventType::ToolCall.as_str(), "tool/call");
        assert_eq!(EventType::ToolResult.as_str(), "tool/result");
        assert_eq!(EventType::Verdict.as_str(), "verdict/status");
        assert_eq!(EventType::AssistantReply.as_str(), "assistant/reply");
        assert_eq!(EventType::Usage.as_str(), "assistant/usage");
        assert_eq!(EventType::TurnEnd.as_str(), "turn/end");
    }

    #[test]
    fn emits_monotonic_seq_and_roundtrips() {
        let dir = tmp_dir();
        let mut stream = SessionEventStream::open(dir.clone(), "job-a", "job-a", Redaction::default()).unwrap();
        let t = ts();
        stream.emit(&t, EventType::TurnStart, json!({})).unwrap();
        stream.emit(&t, EventType::UserMessage, json!({ "text": "hi" })).unwrap();
        let rows: Vec<String> = fs::read_to_string(stream.path())
            .unwrap()
            .lines()
            .map(|l| l.to_string())
            .collect();
        assert_eq!(rows.len(), 2);
        let e0: SessionEvent = serde_json::from_str(&rows[0]).unwrap();
        let e1: SessionEvent = serde_json::from_str(&rows[1]).unwrap();
        assert_eq!(e0.seq, 0);
        assert_eq!(e1.seq, 1);
        assert_eq!(e0.event_type, "turn/start");
        assert_eq!(e1.event_type, "user/message");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn period_start_carries_resume_and_choice_detail() {
        let dir = tmp_dir();
        let mut stream = SessionEventStream::open(dir.clone(), "job-c", "job-c", Redaction::default()).unwrap();
        let t = ts();
        let detail = json!({
            "tiers": { "L1": 1, "L2": 2, "L3": 5 },
            "top": [{ "id": "n-1", "tier": "L3", "heat": 0.82, "phase": "liquid" }]
        });
        stream
            .emit_period_start(&t, "hello", 8, 800, Some("run-abc"), Some(&detail))
            .unwrap();
        let rows: Vec<SessionEvent> = fs::read_to_string(stream.path())
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(rows.len(), 3);
        let ctx = &rows[2];
        assert_eq!(ctx.event_type, "context/inject");
        assert_eq!(ctx.data["nodes"], 8);
        assert_eq!(ctx.data["chars"], 800);
        assert_eq!(ctx.data["resume_from"], "run-abc");
        assert_eq!(ctx.data["choice"]["tiers"]["L3"], 5);
        assert_eq!(ctx.data["choice"]["top"][0]["tier"], "L3");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_summary_flattens_last_round_as_history() {
        let dir = tmp_dir();
        let mut stream = SessionEventStream::open(dir.clone(), "job-d", "job-d", Redaction::default()).unwrap();
        let t = ts();
        stream.emit(&t, EventType::TurnStart, json!({})).unwrap();
        stream.emit(&t, EventType::UserMessage, json!({ "text": "用计算器算 7 的 9 次方" })).unwrap();
        stream.emit(&t, EventType::Attempt, json!({ "text": "我将用确定性工具计算，而不是口算。" })).unwrap();
        let summary = read_summary(&dir, "job-d", 400).expect("summary");
        assert!(summary.contains("用计算器算 7 的 9 次方"));
        assert!(summary.contains("我将用确定性工具计算"));
        assert!(summary.contains("human said:"));
        assert!(summary.contains("helix answered:"));
        // Missing period -> None (resuming a vanished episode is a no-op).
        assert!(read_summary(&dir, "run-missing", 400).is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn prose_is_never_a_parent_pointer() {
        // Real ids (the shape Anaphase now mints): an ALLOCATED tail, not a
        // plain derived digest.
        let minted = allocate_period_id("run-8bba24c5ee368a4a", 1_760_000_000);
        assert!(is_period_id(&minted), "a minted period id must be a parent: {minted}");
        // A bare derived digest is a REPLAY HANDLE, not an identity. It must not
        // pass: every period of that input shares it, so accepting it as a
        // parent is how one period's lineage silently points at another's.
        assert!(
            !is_period_id("run-8bba24c5ee368a4a"),
            "a content digest is not an identity"
        );
        assert!(is_job_id("run-8bba24c5ee368a4a"), "but it is a valid job id");
        // The two shapes that actually polluted the live stream.
        assert!(
            !is_period_id("human said: 我最喜欢的数字是 7\nhelix answered: 7"),
            "the human-readable continuation summary must never act as a parent"
        );
        assert!(
            !is_period_id("run-adr0043-t5b-verify"),
            "an arbitrary caller-supplied id must not become a parent just because \
             it carries the run- prefix"
        );
        assert!(!is_period_id(""), "empty is not a parent");
        assert!(!is_period_id("run-"), "the prefix alone is not a parent");
    }

    /// C9 reverse guard: a `job_id` may never be *used* as an identity, even
    /// though the field is present in every row. This is the mistake the K-006
    /// family is made of (a digest standing in for "this run"), so it gets a
    /// test rather than only a comment.
    #[test]
    fn job_id_is_not_an_identity() {
        let dir = tmp_dir().join("not-an-identity");
        fs::create_dir_all(&dir).unwrap();
        let job = "run-bbbb2222";
        let p1 = allocate_period_id(job, 1_760_000_000);
        let p2 = allocate_period_id(job, 1_760_000_001);
        assert_ne!(p1, p2, "two runs of one input must not share an identity");
        for (id, ts) in [(&p1, "t1"), (&p2, "t2")] {
            let mut s = SessionEventStream::open(
                dir.clone(),
                id,
                job,
                Redaction::default(),
            )
            .unwrap();
            s.emit(ts, EventType::UserMessage, json!({ "text": "same input" })).unwrap();
        }
        // Resolving the DIGEST is ambiguous and must say so, not pick one.
        let r = resolve_period(&dir, &PeriodRef::parse(job)).unwrap();
        match r {
            Resolved::Ambiguous(ids) => {
                assert_eq!(ids.len(), 2, "both runs must be listed: {ids:?}");
            }
            other => panic!("a repeated input must not resolve silently: {other:?}"),
        }
        let _ = fs::remove_dir_all(&dir);
    }

    /// The frozen title is content, collapsed and bounded — never empty, never
    /// derived from a clock.
    #[test]
    fn freeze_name_is_bounded_content_not_a_clock() {
        assert_eq!(freeze_name("  记住:   我最喜欢的\n数字是 7  "), Some("记住: 我最喜欢的 数字是 7".to_string()));
        assert_eq!(freeze_name("   "), None, "nothing to freeze");
        assert_eq!(freeze_name(""), None);
        let long = "甲".repeat(NAME_MAX_CHARS + 10);
        let got = freeze_name(&long).unwrap();
        assert_eq!(got.chars().count(), NAME_MAX_CHARS + 1, "bounded + ellipsis");
        assert!(got.ends_with('…'));
        assert_eq!(freeze_name(&"乙".repeat(NAME_MAX_CHARS)).unwrap().chars().count(), NAME_MAX_CHARS, "exactly at the bound needs no ellipsis");
    }

    #[test]
    fn redacts_strings_recursively() {
        let dir = tmp_dir();
        let mut stream = SessionEventStream::open(dir.clone(), "job-b", "job-b", Redaction::new(vec!["sk-secret-key-12345".to_string()]))
        .unwrap();
        let t = ts();
        stream
            .emit(
                &t,
                EventType::Attempt,
                json!({ "text": "call sk-secret-key-12345 now", "nested": { "k": "sk-secret-key-12345" } }),
            )
            .unwrap();
        let row: SessionEvent =
            serde_json::from_str(&fs::read_to_string(stream.path()).unwrap()).unwrap();
        let text = row.data["text"].as_str().unwrap();
        let nested = row.data["nested"]["k"].as_str().unwrap();
        assert!(!text.contains("sk-secret-key-12345"), "raw literal leaked: {text}");
        assert!(!nested.contains("sk-secret-key-12345"), "nested literal leaked: {nested}");
        assert!(text.contains("[REDACTED]"), "no redaction marker: {text}");
        let _ = fs::remove_dir_all(&dir);
    }
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

/// Persist a human-chosen experience name as a `{job_id}.name` sidecar next
/// to the event stream — same directory, same identity, one source of truth
/// shared by every client. The id is validated against the allocated shape so
/// a hostile value can never escape the directory.
///
/// Takes an `id`, not a `job_id`: the sidecar belongs to the period, and a
/// replay handle is not an identity. Callers holding only a `job_id` must
/// resolve it first (`resolve_period`) and pass the resulting period id — or
/// refuse when the resolution is ambiguous.
pub fn rename_period(dir: &std::path::Path, id: &str, name: &str) -> io::Result<()> {
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
const NAME_MAX_CHARS: usize = 40;

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

fn period_name(dir: &std::path::Path, job_id: &str) -> Option<String> {
    let raw = fs::read_to_string(dir.join(format!("{job_id}.name"))).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
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

#[cfg(test)]
mod query_tests {
    use super::*;

    #[test]
    fn lists_periods_newest_first() {
        let dir = test_support::tmp_dir("lists_periods_newest_first");
        // Two periods written out of time order (b first, a second).
        let mut b = SessionEventStream::open(dir.clone(), "job-b", "job-b", Redaction::default()).unwrap();
        let mut a = SessionEventStream::open(dir.clone(), "job-a", "job-a", Redaction::default()).unwrap();
        b.emit("2026-09-07T00:00:00Z", EventType::UserMessage, json!({ "text": "second" })).unwrap();
        b.emit("2026-09-07T00:00:02Z", EventType::TurnEnd, json!({})).unwrap();
        a.emit("2026-09-07T00:00:10Z", EventType::UserMessage, json!({ "text": "first message" })).unwrap();
        a.emit("2026-09-07T00:00:12Z", EventType::TurnEnd, json!({})).unwrap();
        let list = list_periods(&dir, 10).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].job_id, "job-a", "newest first");
        assert_eq!(list[1].job_id, "job-b");
        assert_eq!(list[0].preview, "first message");
        assert_eq!(list[0].count, 2);
        let _ = fs::remove_dir_all(&dir);
    }

    /// A parent that names no existing period must be reported as absent, and a
    /// parent that exists must survive `limit` truncation.
    ///
    /// The dangling value is deliberately SHAPE-VALID (`run-0badc0de` passes
    /// `is_period_id`) so this covers what that check cannot: the writer was right
    /// about the format and the period is simply gone. A consumer receiving it
    /// either reconstructs a thread that is not there or silently treats the period
    /// as a root; `None` is the honest answer.
    ///
    /// The second half pins the ORDER of the two steps. Normalisation runs before
    /// `truncate`, because a parent that merely fell outside the requested window
    /// still exists — nulling it would be a lie about the data rather than a
    /// convenience for the caller.
    #[test]
    fn a_parent_that_does_not_exist_is_reported_as_absent() {
        let dir = test_support::tmp_dir("dangling_parent_is_absent");
        // Identity is allocated, so the parent pointers below are period ids.
        let root_id = allocate_period_id("run-aaaa1111", 1_760_000_000);
        let orphan_id = allocate_period_id("run-cccc3333", 1_760_000_001);
        let child_id = allocate_period_id("run-bbbb2222", 1_760_000_002);
        // Oldest: the parent of the child, and the one limit=1 will truncate.
        let mut root = SessionEventStream::open(dir.clone(), &root_id, "run-aaaa1111", Redaction::default()).unwrap();
        root.emit("2026-09-07T00:00:00Z", EventType::UserMessage, json!({ "text": "root" })).unwrap();
        root.emit("2026-09-07T00:00:01Z", EventType::TurnEnd, json!({})).unwrap();
        // Middle: continues a period that does not exist.
        let mut orphan = SessionEventStream::open(dir.clone(), &orphan_id, "run-cccc3333", Redaction::default()).unwrap();
        orphan.emit("2026-09-07T00:00:05Z", EventType::ContextInject, json!({ "resume_from": "run-0badc0de" })).unwrap();
        orphan.emit("2026-09-07T00:00:06Z", EventType::TurnEnd, json!({})).unwrap();
        // Newest: continues the root, which exists.
        let mut child = SessionEventStream::open(dir.clone(), &child_id, "run-bbbb2222", Redaction::default()).unwrap();
        child.emit("2026-09-07T00:00:10Z", EventType::ContextInject, json!({ "resume_from": root_id })).unwrap();
        child.emit("2026-09-07T00:00:11Z", EventType::TurnEnd, json!({})).unwrap();

        let all = list_periods(&dir, 10).unwrap();
        let by = |id: &str| {
            all.iter()
                .find(|p| p.period_id == id)
                .unwrap()
                .parent
                .clone()
        };
        assert_eq!(by(&child_id), Some(root_id.clone()));
        assert_eq!(by(&orphan_id), None, "a parent that exists nowhere is not a parent");

        let one = list_periods(&dir, 1).unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].period_id, child_id, "newest first");
        assert_eq!(
            one[0].parent,
            Some(root_id),
            "a parent truncated out of the window still EXISTS and must not be nulled"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// T1 + T8: the same input twice. Both periods survive, both are listed,
    /// and the shared `job_id` refuses to resolve to either one.
    #[test]
    fn repeated_input_keeps_both_periods_and_refuses_to_guess() {
        let dir = test_support::tmp_dir("repeated_input_two_periods");
        let job = "run-aaaa1111";
        let first = allocate_period_id(job, 1_760_000_000);
        let second = allocate_period_id(job, 1_760_000_100);

        for (id, ts, text) in [
            (&first, "2026-09-07T00:00:00Z", "first run"),
            (&second, "2026-09-07T00:10:00Z", "second run"),
        ] {
            let mut s = SessionEventStream::open(dir.clone(), id, job, Redaction::default()).unwrap();
            s.emit(ts, EventType::TurnStart, json!({})).unwrap();
            s.emit(ts, EventType::UserMessage, json!({ "text": text })).unwrap();
        }

        // T1: the first run is still readable, by its own id.
        let events = read_period(&dir, &first).unwrap();
        assert_eq!(events.len(), 2, "the earlier run must not have been overwritten");
        assert_eq!(events[1].data["text"], "first run");
        let second_events = read_period(&dir, &second).unwrap();
        assert_eq!(second_events[1].data["text"], "second run");
        assert_ne!(first, second, "two runs must own distinct identities");

        // Both rows reach the list — the truth, not a collapsed single row.
        let all = list_periods(&dir, 10).unwrap();
        assert_eq!(all.len(), 2, "both runs must be listed");
        assert!(all.iter().all(|p| p.job_id == job));
        assert!(all.iter().any(|p| p.period_id == first));
        assert!(all.iter().any(|p| p.period_id == second));

        // T8: opening by the shared digest must ERROR and name the candidates.
        let err = read_period(&dir, job).expect_err("an ambiguous digest must not resolve");
        let msg = err.to_string();
        assert!(msg.contains(&first) && msg.contains(&second), "candidates must be named: {msg}");
        // ...and nothing was mutated by the refusal.
        assert_eq!(read_period(&dir, &first).unwrap().len(), 2);
        assert_eq!(read_period(&dir, &second).unwrap().len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    /// T2 + T9: the positive control. A digest that matches exactly ONE period
    /// still resolves, so the refusal above is about ambiguity rather than a
    /// break in the ordinary path.
    #[test]
    fn a_unique_digest_still_resolves_by_job_id() {
        let dir = test_support::tmp_dir("unique_digest_resolves");
        let job = "run-1234abcd";
        let only = allocate_period_id(job, 1_760_000_000);
        let mut s = SessionEventStream::open(dir.clone(), &only, job, Redaction::default()).unwrap();
        s.emit("2026-09-07T00:00:00Z", EventType::UserMessage, json!({ "text": "hi" })).unwrap();

        let events = read_period(&dir, job).unwrap();
        assert_eq!(events.len(), 1, "a single match resolves without ceremony");
        assert_eq!(events[0].period_id, only);
        let _ = fs::remove_dir_all(&dir);
    }

    /// T5: `seq` restarts every period, so the unique row key is the PAIR
    /// `(period_id, seq)`. Two periods' row 0 are not duplicates of each other.
    #[test]
    fn seq_is_scoped_to_its_period() {
        let dir = test_support::tmp_dir("seq_scoped_to_period");
        let job = "run-9999beef";
        let a = allocate_period_id(job, 1_760_000_000);
        let b = allocate_period_id(job, 1_760_000_001);
        for id in [&a, &b] {
            let mut s = SessionEventStream::open(dir.clone(), id, job, Redaction::default()).unwrap();
            s.emit("2026-09-07T00:00:00Z", EventType::TurnStart, json!({})).unwrap();
        }
        let ea = read_period(&dir, &a).unwrap();
        let eb = read_period(&dir, &b).unwrap();
        assert_eq!(ea[0].seq, 0);
        assert_eq!(eb[0].seq, 0, "seq restarts in the second period");
        // Same seq, different period id => distinct rows, not a duplicate.
        let key_a = (ea[0].period_id.clone(), ea[0].seq);
        let key_b = (eb[0].period_id.clone(), eb[0].seq);
        assert_ne!(key_a, key_b, "(period_id, seq) is the unique row key");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn reads_one_period_by_id() {
        let dir = test_support::tmp_dir("read_one_period_by_id");
        let mut s = SessionEventStream::open(dir.clone(), "job-x", "job-x", Redaction::default()).unwrap();
        s.emit("2026-09-07T00:00:00Z", EventType::TurnStart, json!({})).unwrap();
        s.emit("2026-09-07T00:00:01Z", EventType::UserMessage, json!({ "text": "hi" })).unwrap();
        let events = read_period(&dir, "job-x").unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "turn/start");
        assert_eq!(events[1].event_type, "user/message");
        assert!(read_period(&dir, "missing").is_err(), "unknown id must error");
        let _ = fs::remove_dir_all(&dir);
    }
}

// ---- Crystallization (ADR-0029) ----
//
// UNMET periods are raw ore, rules are the product. `crystallize` scans
// the latest periods, folds each Unmet verdict's physical outcome
// (tool/result) and its check rows into one rule suggestion. The machine
// only suggests — a human reviews before any rule goes live; nothing is
// auto-injected (fail-human, never fail-machine).

/// One rule suggestion distilled from an Unmet period.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrystalSuggestion {
    /// The period that supplied the raw ore.
    pub job_id: String,
    pub tool: String,
    pub expect: String,
    /// Which deterministic checks failed (names).
    pub failed_checks: Vec<String>,
    /// The first failed check's reason (why it failed).
    pub symptom: String,
    /// Outcome fingerprints of the physical runs (join for the full body).
    pub outcome_shas: Vec<String>,
    /// 0-token, human-readable rule template (review, then inject).
    pub suggested_rule: String,
}

/// Scan the latest `limit` periods under `dir` and distill Unmet rounds
/// into rule suggestions, persisted under `{dir}/crystallized/`. Returns
/// the newly suggested rules (empty when nothing unmet).
pub fn crystallize(dir: &std::path::Path, limit: usize) -> io::Result<Vec<CrystalSuggestion>> {
    let mut suggestions = Vec::new();
    let periods = list_periods(dir, limit).unwrap_or_default();
    for p in periods {
        let Ok(events) = read_period(dir, &p.job_id) else {
            continue;
        };
        let mut verdict: Option<String> = None;
        let mut tool: Option<String> = None;
        let mut expect: Option<String> = None;
        let mut failed: Vec<String> = Vec::new();
        let mut symptom = String::new();
        let mut shas: Vec<String> = Vec::new();
        for ev in &events {
            match ev.event_type.as_str() {
                "tool/result" => {
                    tool = ev
                        .data
                        .get("tool")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    if let Some(s) = ev.data.get("outcome_sha").and_then(|v| v.as_str()) {
                        shas.push(s.to_string());
                    }
                }
                "tool/call" => {
                    expect = ev
                        .data
                        .get("expect")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
                "check/status" => {
                    let passed = ev.data.get("passed").and_then(|v| v.as_bool()).unwrap_or(false);
                    if !passed {
                        let check = ev
                            .data
                            .get("check")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?")
                            .to_string();
                        let reason = ev
                            .data
                            .get("reason")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        if symptom.is_empty() {
                            symptom = reason;
                        }
                        failed.push(check);
                    }
                }
                "verdict/status" => {
                    verdict = ev.data.get("status").and_then(|v| v.as_str()).map(|s| s.to_string());
                }
                _ => {}
            }
        }
        if verdict.as_deref() == Some("Unmet") && !failed.is_empty() {
            let tool = tool.unwrap_or_default();
            let expect = expect.unwrap_or_default();
            let rule = format!(
                "gate=hard judge=rule: when expect={} and {} then reject before execution",
                expect,
                failed.join("/")
            );
            suggestions.push(CrystalSuggestion {
                job_id: p.job_id.clone(),
                tool,
                expect,
                failed_checks: failed,
                symptom,
                outcome_shas: shas,
                suggested_rule: rule,
            });
        }
    }
    if !suggestions.is_empty() {
        let out_dir = dir.join("crystallized");
        fs::create_dir_all(&out_dir)?;
        for s in &suggestions {
            let path = out_dir.join(format!("rule-{}.json", s.job_id));
            if let Ok(json) = serde_json::to_string_pretty(s) {
                let mut f = fs::File::create(path)?;
                writeln!(f, "{json}")?;
            }
        }
    }
    Ok(suggestions)
}
