use petgraph::graph::DiGraph;

/// Core cognitive states for the Anaphase-Helix agent
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
pub enum HelixState {
    Perception,
    PreAssessment,
    MemoryRetrieval,
    Reasoning,
    ReflexCheck,   // Added: Somatic reflex arc check
    Execution,
    Reflection,
}

impl HelixState {
    /// All states in DAG order (ADR-0016 D1): one cognitive period walks at
    /// most one pass per state — the DAG is acyclic, so this length is the
    /// natural period-step cap (derived from the enum, not a magic number).
    pub const ALL: [HelixState; 7] = [
        HelixState::Perception,
        HelixState::PreAssessment,
        HelixState::MemoryRetrieval,
        HelixState::Reasoning,
        HelixState::ReflexCheck,
        HelixState::Execution,
        HelixState::Reflection,
    ];
}

/// Build the directed acyclic graph (DAG) for state transitions
pub fn build_state_graph() -> DiGraph<HelixState, &'static str> {
    let graph = DiGraph::new();
    // ... Build the 7-state DAG ...
    graph
}
