//! Security gate wiring tests — ADR-0008 (candidate D'-2).
//!
//! Verifies the pipeline wiring point with a mock gate: Pass / HardOverride
//! proceed; Reject / HitlRequired block the call, short-circuit with an Err,
//! and write a `blocked` ledger record.
//!
//! **B7 changed what `None` means.** It used to be "legacy behaviour" — i.e. no check at
//! all, which is how the production path ran ungated (K-044: `None` read as benign while
//! meaning "no door"). Now **absence is not approval**: with no gate configured, a
//! *high-risk* call is blocked and low-risk calls keep the legacy path. `PermissiveGate`
//! is the explicit, visible opt-out.
//!
//! # I7 (CI-144 §13.3): "no gate installed" must be distinguishable from "gate passed"
//!
//! B7 closed the *decision* half of the absence problem; this file now also pins the
//! *observability* half. Before the `gate` stage event, a run with no door-keeper and a
//! run whose door-keeper said "pass" produced the **same** observable state: a `Met`
//! verdict, an empty block count, no distinguishing record — which is exactly the
//! "0 block is indistinguishable from the gate never running" finding (K-061/K-092).
//! The `gate_presence_*` tests below assert the two are distinguishable, using only the
//! event ring; if the two collapse back into one observation, they go red.
//!
//! # I5 (CI-144 §13.3, DNA proposal B): coverage must equal declared coverage
//!
//! `the_declared_hook_repo_list_covers_every_repo_in_the_workspace` replays
//! `helix-mind/tools/install-hooks.sh`'s `REPOS` declaration against every git repo that
//! actually exists. That list fell behind once (8 of 14, K-104a) and nothing noticed;
//! `--check` reports omissions to whoever runs it, but nothing asserted them.
//!
//! The mock Tentacle is used only to observe whether a call reached the
//! wire (blocked calls must never reach Tentacle).

use std::collections::BTreeMap;
use std::sync::Arc;

use anaphase::adapters::tentacle::GrpcTentacleAdapter;
use anaphase::ledger::{FakeClock, LedgerRecord};
use anaphase::pipeline::{Pipeline, PipelineConfig, PipelineInput};
use anaphase::security::{GateCheck, GateVerdict, SecurityGate};

mod common;
use common::{spawn_mock_tentacle, MockTentacle};

/// Gate that always returns one verdict.
#[derive(Clone)]
struct FixedGate(GateVerdict);

#[async_trait::async_trait]
impl SecurityGate for FixedGate {
    async fn check(&self, _check: &GateCheck) -> GateVerdict {
        self.0.clone()
    }
}

fn input() -> PipelineInput {
    PipelineInput {
        job_id: "tt_job-gate".into(),
        created_at: "2026-09-06T00:00:00Z".into(),
        llm_content: r#"{"calls":[{"tool":"numbers","args":{},"expect":"numbers"}]}"#.into(),
        identity_labels: BTreeMap::new(),
    }
}

/// A call the local classifier marks high-risk. B7 uses that classifier deliberately:
/// it is the *weakest* available judgement (B4 records how weak), so it under-blocks
/// rather than over-blocks. If it ever stops calling `rm` high-risk, the B7 tests below
/// fail — which is the point of routing them through the real classifier.
///
/// `expect` stays inside the grammar's enum (`ok`); only `tool` is free-form, and `tool`
/// is what the classifier reads. (`expect` rejecting an unknown variant is what told me
/// this — the field that is an enum is not the field that carries the risk.)
fn high_risk_input() -> PipelineInput {
    PipelineInput {
        llm_content: r#"{"calls":[{"tool":"rm","args":{},"expect":"ok"}]}"#.into(),
        ..input()
    }
}

async fn wired(mock: MockTentacle) -> Pipeline {
    let (endpoint, _captured, shutdown_tx, handle) = spawn_mock_tentacle(mock).await;
    let config = PipelineConfig::from_codex("knowledge_base/fixture-codex.json").unwrap();
    let pipeline = Pipeline::new(
        GrpcTentacleAdapter::new(&endpoint).await.unwrap(),
        Box::new(FakeClock(1000)),
        config,
    );
    // Keep the server alive for the lifetime of the test.
    std::mem::forget((shutdown_tx, handle));
    pipeline
}

