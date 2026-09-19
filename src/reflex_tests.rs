//! Tests for the reflex deny-list (H2b static rules, §1c normalisation).
//!
//! Split out of `reflex.rs` rather than left inline: the line counter budgets production
//! files, and an inline `#[cfg(test)]` module inside one is counted as production. The
//! checker excludes `*_tests.rs` — the same reason `verdict_tests.rs` and
//! `governance_tests.rs` exist. Moving them took reflex.rs from 57 NCLOC back to 18.

use super::reflex::ReflexArc;

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
