//! Reasoning trace — the *body* half of the Engram imprint.
//!
//! The audit chain (Tuck) never stores request/response bodies — that
//! would be a sensitive data lake (ADR-0004). The *body* of every
//! reasoning call (prompt + response) is recorded here, on the Anaphase
//! side where the prompt is constructed and the response is received.
//!
//! # Security contract (the hard part)
//!
//! The prompt may contain credentials (an API key in the user input, a
//! Bearer header copied into memory, ...). The 2026-09-07 audit found a
//! real key in git history — this module is where a repeat would happen.
//! Therefore:
//! - every record is **redacted** before it touches disk (credential
//!   patterns are replaced with `[REDACTED]`, extra patterns configurable);
//! - records are truncated to a configurable char budget (energy: no
//!   unbounded body files);
//! - the trace is opt-in: `reasoning_trace_path` must be set.
//!
//! # Determinism
//!
//! - `trace_id` is the derived job id (`derive_job_id(input)`), the same
//!   key the pipeline uses — Cellrix joins body + chain by it.
//! - `ts` comes from the caller (the injected Clock when a pipeline is
//!   wired), so replay under a FakeClock is byte-identical.
//! - `seq` is file-backed at open (line count), monotonic across restarts.

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

/// One reasoning round trip (post-redaction, post-truncation).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReasoningEntry {
    /// Monotonic sequence, file-backed (line count at open) — the cursor
    /// for incremental pulls.
    pub seq: u64,
    /// RFC3339 timestamp from the caller (injected Clock when wired).
    pub ts: String,
    /// Derived job id — joins this body to the audit chain and ledger.
    pub trace_id: String,
    /// The reasoning model name.
    pub model: String,
    /// The prompt as sent (redacted, truncated).
    pub prompt: String,
    /// The model response (redacted, truncated).
    pub response: String,
}

/// Credential-pattern redaction applied before any body touches disk.
/// Patterns are case-sensitive literals + a `{key}`-style suffix matcher
/// for the common credential shapes; extra regexes are configurable.
#[derive(Debug, Clone)]
pub struct Redaction {
    /// Extra literal patterns from config (e.g. a project-specific secret).
    extra_literals: Vec<String>,
}

impl Default for Redaction {
    fn default() -> Self {
        Self {
            extra_literals: Vec::new(),
        }
    }
}

impl Redaction {
    /// Patterns are matched on the literal plus the following non-space
    /// token (up to 64 chars), so `sk-4056...f534` becomes `[REDACTED]`
    /// while `sk-` alone in prose is untouched.
    const BASE_PATTERNS: [&'static str; 5] = ["sk-", "Bearer ", "api_key=", "token=", "password="];

    pub fn new(extra_literals: Vec<String>) -> Self {
        Self { extra_literals }
    }

    /// Replace every matched credential with `[REDACTED]`.
    pub fn apply(&self, text: &str) -> String {
        let mut out = text.to_string();
        let base = Self::BASE_PATTERNS.iter().copied();
        let extra = self.extra_literals.iter().map(|s| s.as_str());
        for pat in base.chain(extra) {
            out = Self::redact_one(&out, pat);
        }
        out
    }

    fn redact_one(text: &str, pat: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(idx) = rest.find(pat) {
            let after = &rest[idx + pat.len()..];
            let first = after.chars().next();
            let is_credential = matches!(
                first,
                Some(c) if !c.is_whitespace() && !c.is_ascii_punctuation()
            );
            if !is_credential {
                // Prose use of the pattern (no credential token follows):
                // copy it and keep searching from after the pattern.
                out.push_str(&rest[..idx + pat.len()]);
                rest = &rest[idx + pat.len()..];
                continue;
            }
            out.push_str(&rest[..idx]);
            out.push_str("[REDACTED]");
            // Skip the pattern plus the following token (stop at
            // whitespace or punctuation — the token's end).
            let mut take = 0;
            for (i, c) in after.char_indices() {
                if c.is_whitespace() || c.is_ascii_punctuation() {
                    break;
                }
                take = i + c.len_utf8();
            }
            rest = &rest[idx + pat.len() + take..];
        }
        out.push_str(rest);
        out
    }
}

/// Append-only body trace. `None` trace (off) is the default — opt-in.
#[derive(Debug)]
pub struct ReasoningTrace {
    path: PathBuf,
    max_chars: usize,
    redaction: Redaction,
    seq: AtomicU64,
}

impl ReasoningTrace {
    /// Open (or create) the trace file and count existing lines as the
    /// starting sequence — deterministic across restarts.
    pub fn open(
        path: PathBuf,
        max_chars: usize,
        redaction: Redaction,
    ) -> std::io::Result<Self> {
        let start_seq = match std::fs::read_to_string(&path) {
            Ok(content) => content.lines().count() as u64,
            Err(_) => 0,
        };
        Ok(Self {
            path,
            max_chars,
            redaction,
            seq: AtomicU64::new(start_seq),
        })
    }

