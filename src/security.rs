//! Security gate wiring point (ADR-0008, candidate D'-2).
//!
//! Anaphase defines its own gate contract here and stays decoupled from any
//! concrete security implementation (Tuck included). A gate adapter lives in
//! the deployment/test layer — mirroring Tuck's own rule that transport is
//! handled by an adapter outside the core.
//!
//! Three-gate model (per Tuck P6-T3 / Anaphase hitl.rs):
//!   1. tool audit (registry gate)
//!   2. HITL (execution gate)
//!   3. Tuck (edge physical gate)  <-- this module's wiring point
//!
//! `None` gate = legacy behavior; the pipeline runs exactly as before
//! (110-test baseline untouched). Pass / HardOverride proceed;
//! Reject / HitlRequired block the call and write a `blocked` ledger record.

use async_trait::async_trait;
use std::fmt;

/// One gate check for one tool call.
///
/// Carries only facts; the gate implementation decides policy. `job_id` +
/// `index` reproduce the deterministic trace id `{job_id}#{index}`
/// (ADR-0003) so gate decisions are replayable per call.
#[derive(Debug, Clone)]
pub struct GateCheck {
    pub job_id: String,
    pub index: u32,
    pub tool: String,
    pub args_json: String,
    /// Caller-identity labels forwarded for audit (ADR-0004). Full facts —
    /// the gate implementation decides which label it needs.
    pub identity_labels: std::collections::BTreeMap<String, String>,
}

/// Decision returned by a security gate.
#[derive(Debug, Clone, PartialEq)]
pub enum GateVerdict {
    /// Proceed with execution.
    Pass,
    /// Block execution (policy rejection). Carries the reason.
    Reject(String),
    /// Block execution; escalate to the HITL gate. Carries the reason.
    HitlRequired(String),
    /// Emergency pass (audited). Proceeds on the same execution path as Pass.
    HardOverride,
}

impl GateVerdict {
    /// Whether the call may proceed.
    pub fn permits(&self) -> bool {
        matches!(self, Self::Pass | Self::HardOverride)
    }
}

/// Security gate contract — implemented by the deployment/test layer.
#[async_trait]
pub trait SecurityGate: Send + Sync {
    /// Check one call before execution.
    async fn check(&self, check: &GateCheck) -> GateVerdict;
}

impl fmt::Debug for dyn SecurityGate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecurityGate(_)")
    }
}

/// A gate that permits everything.
///
/// Used by callers who want the wiring exercised without policy
/// (e.g. legacy behavior expressed explicitly). The pipeline's `None`
/// option remains the zero-cost default.
#[derive(Debug, Clone, Default)]
pub struct PermissiveGate;

#[async_trait]
impl SecurityGate for PermissiveGate {
    async fn check(&self, _check: &GateCheck) -> GateVerdict {
        GateVerdict::Pass
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn permissive_gate_permits() {
        let gate = PermissiveGate;
        let check = GateCheck {
            job_id: "job-1".to_string(),
            index: 0,
            tool: "numbers".to_string(),
            args_json: "{}".to_string(),
            identity_labels: std::collections::BTreeMap::new(),
        };
        assert_eq!(gate.check(&check).await, GateVerdict::Pass);
    }

    #[test]
    fn verdict_permission_semantics() {
        assert!(GateVerdict::Pass.permits());
        assert!(GateVerdict::HardOverride.permits());
        assert!(!GateVerdict::Reject("no".into()).permits());
        assert!(!GateVerdict::HitlRequired("ask".into()).permits());
    }
}
