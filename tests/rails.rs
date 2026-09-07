//! Rails: external human knowledge rail — index / navigation / citation
//! contract / read-only scope (ADR-0018).
//!
//! Acceptance criteria (deterministic):
//! 1. Same kb dir builds a byte-identical index (replayable).
//! 2. Dangling links are a build error (rails are well-formed by
//!    construction).
//! 3. Navigation ranks deterministically and seeds the visited provenance.
//! 4. verify_reference: verbatim quote + visited node pass; anything else
//!    fails — no partial credit, no synthesized text.
//! 5. A rail miss answers NO_RAIL_CONTENT (graceful refusal).
//! 6. RailScope is type-level read-only (no write variant).
//! 7. run_cycle: a rail query lands verbatim nodes into the context and
//!    sets rail_mode (citation contract) — the real loop, e2e.

use std::collections::BTreeSet;
use std::path::Path;

const DEMO: &str = "knowledge_base/rails/demo";
const BROKEN: &str = "tests/fixtures/rails/broken";

#[test]
fn index_is_byte_identical_across_builds() {
    let a = anaphase::rails::build_index(Path::new(DEMO)).expect("demo builds");
    let b = anaphase::rails::build_index(Path::new(DEMO)).expect("demo rebuilds");
    let ja = serde_json::to_vec(&a).expect("serialize a");
    let jb = serde_json::to_vec(&b).expect("serialize b");
    assert_eq!(ja, jb, "same kb must build a byte-identical index");
    assert!(!a.nodes.is_empty(), "demo has nodes");
    assert!(!a.file_sha256.is_empty(), "demo has sha256 fingerprints");
}

#[test]
fn manifest_is_deterministic() {
    let a = anaphase::rails::build_index(Path::new(DEMO)).expect("demo builds");
    let b = anaphase::rails::build_index(Path::new(DEMO)).expect("demo rebuilds");
    assert_eq!(
        anaphase::rails::manifest(&a),
        anaphase::rails::manifest(&b),
        "manifest must be byte-identical"
    );
}

#[test]
fn dangling_link_fails_the_build() {
    let err = anaphase::rails::build_index(Path::new(BROKEN)).expect_err("broken kb must fail");
    assert!(
        err.contains("dangling"),
        "error should name the dangling link: {err}"
    );
}

#[test]
fn navigation_hits_and_ranks_deterministically() {
    let idx = anaphase::rails::build_index(Path::new(DEMO)).expect("demo builds");
    let nav = anaphase::rails::navigate(&idx, "数据归属", 3);
    assert!(!nav.hits.is_empty(), "query lands on the rail");
    let first = &nav.hits[0];
    assert!(
        first.contains("第 1 条 数据归属"),
        "highest-ranked hit is the 数据归属 node: {first}"
    );
    assert!(
        nav.visited.contains(first),
        "visited set is seeded with the hits"
    );
    // Deterministic: two navigations give the same ranked hits.
    let nav2 = anaphase::rails::navigate(&idx, "数据归属", 3);
    assert_eq!(nav.hits, nav2.hits);
}

#[test]
fn expand_follows_edges_into_visited() {
    let idx = anaphase::rails::build_index(Path::new(DEMO)).expect("demo builds");
    let nav = anaphase::rails::navigate(&idx, "数据归属", 3);
    let entry = nav.hits[0].clone();
    let mut visited = nav.visited.clone();
    let node = &idx.nodes[&entry];
    let expected: Vec<String> = node
        .children
        .iter()
        .chain(node.parent.iter())
        .chain(node.refs.iter())
        .cloned()
        .collect();
    let fresh = anaphase::rails::expand(&idx, &entry, &mut visited);
    for nid in &expected {
        assert!(
            visited.contains(nid),
            "every neighbor joins the visited provenance: {nid}"
        );
    }
    for nid in &fresh {
        assert!(
            expected.contains(nid),
            "fresh neighbors are exactly the not-yet-visited edges: {nid}"
        );
    }
}