#[tokio::test]
async fn no_gate_is_legacy_compatible() {
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0,3.0,4.0]}"#);
    let pipeline = wired(mock).await;
    let mut pipeline = pipeline.with_security_gate(None);
    let outcome = pipeline.run(input()).await.unwrap();
    assert_eq!(outcome.job_id, "tt_job-gate");
    // Legacy path: a verdict, never a blocked record.
    assert!(pipeline.ledger.records().iter().all(|r| matches!(r, LedgerRecord::Verdict { .. })));
}

/// B7. **Absence of a gate is not approval.** With no door-keeper configured, a
/// high-risk call must be blocked — that is the whole difference between "we have an
/// adapter" and "the pipeline is gated".
///
/// Non-vacuity: the mock **does** serve `rm`, so a regression that lets high-risk calls
/// through fails here by succeeding, instead of erroring for some duller reason.
// guards: no-gate-blocks-high-risk
#[tokio::test]
async fn no_gate_blocks_a_high_risk_call() {
    let mock = MockTentacle::new().with_tool("rm", r#"{"ok":true}"#);
    let pipeline = wired(mock).await;
    let mut pipeline = pipeline.with_security_gate(None);
    let err = pipeline
        .run(high_risk_input())
        .await
        .expect_err("a high-risk call must not proceed when no gate is configured");
    assert!(err.contains("blocked by security gate"), "{err}");
    assert!(
        pipeline
            .ledger
            .records()
            .iter()
            .any(|r| matches!(r, LedgerRecord::Blocked { .. })),
        "the block must be recorded in the ledger, not merely returned as an Err"
    );
}

/// The other half of B7: the legacy promise is **narrowed, not revoked**. A low-risk
/// call still runs with no gate configured, so this is not a blanket fail-closed that
/// would break every existing deployment.
#[tokio::test]
async fn no_gate_still_allows_a_low_risk_call() {
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0]}"#);
    let pipeline = wired(mock).await;
    let mut pipeline = pipeline.with_security_gate(None);
    pipeline
        .run(input())
        .await
        .expect("low-risk calls keep the legacy path");
    assert!(
        pipeline
            .ledger
            .records()
            .iter()
            .all(|r| !matches!(r, LedgerRecord::Blocked { .. })),
        "a low-risk call must not be recorded as blocked"
    );
}

/// B7's escape hatch, and why it is a hatch rather than a default: installing
/// `PermissiveGate` is an **explicit, visible** decision to run without policy.
#[tokio::test]
async fn permissive_gate_is_the_explicit_opt_out() {
    let mock = MockTentacle::new().with_tool("rm", r#"{"ok":true}"#);
    let pipeline = wired(mock).await;
    let mut pipeline = pipeline
        .with_security_gate(Some(Arc::new(anaphase::security::PermissiveGate)));
    pipeline
        .run(high_risk_input())
        .await
        .expect("an explicitly installed PermissiveGate must permit");
}

#[tokio::test]
async fn pass_gate_proceeds_and_verdicts() {
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0,3.0,4.0]}"#);
    let pipeline = wired(mock).await;
    let mut pipeline = pipeline.with_security_gate(Some(Arc::new(FixedGate(GateVerdict::Pass))));
    let outcome = pipeline.run(input()).await.unwrap();
    assert_eq!(outcome.verdict, anaphase::ledger::VerdictStatus::Met);
    assert!(!pipeline.ledger.records().is_empty());
}

#[tokio::test]
async fn hard_override_proceeds() {
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0,3.0,4.0]}"#);
    let pipeline = wired(mock).await;
    let mut pipeline =
        pipeline.with_security_gate(Some(Arc::new(FixedGate(GateVerdict::HardOverride))));
    let outcome = pipeline.run(input()).await.unwrap();
    assert_eq!(outcome.verdict, anaphase::ledger::VerdictStatus::Met);
}

