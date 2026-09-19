/// Fear adapter trait reference (import required for soft reflex)
use crate::adapters::FearAdapter;

/// The static deny-list (H2b, ruled 2026-09-18).
///
/// It lives here, with a name, rather than inline in a constructor at the far end of
/// `main.rs`: a static rule is part of the reflex's contract, and "static" should not
/// mean "buried in the largest file in the repository".
///
/// **It is not runtime-loadable, and the comment that said otherwise is gone.** The
/// field used to carry a note claiming the constraints were read out of memory (L2), and
/// there was no such loader anywhere — no config field, no reader, no writer. The only
/// assignment was a literal in `main.rs`. A declaration that contradicts its
/// implementation is worse than no declaration: the next reader plans against the
/// comment.
///
/// The old wording is described rather than quoted so that the acceptance check for this
/// change — a plain grep for it, which must return nothing — is not tripped by the
/// explanation of why it was removed. A criterion is worth keeping that simple, and the
/// fact survives without the verbatim string.
///
/// **Why it must stay static** (the H2b ruling): if these rules were updatable from L2,
/// then memory that has been poisoned could disarm this deny-list — the one single point
/// that can bypass every other measure (K-033).
pub const DEFAULT_SAFETY_RULES: &[&str] = &["rm -rf /", "shutdown"];

/// Somatic reflex arc for immune system
/// Handles hard (fast) and soft (predictive) safety checks
pub struct ReflexArc {
    pub safety_rules: Vec<String>,
}

impl ReflexArc {
    /// A reflex arc carrying the static deny-list.
    ///
    /// A constructor rather than a struct literal at the call site: it names the intent
    /// once, and it is what lets `main.rs` lose lines instead of gaining them. The first
    /// version of this move expanded a one-line literal into a four-line
    /// `iter().map().collect()` at the constructor, which made the largest file in the
    /// repository three lines larger while claiming to extract from it.
    pub fn with_default_rules() -> Self {
        Self {
            safety_rules: DEFAULT_SAFETY_RULES.iter().map(|r| r.to_string()).collect(),
        }
    }

    /// Hard reflex: O(1) hash matching for forbidden actions
    /// Returns true if action is SAFE, false if BLOCKED
    pub fn hard_reflex(&self, action: &str) -> bool {
        !self.safety_rules.iter().any(|rule| action.contains(rule))
    }

    /// Soft reflex: Calls fear prediction model
    /// Returns p_death (0.0 = no risk, 1.0 = critical risk)
    pub async fn soft_reflex(&self, fear: &dyn FearAdapter, context: &str) -> Result<f64, String> {
        // Call fear prediction and return death probability
        fear.predict_death(context).await
    }
}
