use petgraph::graph::DiGraph;

/// Core cognitive states for the Anaphase-Helix agent
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HelixState {
    Perception,
    PreAssessment,
    MemoryRetrieval,
    Reasoning,
    ReflexCheck,   // Added: Somatic reflex arc check
    Execution,
    Reflection,
}

/// Build the directed acyclic graph (DAG) for state transitions
pub fn build_state_graph() -> DiGraph<HelixState, &'static str> {
    let graph = DiGraph::new();
    // ... Build the 7-state DAG ...
    graph
}
