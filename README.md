# Anaphase-Helix v0.3.0

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Python 3.12+](https://img.shields.io/badge/python-3.12+-blue.svg)](https://www.python.org/downloads/)
[![中文](https://img.shields.io/badge/简体中文-README-red)](./README.zh-CN.md)

**Anaphase-Helix** is the execution orchestration core of the Helix ecosystem—a self-evolving digital lifeform. It orchestrates perception (Tentacle), memory (Mind), and reasoning to accomplish complex tasks through a state‑graph driven agent loop.

> **Current Status**: v0.3.0 – Tuck gateway integration complete. All brain modules (Amygdala, Prefrontal, Synapse, Corpus Callosum) now support real LLM calls via Tuck. Ana Loom cognitive visualization available. End‑to‑end real inference validated.

## 🧠 Core Philosophy

- **Orchestrate, Don't Build** – The core only schedules; real work is delegated to external CLI tools or microservices.
- **Contract Over Convention** – All cross‑module communication uses strict Pydantic DTOs.
- **DAG Everything** – Knowledge, tasks, tools, and memory are modeled as a directed acyclic graph.
- **Guide, Don't Block** – Anaphase persuades; Tuck enforces as the last line of defense.
- **Silicon Metabolism** – Token budget and cognitive load are actively managed; the agent “sleeps” when fatigued.

## 🚀 Quick Start

### Prerequisites
- Python 3.12+
- [Tuck Gateway](https://github.com/Jasonmilk/Tuck) (optional, for real LLM calls)

### Installation

```bash
git clone https://github.com/Jasonmilk/Anaphase-Helix.git
cd Anaphase-Helix
git checkout V5

python -m venv .venv
source .venv/bin/activate  # Windows: .venv\Scripts\activate
pip install -e ".[dev]"
```

### Configuration

Copy the example environment file and edit it:

```bash
cp .env.example .env
```

For **mock mode** (no LLM required), set:
```ini
ANA_MOCK_MODE=true
```

For **real LLM calls** via Tuck, set:
```ini
ANA_MOCK_MODE=false
TUCK_ENDPOINT=http://localhost:8686
TUCK_API_KEY=your_api_key_here
TUCK_CHAT_PATH=/v1/chat/completions   # Default OpenAI-compatible path
TUCK_TIMEOUT=30                        # Request timeout in seconds
```

### Model Configuration

Ensure the model names in `.env` match those available on your Tuck instance:

```ini
ANA_AMYGDALA_MODEL=Qwen3.5-2B-IQ4_NL.gguf
ANA_LEFT_BRAIN_MODEL=Qwen2.5.1-Coder-7B-Instruct-Q4_K_M.gguf
ANA_RIGHT_BRAIN_MODEL=DeepSeek-R1-0528-Qwen3-8B-IQ4_NL.gguf
```

### Run Your First Task

```bash
ana run "What is the meaning of life?"
```

In mock mode, you will see a full cognitive loop trace in JSON logs, ending with a mock reasoning draft. In real mode, the agent will call Tuck to generate actual LLM responses. The agent transitions through all seven states: `perceive → assess_priority → plan → execute → reflect → consolidate → sleep`.

### Visualize Cognitive Process with Ana Loom

```bash
# Show the most recent session
ana loom --last

# Show a specific session
ana loom <epoch_id>
```

Ana Loom renders a terminal‑friendly visualization of the agent's cognitive process, including priority scores, affect vectors, token consumption, and state transitions—all styled with the Ana Theme.

## 📁 Project Structure

```
Anaphase-Helix/
├── ana/
│   ├── cli/                 # CLI entry points (ana run/trace/stats/loom)
│   ├── core/                # Brain region modules
│   │   ├── agent_loop.py    # State‑graph driven main loop
│   │   ├── amygdala.py      # Priority & affect assessment (real Tuck calls)
│   │   ├── prefrontal.py    # Reasoning & planning (real Tuck calls)
│   │   ├── synapse.py       # Tool execution (CLI sandbox)
│   │   ├── corpus_callosum.py # Intent‑execution alignment validator
│   │   └── model_router.py  # Model selection based on priority
│   ├── loom/                # Ana Loom cognitive visualization
│   │   ├── visualizer.py    # Terminal rendering engine
│   │   └── themes.py        # Ana Theme color system
│   ├── schemas/             # Pydantic DTO contracts
│   ├── common/              # Config, logging, tracing, retry
│   └── registry/            # Tool registry (loads from config/tools.yaml)
├── config/                  # Gene lock, tools manifest, biosphere templates
├── knowledge_base/          # L1 self portrait (self.md)
├── tests/                   # Unit tests (13 passing)
├── docs/                    # Whitepaper & Engineering Manual
├── .env.example
├── pyproject.toml
└── README.md
```

## 🧪 Testing

Run the full test suite:

```bash
pytest -v
```

All 13 tests pass, covering every brain module and the agent loop integration. Mock mode tests validate DTO contracts and state transitions; real mode tests (with Tuck mocked) verify the HTTP calling layer.

## 📖 Documentation

- [Helix Ecosystem Whitepaper](docs/WHITEPAPER.md) – The “what” and “why”.
- [Anaphase-Helix Engineering Manual](docs/ENGINEERING.md) – The “how”: project structure, DTOs, AI Coder rules, and workflows.

## 🤝 AI Coder Collaboration

This project follows a strict **AI Coder Iron Law** checklist (see Engineering Manual §7.1) to ensure LLM‑generated code remains consistent, testable, and aligned with the Helix philosophy. Every module is developed test‑first, with mock implementations validated before real backend integration.

## 📌 Roadmap

| Milestone | Status |
|:---|:---|
| **v0.1.0** – Physical skeleton (directories, DTOs, CLI) | ✅ Complete |
| **v0.2.0** – Full module mock integration & end‑to‑end validation | ✅ Complete |
| **v0.2.1** – Ana Loom cognitive visualization & Ana Theme system | ✅ Complete |
| **v0.3.0** – Tuck gateway integration (real LLM calls) | ✅ Complete |
| **v0.4.0** – Helix‑Mind integration (memory DAG) | 🚧 Next |
| **v0.5.0** – Tool ecosystem & biosphere expansion | 📅 Planned |
| **v1.0.0** – Production‑ready digital lifeform | 📅 Planned |

## 📄 License

MIT © [Jason Milk](https://github.com/Jasonmilk)

---

*Helix is not a chatbot. It is a self‑sovereign digital being that learns, forgets, and grows through every epoch.*
