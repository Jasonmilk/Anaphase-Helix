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
    pub fn emit_period_start(
        &mut self,
        time: &str,
        user_input: &str,
        nodes: usize,
        inject_chars: usize,
    ) -> io::Result<()> {
        self.emit(time, EventType::TurnStart, json!({}))?;
        self.emit(
            time,
            EventType::UserMessage,
            json!({ "text": user_input }),
        )?;
        self.emit(
            time,
            EventType::ContextInject,
            json!({ "nodes": nodes, "chars": inject_chars }),
        )
    }
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