#[tokio::test]
async fn reject_gate_blocks_and_records_blocked() {
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0,3.0,4.0]}"#);
    let mut pipeline = wired(mock.clone()).await;
    pipeline = pipeline
        .with_security_gate(Some(Arc::new(FixedGate(GateVerdict::Reject(
            "policy: catastrophic".into(),
        )))));

    let err = pipeline.run(input()).await.unwrap_err();
    assert!(err.contains("blocked by security gate"), "err = {err}");

    // The call never reached the wire.
    assert!(mock.captured_trace_ids.all().is_empty());

    // The ledger carries exactly one `blocked` record.
    let blocked: Vec<_> = pipeline
        .ledger
        .records()
        .iter()
        .filter(|r| matches!(r, LedgerRecord::Blocked { .. }))
        .collect();
    assert_eq!(blocked.len(), 1, "expected one blocked record");
    match blocked[0] {
        LedgerRecord::Blocked { job_id, tool, index, reason, .. } => {
            assert_eq!(job_id, "tt_job-gate");
            assert_eq!(tool, "numbers");
            assert_eq!(*index, 0);
            assert_eq!(reason, "policy: catastrophic");
        }
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn hitl_required_blocks_and_records_blocked() {
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0,3.0,4.0]}"#);
    let mut pipeline = wired(mock.clone()).await;
    pipeline = pipeline.with_security_gate(Some(Arc::new(FixedGate(
        GateVerdict::HitlRequired("human confirmation required".into()),
    ))));

    let err = pipeline.run(input()).await.unwrap_err();
    assert!(err.contains("human confirmation required"), "err = {err}");
    assert!(mock.captured_trace_ids.all().is_empty());

    let blocked = pipeline
        .ledger
        .records()
        .iter()
        .filter(|r| matches!(r, LedgerRecord::Blocked { .. }))
        .count();
    assert_eq!(blocked, 1);
}

#[tokio::test]
async fn gate_check_carries_full_facts() {
    // The GateCheck handed to the gate must carry job/index/tool/args/labels
    // so the gate implementation can audit and decide on real facts.
    let (tx, rx) = tokio::sync::oneshot::channel();
    let gate = FactCapturingGate::new(tx);
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0,3.0,4.0]}"#);
    let pipeline = wired(mock).await;
    let mut pipeline = pipeline.with_security_gate(Some(Arc::new(gate)));

    let mut labels = BTreeMap::new();
    labels.insert("identity".to_string(), "anaphase_test".to_string());
    let mut input = input();
    input.identity_labels = labels;

    let _ = pipeline.run(input).await;
    let check = rx.await.unwrap();
    assert_eq!(check.job_id, "tt_job-gate");
    assert_eq!(check.index, 0);
    assert_eq!(check.tool, "numbers");
    assert_eq!(check.identity_labels.get("identity").map(|s| s.as_str()), Some("anaphase_test"));
}

/// Captures the last GateCheck it saw.
struct FactCapturingGate(std::sync::Mutex<Option<tokio::sync::oneshot::Sender<GateCheck>>>);

impl FactCapturingGate {
    fn new(tx: tokio::sync::oneshot::Sender<GateCheck>) -> Self {
        Self(std::sync::Mutex::new(Some(tx)))
    }
}

#[async_trait::async_trait]
impl SecurityGate for FactCapturingGate {
    async fn check(&self, check: &GateCheck) -> GateVerdict {
        if let Some(tx) = self.0.lock().unwrap().take() {
            let _ = tx.send(check.clone());
        }
        GateVerdict::Pass
    }
}

// ============================================================================
// I7 — "no gate installed" must be distinguishable from "gate installed and passed"
// ============================================================================
//
// Cited, not duplicated: this repo already has an I7-shaped precedent for "a direction
// must be explicit, never a default" — `run_cycle::safety_gate::OnAuditError`
// (`{ ReportSuccess, Block }`), held by the table-driven assertion at
// `src/run_cycle/tests.rs:1092-1120`, which pins both production call sites in opposite
// directions and forbids adding a `Default`. That precedent models the *two outcomes of
// a gate error*; this file models the *absence of the gate object itself*, which that
// assertion does not cover. Same principle, different object — so a new assertion here
// adds a case rather than restating the existing one.
//
// Second scope limit (so absence is not overclaimed): what an event *can* witness is
// "a gate object existed / did not exist". A concrete `SecurityGate` that is installed
// and silently never called is still indistinguishable from a call it permitted — the
// gate's *invocation* leaves no trace. That is this section's remaining hole, and it is
// not closable from the pipeline side (the trait exposes only `check`).

