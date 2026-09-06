//! Wake-up trigger tests — P10d (ADR-0032).
//!
//! Anaphase looks at Mind's agenda once per interaction cycle: due alarms
//! with a supported action run the consolidate chain (action maps 1:1 to a
//! helix_consolidate kind), then ack done. Unknown actions are acked done
//! (released, never deadlocked); unavailable wake-up degrades silently.

use std::sync::{Arc, Mutex};

use anaphase::adapters::*;
use anaphase::run_cycle::AgentLoop;
use anaphase::reflex::ReflexArc;

/// Memory adapter with wake-up implemented (probe for the trigger path).
struct WakeupMemory {
    /// Due alarms returned by wakeup (injected per test).
    alarms: Mutex<Vec<WakeupAlarm>>,
    /// wakeup failure flag (adapter unavailable simulation).
    fail_wakeup: bool,
    /// consolidate calls (kind).
    consolidated: Mutex<Vec<String>>,
    /// ack calls (claim_id, status).
    acked: Mutex<Vec<(String, String)>>,
}

impl WakeupMemory {
    fn new(alarms: Vec<WakeupAlarm>) -> Self {
        Self {
            alarms: Mutex::new(alarms),
            fail_wakeup: false,
            consolidated: Mutex::new(vec![]),
            acked: Mutex::new(vec![]),
        }
    }
    fn failing() -> Self {
        let mut m = Self::new(vec![]);
        m.fail_wakeup = true;
        m
    }
    fn consolidate_calls(&self) -> Vec<String> {
        self.consolidated.lock().unwrap().clone()
    }
    fn ack_calls(&self) -> Vec<(String, String)> {
        self.acked.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl MemoryAdapter for WakeupMemory {
    async fn query(
        &self,
        _query: &str,
        _include_recessive: bool,
    ) -> Result<QueryResult, String> {
        Ok(QueryResult {
            nodes: vec![],
            impasse_level: 0,
            suggested_actions: vec![],
        })
    }
    async fn remember(&self, _content: &str) -> Result<(), String> {
        Ok(())
    }
    async fn wakeup(&self, _jitter_minutes: u32) -> Result<Vec<WakeupAlarm>, String> {
        if self.fail_wakeup {
            return Err("wakeup unavailable".to_string());
        }
        Ok(self.alarms.lock().unwrap().clone())
    }
    async fn wakeup_ack(&self, claim_id: &str, status: &str) -> Result<(), String> {
        self.acked
            .lock()
            .unwrap()
            .push((claim_id.to_string(), status.to_string()));
        Ok(())
    }
    async fn consolidate(&self, kind: &str) -> Result<(), String> {
        self.consolidated.lock().unwrap().push(kind.to_string());
        Ok(())
    }
}

fn alarm(action: &str) -> WakeupAlarm {
    WakeupAlarm {
        job_id: format!("job-{}", action),
        action: action.to_string(),
        due_at: "2026-09-06T08:00:00+00:00".to_string(),
        mode: "jittered".to_string(),
        claim_id: format!("claim#job-{}", action),
    }
}

fn make_agent(memory: Arc<dyn MemoryAdapter>) -> AgentLoop {
    AgentLoop::new(
        memory,
        Arc::new(NoopReasoningAdapter),
        Arc::new(NoopToolAdapter),
        Arc::new(NoopSafetyAdapter),
        Arc::new(NoopUiAdapter),
        Arc::new(NoopFearAdapter),
        ReflexArc {
            safety_rules: vec![],
        },
    )
}

#[tokio::test]
async fn due_alarm_runs_consolidate_then_acks_done() {
    let memory = Arc::new(WakeupMemory::new(vec![alarm("hibernate")]));
    let mut agent = make_agent(memory.clone());

    agent.run_cycle("早上好").await.unwrap();

    // Supported action ("hibernate", protocol default whitelist) → the
    // consolidate chain runs, then the claim is released.
    assert_eq!(
        memory.consolidate_calls(),
        vec!["hibernate".to_string()],
        "supported action runs the consolidate chain"
    );
    assert_eq!(
        memory.ack_calls(),
        vec![("claim#job-hibernate".to_string(), "done".to_string())],
        "claim acked done after execution"
    );
}

#[tokio::test]
async fn unsupported_action_acked_without_consolidate() {
    let memory = Arc::new(WakeupMemory::new(vec![alarm("weird")]));
    let mut agent = make_agent(memory.clone());

    agent.run_cycle("早上好").await.unwrap();

    // Not in the configured whitelist → no execution, claim released
    // (never deadlocked), warning logged.
    assert!(
        memory.consolidate_calls().is_empty(),
        "unsupported action must not run consolidate"
    );
    assert_eq!(
        memory.ack_calls(),
        vec![("claim#job-weird".to_string(), "done".to_string())],
        "unsupported claim released"
    );
}

#[tokio::test]
async fn no_due_alarm_no_calls() {
    let memory = Arc::new(WakeupMemory::new(vec![]));
    let mut agent = make_agent(memory.clone());

    agent.run_cycle("早上好").await.unwrap();

    assert!(memory.consolidate_calls().is_empty());
    assert!(memory.ack_calls().is_empty());
}

#[tokio::test]
async fn unavailable_wakeup_degrades_silently() {
    let memory = Arc::new(WakeupMemory::failing());
    let mut agent = make_agent(memory.clone());

    // Cycle completes normally; wake-up failure is a non-event.
    let outcome = agent.run_cycle("早上好").await.unwrap();
    assert!(outcome.success);
    assert!(memory.consolidate_calls().is_empty());
    assert!(memory.ack_calls().is_empty());
}