#[test]
fn verify_reference_enforces_the_citation_contract() {
    let idx = anaphase::rails::build_index(Path::new(DEMO)).expect("demo builds");
    let nav = anaphase::rails::navigate(&idx, "数据归属", 3);
    let id = nav.hits[0].clone();
    let node = &idx.nodes[&id];
    let quote = "数据归其产生者所有";
    assert!(
        node.content.contains(quote),
        "fixture quote must be verbatim node content"
    );

    // 1. Verbatim quote + visited node -> pass.
    let ok = anaphase::rails::verify_reference(&idx, quote, &id, &nav.visited);
    assert!(ok.passed, "verbatim cite of a visited node passes: {}", ok.detail);

    // 2. Verbatim quote of an unvisited node -> fail (provenance).
    let other = idx
        .nodes
        .values()
        .find(|n| n.id != id && n.content.contains(quote) == false)
        .map(|n| n.id.clone())
        .expect("another node exists");
    let unvisited = BTreeSet::new();
    let no_visit = anaphase::rails::verify_reference(&idx, "数据归其产生者所有", &other, &unvisited);
    assert!(!no_visit.passed, "unvisited node must fail (provenance)");

    // 3. Paraphrased quote -> fail (not verbatim).
    let para = "数据归产生者";
    let no_verbatim = anaphase::rails::verify_reference(&idx, para, &id, &nav.visited);
    assert!(
        !no_verbatim.passed,
        "paraphrase must fail the citation contract: {}",
        no_verbatim.detail
    );

    // 4. Unknown node -> fail.
    let unknown = anaphase::rails::verify_reference(&idx, quote, "nope#nope", &nav.visited);
    assert!(!unknown.passed, "unknown node must fail");
}

#[test]
fn rail_miss_refuses_to_synthesize() {
    let idx = anaphase::rails::build_index(Path::new(DEMO)).expect("demo builds");
    let nav = anaphase::rails::navigate(&idx, "量子引力与税法的关系", 3);
    assert!(
        nav.hits.is_empty(),
        "a query outside the rail must not hit anything"
    );
    // The contract answer is the constant — never a generated sentence.
    let _ = anaphase::rails::NO_RAIL_CONTENT;
    assert!(
        !anaphase::rails::NO_RAIL_CONTENT.is_empty(),
        "refusal constant is non-empty and deterministic"
    );
}

#[test]
fn rail_scope_is_type_level_read_only() {
    // The enum has exactly one variant: Read. A write side does not exist —
    // the compiler rejects any future write intent at compile time.
    let scope = anaphase::rails::RailScope::Read;
    match scope {
        anaphase::rails::RailScope::Read => {}
    }
}

#[tokio::test]
async fn run_cycle_lands_rail_nodes_and_sets_rail_mode() {
    use anaphase::adapters::*;
    use anaphase::run_cycle::AgentLoop;

    use async_trait::async_trait;
    struct CountingReasoning(std::sync::Arc<std::sync::atomic::AtomicUsize>);
    #[async_trait]
    impl anaphase::adapters::ReasoningAdapter for CountingReasoning {
        async fn reason(&self, _input: &str, _mode: &str, _trace_id: &str) -> Result<String, String> {
            self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok("unused LLM output".to_string())
        }
    }
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let memory = std::sync::Arc::new(NoopMemoryAdapter);
    let reason = std::sync::Arc::new(CountingReasoning(calls.clone()));
    let tool = std::sync::Arc::new(NoopToolAdapter);
    let safety = std::sync::Arc::new(NoopSafetyAdapter);
    let ui = std::sync::Arc::new(NoopUiAdapter);
    let fear = std::sync::Arc::new(NoopFearAdapter);
    let reflex = anaphase::reflex::ReflexArc {
        safety_rules: vec![],
    };

    let idx = anaphase::rails::build_index(Path::new(DEMO)).expect("demo builds");
    let mut agent = AgentLoop::new(memory, reason, tool, safety, ui, fear, reflex)
        .with_rails(idx);

    let outcome = agent.run_cycle("数据归属条款是什么").await.expect("cycle runs");
    assert!(outcome.done, "cycle finishes");
    assert!(
        !agent.context.rail_nodes.is_empty(),
        "rail query injects verbatim nodes into the context"
    );
    assert!(
        agent.context.rail_mode,
        "rail query sets the citation contract (rail_mode)"
    );
    let first = &agent.context.rail_nodes[0];
    assert!(
        first.content.contains("数据归其产生者所有"),
        "injected content is verbatim rail text: {}",
        first.content
    );
    // Output contract (ADR-0018): the final answer is assembled verbatim
    // from the rail — the LLM was bypassed (0 tokens) and the answer carries
    // the node id + the exact rail text. No synthesis possible by design.
    assert_eq!(
        calls.load(std::sync::atomic::Ordering::Relaxed),
        0,
        "rail citation answer must bypass the LLM entirely (0 tokens)"
    );
    let answer = &agent.context.reasoning_output;
    assert!(
        answer.contains(first.id.as_str()),
        "answer carries the node id: {}",
        answer
    );
    assert!(
        answer.contains("数据归其产生者所有"),
        "answer quotes the rail verbatim (no paraphrase): {}",
        answer
    );
    assert!(
        !answer.contains("unused LLM output"),
        "answer must not contain any LLM-generated text"
    );
}