/// What the run recorded about *whether a gate was there at all*.
///
/// Reads the stage-3 `gate` events. The pipeline emits exactly one per call, before the
/// call is checked, so this is a record of presence — not of the verdict (a verdict is
/// only visible when it blocks, as a `blocked` ledger record).
///
/// Reachable by production consumers: `src/main.rs` exposes the ring over
/// `GET /v1/agent/events` (`after` cursor, plus `dropped`), so the record is not
/// confined to the test that reads it here.
fn gate_presence_events(pipeline: &Pipeline) -> Vec<String> {
    pipeline
        .events
        .lock()
        .unwrap()
        .events()
        .iter()
        .filter(|e| e.stage == 3 && e.phase == "gate")
        .map(|e| e.detail.clone())
        .collect()
}

/// No gate installed ⇒ the run says so, in as many words.
///
/// Red if the absence is expressed as anything that could also mean "a gate passed":
/// an empty detail, a `gate=pass`, or no event at all (the empty vector is asserted
/// against, so "quietly emit nothing" fails too).
#[tokio::test]
async fn no_gate_installed_is_recorded_as_absent_not_as_a_pass() {
    let mock = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0]}"#);
    let mut pipeline = wired(mock).await.with_security_gate(None);

    pipeline
        .run(input())
        .await
        .expect("low-risk calls keep the legacy path with no gate");

    let presence = gate_presence_events(&pipeline);
    assert_eq!(
        presence.len(),
        1,
        "one call ⇒ exactly one gate-presence record; quiet omission is the K-061 failure \
         mode (0 observations, indistinguishable from 'the gate never ran'): {presence:?}"
    );
    assert!(
        is_absence(&presence),
        "with no gate installed the record must name the absence (`gate=none`); it said \
         {:?} — a record that does not name the absence is the silence this invariant \
         exists to break (a record that says the gate passed is worse: it asserts a gate \
         that was never there)",
        presence
    );
}

/// A run's presence records mark "**no gate object exists**" — not merely "not a pass".
///
/// The distinction matters because *differs from a pass* is too weak to be the criterion:
/// a record that says `gate=passed` when there was no gate also differs from a real
/// gate's name. The absence has to be **named as absence**, exactly.
fn is_absence(presence: &[String]) -> bool {
    !presence.is_empty() && presence.iter().all(|p| p == "gate=none")
}

/// A gate that permits everything is the *same decision* as no gate on a low-risk call —
/// and must still be a **different observation**, with the absence named as absence.
///
/// This is the invariant in one test: the verdicts agree (so the gate's policy did not
/// change), and the presence records differ **in the right direction** (so "no door" and
/// "a door that said yes" are not the same fact). Red if the run stops recording either
/// one, if both collapse to the same string, or if the no-gate run is recorded as a pass
/// (the "0 block == the gate ran and passed" conflation).
#[tokio::test]
async fn gate_presence_distinguishes_none_from_an_installed_passing_gate() {
    let mock_none = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0]}"#);
    let mut with_none = wired(mock_none).await.with_security_gate(None);
    let outcome_none = with_none.run(input()).await.expect("runs with no gate");

    let mock_pass = MockTentacle::new().with_tool("numbers", r#"{"series":[1.0,2.0]}"#);
    let mut with_pass = wired(mock_pass)
        .await
        .with_security_gate(Some(Arc::new(FixedGate(GateVerdict::Pass))));
    let outcome_pass = with_pass.run(input()).await.expect("runs with a passing gate");

    // Control: the *decision* really is the same in both runs — otherwise "different
    // observation" would be satisfied trivially and prove nothing about the I7 case.
    assert_eq!(
        outcome_none.verdict, outcome_pass.verdict,
        "control: a passing gate and no gate must reach the same verdict for this input"
    );
    let blocked = |p: &Pipeline| {
        p.ledger
            .records()
            .iter()
            .filter(|r| matches!(r, LedgerRecord::Blocked { .. }))
            .count()
    };
    assert_eq!(blocked(&with_none), 0, "control: no gate must not block a low-risk call");
    assert_eq!(blocked(&with_pass), 0, "control: a passing gate must not block");

    let presence_none = gate_presence_events(&with_none);
    let presence_pass = gate_presence_events(&with_pass);
    assert!(
        !presence_none.is_empty() && !presence_pass.is_empty(),
        "both runs must record presence: none={presence_none:?} pass={presence_pass:?}"
    );
    assert!(
        presence_none.iter().zip(presence_pass.iter()).all(|(n, p)| {
            n.starts_with("gate=") && p.starts_with("gate=") && n != p
        }),
        "I7 RED: 'no gate installed' ({presence_none:?}) and 'gate installed and it passed' \
         ({presence_pass:?}) are not distinguishable — 0 block versus 'the gate never ran' \
         (K-061/K-092). The absence must be an explicit, observable fact."
    );
    assert!(
        is_absence(&presence_none),
        "I7 RED: the no-gate run must record the **absence** ({presence_none:?}), not just \
         a string that happens to differ from the installed gate's name — otherwise \
         'no gate' is still spelled like a verdict"
    );
    assert!(
        !is_absence(&presence_pass),
        "I7 RED: the passing-gate run ({presence_pass:?}) must not claim there was no gate \
         — it had one, and the record has to say which"
    );
}