    /// Append one redacted, truncated entry. `ts` is caller-supplied
    /// (injected Clock when wired) so replay is deterministic.
    pub fn record(
        &self,
        ts: &str,
        trace_id: &str,
        model: &str,
        prompt: &str,
        response: &str,
    ) -> std::io::Result<()> {
        let entry = ReasoningEntry {
            seq: self.seq.fetch_add(1, Ordering::Relaxed),
            ts: ts.to_string(),
            trace_id: trace_id.to_string(),
            model: model.to_string(),
            prompt: truncate(&self.redaction.apply(prompt), self.max_chars),
            response: truncate(&self.redaction.apply(response), self.max_chars),
        };
        let mut line = serde_json::to_string(&entry).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;
        line.push('\n');
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        f.write_all(line.as_bytes())
    }
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        let cut: String = text.chars().take(max_chars).collect();
        format!("{cut}…[truncated]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_credential_shapes() {
        let r = Redaction::default();
        let out = r.apply("use sk-4056aabbccddeeff0011 and Bearer abc123 plus api_key=xyz");
        assert!(!out.contains("sk-4056"), "key leaked: {out}");
        assert!(!out.contains("abc123"), "bearer leaked: {out}");
        assert!(!out.contains("xyz"), "api_key leaked: {out}");
        assert!(out.contains("[REDACTED]"));
        // Prose containing the pattern but no token is untouched.
        let prose = r.apply("the sk- prefix means secret key");
        assert!(prose.contains("sk-"), "prose must survive: {prose}");
    }

    #[test]
    fn extra_patterns_from_config() {
        let r = Redaction::new(vec!["deepseek-".to_string()]);
        let out = r.apply("my deepseek-9f8e7d key");
        assert!(!out.contains("deepseek-9f8e7d"));
        assert!(out.contains("[REDACTED]"));
    }

    #[test]
    fn appends_redacted_truncated_records() {
        let dir = std::env::temp_dir().join(format!("trace-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("reasoning.jsonl");
        let _ = std::fs::remove_file(&path);

        let trace = ReasoningTrace::open(path.clone(), 8, Redaction::default()).unwrap();
        trace
            .record(
                "2026-09-07T05:28:00Z",
                "run-deadbeef#0",
                "deepseek-v4-flash",
                "hello sk-4056world",
                "a response that is quite long",
            )
            .unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let entry: ReasoningEntry = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(entry.seq, 0);
        assert_eq!(entry.trace_id, "run-deadbeef#0");
        assert!(!entry.prompt.contains("sk-4056"), "prompt leaked: {}", entry.prompt);
        assert!(entry.response.contains("[truncated]"));
        assert!(
            entry.response.chars().count() < 40,
            "truncation broken: {}",
            entry.response
        );

        // Restart: seq continues from the line count (deterministic).
        let trace2 = ReasoningTrace::open(path.clone(), 8, Redaction::default()).unwrap();
        trace2
            .record("2026-09-07T05:28:01Z", "run-beef#1", "m", "p", "r")
            .unwrap();
        let content2 = std::fs::read_to_string(&path).unwrap();
        let last: ReasoningEntry =
            serde_json::from_str(content2.lines().last().unwrap()).unwrap();
        assert_eq!(last.seq, 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
