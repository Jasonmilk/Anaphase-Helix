//! The safety gate: HITL approval and the safety audit, in one place.
//!
//! **Pure move (ADR-0042).** Both call sites lived in the `Execution` arm — one
//! in the legacy string path, one in `execute_structured` — about 350 lines
//! apart. Nothing here behaves differently from what it replaced.
//!
//! They are together now because they **disagree**, and the disagreement was
//! invisible while they were apart. On an audit `Err` the legacy path reports
//! the period as SUCCESSFUL, and the structured path reports it as FAILED. What
//! routes between those two answers is, in `mod.rs`:
//!
//! ```text
//! if self.pipeline.is_some() && !self.context.calls.is_empty() { … structured … }
//! ```
//!
//! So the system's safety direction is decided by **whether a pipeline happens
//! to be configured**, not by any safety judgement. That is K-033, and it is the
//! third layer of a family: B14′ folds a construction failure and an absent
//! `security_gate` into the same `None`; K-025 has two error paths that answer
//! oppositely; K-033 makes the answer depend on a config item —
//! `tuck_endpoint: None` by default (`config.rs:310`).
//!
//! ⇒ **Recorded here, not resolved.** The direction the two paths should share
//! is a ruling (H1/H5), and this segment's rule is move-plus-observe.

use super::TransitionCondition;
use crate::adapters::SafetyAdapter;
use crate::hitl::HITLApprover;
use tracing::warn;

/// What to do when the safety audit cannot run.
///
/// Both variants are in use today. That is the defect, and naming them after
/// their effect rather than after "open/closed" is deliberate: the first one
/// does not merely fail open, it *reports success* for an action it refused to
/// run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OnAuditError {
    /// K-033 — the legacy path. Report the period successful without running the
    /// tool. Preserved by this move because the move changes no behaviour; it is
    /// the defect, not an oversight.
    ReportSuccess,
    /// K-033 — the structured path. Report the period failed.
    Block,
}

/// The gate's verdict for one action.
///
/// Not `Copy`: `TransitionCondition` carries `Clone` only, and it is the thing a
/// refusal hands back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum GateVerdict {
    /// Both checks cleared. The caller may run the tool.
    Cleared,
    /// Do not run the tool, and report this transition. Read this together with
    /// the transition it carries: `Refused(Success)` is K-033 made visible.
    Refused(TransitionCondition),
}

/// HITL approval, then the safety audit.
///
/// HITL blocking — including its `Err` — is `Failure` at **both** sites, so it is
/// not a parameter. If that ever diverges, the divergence belongs in the same
/// breath as the audit one rather than hidden in a call argument; the point of
/// this file is that the disagreements are readable side by side.
///
/// Every message here names its direction. A fail-open path and a fail-closed
/// path that both log at `warn!` have the same loudness and opposite meanings,
/// so loudness alone cannot tell a reader which one fired (P13).
pub(super) async fn admit(
    hitl: &HITLApprover,
    safety: &dyn SafetyAdapter,
    tool: &str,
    actions: &[String],
    on_audit_error: OnAuditError,
) -> GateVerdict {
    match hitl.check_approval(tool, actions) {
        Ok(true) => {}
        Ok(false) => {
            warn!("[SafetyGate] HITL rejected {tool:?}: high-risk action blocked");
            return GateVerdict::Refused(TransitionCondition::Failure);
        }
        Err(e) => {
            warn!("[SafetyGate] HITL unavailable for {tool:?}, blocked (fail-closed): {e}");
            return GateVerdict::Refused(TransitionCondition::Failure);
        }
    }
    match safety.audit("execute", tool).await {
        Ok(true) => GateVerdict::Cleared,
        Ok(false) => {
            warn!("[SafetyGate] Safety audit rejected {tool:?}");
            GateVerdict::Refused(TransitionCondition::Failure)
        }
        Err(e) => match on_audit_error {
            // The message says ALLOWING in capitals on purpose: this is the one
            // branch in the file that lets an unaudited action proceed to a
            // success report, and it must not read like its neighbour below.
            OnAuditError::ReportSuccess => {
                warn!(
                    "[SafetyGate] Safety audit unavailable for {tool:?}: ALLOWING and reporting \
                     success without running it (K-033, legacy path): {e}"
                );
                GateVerdict::Refused(TransitionCondition::Success)
            }
            OnAuditError::Block => {
                warn!("[SafetyGate] Safety audit unavailable for {tool:?}, blocked (fail-closed): {e}");
                GateVerdict::Refused(TransitionCondition::Failure)
            }
        },
    }
}