// ============================================================================
// I5 — declared coverage must equal actual coverage
// ============================================================================

/// A repo carries `.git` as a directory (normal clone) or as a *file*
/// (`gitdir: …`, used by linked worktrees and submodules) — both count.
fn repo_root(dir: &std::path::Path) -> bool {
    dir.join(".git").exists()
}

/// Every git repo under `root`, as paths relative to `root`, sorted.
///
/// Hidden directories (`.git`, `.helix`, …) are skipped; a directory that is itself a
/// repo is not descended into, so no repo counts twice.
fn repos_under(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy().to_string();
        if name.starts_with('.') || name == "target" {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        if repo_root(&path) {
            out.push(rel);
        } else {
            repos_under(root, &path, out);
        }
    }
}

/// The quoted value of a shell declaration: `NAME="…"` → the text between quotes.
///
/// Handles multi-line values, which the exclusion list is (a `"` opens, the body
/// follows, a `"` closes). A parser that only read to end-of-line would silently read
/// an empty exclusion list — and an empty exclusion list reads as "nothing is
/// excluded", which is the vacuous-green direction this whole checker exists to avoid.
fn shell_declaration(script: &str, name: &str) -> Option<String> {
    let marker = format!("{name}=\"");
    let start = script.find(&marker)? + marker.len();
    let rest = &script[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// The `REPOS="…"` declaration inside a script, as written.
fn declared_repos(script: &str) -> Option<String> {
    shell_declaration(script, "REPOS")
}

/// The exclusion rows: `<repo> | <status> | <reason>`, comments/blank lines ignored.
///
/// A row missing a field comes back with that field empty so the caller can fail on it.
/// Tolerating a short row here would make "no reason given" indistinguishable from
/// "reason given" — exactly the absence-posing-as-declaration this checker exists for.
///
/// `status` is parsed separately from `reason` on purpose: "why this is out" and "who
/// still has to rule on it" are two facts, and a row that merges them reads as decided
/// when it is only pending (or vice versa).
fn excluded_repos(script: &str) -> Vec<(String, String, String)> {
    shell_declaration(script, "EXCLUDE")
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let mut parts = l.splitn(3, '|').map(|p| p.trim().to_string());
            (
                parts.next().unwrap_or_default(),
                parts.next().unwrap_or_default(),
                parts.next().unwrap_or_default(),
            )
        })
        .collect()
}

/// The statuses the checker knows. A new status must be added here deliberately —
/// an unrecognised one would otherwise pass as "some status we don't understand",
/// which is how a declaration quietly stops meaning anything.
const KNOWN_EXCLUDE_STATUSES: [&str; 2] = ["pending-human", "decided"];

