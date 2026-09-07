# Anaphase-Helix

![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)
![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)
![Build](https://img.shields.io/badge/Build-Passing-brightgreen.svg)
![Style](https://img.shields.io/badge/Code%20Style-Google-black.svg)
[![Tests](https://img.shields.io/badge/tests-211%2F211%20passed-green)](#)

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
- 🛤️ **Rails** (ADR-0018) — external human-authored knowledge rails
  (statutes / SOPs): deterministic mddag index (SHA-256 version-frozen,
  dangling-link builds fail) + deterministic navigation (CJK bigrams, no
  embeddings) + citation contract (`verify_reference`: verbatim quote +
  visited provenance) + graceful refusal (`NO_RAIL_CONTENT`) — Helix may
  only select an existing rail edge, never synthesize one. Rail hits short-
  circuit Reasoning (0 tokens): the answer is assembled verbatim from the
  injected nodes + node ids — no LLM, no synthesis possible. Read-only at
  the type level (`RailScope::Read`). Demo kb: `knowledge_base/rails/demo/`
- 📡 **Stage Event Bus** (ADR-0019) — append-only process white-box: the
  six pipeline stages emit begin/end/verdict events on one deterministic
  trace id (derived job id); pull via `GET /v1/agent/events?after=N`
  (incremental cursor, no push). Events = process, ledger = fact,
  evidence = support. Capacity from codex contract (zero hardcoding)
- 🚗 **CI-144 Transport Layer** (ADR-0017) — `--stdio` speaks the ecosystem's
  common dialect: CIB/1.0 handshake → MessagePack frames (LE u32 length prefix)
  → Manifest (first frame) → 1s snapshot push → ActionRequest/Response
  (`status` / `send_message` through the real run_cycle). Vendored protocol
  types in `src/ci144/` (serde field-for-field with Cellrix); protocol layer
  stays business-free via an injected action callback. Accepts both the native
  `--stdio` and the ecosystem launcher convention `--mode stdio` (Cellrix
  `--exec` speaks one launch contract to every agent). Live-verified against
  the real binary (`cargo test --test ci144_live -- --ignored`) and through
  the real Cellrix cockpit (`manifest`/`snapshot`/`action` subcommands)
- 🌐 **M1.5 Real Tentacle Connectivity** — `tests/tentacle_live.rs` drives the
  pipeline against a real `tentacle --transport grpc` + real fixture plugins
  (manifest+js, SHA-256 pinned); identity_labels / seen_entropy_bloom semantics
  (ADR-0004); run_cycle Execution resolves real tool names (echo fallback)
- 🌙 **P10 Cognitive Craft + Wake-up, live-verified** — `tests/mind_live.rs`
  drives the REAL `helix-mind-cli` binary over the real wire: `craft`
  (deterministic trace_id, 0-token synthesis), `ana_wakeup` agenda poll and
  `helix_consolidate hibernate` (ADR-0031/0032). Requires a built Mind binary:
  `cargo build -p helix-mind-cli` (../Helix-Mind), then
  `cargo test --test mind_live -- --ignored`
- 🧩 **Candidate E: run_cycle ↔ pipeline merge** (ADR-0005) — structured
  Reasoning output protocol (`{"calls":[...],"impasse":bool}`) replaces
  `contains("tool_call")` string matching; the six pipeline stages land in the
  cognitive states (Reasoning parses + assembles, Execution executes + records
  evidence, Reflection checks criteria + writes the verdict ledger); all five
  historical run_cycle hardcodings are now config-sourced (`RunCycleConfig`)
- 🧭 **O-1: 0-Token Triage + Ecosystem Lights** (ADR-0016) — the
  orchestration philosophy lands physically: `!tool {"json":...}` structured
  commands are parsed in Perception and skip the LLM entirely (proven by a
  counting reasoning adapter — zero calls); `probe_ecosystem` maps config
  endpoints to physical lights once per task (TCP connect / UDS file,
  fail-open; Cellrix = Native glove); AgentContext + AgentSnapshot carry the
  lights so the cockpit shows *what the body has in hand*; free text still
  reaches the LLM (no regression)
- 🔄 **Single-Period Primitive** (ADR-0016 D1) — `run_cycle()` is an atomic
  primitive: one walk of the 7-state DAG returns `CycleOutcome{done,success,
  impasse}`; the caller owns the looping policy (cap from config as the
  anti-infinite-loop fuse; period-step cap derived from the enum length).
  Module renamed `agent_loop` → `run_cycle` — the name now matches the
  semantics (body = AgentLoop type, heartbeat = run_cycle, life = caller loop)
- 🧭 **Candidate F: Session-as-Experience** (ADR-0006) — Helix has no "session
  container": a conversation is an *episode* it lives (L3 experience). Each
  reflection write carries `{"episode":"ep-<id>#<step>"}` provenance; closing
  an episode writes a digest (id/turns/first-input) through the existing
  remember channel. Three modes — `Drive` (human at the wheel, Noop mind),
  `Partner` (memory-bearing collaboration, default), `Survive` (Mind
  autonomous, reserved) — are config-sourced, with zero runtime branches
  (assembly-time adapter choice does the isolation)
- 🔁 **Replay-Guard Fingerprint + Bootstrap Wiring** (ADR-0007) — `seen_entropy_bloom`
  carries a real deterministic fingerprint (`bl-` + FNV-1a over `{tool}#{params}`, shared primitive)
  instead of the `""` placeholder; configuring `tentacle_endpoint` wires the six-stage pipeline at
  startup (fail-open: empty/unreachable falls back to the legacy echo path)
- ⏱️ **Injectable Clock & Determinism Clamps** — FakeClock tests, derived trace_id,
  BTreeMap over HashMap, no endpoint leakage
- 🛠️ **Safety-First Execution** — Audited tool calls & immune system interception
- 🚀 **Zero-Dependency Boot** — Runs fully offline without any external services
- ⚖️ **O-6: Judge-Backend Selection** (ADR-0024) — the complexity judge
  point is now backend-selectable: `rules` (default, zero tokens) or
  `small_llm` (3B-class classifier via an OpenAI-compatible endpoint, e.g.
  Tuck's local llama); any failure falls back to rules (fail-safe,
  determinism first); fixed a leftover literal pair (10/40) in the old
  complexity heuristic — thresholds now come from `MindConfig` only
- 🧠 **O-5: On-Demand Cognitive Injection** (ADR-0023) — the broken link is
  fixed: memory nodes retrieved in MemoryRetrieval are folded into the
  Reasoning prompt (budget-capped `memory_inject_chars`, 25-round context
  grows ~zero; fold marker = "more memory on demand"); demo input is
  source-ized via `--input` / `[anaphase] smoke_input` (zero-hardcoding)
- 🧠 **O-4: Cognitive-Craft Trigger Verified** (ADR-0022) — the partner-mode
  chain `MemoryRetrieval → helix_query → suggested_actions → orchestration` is
  proven at the gRPC wire layer against the mock Mind; Drive mode never
  contacts Mind (assembly-gated, zero runtime branch); all 12 adapter literals
  moved to `MindConfig` (`[anaphase.mind]`, DNA principle 11 zero-hardcoding)
- ✅ **Full Test Coverage** — 211/211 passing (lib + integration suites + rails + stage events + mind trigger + wakeup trigger + memory injection + judge backends) incl. security gate + Tuck gate + D'-4 live + cockpit snapshot + bootstrap env + CI-144 transport + P10 live) +
  3 live e2e + 1 CI-144 live probe (#[ignore], real Tentacle / real binary)

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
│   ├── ci144/              # CI-144 transport: vendored protocol types + server loop (ADR-0017)
│   └── hitl.rs / lifecycle.rs / task_dag.rs / gloves.rs
└── tests/
    ├── common/mod.rs       # Shared MockTentacle + StructuredReasoning stub
    ├── integration_test.rs # Noop adapter + reflex + cognitive cycle (16)
    ├── mind_integration.rs # Mock Mind gRPC closed loop (9)
    ├── mock_tentacle.rs    # M1-T0/T7: adapter roundtrip + 3 branches (4)
    ├── pipeline_e2e.rs     # MET/UNMET/deterministic replay (3)
    ├── tentacle_live.rs    # Real Tentacle e2e (3, #[ignore]) — incl. run_cycle chain
    ├── run_cycle_pipeline.rs # Candidate E: run_cycle ↔ pipeline full merge (8)
    ├── ci144_transport.rs  # CI-144 protocol suite: handshake/frame/projection/duplex (6)
    └── ci144_live.rs       # Real-binary CI-144 roundtrip (1, #[ignore])
```

## Quick Start

**One command, full stack** (candidate G-4 ADR-0011 + G-5 ADR-0012 + G-6 ADR-0013 + G-7 ADR-0015) — Tentacle (gRPC + fixture plugins) → **Mind (subconscious: `helix-mind-cli`, minimal default config auto-written to `.helix/mind/` once, gene_lock from the repo example; port auto-shifted on collision)** → Anaphase (endpoints injected **process-globally** via env — daemon, cockpit agent and web panel all inherit; `config.toml` untouched) → **WebUI panel (`cellrix-web`, :8080 by default, `WEB_PORT` overrides)** → readiness probes → **interactive menu** (tty): 1 open cockpit (Enter) / 2 status / 3 config hints / **4 configure LLM (guided base_url/model/api_key, key hidden, auto-backup config.toml.bak)** / 5 exit (q). **The cockpit's stdio agent IS Anaphase itself** (`anaphase --mode stdio`, same full assembly as the daemon — Mind + Tentacle + pipeline + rails): the driver talks to the real Helix, not a mock. **Chat**: focus the `Send message` button (snapshot exposes `needs_input`) and press Enter → a text input opens at the bottom; type, Enter sends (one cognitive period: Mind retrieval → LLM reasoning → Tentacle execution → ledger), Esc cancels. The reply renders in the input row. Config is loaded from an absolute path (`ANAPHASE_CONFIG`, injected by `up`) so any working dir works. One command, then choices only — no commands to remember. Non-tty (scripts/CI) degrades to plain hold-until-Ctrl+C.

```bash
cargo run --bin up               # full stack: tentacle + mind + anaphase, probes all ready
cargo run --bin up -- --cockpit  # + Cellrix cockpit TUI in the foreground
```

Prereqs (build once, then `up` just works):
- `helix-tentacle`: `cargo build` → `target/debug/tentacle`
- `Helix-Mind`: `cargo build -p helix-mind-cli` → `target/debug/helix-mind-cli` (fail-open: missing Mind → Anaphase runs without subconscious, never blocks)
- `Cellrix`: `cargo build` → `target/debug/cellrix-cli` + `target/debug/mock-agent`

All knobs optional: `HELIX_TENTACLE` (binary path), `HELIX_FIXTURES_DIR`,
`HELIX_TENTACLE_PORT`; Anaphase receives `ANAPHASE_TENTACLE_ENDPOINT` /
`ANAPHASE_REASONING_ENDPOINT` via env (12-factor, config.toml untouched).
Fail-open: missing Tentacle binary → Noop offline mode, never blocks.
Exit: Ctrl+C (whole process group receives SIGINT together).

**Single-process Noop loop** (no external services — shows the 7-state cycle):

```bash
cargo run
```

You will see a full cycle:
`Perception → PreAssessment → MemoryRetrieval → Reasoning → ReflexCheck → Execution → Reflection`

### Fail-closed Tuck gate (2026-09-07)

When `tuck_endpoint` is configured, Anaphase refuses to reason while Tuck
is unreachable — **Tuck down = Helix stops thinking** (SPOF explicitly
accepted, 网关可用性换审计完整性). The process stays alive to keep showing
the honest state on the panel; only reasoning halts:

```
⚠️ Tuck 不在岗，已停止工作：tuck unreachable (http://127.0.0.1:60052): ...
   请恢复 Tuck（如运行 `tuck` 网关）后重新运行本命令。
```

`/v1/health` includes the `tuck` check (configured → probed; empty → honest
`not configured`, never judged). Unconfigured Tuck = no gate (按需驱动).

### One-to-one binding (2026-09-07) — Anaphase is the challenger

`POST /v1/bind/start` mints a 6-digit one-time pairing code (10 min);
`POST /v1/bind/confirm {pairing_code}` (human confirmed, HITL) mints
`device_id + secret` persisted to `~/.cellrix/anaphase-identity.json`
(0600). Once bound, every cap_http endpoint except `/v1/bind/*` and
`/v1/health` requires `Authorization: Bearer v1.<id>.<ts>.<nonce>.<hmac>`:
known device + ±60s window + one-time nonce + HMAC (防重放双保险). Unbound
= open, honestly reported (`/v1/bind/status`). The client half lives in
`~/.cellrix/identity.toml` (0600) — never in git.

## Testing

Run the full suite (**211/211 passing**):
```bash
cargo test
```

Coverage:
- **lib (56)**: adapters, reflex, contract (incl. reasoning-output parsing + job-id/episode-id/bloom
  derivation), evidence, criteria, ledger (incl. RFC3339 rendering), lifecycle, task_dag, gloves
- **integration_test (16)**: Noop adapters, hard/soft reflex, dangerous-action block, cognitive cycle, M1.5-T6 real-tool resolution
- **mind_integration (9)**: mock Mind gRPC closed loop, trace passthrough, budget_tier, P11b actions
- **mock_tentacle (4)**: Tentacle v1 roundtrip, trace_id verbatim, failure branch, transport error
- **pipeline_e2e (3)**: MET verdict, UNMET + retry_due + reopen scan, deterministic replay (byte-identical)
- **run_cycle_pipeline (8)**: candidate-E full chain (MET/UNMET/no-plan/deterministic replay) +
  run_config-driven behavior (cycle cap, soft-reflex threshold, amygdala vector, mode, placeholder)
- **ci144_transport (6)**: CIB/1.0 handshake (accept/reject), frame round-trip,
  AgentSnapshot→SemanticSnapshot projection shape, vendored serde shape,
  full duplex session (Manifest → push → status/send_message/unknown actions)
- **ci144_live (1, #[ignore])**: real `anaphase --stdio` binary — handshake →
  Manifest → snapshot push → actions → clean EOF exit
- **episode_lifecycle (10)**: candidate-F experience boundary (deterministic episode id via shared
  FNV-1a, begin/end lifecycle, auto-close of previous episode, L3 provenance on reflection notes,
  verbatim legacy writes, mode serde roundtrip + config load)
- **replay_guard (4)**: candidate-D' partial — real entropy fingerprint on the wire (`bl-` + FNV-1a over
  `{tool}#{params}`, replay-stable), `resolve_pipeline` fail-open (empty/unreachable endpoint -> None),
  configured endpoint wires the pipeline (ADR-0007)
- **tentacle_live (3, #[ignore])**: real Tentacle gRPC + real fixture plugins (manual integration)

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

### Engram body trace (reasoning round trips, 2026-09-07)

The Tuck audit chain stores *metadata* (it never touches bodies — that would
be a sensitive data lake). The **body** of every reasoning call (prompt +
response) is recorded here, on the side that constructs the prompt:

```toml
[anaphase]
reasoning_trace_path = "reasoning-trace.jsonl"   # opt-in; None = off
reasoning_trace_max_chars = 4096                 # post-redaction truncation
reasoning_redact_patterns = []                   # extra credential literals
```

- **Redacted before disk** (sk- / Bearer / api_key= / token= / password= +
  config extras) — the 2026-09-07 key-leak audit is the reason this exists
- **Keyed by the derived job id** (`trace_id = derive_job_id(input)`) — the
  same key the audit chain and ledger carry, so Cellrix's Engram view joins
  body + chain + verdict for one round
- Timestamps from the injected Clock (deterministic replay)

**Read-back endpoint (full-text replay, 2026-09-07)** — the cap_http server
serves the bodies on demand (redacted on write, read-only, no hot index):

```text
GET /v1/trace?trace_id=run-...&limit=20   # one round's bodies (or newest N)
```

Cellrix's Engram detail panel calls this per selected row — click an audit
entry and read the exact prompt + response of that round (silicon/carbon
side by side, no ambiguity).

**Self-check endpoint (ecosystem watch-table, 2026-09-07)** — `/v1/health`
reports the physical readiness of Anaphase's own organs:

```text
GET /v1/health
# {"ok":true,"checks":[{"name":"trace","configured":true,"ok":true,...},...]}
```

Every check is config-derived and probed — nothing guessed (0 hardcoding):
write targets (trace/ledger/session) check their parent dir; six endpoints
are TCP-probed with a deterministic 2s timeout (blocking connect on a helper
thread — `connect_timeout` is flaky on macOS loopback); judge backend and
cap_http report their config source; credentials report presence only, the
value is never exposed. Empty-string config = unconfigured = not judged.

One source, three consumers: the Cellrix panel renders it (human watch),
Helix-Mind reads it on demand before acting (AI watch-table, 按需看表 —
never polled in a loop), operators curl it. No second copy of state.

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
