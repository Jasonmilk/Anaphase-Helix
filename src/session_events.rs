// Session event stream — the "session as experience" body (ADR-0023).
// One cognitive period appends its structured events to a single JSONL
// file keyed by the derived job id:
//
//   {"type":"turn/start","seq":0,"time":"<rfc3339>","data":{...}}
//
// The job id is the same join key the body trace and the Tuck audit chain
// carry (Engram), so a client renders one period as a turn timeline
// (USER / CONTEXT / ATTEMPT / TOOL / VERDICT badges) from this one source.
//
// Events carry summaries only: the full prompt/response bodies live in the
// reasoning trace. Credentials are redacted on write (recursive string
// pass), so the stream never becomes a sensitive data lake.

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::trace::Redaction;

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
    /// A tool call assembled into the deterministic job (args summary).
    ToolCall,
    /// A tool execution result (evidence row summary).
    ToolResult,
    /// The criteria verdict (MET / UNMET / blocked).
    Verdict,
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
            EventType::ToolCall => "tool/call",
            EventType::ToolResult => "tool/result",
            EventType::Verdict => "verdict/status",
            EventType::TurnEnd => "turn/end",
        }
    }
}

/// One append-only stream row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    /// Monotonic row number within the period (deterministic replay).
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
}

impl SessionEventStream {
    /// Open (create) the stream for one job id under `dir`. The directory
    /// is created on demand; the redactor reuses the trace credential
    /// shapes plus any config literals.
    pub fn open(dir: PathBuf, job_id: &str, redact: Redaction) -> io::Result<Self> {
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{job_id}.events.jsonl"));
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        Ok(SessionEventStream {
            seq: 0,
            path,
            file,
            redact,
        })
    }

    /// Append one event. `time` comes from the caller (injected clock).
    /// The data payload is redacted recursively before it touches disk.
    pub fn emit(&mut self, time: &str, event_type: EventType, data: Value) -> io::Result<()> {
        let row = SessionEvent {
            event_type: event_type.as_str().to_string(),
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
pub fn read_summary(dir: &PathBuf, job_id: &str, max_chars: usize) -> Option<String> {
    let path = dir.join(format!("{job_id}.events.jsonl"));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ts() -> String {
        crate::ledger::unix_secs_to_rfc3339(1_700_000_000)
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
        assert_eq!(EventType::TurnEnd.as_str(), "turn/end");
    }

    #[test]
    fn emits_monotonic_seq_and_roundtrips() {
        let dir = std::env::temp_dir().join("anaphase-session-events-test-1");
        let _ = fs::remove_dir_all(&dir);
        let mut stream = SessionEventStream::open(dir.clone(), "job-a", Redaction::default()).unwrap();
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
        let dir = std::env::temp_dir().join("anaphase-session-events-test-3");
        let _ = fs::remove_dir_all(&dir);
        let mut stream = SessionEventStream::open(dir.clone(), "job-c", Redaction::default()).unwrap();
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
        let dir = std::env::temp_dir().join("anaphase-session-events-test-4");
        let _ = fs::remove_dir_all(&dir);
        let mut stream = SessionEventStream::open(dir.clone(), "job-d", Redaction::default()).unwrap();
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

    fn redacts_strings_recursively() {
        let dir = std::env::temp_dir().join("anaphase-session-events-test-2");
        let _ = fs::remove_dir_all(&dir);
        let mut stream = SessionEventStream::open(
            dir.clone(),
            "job-b",
            Redaction::new(vec!["sk-secret-key-12345".to_string()]),
        )
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

/// Read one period's full event stream by job id (on-demand query).
pub fn read_period(dir: &std::path::Path, job_id: &str) -> io::Result<Vec<SessionEvent>> {
    let path = dir.join(format!("{job_id}.events.jsonl"));
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

/// One period's list summary (session-management sidebar, Engram v2).
/// Reads only each file's first row (user/message preview + start time)
/// and last row (end time) — on-demand, never a hot index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PeriodSummary {
    pub job_id: String,
    pub first_ts: String,
    pub last_ts: String,
    pub count: u64,
    /// Bounded first-human-prompt preview (protocol default 120 chars).
    pub preview: String,
}

/// List periods from the event directory, newest first. `limit` bounds the
/// returned window (protocol default lives at the caller, not here).
pub fn list_periods(dir: &std::path::Path, limit: usize) -> io::Result<Vec<PeriodSummary>> {
    let mut out = Vec::new();
    let entries = fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(job_id) = name.strip_suffix(".events.jsonl") else {
            continue;
        };
        let body = match fs::read_to_string(entry.path()) {
            Ok(b) => b,
            Err(_) => continue, // torn file mid-write: skip, never fail the list
        };
        let mut first_ts = String::new();
        let mut last_ts = String::new();
        let mut preview = String::new();
        let mut count: u64 = 0;
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
                if row.event_type == EventType::UserMessage.as_str() {
                    if let Some(t) = row.data.get("text").and_then(|v| v.as_str()) {
                        // Protocol default preview bound (README Engram
                        // section); summaries are bounded by construction.
                        preview = t.chars().take(120).collect();
                    }
                }
            }
            last_ts = row.time;
        }
        if count == 0 {
            continue;
        }
        out.push(PeriodSummary {
            job_id: job_id.to_string(),
            first_ts,
            last_ts,
            count,
            preview,
        });
    }
    // Newest first by first event timestamp; tie-break by job id for
    // determinism (same data, same order).
    out.sort_by(|a, b| b.first_ts.cmp(&a.first_ts).then_with(|| a.job_id.cmp(&b.job_id)));
    out.truncate(limit);
    Ok(out)
}

#[cfg(test)]
mod query_tests {
    use super::*;

    #[test]
    fn lists_periods_newest_first() {
        let dir = std::env::temp_dir().join("anaphase-session-events-test-3");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // Two periods written out of time order (b first, a second).
        let mut b = SessionEventStream::open(dir.clone(), "job-b", Redaction::default()).unwrap();
        let mut a = SessionEventStream::open(dir.clone(), "job-a", Redaction::default()).unwrap();
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

    #[test]
    fn reads_one_period_by_id() {
        let dir = std::env::temp_dir().join("anaphase-session-events-test-4");
        let _ = fs::remove_dir_all(&dir);
        let mut s = SessionEventStream::open(dir.clone(), "job-x", Redaction::default()).unwrap();
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