/// Positive control for the I5 reader and the walk.
///
/// Without this, a parser that returns `None` (or a walk that finds no repos) is
/// indistinguishable from "the declaration covers everything" — the checker would be
/// green because it read nothing. I5's whole subject is exactly that confusion, so the
/// checker must not commit it.
///
/// Red if `declared_repos`/`excluded_repos` stop reading real declarations, if a
/// multi-line value gets truncated (the "empty exclusion list" case), or if the walk
/// stops finding repos — or starts reporting things that are not repo roots.
///
/// Deliberately no magic count and no named repo: a fixed number is a threshold whose
/// basis is unstated, and a named repo is an enumeration that expires. Both are read
/// off the live workspace instead (canary only: the walk must see *something*, and it
/// must be wrong to say it sees nothing).
#[test]
fn the_i5_reader_actually_reads_the_declaration_and_the_workspace() {
    // Parser: a real declaration is read, quoted and all; a script without one is None
    // (not an empty list, which would silently mean "covers nothing").
    let sample = "#!/usr/bin/env bash\nHERE=x\nREPOS=\"alpha beta commonintents/.github\"\n";
    assert_eq!(
        declared_repos(sample).as_deref(),
        Some("alpha beta commonintents/.github"),
        "the reader must return the declaration it sees"
    );
    assert_eq!(
        declared_repos("#!/usr/bin/env bash\nHOOKS=x\n"),
        None,
        "no declaration must read as None — an empty list would look like a passing check"
    );
    // The multi-line form is the one this checker actually depends on: truncated at
    // the first newline it would read as "" and the exclusion checker would be vacuous.
    let multi = "REPOS=\"a b\"\nEXCLUDE=\"\nx | decided | because\n\"\nMODE=x\n";
    assert_eq!(
        excluded_repos(multi),
        vec![(
            "x".to_string(),
            "decided".to_string(),
            "because".to_string()
        )],
        "a multi-line declaration must be read whole, not truncated at its first newline"
    );
    // Status and reason are separate fields: a reason containing the separator must
    // still leave the status readable, and a two-field row must surface an empty status.
    assert_eq!(
        excluded_repos("EXCLUDE=\"x | pending-human | a | b\n\""),
        vec![(
            "x".to_string(),
            "pending-human".to_string(),
            "a | b".to_string()
        )],
        "a reason may itself contain the separator"
    );
    assert_eq!(
        excluded_repos("EXCLUDE=\"x | only-a-reason\n\""),
        vec![(
            "x".to_string(),
            "only-a-reason".to_string(),
            String::new()
        )],
        "the field after the first separator is the STATUS, so a short row must surface \
         an empty reason rather than silently promoting a prose reason into that slot"
    );

    // Walk: the workspace we are compiled into has repos, and every path the walk
    // reports is a repo root. If the walk finds nothing, the coverage assertion that
    // uses it is vacuously green — so "found nothing" must be an error here.
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let ws = manifest_dir
        .parent()
        .expect("the workspace root holds anaphase-helix")
        .to_path_buf();
    let mut actual: Vec<String> = Vec::new();
    repos_under(&ws, &ws, &mut actual);
    assert!(
        !actual.is_empty(),
        "the walk found no repos under {} — every coverage claim built on it would be \
         vacuously green (the K-104a failure mode, with the sign flipped)",
        ws.display()
    );
    assert!(
        actual.iter().all(|r| !r.is_empty() && repo_root(&ws.join(r))),
        "every path the walk reports must be a non-empty repo root: {actual:?}"
    );
}

