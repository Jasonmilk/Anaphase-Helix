//! Unit tests for the HTTP security-gate adapter.
//!
//! A child module of `security_gate` (mounted with `#[path]`), so it can reach the
//! private decision helpers. It lives in its own file because the line budget counts an
//! inline `#[cfg(test)] mod` as production NCLOC: the same tests cost 246 lines there
//! and nothing here (test paths are excluded), and the file that ships is the smaller
//! one. `src/reflex_tests.rs` was split out for the same reason.

use super::*;
use crate::security::GateCheck;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

fn check() -> GateCheck {
    GateCheck {
        job_id: "job-1".to_string(),
        index: 0,
        tool: "danger_tool".to_string(),
        args_json: r#"{"target":"/tmp"}"#.to_string(),
        identity_labels: BTreeMap::new(),
    }
}

#[derive(Debug)]
struct FakeTransport {
    reply: Result<(u16, String), String>,
    sent: Arc<Mutex<Vec<(String, String)>>>,
}

impl FakeTransport {
    fn replying(
        status: u16,
        body: &str,
    ) -> (Box<dyn GateTransport>, Arc<Mutex<Vec<(String, String)>>>) {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let t = FakeTransport {
            reply: Ok((status, body.to_string())),
            sent: sent.clone(),
        };
        (Box::new(t), sent)
    }

    fn failing(why: &str) -> Box<dyn GateTransport> {
        Box::new(FakeTransport {
            reply: Err(why.to_string()),
            sent: Arc::new(Mutex::new(Vec::new())),
        })
    }
}

#[async_trait]
impl GateTransport for FakeTransport {
    async fn post_json(&self, url: &str, body: String) -> Result<(u16, String), String> {
        self.sent.lock().unwrap().push((url.to_string(), body));
        self.reply.clone()
    }
}

async fn verdict_from(status: u16, body: &str) -> GateVerdict {
    let (t, _) = FakeTransport::replying(status, body);
    HttpSecurityGate::with_transport("http://gate.test/v1/gate", t)
        .check(&check())
        .await
}

#[test]
fn every_failure_path_fails_closed() {
    let cases: [(&str, GateVerdict); 4] = [
        ("transport", unreachable_verdict("connection refused")),
        ("http 500", verdict_for(500, "{}")),
        ("not json", verdict_for(200, "<html/>")),
        ("unknown decision", verdict_for(200, r#"{"decision":"maybe"}"#)),
    ];
    for (label, verdict) in cases {
        assert!(!verdict.permits(), "{label}: must not permit");
        assert!(
            matches!(verdict, GateVerdict::HitlRequired(_)),
            "{label}: must ask a human, got {verdict:?}"
        );
    }
}

/// The same table, but driven through `check` — so a future edit that stops consulting
/// the transport, or maps its error to `Pass`, is caught here and not only in the unit
/// of the mapping function.
#[tokio::test]
async fn check_never_permits_on_any_failure() {
    let blocked = [
        ("http 503", verdict_from(503, "upstream down").await),
        ("http 404", verdict_from(404, "").await),
        ("not json", verdict_from(200, "<html/>").await),
        (
            "unknown decision",
            verdict_from(200, r#"{"decision":"maybe"}"#).await,
        ),
        ("missing decision", verdict_from(200, "{}").await),
    ];
    for (label, verdict) in blocked {
        assert!(
            !verdict.permits(),
            "{label}: must not permit, got {verdict:?}"
        );
    }
    let transport_error = HttpSecurityGate::with_transport(
        "http://gate.test/v1/gate",
        FakeTransport::failing("connection refused"),
    )
    .check(&check())
    .await;
    assert!(
        !transport_error.permits(),
        "a transport error is not an approval: {transport_error:?}"
    );
}

#[tokio::test]
async fn the_door_keepers_decisions_map_onto_the_local_verdicts() {
    assert_eq!(
        verdict_from(200, r#"{"decision":"pass"}"#).await,
        GateVerdict::Pass
    );
    assert_eq!(
        verdict_from(200, r#"{"decision":"reject","reason":"policy"}"#).await,
        GateVerdict::Reject("policy".to_string())
    );
    assert_eq!(
        verdict_from(200, r#"{"decision":"hitl_required","reason":"ask"}"#).await,
        GateVerdict::HitlRequired("ask".to_string())
    );
    // The mirror: a decision Anaphase cannot itself produce must still arrive.
    assert_eq!(
        verdict_from(200, r#"{"decision":"hard_override"}"#).await,
        GateVerdict::HardOverride
    );
}

/// Non-vacuity: the decision must be about **this** call, so the tool name and the job
/// id have to be on the wire, and the configured URL must be the one used. A gate
/// consulted with an empty body would still "return Pass" against any mock.
#[tokio::test]
async fn the_check_is_what_gets_sent() {
    let (t, sent) = FakeTransport::replying(200, r#"{"decision":"pass"}"#);
    let gate = HttpSecurityGate::with_transport("http://flowmodus.test/api/security/gate", t);
    assert_eq!(gate.check(&check()).await, GateVerdict::Pass);

    let sent = sent.lock().unwrap().clone();
    assert_eq!(sent.len(), 1, "the gate must be consulted exactly once");
    assert_eq!(sent[0].0, "http://flowmodus.test/api/security/gate");
    let body: serde_json::Value = serde_json::from_str(&sent[0].1).expect("sent body is JSON");
    assert_eq!(body["tool"], "danger_tool");
    assert_eq!(body["job_id"], "job-1");
}

/// A blocking verdict must say *why*. `reqwest`'s top-level message names neither the
/// timeout nor the refused connection, so the cause chain is flattened in.
#[test]
fn a_blocking_reason_carries_its_cause_chain() {
    let inner = std::io::Error::new(std::io::ErrorKind::TimedOut, "operation timed out");
    let outer = std::io::Error::new(std::io::ErrorKind::Other, inner);
    let text = describe(&outer);
    assert!(
        text.contains("operation timed out"),
        "cause chain lost: {text}"
    );
}

/// The one **real** transport test: a gate that is configured but has nobody listening
/// must block, and must say that is why. No mock is involved, so a mock that misbehaves
/// cannot make this pass for the wrong reason.
#[tokio::test]
async fn an_unreachable_gate_does_not_permit() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/gate", listener.local_addr().unwrap());
    drop(listener);
    let gate = HttpSecurityGate::new(&url, Duration::from_secs(2));
    let verdict = gate.check(&check()).await;
    assert!(
        !verdict.permits(),
        "no door-keeper must never read as approval: {verdict:?}"
    );
    assert!(
        format!("{verdict:?}").contains("unreachable"),
        "the block must name its direction: {verdict:?}"
    );
}

#[test]
fn default_gate_timeout_is_bounded() {
    assert!(DEFAULT_GATE_TIMEOUT <= Duration::from_secs(2));
}
