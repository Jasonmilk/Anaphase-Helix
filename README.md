# Anaphase-Helix

![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)
![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)
![Build](https://img.shields.io/badge/Build-Passing-brightgreen.svg)
![Style](https://img.shields.io/badge/Code%20Style-Google-black.svg)
[![Tests](https://img.shields.io/badge/tests-94%2F94%20passed-green)](#)

**The silicon-based operating system & physical brain for digital lifeforms.
Perceive, reason, act, remember, and immunize — the body that houses the soul.**

## Overview

Anaphase-Helix is the **digital exoskeleton** for the Helix digital lifeform.
It is not a chatbot, toolchain, or AI assistant. It is the autonomous somatic system
that hosts Helix-Mind (the "soul"), delivering perception, cognitive scheduling,
tool execution, memory access, and innate safety immunity.

Built around the philosophy **Orchestrate, Don’t Build**,
Anaphase does not implement logic — it **coordinates systems**
via the CommonIntents protocol stack with zero hard coupling.

## Core Features

- 🧠 **7-State DAG Cognitive Loop** — Declarative state machine, no if-else chains
- 🛡️ **Somatic Reflex Arc** — Hard reflex (L0 gene lock) + soft reflex (fear prediction)
- 🔌 **Pluggable Adapters** — All services degrade gracefully to Noop (fail-open/closed per DNA)
- 🧬 **Amygdala Pre-Assessment** — Onboard emotional vector algorithm (no external dependencies)
- 🔬 **M1 Deterministic Pipeline** — Replayable single-pass closed loop:
  LLM calls → tt_job → gRPC Tentacle → evidence → criteria → JSONL ledger
  (byte-identical replay, zero hardcoding, ADR-0003)
- 🌐 **M1.5 Real Tentacle Connectivity** — `tests/m1_e2e_live.rs` drives the
  pipeline against a real `tentacle --transport grpc` + real fixture plugins
  (manifest+js, SHA-256 pinned); identity_labels / seen_entropy_bloom semantics
  (ADR-0004); run_cycle Execution resolves real tool names (echo fallback)
- 🧩 **Candidate E: run_cycle ↔ pipeline merge** (ADR-0005) — structured
  Reasoning output protocol (`{"calls":[...],"impasse":bool}`) replaces
  `contains("tool_call")` string matching; the six pipeline stages land in the
  cognitive states (Reasoning parses + assembles, Execution executes + records
  evidence, Reflection checks criteria + writes the verdict ledger); all five
  historical run_cycle hardcodings are now config-sourced (`RunCycleConfig`)
- ⏱️ **Injectable Clock & Determinism Clamps** — FakeClock tests, derived trace_id,
  BTreeMap over HashMap, no endpoint leakage
- 🛠️ **Safety-First Execution** — Audited tool calls & immune system interception
- 🚀 **Zero-Dependency Boot** — Runs fully offline without any external services
- ✅ **Full Test Coverage** — 94/94 passing (lib + 5 integration suites) +
  3 live e2e (#[ignore], real Tentacle)

## Project Structure

```
anaphase-helix/
├── Cargo.toml              # Rust package & dependencies
├── config.toml             # Service endpoints & cognitive config
├── proto/
│   └── tentacle.proto      # Vendored Tentacle v1 protocol (verbatim, @ 26bd357)
├── knowledge_base/
│   └── fixture-codex.json  # Criteria rules + retry policy (single source of truth)
├── docs/
│   ├── contracts/          # tt_job.schema.json + fixture-data-shapes.md + reasoning-output.md
│   ├── decisions/          # ADR-0001..0005 (decision records)
│   ├── design/             # candidate-e-recon.md (E-T1 exploration notes)
│   └── PLAN.md / GROWTH.md / DNA.md / RNA.md   # phyt-DNA methodology
├── src/
│   ├── lib.rs              # Core library (public API + modules)
│   ├── agent_loop.rs       # 7-state cognitive engine (DAG-driven)
│   │                       #   + M1.5-T6: Execution resolves real tool name
│   │                       #   + E: structured Reasoning + pipeline merge (ADR-0005)
│   ├── reflex.rs           # Somatic immune system (hard + soft reflex)
│   ├── adapters/           # Adapter traits + Noop fallback + gRPC/HTTP impls
│   │                       #   + M1.5-T5: identity_labels/bloom semantics
│   ├── config.rs           # Configuration loader + RunCycleConfig (zero hardcoding)
│   ├── contract/           # tt_job types + parse_llm_calls (M1)
│   ├── evidence/           # Append-only evidence store (M1)
│   ├── criteria/           # Six pure deterministic checkers (M1)
│   ├── ledger/             # Append-only JSONL verdict ledger + Clock (M1)
│   ├── pipeline/           # Six-stage deterministic pipeline (M1)
│   └── hitl.rs / lifecycle.rs / task_dag.rs / gloves.rs
└── tests/
    ├── common/mod.rs       # Shared MockTentacle + StructuredReasoning stub
    ├── integration_test.rs # Noop adapter + reflex + cognitive cycle (16)
    ├── mind_integration.rs # Mock Mind gRPC closed loop (9)
    ├── mock_tentacle.rs    # M1-T0/T7: adapter roundtrip + 3 branches (4)
    ├── m1_e2e.rs           # M1-T8: MET/UNMET/deterministic replay (3)
    ├── m1_e2e_live.rs      # Real Tentacle e2e (3, #[ignore]) — incl. run_cycle chain
    └── run_cycle_pipeline.rs # Candidate E: run_cycle ↔ pipeline full merge (8)
```

## Quick Start

Run the **7-state cognitive loop** in Noop mode (no external services required):

```bash
cargo run
```

You will see a full cycle:
`Perception → PreAssessment → MemoryRetrieval → Reasoning → ReflexCheck → Execution → Reflection`

## Testing

Run the full suite (**94/94 passing**):
```bash
cargo test
```

Coverage:
- **lib (54)**: adapters, reflex, contract (incl. reasoning-output parsing + job-id derivation),
  evidence, criteria, ledger (incl. RFC3339 rendering), lifecycle, task_dag, gloves
- **integration_test (16)**: Noop adapters, hard/soft reflex, dangerous-action block, cognitive cycle, M1.5-T6 real-tool resolution
- **mind_integration (9)**: mock Mind gRPC closed loop, trace passthrough, budget_tier, P11b actions
- **mock_tentacle (4)**: Tentacle v1 roundtrip, trace_id verbatim, failure branch, transport error
- **m1_e2e (3)**: MET verdict, UNMET + retry_due + reopen scan, deterministic replay (byte-identical)
- **run_cycle_pipeline (8)**: candidate-E full chain (MET/UNMET/no-plan/deterministic replay) +
  run_config-driven behavior (cycle cap, soft-reflex threshold, amygdala vector, mode, placeholder)
- **m1_e2e_live (3, #[ignore])**: real Tentacle gRPC + real fixture plugins (manual integration)

## Architecture

### 7-State Cognitive Cycle (DAG)
```
Perception
    ↳ PreAssessment (Amygdala)
        ↳ MemoryRetrieval
            ↳ Reasoning
                ↳ ReflexCheck (Immunity)
                    ↳ Execution
                        ↳ Reflection
                            ↳ Perception (loop)
```

### M1 Deterministic Pipeline (ADR-0003)

Six independently-testable stages — no giant `run()` blob:

| Stage | Function | Kind |
|---|---|---|
| 1. Parse LLM calls | `contract::parse_llm_calls` | pure |
| 2. Assemble tt_job | `Pipeline::assemble_tt_job` | pure |
| 3. gRPC execute | `Pipeline::execute_calls` | IO (Tentacle v1) |
| 4. Record evidence | `Pipeline::record_evidence` | in-memory |
| 5. Criteria check | `Pipeline::check_results` | pure |
| 6. Verdict ledger | `Pipeline::build_verdict` | in-memory |

Verdict semantics: **MET** closes the job; **UNMET** carries `retry_due`
(`now + base_delay`, from fixture-codex) + `parent_id` lineage for M1.5
cross-session requeue. Reopen = queue consumption (M1.5).

### Candidate E: run_cycle ↔ pipeline mapping (ADR-0005)

The six pipeline stages now land in the cognitive states (ADR-0003 decision 9):

| Pipeline stage | run_cycle state |
|---|---|
| 1. Parse LLM calls | `Reasoning` — `parse_reasoning_output` (structured protocol) |
| 2. Assemble tt_job | `Reasoning` tail — deterministic envelope (job_id = FNV-1a, created_at = clock) |
| 3. gRPC execute | `Execution` — `execute_structured` → `Pipeline::execute_calls` |
| 4. Record evidence | `Execution` tail — `record_evidence` |
| 5. Criteria check | `Reflection` — `Pipeline::check_results` |
| 6. Verdict ledger | `Reflection` tail — `build_verdict` + `ledger.append` |

Reasoning output protocol (see `docs/contracts/reasoning-output.md`):
`{"calls":[...],"impasse":bool}` or a bare `[...]` array — the legacy
`contains("tool_call")` string matching is gone. All five historical run_cycle
hardcodings are config-sourced via `RunCycleConfig` (DNA principle 11).

### Adapter Layer (Pluggable & Fault-Tolerant)

| Adapter            | Purpose                  | Fallback Behavior               |
|--------------------|--------------------------|----------------------------------|
| MemoryAdapter      | Helix-Mind access        | Empty memory results (fail-open) |
| ReasoningAdapter   | LLM inference (HTTP)     | Dummy response                   |
| ToolAdapter        | Tentacle execution (gRPC v1) | Unavailable stub             |
| SafetyAdapter      | Tuck security audit      | Allow all actions                |
| UiAdapter          | Cellrix terminal UI      | Silent no-op                     |
| FearAdapter        | Death-prediction engine  | p_death = 0.0 (no risk)          |

### Somatic Reflex Arc

- **Hard Reflex** — O(1) L0 gene-lock check • zero latency • unblockable
- **Soft Reflex** — Lightweight fear-prediction • threshold-based filtering
- **Immunity First** — Reflex runs *before* execution, always

## Configuration

Edit `config.toml` to connect real services:
- Empty endpoints automatically use Noop adapters
- Models, safety rules, and thresholds all configurable
- Criteria rules & retry policy: `knowledge_base/fixture-codex.json` (zero hardcoding)

## Philosophy

Anaphase implements the Helix Design Philosophy + 3 core exoskeleton axioms:

- **Axiom A**: The exoskeleton protects the soul — it never replaces will
- **Axiom B**: Minimal compute, on-demand activation, fear is endogenous
- **Axiom C**: Graceful degradation, standalone runtime, inherited immunity

Plus **DNA principle 11 — Zero Hardcoding**: every literal (thresholds, placeholders,
model names, loop caps) must have a source — config / contract / derivation. Protocol
optional fields use protocol-default empty values.

## Ecosystem

Anaphase orchestrates a modular, decoupled ecosystem:
- **Helix-Mind**: Memory & consciousness
- **FlowModus**: Reasoning
- **Tentacle**: Tool execution (v1 gRPC protocol vendored)
- **Tuck**: Safety
- **Cellrix**: UI

All communicate via **CommonIntents** — no code-level binding.

## License

Apache License 2.0 — Full open source, commercial-friendly.
