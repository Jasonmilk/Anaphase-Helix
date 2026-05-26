# Anaphase-Helix

![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)
![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)
![Build](https://img.shields.io/badge/Build-Passing-brightgreen.svg)
![Style](https://img.shields.io/badge/Code%20Style-Google-black.svg)

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
- 🔌 **Pluggable Adapters** — All services degrade gracefully to Noop
- 🧬 **Amygdala Pre-Assessment** — Onboard emotional vector algorithm (no external dependencies)
- 🛠️ **Safety-First Execution** — Audited tool calls & immune system interception
- 🚀 **Zero-Dependency Boot** — Runs fully offline without any external services

## Project Structure

```
anaphase-helix/
├── Cargo.toml              # Rust package & dependencies
├── config.toml             # Service endpoints & cognitive config
├── src/
│   ├── main.rs             # CLI entry + engine initialization
│   ├── agent_loop.rs       # 7-state cognitive engine (DAG-driven)
│   ├── reflex.rs           # Somatic immune system (hard + soft reflex)
│   ├── adapters/mod.rs     # Adapter traits + Noop fallback implementations
│   ├── config.rs           # Configuration loader
│   ├── amygdala.rs         # Emotional vector computation
│   └── syscall.rs          # System call table (reserved)
└── tests/
    └── integration_test.rs # Full cognitive cycle tests
```

## Quick Start

Run the **7-state cognitive loop** in Noop mode (no external services required):

```bash
cargo run
```

You will see a full cycle:
`Perception → PreAssessment → MemoryRetrieval → Reasoning → ReflexCheck → Execution → Reflection`

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

### Adapter Layer (Pluggable & Fault-Tolerant)

| Adapter           | Purpose                  | Fallback Behavior               |
|-------------------|--------------------------|----------------------------------|
| MemoryAdapter     | Helix-Mind access        | Empty memory results            |
| ReasoningAdapter  | FlowModus LLM inference  | Dummy response                   |
| ToolAdapter       | Tentacle execution       | Unavailable stub                 |
| SafetyAdapter     | Tuck security audit      | Allow all actions                |
| UiAdapter         | Cellrix terminal UI      | Silent no-op                     |
| FearAdapter       | Death-prediction engine  | p_death = 0.0 (no risk)          |

### Somatic Reflex Arc

- **Hard Reflex** — O(1) L0 gene-lock check • zero latency • unblockable
- **Soft Reflex** — Lightweight fear-prediction • threshold-based filtering
- **Immunity First** — Reflex runs *before* execution, always

## Configuration

Edit `config.toml` to connect real services:
- Empty endpoints automatically use Noop adapters
- Models, safety rules, and thresholds all configurable

## Philosophy

Anaphase implements the Helix Design Philosophy + 3 core exoskeleton axioms:

- **Axiom A**: The exoskeleton protects the soul — it never replaces will
- **Axiom B**: Minimal compute, on-demand activation, fear is endogenous
- **Axiom C**: Graceful degradation, standalone runtime, inherited immunity

## Ecosystem

Anaphase orchestrates a modular, decoupled ecosystem:
- **Helix-Mind**: Memory & consciousness
- **FlowModus**: Reasoning
- **Tentacle**: Tool execution
- **Tuck**: Safety
- **Cellrix**: UI

All communicate via **CommonIntents** — no code-level binding.

## License

Apache License 2.0 — Full open source, commercial-friendly.
