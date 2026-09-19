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

/// Collapse whitespace runs and lower-case, so a rule cannot be evaded by spacing or case.
///
/// Both the action and the rule go through it, so a rule written with unusual spacing
/// still matches a normally-spaced action.
fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

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

    /// Hard reflex: is this action SAFE (`true`) or forbidden (`false`)?
    ///
    /// **This is a MITIGATION, not a fix.** Substring matching has an unbounded bypass
    /// space — `rm -r''f /`, `rm -rf $(echo /)`, a symlink, a shell builtin, an alias.
    /// What it now does is stop the three trivial spellings that were free:
    ///
    ///     rm  -rf /     extra space            (was SAFE — a bypass)
    ///     rm -rf  /     space before the slash (was SAFE — a bypass)
    ///     RM -RF /      case                   (was SAFE — a bypass, and the cheapest)
    ///
    /// Measured before the change; all three returned SAFE. Normalising whitespace runs
    /// and case closes them. It does NOT close the class, and nothing here should be read
    /// as claiming otherwise.
    ///
    /// **The over-strict side is deliberately unchanged**: `rm -rf /tmp/foo` is still
    /// blocked, because the rule is a prefix and the action contains it. Relaxing that
    /// would be a loosening in the safety direction, which is a different ruling.
    pub fn hard_reflex(&self, action: &str) -> bool {
        let action = normalize(action);
        !self.safety_rules.iter().any(|rule| action.contains(&normalize(rule)))
    }

    /// Soft reflex: Calls fear prediction model
    /// Returns p_death (0.0 = no risk, 1.0 = critical risk)
    pub async fn soft_reflex(&self, fear: &dyn FearAdapter, context: &str) -> Result<f64, String> {
        // Call fear prediction and return death probability
        fear.predict_death(context).await
    }
}

#[cfg(test)]
mod reflex_normalisation_tests {
    use super::*;

    fn arc() -> ReflexArc {
        ReflexArc::with_default_rules()
    }

    // guards: reflex-normalised
    /// **Mutation 1 — the bypasses must be blocked.**
    ///
    /// Before normalisation all three of these returned SAFE. Each is a free evasion:
    /// no tooling, no cryptography, just a keystroke.
    #[test]
    fn spacing_and_case_no_longer_evade_the_deny_list() {
        for action in ["rm -rf /", "rm  -rf /", "rm -rf  /", "RM -RF /", "  rm   -RF   /  "] {
            assert!(
                !arc().hard_reflex(action),
                "{action:?} must be BLOCKED. If this passes, the rule can be evaded by \
                 spacing or case alone, which is how it was before normalisation."
            );
        }
    }

    /// **Mutation 2 — the over-strict side must NOT change.**
    ///
    /// `rm -rf /tmp/foo` contains the rule, so it is blocked today. That is over-strict,
    /// and it is deliberately left alone: relaxing it is a loosening in the safety
    /// direction and needs its own ruling. This assertion exists so nobody "fixes" it
    /// while touching normalisation.
    #[test]
    fn the_over_strict_side_is_unchanged() {
        assert!(
            !arc().hard_reflex("rm -rf /tmp/foo"),
            "the prefix rule still matches a longer path; that over-strictness is known \
             and recorded, not accidentally removed here"
        );
        assert!(
            !arc().hard_reflex("shutdown -h now"),
            "same for the other rule"
        );
    }

    /// And a genuinely safe action must still pass, or the mitigation has become a
    /// second bug: a reflex that blocks everything is not safer, it is broken.
    #[test]
    fn unrelated_actions_still_pass() {
        for action in ["echo hello", "ls -la", "cat README.md"] {
            assert!(arc().hard_reflex(action), "{action:?} must remain SAFE");
        }
    }
}
