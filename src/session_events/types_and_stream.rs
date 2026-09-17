//! The wire vocabulary and the append-only per-period sink.

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::trace::Redaction;
use super::identity::{is_period_id, PeriodRef};
use super::naming::{freeze_name, period_name, rename_period};


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

