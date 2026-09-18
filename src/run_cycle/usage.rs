//! Usage metering — the `assistant/usage` row, and nothing else.
//!
//! Its own module rather than part of a "safety" file: this is accounting, and a
//! file named for security that contains the metering code is a file whose name
//! lies. Naming was the point of splitting run_cycle at all (ADR-0042).
//!
//! It is a free function rather than an `impl AgentLoop` block because the loop's
//! fields are private to its module, and a child module cannot write an inherent
//! impl for it. Taking the loop by reference keeps the fields private and keeps
//! the move purely mechanical.

use super::AgentLoop;

/// Append one `assistant/usage` row for the round trip the adapter just finished
/// (ADR-0038). Called per call, never once per period: a retried or finalized
/// call was billed too, and dropping it would silently lose a physical fact.
///
/// Raw wire facts only, and nothing is written when the upstream reported no
/// usage — an absent field stays absent rather than becoming a fabricated zero.
pub(super) fn emit_usage(agent: &mut AgentLoop) {
    let meta = agent.reason.last_meta();
    let Some(usage) = meta.usage else { return };
    let Some(ev) = agent.session_events.as_mut() else { return };
    let ts = crate::ledger::unix_secs_to_rfc3339(agent.clock.now());
    let _ = ev.emit(
            &ts,
            crate::session_events::EventType::Usage,
            serde_json::json!({
                "model": meta.model,
                "prompt_tokens": usage.prompt_tokens,
                "completion_tokens": usage.completion_tokens,
                "cached_tokens": usage.cached_tokens,
                "reasoning_tokens": usage.reasoning_tokens,
            }),
        );
}
