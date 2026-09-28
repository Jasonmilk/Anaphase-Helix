//! THE JUDGE, NOT THE RESULT (ADR-0048 §210).
//!
//! `run_cycle` must be unable to reach reasoning when the audit/LLM gateway is down. The wrong way
//! to get there is to STORE the last probe's result: a stored result is a cache, a cache expires,
//! and refreshing it needs a background task — one new mechanism, in exchange for one guard.
//! Saltzer & Schroeder (1975, Complete Mediation) name this directly:
//!   "proposals to gain performance by remembering the result of an authority check be examined
//!    skeptically. If a change in authority occurs, such remembered results must be systematically
//!    updated."
//!
//! So what is injected here is a JUDGE that is asked every time (O(1), no IO in the common path),
//! and the memory it keeps points ONE WAY:
//!
//!   remembering GOOD  ⇒ when it goes stale, the guard ALLOWS  ⇒ fail-OPEN   ⇒ dangerous
//!   remembering BAD   ⇒ when it goes stale, the guard REFUSES ⇒ fail-CLOSED ⇒ fail-safe
//!
//! A circuit breaker (Nygard, *Release It!*) has exactly that shape: `Closed` passes and counts
//! failures from the REAL call (no extra probe — a per-request probe is what measured 53.51 req/s
//! against 160 normal); `Open` refuses in O(1) **without touching the downstream**; `HalfOpen`
//! lets a small probe through after a cooldown, which is the only place a probe happens.
//!
//! This module is deliberately NOT wired into `run_cycle` yet: wiring any subset of
//! {sink, failure-feeding, refusal-record} produces either the 10-second cliff (§208) or a guard
//! with zero consumers (§207). The hookup is one atomic change and is declared as the next step.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The cooldown the breaker waits before allowing a single half-open probe. Declared once, with its
/// reason: `health::tcp_reachable` budgets 10s for one connect, so a shorter cooldown would re-probe
/// a dependency that has not had time to answer the previous probe.
pub const GATE_COOLDOWN: std::time::Duration = std::time::Duration::from_secs(10);

/// Where the guard stands. `Unconfigured` is the DEFAULT and means "ungoverned": it passes, and
/// `governance::warning` announces that at startup — a deliberate policy (`health.rs:155-160`),
/// not an oversight, pinned by `gate_unconfigured_passes`.
#[derive(Debug, Clone)]
pub enum Gate {
    Unconfigured,
    Breaker(std::sync::Arc<TuckBreaker>),
}

impl Default for Gate {
    fn default() -> Self {
        Gate::Unconfigured
    }
}

impl Gate {
    /// Build the judge from CONFIG (ADR-0048 §233). Two-sided by construction and by test:
    /// a configured endpoint yields a breaker; an ABSENT or EMPTY endpoint yields `Unconfigured`,
    /// which is the announced-ungoverned policy — not a silent pass and not an accidental breaker.
    pub fn from_config(tuck_endpoint: Option<&str>, cooldown: std::time::Duration) -> Gate {
        match tuck_endpoint.map(str::trim).filter(|s| !s.is_empty()) {
            Some(ep) => Gate::Breaker(std::sync::Arc::new(TuckBreaker::new(ep, cooldown))),
            None => Gate::Unconfigured,
        }
    }

    /// Ask the judge. O(1) and IO-free unless the breaker is `HalfOpen` (the one place a probe is
    /// allowed to happen) or `Closed` **and** the caller asked for a verification probe.
    pub fn check(&self) -> Result<(), String> {
        match self {
            Gate::Unconfigured => Ok(()),
            Gate::Breaker(b) => b.check(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateState {
    Closed,
    Open,
    HalfOpen,
}

impl std::fmt::Display for GateState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            GateState::Closed => "closed",
            GateState::Open => "open",
            GateState::HalfOpen => "half-open",
        })
    }
}

#[derive(Debug)]
struct Inner {
    state: GateState,
    opened_at: Option<Instant>,
    last_reason: Option<String>,
}

