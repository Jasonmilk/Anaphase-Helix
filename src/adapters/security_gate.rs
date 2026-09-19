//! HTTP security-gate adapter — the door the production path was missing.
//!
//! `SecurityGate` is Anaphase's **local** contract (`src/security.rs`), deliberately
//! free of any door-keeper's types (`ADR-0008 D2`). The decision itself is made by the
//! door-keeper and reached over the wire the way every other organ is reached: a
//! configured endpoint plus an adapter. Anaphase already carries `flowmodus_endpoint`
//! / `judge_endpoint` and health-checks them (`src/health.rs`), so the peer side is an
//! endpoint on that existing serve face — not a new surface in another organ.
//!
//! # Fail-closed, and why every error path is spelled out
//!
//! P6: an optional must default to **strict**, not to absent. A gate that cannot be
//! reached has not approved anything, so **every** failure below returns
//! `HitlRequired` and **never** `Pass`:
//!
//! | failure | verdict |
//! |---|---|
//! | transport error / timeout | `HitlRequired` |
//! | non-2xx status | `HitlRequired` |
//! | body is not the expected JSON | `HitlRequired` |
//! | decision string not recognised | `HitlRequired` |
//! | the check cannot even be serialised | `HitlRequired` |
//!
//! Returning `Pass` on any of these is the same defect H5 fixed for `soft_reflex`:
//! an unavailable check becomes indistinguishable from a passed one. The message names
//! its direction on purpose — a fail-open and a fail-closed branch that both log at
//! `warn!` have the same loudness and opposite meanings, so loudness alone cannot tell
//! a reader which one fired.
//!
//! # Shape: one untestable call, everything else tested
//!
//! The single thing a unit test cannot do for itself is put bytes on a socket, so that
//! and only that sits behind [`GateTransport`]. `check` — serialise, send, map — is then
//! covered end to end with a fake transport, including the success paths, and the
//! uncovered surface is one branch-free `reqwest` call rather than the whole adapter.
//! The mapping itself is pure (`verdict_for` / `unreachable_verdict`).
//!
//! **Open item (2026-09-20, unresolved).** A loopback mock server inside `cargo test`
//! did not work here: the adapter's request timed out against a hand-rolled HTTP/1.1
//! responder on both a tokio task and a blocking `std::thread`, while a separate
//! diagnostic proved `reqwest` itself reaches a tokio listener fine — so the fault is in
//! the mock, not the client. Rather than keep tests that pass for the wrong reason, the
//! transport is exercised for real only in its **failure** direction
//! (`an_unreachable_gate_does_not_permit`); the success direction goes through the fake.
//! Restoring a real success-direction socket test is tracked in the ledger.

use crate::security::{GateCheck, GateVerdict, SecurityGate};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

/// One gate decision must not outlive the cycle it guards.
pub const DEFAULT_GATE_TIMEOUT: Duration = Duration::from_millis(800);

/// What the door-keeper returns. `reason` is carried for the two blocking decisions.
#[derive(Debug, Deserialize)]
struct GateResponse {
    decision: String,
    #[serde(default)]
    reason: String,
}

/// A gate that could not be consulted has **not** approved anything.
fn unreachable_verdict(why: &str) -> GateVerdict {
    GateVerdict::HitlRequired(format!("security gate unreachable: {why}"))
}

/// Flatten an error and its causes. A bare `reqwest` message ("error sending request
/// for url …") names neither the timeout nor the refused connection, and a blocking
/// decision that cannot say *why* it blocked is not actionable.
fn describe(e: &(dyn std::error::Error + 'static)) -> String {
    let mut out = e.to_string();
    let mut cur = e.source();
    while let Some(s) = cur {
        out.push_str(" <- ");
        out.push_str(&s.to_string());
        cur = s.source();
    }
    out
}

/// Map an already-read response onto a verdict. Pure.
fn verdict_for(status: u16, body: &str) -> GateVerdict {
    if !(200..300).contains(&status) {
        return GateVerdict::HitlRequired(format!("security gate returned HTTP {status}"));
    }
    match serde_json::from_str::<GateResponse>(body) {
        Ok(r) => match r.decision.as_str() {
            "pass" => GateVerdict::Pass,
            "reject" => GateVerdict::Reject(r.reason),
            "hitl_required" => GateVerdict::HitlRequired(r.reason),
            // The mirror of the door-keeper's own emergency pass. It reaches the same
            // execution path as `Pass`; the audit evidence lives on the far side of the
            // contract (`SecurityGateResponse.audit_entry_id`), not here.
            "hard_override" => GateVerdict::HardOverride,
            other => GateVerdict::HitlRequired(format!(
                "security gate returned an unknown decision {other:?}"
            )),
        },
        Err(e) => GateVerdict::HitlRequired(format!("security gate response is not parseable: {e}")),
    }
}

/// The one thing a unit test cannot do for itself: put bytes on a socket.
#[async_trait]
pub trait GateTransport: Send + Sync + std::fmt::Debug {
    /// Returns `(status, body)`, or a transport failure already flattened to text.
    async fn post_json(&self, url: &str, body: String) -> Result<(u16, String), String>;
}

#[derive(Debug)]
struct ReqwestTransport(reqwest::Client);

#[async_trait]
impl GateTransport for ReqwestTransport {
    async fn post_json(&self, url: &str, body: String) -> Result<(u16, String), String> {
        let resp = self
            .0
            .post(url)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| describe(&e))?;
        let status = resp.status().as_u16();
        let text = resp.text().await.map_err(|e| describe(&e))?;
        Ok((status, text))
    }
}

/// Asks a door-keeper over HTTP. The URL is the **full** decision endpoint and comes
/// from configuration — the path is a deployment fact, not a constant in this file
/// (DNA principle 11: 0 hardcoding).
#[derive(Debug)]
pub struct HttpSecurityGate {
    url: String,
    transport: Box<dyn GateTransport>,
}

impl HttpSecurityGate {
    pub fn new(url: &str, timeout: Duration) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("reqwest client build");
        Self {
            url: url.to_string(),
            transport: Box::new(ReqwestTransport(client)),
        }
    }

    /// Test seam: the same `check` with the socket replaced.
    #[cfg(test)]
    fn with_transport(url: &str, transport: Box<dyn GateTransport>) -> Self {
        Self {
            url: url.to_string(),
            transport,
        }
    }
}

#[async_trait]
impl SecurityGate for HttpSecurityGate {
    async fn check(&self, check: &GateCheck) -> GateVerdict {
        let body = match serde_json::to_string(check) {
            Ok(b) => b,
            // Fail-closed even here: a check that cannot be put on the wire has not been
            // approved, and serialisation is the one failure that never reaches the
            // transport's own error path.
            Err(e) => return unreachable_verdict(&format!("cannot serialise the check: {e}")),
        };
        match self.transport.post_json(&self.url, body).await {
            Ok((status, body)) => verdict_for(status, &body),
            Err(e) => unreachable_verdict(&e),
        }
    }
}

#[cfg(test)]
#[path = "security_gate_tests.rs"]
mod security_gate_tests;
