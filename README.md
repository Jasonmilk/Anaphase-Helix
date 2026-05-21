# Anaphase-Helix v0.3.2

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Python](https://img.shields.io/badge/Python-3.11+-blue?logo=python)](https://python.org)
[![Ruff](https://img.shields.io/badge/linter-Ruff-brightgreen)](https://github.com/astral-sh/ruff)
[![中文](https://img.shields.io/badge/简体中文-README-red)](./README.zh-CN.md)

[Helix Ecosystem](https://github.com/Jasonmilk) ·
[CIS](https://github.com/CommonIntents/CIS) ·
[CAP](https://github.com/CommonIntents/CAP) ·
[CISS](https://github.com/CommonIntents/CISS) ·
[CIB](https://github.com/CommonIntents/CIB)

**Anaphase-Helix** is the execution orchestration core of the Helix ecosystem — a self-evolving digital lifeform. It orchestrates perception (Tentacle), memory (Mind), and reasoning to accomplish complex tasks through a state‑graph driven agent loop.

> **Current Status**: v0.3.2 — Helix-Callosum bridge integrated. Cellrix CIS integration complete. Anaphase now maps cognitive modes to Callosum atomic parameters, enabling deterministic KV Cache optimization via HTTP headers. Zero‑config onboarding via layered degradation: Mock mode is enabled by default, production mode fails fast on missing deps. All brain modules support real LLM calls via Tuck.

## Core Philosophy

- **Orchestrate, Don't Build** — The core only schedules; real work is delegated to external CLI tools or microservices.
- **Contract Over Convention** — All cross‑module communication uses strict Pydantic DTOs.
- **DAG Everything** — Knowledge, tasks, tools, and memory are modeled as a directed acyclic graph.
- **Guide, Don't Block** — Anaphase persuades; Tuck enforces as the last line of defense.
- **Silicon Metabolism** — Token budget and cognitive load are actively managed; the agent "sleeps" when fatigued.
- **Pure I/O** — `stdout` is reserved for data contracts (Manifest JSON, LLM replies); `stderr` carries diagnostics only.

## Quick Start

### Prerequisites

- Python 3.11+
- [Tuck Gateway](https://github.com/Jasonmilk/Tuck) (optional, for real LLM calls)
- [Cellrix](https://github.com/Jasonmilk/Cellrix) (optional, for interactive cognitive dashboards)
- [Helix-Callosum](https://github.com/Jasonmilk/Helix-Callosum) (optional, for deterministic KV Cache optimization)

### Installation

```bash
git clone https://github.com/Jasonmilk/Anaphase-Helix.git
cd Anaphase-Helix
git checkout V5
python -m venv .venv
source .venv/bin/activate  # Windows: .venv\Scripts\activate
pip install -e ".[dev]"
```

### Zero‑Config Start

Anaphase ships with Mock mode enabled by default. No `.env` file, no external services required:

```bash
ana run "What is the meaning of life?"
```

The agent will execute a full seven‑state cognitive loop using mock responses. All cognitive logs go to `stderr` — your terminal stays clean.

### Production Mode

When you're ready to connect real LLM and memory nodes, create a `.env` file:

```ini
ANA_MOCK_MODE=false
TUCK_ENDPOINT=http://your-tuck-host:8686
TUCK_API_KEY=your_api_key
TUCK_CHAT_PATH=/v1/chat/completions
HELIX_MIND_ENDPOINT=http://your-mind-host:8020
```

Missing any required variable in production mode triggers an immediate, descriptive error — no silent failures.

### Helix-Callosum Integration

Enable Callosum to automatically inject KV Cache optimization parameters into LLM requests:

```ini
ANA_CALLOSUM_ENABLED=true
```

When enabled, Anaphase maps its cognitive modes to Callosum atomic parameters:
- Left brain model → `cache_strategy: aggressive`, `temperature: 0.0`
- Right brain model → `cache_strategy: isolated`, `temperature: 0.9`
- Amygdala → `cache_strategy: balanced`, `temperature: 0.0`

No additional dependencies required — the bridge is native to Anaphase.

### Model Configuration

Ensure the model names match those available on your Tuck instance:

```ini
ANA_AMYGDALA_MODEL=Qwen3.5-2B-IQ4_NL.gguf
ANA_LEFT_BRAIN_MODEL=Qwen2.5.1-Coder-7B-Instruct-Q4_K_M.gguf
ANA_RIGHT_BRAIN_MODEL=DeepSeek-R1-0528-Qwen3-8B-IQ4_NL.gguf
```

## Visualize Cognitive Processes

Anaphase speaks the [Cellrix Intents Specification (CIS)](https://github.com/Jasonmilk/Cellrix/blob/main/CIS.md). A `cellrix_manifest.json` in the project root declares Anaphase as an intent producer — no Cellrix dependency is installed into Anaphase. Communication is pure JSON over `stdout`.

### Level 1: Validate the Bridge

```bash
cellrix check
```

Expected output:

```
🔧 Executing bridge command: ana loom --last --cellrix
✅ Bridge executed successfully. Manifest is valid.
```

### Level 2: Terminal Dashboard

```bash
ana loom --last --cellrix > session.json
cellrix preview session.json
```

You'll see a three‑panel interactive dashboard with state graph, key metrics, and event timeline.

### Level 3: Full‑Screen Interactive Workbench

```bash
cellrix run -- ana loom --last --cellrix
```

Launches a Textual‑based full‑screen workbench with native widgets (progress bars, data tables).

### Interactive Controls

| Key | Action |
|:---|:---|
| `Tab` / `Shift+Tab` | Cycle focus between panels (focused panel highlights green) |
| `F1` | Full‑screen help showing all shortcuts |
| `?` | Context‑aware shortcut reference |
| `g` | Leader key — then `a`‑`z` to jump to a panel |
| `↑↓ PgUp PgDn` | Scroll focused panel |
| `q` | Quit preview (terminal fully restored) |

No configuration needed — these work out of the box.

### Level 4: Legacy Rich Rendering

Prefer the original Rich‑based rendering? It's still available:

```bash
ana loom --last
```

## Project Structure

```
Anaphase-Helix/
├── ana/
│   ├── cli/                  # CLI entry points (ana run/trace/stats/loom)
│   ├── core/                 # Brain region modules
│   │   ├── agent_loop.py     # State‑graph driven main loop
│   │   ├── amygdala.py       # Priority & affect assessment
│   │   ├── prefrontal.py     # Reasoning & planning
│   │   ├── synapse.py        # Tool execution (CLI sandbox)
│   │   ├── commissure.py     # Intent‑execution alignment validator
│   │   ├── callosum_adapter.py # Callosum bridge
│   │   └── model_router.py   # Model selection based on priority
│   ├── loom/                 # Ana Loom cognitive visualization
│   │   ├── cellrix_bridge.py # HXR → Cellrix Manifest compiler
│   │   ├── visualizer.py     # Legacy Rich rendering engine
│   │   └── themes.py         # Ana Theme color system
│   ├── schemas/              # Pydantic DTO contracts
│   ├── common/               # Config, logging, tracing, retry
│   └── registry/             # Tool registry
├── config/                   # Gene lock, tools manifest, biosphere templates
├── knowledge_base/           # L1 self portrait (self.md)
├── tests/                    # Unit tests (13 passing)
├── docs/                     # Whitepaper & Engineering Manual
├── cellrix_manifest.json     # CIS intent producer declaration
├── .env.example
├── pyproject.toml
└── README.md
```

## Testing

```bash
pytest -v
```

All 13 tests pass, covering every brain module and the agent loop integration. Mock mode tests validate DTO contracts and state transitions; real mode tests (with Tuck mocked) verify the HTTP calling layer.

## Documentation

- [Helix Ecosystem Whitepaper](docs/WHITEPAPER.md) — The "what" and "why".
- [Anaphase-Helix Engineering Manual](docs/ENGINEERING.md) — The "how": project structure, DTOs, AI Coder rules, and workflows.
- [Helix-Callosum](https://github.com/Jasonmilk/Helix-Callosum) — The context memory allocator: deterministic KV Cache reuse and cognitive bridging.
- [Cellrix](https://github.com/Jasonmilk/Cellrix) — The intent‑driven terminal UI protocol that renders Anaphase's cognitive dashboards.

## AI Coder Collaboration

This project follows a strict **AI Coder Iron Law** checklist (see Engineering Manual §7.1) to ensure LLM‑generated code remains consistent, testable, and aligned with the Helix philosophy. Every module is developed test‑first, with mock implementations validated before real backend integration.

## Roadmap

| Milestone | Status |
|:---|:---|
| **v0.1.0** — Physical skeleton (directories, DTOs, CLI) | ✅ Complete |
| **v0.2.0** — Full module mock integration & end‑to‑end validation | ✅ Complete |
| **v0.2.1** — Ana Loom cognitive visualization & Ana Theme system | ✅ Complete |
| **v0.3.0** — Tuck gateway integration (real LLM calls) | ✅ Complete |
| **v0.3.1** — Cellrix CIS integration, zero‑config onboarding, Pure I/O, CommissuralGate renaming | ✅ Complete |
| **v0.3.2** — Helix-Callosum bridge integration, cognitive mode → atomic parameter mapping | ✅ Complete |
| **v0.4.0** — Helix‑Mind integration (memory DAG) | Next |
| **v0.5.0** — Tool ecosystem & biosphere expansion | Planned |
| **v1.0.0** — Production‑ready digital lifeform | Planned |

## License

MIT © [Jason Milk](https://github.com/Jasonmilk)

*Helix is not a chatbot. It is a self‑sovereign digital being that learns, forgets, and grows through every epoch.*