/// A three-state breaker whose only remembered fact is a FAILURE.
pub struct TuckBreaker {
    endpoint: String,
    cooldown: Duration,
    inner: Mutex<Inner>,
    /// Injectable prober: the real one is `health::tcp_reachable`; tests pass a counter so the
    /// O(1) property ("Open touches nothing") is measured rather than asserted by prose.
    prober: Box<dyn Fn(&str) -> Result<(), String> + Send + Sync>,
}

impl std::fmt::Debug for TuckBreaker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TuckBreaker")
            .field("endpoint", &self.endpoint)
            .field("cooldown", &self.cooldown)
            .finish()
    }
}

impl TuckBreaker {
    pub fn new(endpoint: impl Into<String>, cooldown: Duration) -> Self {
        Self {
            endpoint: endpoint.into(),
            cooldown,
            inner: Mutex::new(Inner { state: GateState::Closed, opened_at: None, last_reason: None }),
            prober: Box::new(crate::health::tcp_reachable),
        }
    }

    /// A breaker with an injectable prober (tests, and any caller that owns its own health check).
    pub fn with_prober(
        endpoint: impl Into<String>,
        cooldown: Duration,
        prober: Box<dyn Fn(&str) -> Result<(), String> + Send + Sync>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            cooldown,
            inner: Mutex::new(Inner { state: GateState::Closed, opened_at: None, last_reason: None }),
            prober,
        }
    }

    pub fn state(&self) -> GateState {
        self.inner.lock().map(|g| g.state).unwrap_or(GateState::Open)
    }

    pub fn last_reason(&self) -> Option<String> {
        self.inner.lock().ok().and_then(|g| g.last_reason.clone())
    }

    /// `Closed` ⇒ pass (failures arrive through `record_failure`, from the REAL call).
    /// `Open`   ⇒ refuse in O(1) if the cooldown has not elapsed; otherwise go `HalfOpen`.
    /// `HalfOpen` ⇒ probe once: success closes it, failure re-opens it.
    pub fn check(&self) -> Result<(), String> {
        let state = {
            let mut g = match self.inner.lock() {
                Ok(g) => g,
                /* A poisoned lock means a holder panicked; refusing is the fail-safe direction. */
                Err(_) => return Err("gate: TUCK-GATE-REFUSED (breaker lock poisoned)".to_string()),
            };
            if g.state == GateState::Open {
                match g.opened_at {
                    Some(t) if t.elapsed() >= self.cooldown => {
                        g.state = GateState::HalfOpen;
                        GateState::HalfOpen
                    }
                    _ => GateState::Open,
                }
            } else {
                g.state
            }
        };
        match state {
            GateState::Closed => Ok(()),
            GateState::Open => Err(format!(
                "TUCK-GATE-REFUSED: breaker open ({}) since last failure: {}",
                self.endpoint,
                self.last_reason().unwrap_or_else(|| "unreachable".to_string())
            )),
            GateState::HalfOpen => match (self.prober)(&self.endpoint) {
                Ok(()) => {
                    self.record_success();
                    Ok(())
                }
                Err(e) => {
                    self.record_failure(&e);
                    Err(format!("TUCK-GATE-REFUSED: half-open probe failed: {e}"))
                }
            },
        }
    }

    /// A failure of the REAL call (not of a probe) opens the breaker.
    pub fn record_failure(&self, reason: &str) {
        if let Ok(mut g) = self.inner.lock() {
            g.state = GateState::Open;
            g.opened_at = Some(Instant::now());
            g.last_reason = Some(reason.to_string());
        }
    }

    pub fn record_success(&self) {
        if let Ok(mut g) = self.inner.lock() {
            g.state = GateState::Closed;
            g.opened_at = None;
            g.last_reason = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn counting(calls: Arc<AtomicUsize>) -> Box<dyn Fn(&str) -> Result<(), String> + Send + Sync> {
        Box::new(move |_ep: &str| {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }

    #[test]
    fn gate_from_config_is_two_sided() {
        use std::time::Duration;
        let none = Gate::from_config(None, Duration::from_secs(5));
        assert!(matches!(none, Gate::Unconfigured), "absent endpoint ⇒ announced-ungoverned");
        let empty = Gate::from_config(Some("   "), Duration::from_secs(5));
        assert!(matches!(empty, Gate::Unconfigured), "blank endpoint is NOT a configured one");
        let some = Gate::from_config(Some("http://127.0.0.1:60052"), Duration::from_secs(5));
        assert!(matches!(some, Gate::Breaker(_)), "a configured endpoint yields a breaker");
        /* And the breaker starts CLOSED, so the first ask does not consult anything: the cost of
         * asking is paid only where it is needed (§210). */
        if let Gate::Breaker(b) = &some { assert_eq!(b.state(), GateState::Closed); }
    }

    #[test]
    fn unconfigured_passes_and_is_the_default() {
        assert!(Gate::default().check().is_ok());
        assert!(Gate::Unconfigured.check().is_ok());
    }

    #[test]
    fn closed_does_not_probe() {
        /* Closed must be IO-free: a per-request probe is what measured 33.4% of normal throughput. */
        let calls = Arc::new(AtomicUsize::new(0));
        let b = TuckBreaker::with_prober("tuck:1", Duration::from_secs(5), counting(calls.clone()));
        assert!(b.check().is_ok());
        assert!(b.check().is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 0, "Closed must not touch the downstream");
    }

    #[test]
    fn open_refuses_in_o1_without_touching_downstream() {
        let calls = Arc::new(AtomicUsize::new(0));
        let b = Arc::new(TuckBreaker::with_prober(
            "tuck:1",
            Duration::from_secs(30),
            counting(calls.clone()),
        ));
        b.record_failure("connect timeout (10s)");
        let g = Gate::Breaker(b.clone());
        let t0 = Instant::now();
        let r = g.check();
        let dt = t0.elapsed();
        assert!(r.is_err(), "an open breaker refuses");
        assert!(r.unwrap_err().starts_with("TUCK-GATE-REFUSED"));
        assert_eq!(calls.load(Ordering::SeqCst), 0, "Open must not probe");
        assert!(dt < Duration::from_millis(100), "O(1): measured {dt:?}");
    }

    #[test]
    fn half_open_probes_once_and_a_success_closes_it() {
        let calls = Arc::new(AtomicUsize::new(0));
        let b = TuckBreaker::with_prober("tuck:1", Duration::from_millis(1), counting(calls.clone()));
        b.record_failure("down");
        std::thread::sleep(Duration::from_millis(5));
        assert!(b.check().is_ok(), "after the cooldown the probe is allowed");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(b.state(), GateState::Closed);
    }

    #[test]
    fn half_open_failure_reopens_and_keeps_the_reason() {
        let b = TuckBreaker::with_prober(
            "tuck:1",
            Duration::from_millis(1),
            Box::new(|_ep: &str| Err("still down".to_string())),
        );
        b.record_failure("down");
        std::thread::sleep(Duration::from_millis(5));
        let e = b.check().unwrap_err();
        assert!(e.contains("half-open probe failed"), "{e}");
        assert_eq!(b.state(), GateState::Open);
        assert_eq!(b.last_reason().as_deref(), Some("still down"));
    }

    #[test]
    fn the_breaker_refuses_rather_than_allows_when_it_cannot_tell() {
        /* The memory direction is the whole point: refusing is fail-safe, allowing is not.
         * This asserts the direction by construction: a recorded failure can only be cleared by
         * an OBSERVED success, never by the passage of time (only OPEN ⇒ HALF-OPEN moves on time). */
        let b = TuckBreaker::with_prober(
            "tuck:1",
            Duration::from_secs(3600),
            Box::new(|_ep: &str| Ok(())),
        );
        b.record_failure("down");
        for _ in 0..3 {
            assert!(b.check().is_err(), "an hour of cooldown must not turn a failure into an allow");
        }
    }
}
