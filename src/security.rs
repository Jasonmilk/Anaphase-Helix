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
//!
//! # ⚠️ MEASURED 2026-09-20: no gate is installed anywhere on the production path
//!
//! `with_security_gate` is called **only from `tests/`**; `PipelineConfig` defaults the
//! field to `None`; `src/main.rs` never calls the setter; and `tuck-core` is a
//! **dev-only** dependency. So in production **every tool call executes ungated**.
//!
//! "`None` = legacy behavior" is accurate but reads as benign. It means **there is no
//! door**, not "the old door still works" — a distinction this project keeps having to
//! relearn (a declaration standing in for a check).
//!
//! **Do not paper over this with `PermissiveGate`.** It permits everything, so the
//! pipeline would *look* gated while nothing is checked — worse than `None`, because the
//! absence would stop being visible.
//!
//! Installing a real gate needs a door-keeper surface. Measured: Tuck serves the gate as
//! a **library API** (`TuckSecurityGate::process` → `SecurityGateResponse`), and its
//! gateway exposes **no route** for a gate decision (`tuck-gateway` routes
//! `/v1/chat/completions` only). So an adapter needs either a Tuck-side endpoint or a
//! deployment-layer binary that links both — cross-organ work, tracked in the ledger.
//!
//! # ✅ UPDATED 2026-10-09: the door-keeper surface now exists (Tuck side)
//!
//! The paragraph above is kept as written (it was true on 2026-09-20, and a correction
//! here is a **new record, not a rewrite** — the same rule this project applies to
//! ledgers). What changed: **Tuck now serves the gate over HTTP.**
//!
//! * `POST /v1/security/gate` (Tuck `crates/tuck-gateway/src/lib.rs`, K16 M1a) —
//!   request/response shapes chosen to **mirror this module's contract exactly**
//!   (`GateCheck` / `GateResponse{decision, reason}`), so no mapping table is needed.
//! * It **obeys the rule above**: on the Tuck side the handler reports **`gate=none`
//!   when no admission table is installed** (Tuck's I7, `pipeline/mod.rs:171`). It does
//!   **not** paper over absence — so it is the opposite of the `PermissiveGate` failure
//!   mode this comment warns about.
//! * It lands **one audit row per verdict** on the Tuck side (K16 M1b-2b), using the
//!   same vocabulary it returns over HTTP (one name, one thing).
//!
//! ⇒ So the remaining work is **no longer cross-organ**: it is this repo's side only —
//! an adapter that POSTs `GateCheck` to that route, plus the wiring in `main.rs`
//! (**still observe mode first**: install the door, change no behavior, and only then
//! decide about flipping it to enforcing). Tracked as K16 M2.

use async_trait::async_trait;
use std::fmt;

/// One gate check for one tool call.
///
/// Carries only facts; the gate implementation decides policy. `job_id` +
/// `index` reproduce the deterministic trace id `{job_id}#{index}`
/// (ADR-0003) so gate decisions are replayable per call.
#[derive(Debug, Clone, serde::Serialize)]
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
    /// Emergency pass. Proceeds on the same execution path as Pass.
    ///
    /// **This line used to say "(audited)". That was a self-certification with no check
    /// behind it** (corrected 2026-09-20). Anaphase produces no `HardOverride` at all —
    /// this variant is the mirror of the door-keeper's `GateDecision::HardOverride`, and
    /// the audit evidence lives on the far side of that contract
    /// (`SecurityGateResponse.audit_entry_id`). A label is an index; the criterion is the
    /// gate. With no gate installed, nothing here is audited by anything.
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

/* THE DECLARATION WAS DELETED (ADR-0048 §212): the string it returned said "every tool call
 * executes UNGATED", and that is FALSE — measured, pipeline/mod.rs:198-212 blocks high-risk
 * calls when the gate is None ("absence is not approval (B7)") and writes a `blocked` ledger
 * record. Worse, the absence was ALREADY declared PER CALL by `gate_presence()`
 * (pipeline/mod.rs:173, I7 §13.3), so this was a THIRD host for one fact — and it had no
 * consumer. A false statement with zero consumers is the worst combination available:
 * 2 true states collapsed into 1 observation (H = 0.8595, I = 0.0000 bits).
 * The treatment had become the case. */

#[cfg(test)]
mod tests {
    #[test]
    fn hard_override_is_not_folded_into_pass_at_the_verdict_level() {
        /* §211 measured: `permits()` maps 4 verdicts to 2 values (I = 1.0000 of H = 2.0000, 50%
         * lost), so an EMERGENCY override is indistinguishable from an ordinary pass. The outlet
         * must exist at least as a label, so a caller can write it to the ledger. */
        assert!(GateVerdict::Pass.permits() && GateVerdict::HardOverride.permits());
        assert_ne!(format!("{:?}", GateVerdict::Pass), format!("{:?}", GateVerdict::HardOverride),
            "the two permitting verdicts must be distinguishable by name");
    }

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
