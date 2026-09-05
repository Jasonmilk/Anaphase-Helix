//! Episode (experience boundary) lifecycle tests — ADR-0006.
//!
//! Session-as-experience: deterministic episode ids (shared FNV-1a
//! primitive, no UUID), begin/end lifecycle, L3 provenance on reflection
//! notes, and mode semantics. Fully deterministic and replayable.

use std::sync::{Arc, Mutex};

use anaphase::adapters::*;
use anaphase::agent_loop::AgentLoop;
use anaphase::config::{Config, Mode, RunCycleConfig};
use anaphase::contract::derive_episode_id;
use anaphase::reflex::ReflexArc;

/// Memory adapter that records every remember() payload (deterministic probe).
struct RecordingMemory {
    notes: Mutex<Vec<String>>,
}

impl RecordingMemory {
    fn new() -> Self {
        Self {
            notes: Mutex::new(vec![]),
        }
    }
    fn notes(&self) -> Vec<String> {
        self.notes.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl MemoryAdapter for RecordingMemory {
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
    async fn remember(&self, content: &str) -> Result<(), String> {
        self.notes.lock().unwrap().push(content.to_string());
        Ok(())
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
async fn begin_episode_uses_deterministic_id() {
    let agent = make_agent(Arc::new(RecordingMemory::new()));
    let mut agent = agent;
    let id = agent.begin_episode("hello").await;
    // Same shared FNV-1a primitive as job ids — deterministic replay.
    assert_eq!(id, derive_episode_id("hello"));
    assert_eq!(id, "ep-a430d84680aabd0b");
    assert!(id.starts_with("ep-"));
    assert_eq!(id.len(), 3 + 16);
    assert_eq!(agent.begin_episode("hello").await, id);
}

#[tokio::test]
async fn begin_episode_starts_at_step_zero() {
    let agent = make_agent(Arc::new(RecordingMemory::new()));
    let mut agent = agent;
    agent.begin_episode("hello").await;
    let ep = agent.episode.expect("episode must be active");
    assert_eq!(ep.step, 0);
    assert_eq!(ep.first_input, "hello");
}

#[tokio::test]
async fn end_episode_writes_digest_and_clears_boundary() {
    let memory = Arc::new(RecordingMemory::new());
    let agent = make_agent(memory.clone());
    let mut agent = agent;
    agent.begin_episode("hello").await;
    // Zero cycles: begin counts as one lived turn (the anchor itself).
    let digest = agent.end_episode().await.expect("digest expected");
    assert_eq!(digest.episode_id, derive_episode_id("hello"));
    assert_eq!(digest.turns, 1);
    assert_eq!(digest.first_input, "hello");
    assert!(agent.episode.is_none(), "boundary must be cleared");

    // The digest reaches L3 through the existing remember channel (no new
    // RPC, ADR-0006 D2) and carries the episode id for the recap.
    let notes = memory.notes();
    assert_eq!(notes.len(), 1);
    let parsed: serde_json::Value = serde_json::from_str(&notes[0]).unwrap();
    assert_eq!(parsed["digest"], "episode_close");
    assert_eq!(parsed["episode"], derive_episode_id("hello"));
    assert_eq!(parsed["turns"], 1);
    assert_eq!(parsed["first_input"], "hello");
}

#[tokio::test]
async fn end_episode_is_idempotent_when_inactive() {
    let agent = make_agent(Arc::new(RecordingMemory::new()));
    let mut agent = agent;
    assert!(agent.end_episode().await.is_none());
}

#[tokio::test]
async fn begin_auto_closes_previous_episode() {
    let memory = Arc::new(RecordingMemory::new());
    let agent = make_agent(memory.clone());
    let mut agent = agent;
    let first = agent.begin_episode("first").await;
    let second = agent.begin_episode("second").await;
    assert_ne!(first, second);
    // Previous experience was closed first: its digest is recorded, so no
    // experience is ever silently dropped.
    let notes = memory.notes();
    assert_eq!(notes.len(), 1);
    let parsed: serde_json::Value = serde_json::from_str(&notes[0]).unwrap();
    assert_eq!(parsed["episode"], first);
    let ep = agent.episode.expect("second episode active");
    assert_eq!(ep.id, second);
}

#[tokio::test]
async fn cycle_advances_episode_step() {
    let agent = make_agent(Arc::new(RecordingMemory::new()));
    let mut agent = agent;
    agent.begin_episode("hello").await;
    agent.run_cycle("hello").await.unwrap();
    let ep = agent.episode.expect("episode still active");
    assert_eq!(ep.step, 1);
}

#[tokio::test]
async fn reflection_note_carries_episode_provenance() {
    let memory = Arc::new(RecordingMemory::new());
    let agent = make_agent(memory.clone());
    let mut agent = agent;
    agent.begin_episode("hello").await;
    agent.run_cycle("hello").await.unwrap();
    // The reflection write carries `{episode_id}#{step}` as a structured
    // JSON field — Mind's L3 content keeps structured records (ADR-0006 D1).
    let notes = memory.notes();
    assert_eq!(notes.len(), 1);
    let parsed: serde_json::Value = serde_json::from_str(&notes[0]).unwrap();
    assert_eq!(parsed["episode"], format!("{}#1", derive_episode_id("hello")));
    assert!(parsed["note"].as_str().unwrap().contains("Cycle completed"));
}

#[tokio::test]
async fn no_episode_writes_note_verbatim() {
    let memory = Arc::new(RecordingMemory::new());
    let agent = make_agent(memory.clone());
    let mut agent = agent;
    agent.run_cycle("hello").await.unwrap();
    // Legacy path: without an episode the note is written verbatim —
    // strictly backwards compatible (no JSON wrapper, no episode field).
    let notes = memory.notes();
    assert_eq!(notes.len(), 1);
    assert!(notes[0].contains("Cycle completed"));
    assert!(!notes[0].contains("\"episode\""));
}

#[test]
fn mode_serde_roundtrip_and_default() {
    let m: Mode = serde_json::from_str("\"drive\"").unwrap();
    assert_eq!(m, Mode::Drive);
    let m: Mode = serde_json::from_str("\"survive\"").unwrap();
    assert_eq!(m, Mode::Survive);
    assert_eq!(serde_json::to_string(&Mode::Partner).unwrap(), "\"partner\"");
    // Helix's native state is the memory-bearing partner (ADR-0006 D3).
    assert_eq!(Mode::default(), Mode::Partner);
    assert_eq!(RunCycleConfig::default().mode, Mode::Partner);
}

#[test]
fn config_loads_mode_from_toml() {
    // The mode field parses through the real config stack (snake_case
    // serde rename); the other constants keep their documented values.
    let rc: RunCycleConfig = toml::from_str(
        "amygdala_default_vector = [0.7, 0.3, 0.2]\n\
         reasoning_mode = \"left_brain\"\n\
         soft_reflex_threshold = 0.7\n\
         execution_placeholder = \"echo\"\n\
         cycle_cap = 7\n\
         mode = \"drive\"\n",
    )
    .unwrap();
    assert_eq!(rc.mode, Mode::Drive);
    // Missing section = documented protocol defaults (mode = partner).
    assert_eq!(RunCycleConfig::default().mode, Mode::Partner);
}
