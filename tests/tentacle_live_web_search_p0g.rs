// P0-G judgment criterion (2026-09-16): 3 consecutive web_search runs over the
// REAL tentacle binary + REAL fixture plugins, each plan OMITTING `expect`
// (the exact shape that died in parsing 6/6 times before the patch).
//
// Pass = every run emits a `tool/call` event for web_search AND a `tool/result`
// with ok:true — i.e. the call is dispatched and the Bing HTML endpoint
// actually returns. Before P0-G these runs died in parse_reasoning_output and
// no tool event ever appeared (the "silent, traceless" failure).
//
// Requirements (manual run, hence #[ignore]):
//   1. helix-tentacle binary built:  cargo build -p tentacle  (in ../helix-tentacle)
//   2. node on PATH (fixture executables are .js)
//   3. Run from anaphase-helix:
//        cargo test --test tentacle_live_web_search_p0g -- --ignored --nocapture
//   Override the binary path with TENTACLE_BIN if the default relative path
//   (../helix-tentacle/target/debug/tentacle) is not correct.

mod common;

use anaphase::adapters::*;
use anaphase::run_cycle::AgentLoop;
use anaphase::pipeline::{Pipeline, PipelineConfig};
use anaphase::reflex::ReflexArc;
use common::{connect_tentacle, free_port, spawn_real_tentacle, StructuredReasoning};
use anaphase::ledger::{FakeClock, VerdictStatus};
use std::sync::Arc;

/// One full live loop: real Tentacle + fixture plugins, a plan that OMITS
/// `expect` (P0-G), and a session-event stream to prove dispatch happened.
async fn run_web_search_loop(
    query: &str,
    events_dir: &std::path::Path,
) -> (Vec<anaphase::ledger::LedgerRecord>, String) {
    let port = free_port();
    let mut child = spawn_real_tentacle(port);
    let endpoint = format!("http://127.0.0.1:{}", port);
    let tentacle = connect_tentacle(&endpoint).await;

    let pipe_config = PipelineConfig::from_codex("knowledge_base/fixture-codex.json").unwrap();
    let pipeline = Pipeline::new(tentacle, Box::new(FakeClock(1000)), pipe_config);

    let mut agent = AgentLoop::new(
        Arc::new(NoopMemoryAdapter),
        Arc::new(StructuredReasoning {
            // P0-G shape: web_search plan WITHOUT `expect` — pre-patch this
            // failed `calls schema mismatch` and no tool event ever fired.
            output: format!(
                r#"{{"calls":[{{"tool":"web_search","args":{{"q":"{}","max":3}}}}]}}"#,
                query
            ),
        }),
        Arc::new(NoopToolAdapter),
        Arc::new(NoopSafetyAdapter),
        Arc::new(NoopUiAdapter),
        Arc::new(NoopFearAdapter),
        ReflexArc { safety_rules: vec![] },
    )
    .with_pipeline(pipeline);

    // ProveTrack turn timeline: per-period event stream in a scratch dir.
    agent.session_events_dir = Some(events_dir.to_path_buf());

    let _ = agent.run_cycle(&format!("用 web_search 搜索 {query}")).await;

    let records = agent.pipeline.as_ref().unwrap().ledger.records().to_vec();
    let _ = child.kill();
    let _ = child.wait();
    (records, format!("run-{:016x}", anaphase::contract::fnv64(&format!("用 web_search 搜索 {query}"))))
}

#[tokio::test]
#[ignore = "requires real tentacle binary + node (manual integration)"]
async fn p0g_web_search_three_consecutive_runs_dispatch() {
    let scratch = std::env::temp_dir().join("anaphase-p0g-judge");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).unwrap();

    let queries = ["rust async", "helix architecture", "weather forecast"];
    let mut dispatched = 0;
    let mut executed_ok = 0;

    for (i, q) in queries.iter().enumerate() {
        let (records, job_id) = run_web_search_loop(q, &scratch).await;

        // 1) The tool/call event must exist for this period (P0-G dispatch).
        let events_file = scratch.join(format!("{job_id}.events.jsonl"));
        let body = std::fs::read_to_string(&events_file).unwrap_or_else(|e| {
            panic!("run {i}: session events file missing ({e}) — dispatch never happened")
        });
        assert!(
            body.contains("\"type\":\"tool/call\"") && body.contains("web_search"),
            "run {i}: no tool/call for web_search in events: {body}"
        );
        dispatched += 1;

        // 2) The tool/result must be ok:true — the Bing endpoint really answered.
        let result_line = body
            .lines()
            .find(|l| l.contains("\"type\":\"tool/result\""))
            .expect("run {i}: no tool/result event");
        assert!(
            result_line.contains("\"ok\":true"),
            "run {i}: web_search returned failure: {result_line}"
        );
        executed_ok += 1;

        // 3) The period must close with a verdict (chain completed end to end).
        let has_verdict = records
            .iter()
            .any(|r| matches!(r, anaphase::ledger::LedgerRecord::Verdict { .. }));
        assert!(has_verdict, "run {i}: no verdict in ledger");
        let met = records.iter().any(|r| {
            matches!(
                r,
                anaphase::ledger::LedgerRecord::Verdict { status: VerdictStatus::Met, .. }
            )
        });
        println!("run {i}: q={q} dispatched=ok executed=ok verdict_met={met}");
    }

    assert_eq!(dispatched, 3, "all 3 runs must dispatch a tool/call");
    assert_eq!(executed_ok, 3, "all 3 runs must execute web_search ok");
    let _ = std::fs::remove_dir_all(&scratch);
}
