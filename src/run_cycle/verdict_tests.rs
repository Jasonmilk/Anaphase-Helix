//! Tests for period verdicts (B22).
//!
//! Split out of `verdict.rs` rather than left inline: the line budget counts
//! production files, and an inline `#[cfg(test)]` module inside a production
//! file is budgeted as production. The checker excludes `*_tests.rs` (spec §3),
//! which is the same reason `session_events` keeps `tests.rs` beside its code.
//! Nothing about the tests changed in the move except the imports.

use super::verdict::{period_body, PeriodVerdict};
use super::CycleOutcome;

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(done: bool, success: bool, impasse: bool) -> CycleOutcome {
        CycleOutcome { done, success, impasse }
    }

    /// The invariant the loop relies on, asserted rather than assumed.
    #[test]
    fn an_incomplete_period_is_always_an_impasse() {
        for done in [true, false] {
            for success in [true, false] {
                for impasse in [true, false] {
                    let v = PeriodVerdict::from_outcome(&outcome(done, success, impasse));
                    if !v.done() {
                        assert!(
                            v.reason().is_impasse(),
                            "!done must name an impasse, got {:?} for done={done} success={success} impasse={impasse}",
                            v.reason()
                        );
                    }
                }
            }
        }
    }

    /// M-a on the property itself: flip `done` off a success and the claim must
    /// die with it. If this stays green, `claims_success` is not reading `done`.
    #[test]
    fn flipping_done_off_kills_the_success_claim() {
        let completed = PeriodVerdict::from_outcome(&outcome(true, true, false));
        assert!(completed.claims_success());
        let incomplete = PeriodVerdict::from_outcome(&outcome(false, true, false));
        assert!(
            !incomplete.claims_success(),
            "a period that did not complete claimed success"
        );
        assert!(!incomplete.success(), "the raw field must be masked too");
    }

    /// M-b: the invariant must be imposed by the constructor, not inherited from
    /// a well-behaved caller. `CycleOutcome` has public fields, so this
    /// combination is constructible from outside the crate even though the loop
    /// never produces it — and it is the combination that hands a transport a
    /// success claim for a period that did not complete.
    #[test]
    fn a_hand_built_incomplete_success_cannot_reach_a_channel() {
        for impasse in [true, false] {
            let hostile = outcome(false, true, impasse);
            let v = PeriodVerdict::from_outcome(&hostile);
            assert!(
                !v.claims_success(),
                "from_outcome copied success={} through done={}",
                hostile.success,
                hostile.done
            );
            assert_ne!(period_body(&v, "r", "j", None)["success"], true);
        }
    }

    /// Every reachable verdict, so the body test below cannot pass by sampling.
    fn every_reachable_verdict() -> Vec<PeriodVerdict> {
        let mut v = Vec::new();
        for done in [true, false] {
            for success in [true, false] {
                for impasse in [true, false] {
                    v.push(PeriodVerdict::from_outcome(&outcome(done, success, impasse)));
                }
            }
        }
        v.push(PeriodVerdict::cap_exhausted());
        v
    }

    #[test]
    fn no_channel_body_reports_an_incomplete_period_as_a_success() {
        for v in every_reachable_verdict() {
            let body = period_body(&v, "reply", "job", None);
            assert_eq!(body["done"], v.done());
            assert_eq!(body["success"], v.success());
            assert_eq!(body["impasse"], v.impasse());
            assert_eq!(body["reason"], v.reason_str());
            if !v.done() {
                assert_ne!(
                    body["success"], true,
                    "body claimed success for an incomplete period: {body}"
                );
                assert_ne!(body["reason"], "completed");
            }
        }
    }

    /// The reason token is what a client matches on, so it must be present and
    /// must not be the completed token whenever anything is off.
    #[test]
    fn the_reason_token_tracks_the_state() {
        for v in every_reachable_verdict() {
            if v.is_reportable() {
                assert_ne!(body_reason(&v), "completed");
            } else {
                assert_eq!(body_reason(&v), "completed");
            }
        }
    }

    fn body_reason(v: &PeriodVerdict) -> String {
        period_body(v, "", "", None)["reason"]
            .as_str()
            .unwrap()
            .to_string()
    }
}