/// I5: `install-hooks.sh` must declare the *whole* scope — what it hooks (`REPOS`) and
/// what it deliberately does not (`EXCLUDE`), the latter with a reason for each row.
///
/// Red on every direction that matters:
/// - a git repo in neither list (the K-104a defect: the list said 8 while the workspace
///   held 14+, so the ADR gate and the `[large]` gate silently did not run there);
/// - a `REPOS` name that is not a repo (a declaration that outlives its subject);
/// - an `EXCLUDE` name that is not a repo (an exclusion list expires the same way);
/// - an `EXCLUDE` row with no reason (a silent exclusion is the absence-of-declaration
///   this checker exists to make visible, wearing a declaration's clothes).
///
/// A deliberately excluded repo still trips this gate when it is *new*, because only
/// the repos named in `EXCLUDE` are excluded — the list cannot follow reality.
#[test]
fn the_declared_hook_repo_list_covers_every_repo_in_the_workspace() {
    // tests/ -> anaphase-helix -> workspace root, and the sibling repo beside it.
    // `CARGO_MANIFEST_DIR` is baked in at compile time, so when the checker and its
    // subject part ways the failure must say *where* it looked, not just "not found".
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let ws = manifest_dir
        .parent()
        .expect("the workspace root is the directory holding anaphase-helix")
        .to_path_buf();
    let script_path = ws.join("helix-mind").join("tools").join("install-hooks.sh");
    let script = std::fs::read_to_string(&script_path).unwrap_or_else(|e| {
        panic!(
            "cannot read the declaration under test at {}: {e} — a checker that cannot \
             read its subject must not report success",
            script_path.display()
        )
    });

    let raw = declared_repos(&script).unwrap_or_else(|| {
        panic!(
            "{} has no `REPOS=` declaration — the parser and the mechanism have parted \
             ways; refusing to give a confident answer",
            script_path.display()
        )
    });
    let declared: Vec<String> = raw
        .split_whitespace()
        .map(|s| s.trim_matches('/').to_string())
        .collect();
    assert!(
        !declared.is_empty(),
        "REPOS= is empty — an empty list makes this checker vacuously green"
    );

    // Negative control: a script with no `EXCLUDE=` at all reads as "nothing is
    // excluded". That is why the exclusion list must exist and be non-empty here —
    // otherwise "we excluded six repos" and "we never thought about it" are the same
    // input to this check.
    let excluded = excluded_repos(&script);
    assert!(
        shell_declaration(&script, "EXCLUDE").is_some(),
        "{} has no `EXCLUDE=` declaration — with none, every unlisted repo is either \
         silently uncovered or silently excluded and this check cannot tell which",
        script_path.display()
    );

    for (repo, status, reason) in &excluded {
        assert!(
            !reason.is_empty(),
            "EXCLUDE row {repo:?} carries no reason — an unexplained exclusion is the \
             missing declaration this check exists to surface"
        );
        assert!(
            KNOWN_EXCLUDE_STATUSES.contains(&status.as_str()),
            "EXCLUDE row {repo:?} has status {status:?}, which this checker does not \
             know (known: {KNOWN_EXCLUDE_STATUSES:?}) — an unknown status reads as \
             'some status we do not understand', which is a declaration that stopped \
             meaning anything"
        );
        // A row that is still pending must say what approval would change; otherwise
        // "pending" is a reason-less exemption wearing a status.
        if status == "pending-human" {
            assert!(
                reason.contains("Approving"),
                "EXCLUDE row {repo:?} is `pending-human` but its reason does not say \
                 what approving would change, so the row cannot be acted on and becomes \
                 an indefinite exemption: {reason:?}"
            );
        }
        assert!(
            repo_root(&ws.join(repo)),
            "EXCLUDE names {repo:?}, which is not a git repository in {} — an exclusion \
             outliving its subject is the same defect one step later",
            ws.display()
        );
    }

    for rel in &declared {
        assert!(
            repo_root(&ws.join(rel)),
            "the declaration names {rel:?}, which is not a git repository in {} — \
             a declaration that outlives its subject is the same defect one step later",
            ws.display()
        );
    }

    let mut actual: Vec<String> = Vec::new();
    repos_under(&ws, &ws, &mut actual);
    actual.sort();

    let excluded_names: Vec<String> = excluded.iter().map(|(r, _, _)| r.clone()).collect();
    let missing: Vec<&String> = actual
        .iter()
        .filter(|rel| !declared.contains(rel) && !excluded_names.contains(rel))
        .collect();
    assert!(
        missing.is_empty(),
        "I5 RED: {} repo(s) exist in the workspace but are in neither install-hooks.sh's \
         REPOS list nor its EXCLUDE list: {missing:?}. Declared coverage is not actual \
         coverage — the hooks (ADR first-line gate, [large] marker gate) do not run in \
         those repos, and nothing said so (K-104a). Declared: {declared:?}. Excluded: \
         {excluded_names:?}. Actual: {actual:?}",
        missing.len()
    );
}
