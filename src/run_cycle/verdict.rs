//! How a period ended, in a form every channel can report (B22).
//!
//! The loop already computed `done` / `success` / `impasse`, but each consumer
//! re-derived what to *say* about them: the HTTP handler reported `done` and
//! dropped the other two, the CI-144 action mapped `!done` to a failure, and the
//! CLI printed a sentence. Three transports, three judgements.
//!
//! The consequence was not a wrong word in one log line. An impasse the model
//! itself declared (`{"impasse": true}`) reached the ring buffer — which only
//! the panel reads — and, when `session_events_dir` was unset, went nowhere
//! else. `ReflexCheck`'s silent `Impass` return says the same thing from the
//! other side: the path produced no log at all.
//!
//! So one period gets one verdict. Transports may differ in how they carry it;
//! they do not get to differ in what they concluded.

use super::CycleOutcome;

/// Why a period ended, as a stable token.
///
/// Stability is the point: this is the machine-readable end of the report, so
/// a consumer may match on it. Renaming a variant is a contract change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    /// The machine returned to Perception.
    Completed,
    /// The model declared it could not proceed (`impasse: true`), or the
    /// reflection criterion came back Unmet.
    Impasse,
    /// No transition rule existed for the (state, condition) reached.
    UndefinedTransition,
    /// The caller's own budget ran out before the machine returned. Not
    /// knowable by the engine: ADR-0016 D1 keeps looping policy in the caller,
    /// so the caller composes this onto the last period's verdict.
    CycleCapExhausted,
}

impl EndReason {
    /// The wire token.
    pub fn as_str(self) -> &'static str {
        match self {
            EndReason::Completed => "completed",
            EndReason::Impasse => "impasse",
            EndReason::UndefinedTransition => "undefined-transition",
            EndReason::CycleCapExhausted => "cycle-cap-exhausted",
        }
    }

    /// Whether an impasse is the honest word for it.
    pub fn is_impasse(self) -> bool {
        matches!(self, EndReason::Impasse | EndReason::UndefinedTransition)
    }
}

/// One period, one verdict.
///
/// The fields are private and the two constructors are the only ways in, both
/// of which enforce the invariant below. That is deliberate: `CycleOutcome` has
/// public fields, so a caller can build `done: false, success: true`, and a
/// verdict type that copied that through would hand the contradiction to every
/// transport. The invariant is not re-derived per consumer — it is imposed here,
/// once, where it cannot be forgotten.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeriodVerdict {
    done: bool,
    success: bool,
    impasse: bool,
    reason: EndReason,
}

impl PeriodVerdict {
    /// The verdict for a period the engine finished on its own.
    ///
    /// `!done` implies an undefined transition *by construction* — the loop
    /// assigns `done = !undefined_transition` and folds `undefined_transition`
    /// into `impasse`. `!done && !impasse` is therefore representable in
    /// `CycleOutcome` but unreachable; it is reported as an undefined
    /// transition rather than silently called a completion.
    ///
    /// `success` is masked by `done` here rather than trusted. The loop already
    /// forces `success = false` for an incomplete period (see `run_cycle`), so
    /// this is redundant for that caller — and load-bearing for any other, which
    /// is the point: the guarantee should not depend on every caller having
    /// remembered.
    pub fn from_outcome(o: &CycleOutcome) -> Self {
        let reason = if !o.done {
            EndReason::UndefinedTransition
        } else if o.impasse {
            EndReason::Impasse
        } else {
            EndReason::Completed
        };
        Self {
            done: o.done,
            success: o.done && o.success,
            impasse: o.impasse,
            reason,
        }
    }

    /// The caller gave up before the engine returned.
    pub fn cap_exhausted() -> Self {
        Self {
            done: false,
            success: false,
            impasse: false,
            reason: EndReason::CycleCapExhausted,
        }
    }

    pub fn done(&self) -> bool {
        self.done
    }

    /// Whether the period both completed and satisfied its criteria.
    pub fn success(&self) -> bool {
        self.success
    }

    /// Whether an impasse was declared or fallen into during the period.
    pub fn impasse(&self) -> bool {
        self.impasse
    }

    pub fn reason(&self) -> EndReason {
        self.reason
    }

    /// **The property every channel is held to**: a period that did not
    /// complete is never reported as one that did.
    ///
    /// Stated once, here, instead of being re-derived by each consumer. The
    /// callers were fixed to agree with it; this makes the agreement
    /// structural, so a fourth caller cannot quietly disagree.
    pub fn claims_success(&self) -> bool {
        self.done && self.success
    }

    /// Whether this verdict is worth telling someone about without being asked.
    ///
    /// True for anything that is not a plain completion. The loop logs on this
    /// rather than on the panel's liveness: the ring buffer is read only when a
    /// panel is open, so an impasse reported there alone is an impasse reported
    /// to nobody.
    pub fn is_reportable(&self) -> bool {
        !(self.done && !self.impasse)
    }

    /// The wire token for why it ended.
    pub fn reason_str(&self) -> &'static str {
        self.reason.as_str()
    }
}

/// The body every HTTP-shaped channel returns for a finished period.
///
/// Lives here rather than in the handler so the property above is *testable*
/// instead of merely intended. Pure: no clock, no transport, no status code —
/// choosing a status stays the caller's business (see the note on `reason`:
/// every field a client needs to avoid mistaking this for a success is in the
/// body, because the status is not currently a reliable signal).
pub fn period_body(
    v: &PeriodVerdict,
    reply: &str,
    job_id: &str,
    model: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "reply": reply,
        "done": v.done,
        "success": v.success,
        "impasse": v.impasse,
        "reason": v.reason.as_str(),
        "job_id": job_id,
        "model": model,
    })
}
/// The verdict as one JSON object — the detail string for the cycle event.
///
/// Kept as `Display` rather than a method so the shape can be used anywhere a
/// `{}` is accepted, and so the pre-existing event detail does not change shape
/// underneath consumers that already parse it: the three original keys stay,
/// `reason` is added.
impl std::fmt::Display for PeriodVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{\"done\":{},\"success\":{},\"impasse\":{},\"reason\":\"{}\"}}",
            self.done,
            self.success,
            self.impasse,
            self.reason.as_str()
        )
    }
}
