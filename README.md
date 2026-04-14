# Anaphase-Helix

**The Synapse Execution Layer of the Helix Ecosystem**

Anaphase-Helix is the orchestrator and execution engine of Helix. It manages the Agent Loop, tool interception, model routing, and cognitive auditing. The CLI entry point is `ana` (a shorthand for **An**aphase).

---

## 🧬 Core Capabilities (Current)

### ✅ Stable Foundation (v0.1.0)

- **Manual `.env` Configuration Loading**: Guaranteed loading of environment variables regardless of working directory or editable install quirks.
- **Four‑Brain Model Routing**:
  - `ANA_AMYGDALA_MODEL` – Lightweight model for priority/emotion assessment.
  - `ANA_LEFT_BRAIN_MODEL` – Logical/code execution model.
  - `ANA_RIGHT_BRAIN_MODEL` – Deep reasoning/creative model.
  - `ANA_CEREBELLUM_MODEL` – Reserved for future action‑timing tasks (configurable, not yet used).
- **Synapse (Tool Execution Layer)**: Successor to Harness; validates schemas, enforces L0 gene lock, and executes CLI tools.
- **Mock Backend**: Toggleable via `ANA_MOCK_MODE` for testing without a real LLM.
- **HXR Audit Logging**: Structured JSONL logs of every cognitive step.
- **Amygdala Module**: Priority scoring, emotion tagging, and intent classification (`chat`, `knowledge_retrieval`, `social_graph_read`, `task`).
- **L0 Gene Lock**: Core behavioral rules loaded from `config/gene_lock.md` and injected into all prompts.
- **L1 Self Portrait**: Loads core traits and reply style from `knowledge_base/self.md`.
- **Social Graph Awareness**: Detects known persons in tasks and injects relevant context from `knowledge_base/social_graph.md`.
- **Dynamic Context Assembly**: Builds conversation style based on detected emotion, and loads memory/social context on demand.

### 🚧 Planned / In Progress

- `ana kb` subcommands – Knowledge base management (requires Helix‑Mind).
- `ana tentacle` – External search integration.
- `ana queue` – Multi‑level task scheduling (Q0–Q3).
- `ana lock` – Gene lock hot‑reload.
- Full DuckDB vector/FT indexing (currently using placeholder memory retrieval).

---

## 🚀 Quick Start

### 1. Environment Setup

```bash
# Create and activate virtual environment
python -m venv .venv
source .venv/bin/activate   # Windows: .venv\Scripts\activate

# Install in editable mode with dev dependencies
pip install -e ".[dev]"

# Copy configuration template
cp .env.example .env
# Edit .env with your actual Tuck endpoint and model names
```

### 2. Verify Installation

```bash
# The CLI command is 'ana' (shorthand for Anaphase)
ana --help
ana --version
```

### 3. Run a Task

```bash
# Use mock backend (default in .env.example, no real LLM required)
ana run "What is 2+2?"

# Disable mock mode to call real LLMs (set ANA_MOCK_MODE=false in .env)
ana run "Explain the factory pattern in Python"

# Direct chat mode (bypass tool loop, pure conversation)
ana run "Tell me a joke" --direct

# JSON output
ana run "echo hello" --json
```

---

## 📁 Command Tree (Current)

```
ana
├── run <task>           # Start Agent Loop
├── trace <session_id>   # Replay inference trace from HXR logs
├── stats                # Show session statistics
├── --version
└── --help

# Planned (not yet implemented):
#   kb, queue, tentacle, lock
```

---

## ⚙️ Configuration

All settings are controlled via `.env`. An example template is provided as `.env.example`.

```ini
# Tuck Model Gateway
TUCK_ENDPOINT=http://10.0.0.54:8686
TUCK_API_KEY=your-api-key

# Helix-Mind Memory Service
HELIX_MIND_ENDPOINT=http://10.0.0.2:8020

# Four Brain Models (adjust to your Tuck available models)
ANA_AMYGDALA_MODEL=Qwen3.5-2B-IQ4_NL.gguf
ANA_CEREBELLUM_MODEL=Qwen3.5-2B-IQ4_NL.gguf
ANA_LEFT_BRAIN_MODEL=Qwen2.5.1-Coder-7B-Instruct-Q4_K_M.gguf
ANA_RIGHT_BRAIN_MODEL=DeepSeek-R1-0528-Qwen3-8B-IQ4_NL.gguf

# Embedding Model (pure vectorization, NO reasoning)
ANA_EMBEDDING_MODEL=Qwen3.5-2B-IQ4_NL.gguf

# Development / Testing
ANA_MOCK_MODE=true        # Set to false for real LLM calls

# Paths
ANA_HXR_DIR=./memory_dag/sessions
ANA_GENE_LOCK_PATH=./knowledge_base/l0_gene_lock.md
ANA_LOG_LEVEL=INFO
```

---

## 🧠 Knowledge Base Files

| File | Purpose |
|:---|:---|
| `config/gene_lock.md` | L0 immutable behavioral constitution. |
| `knowledge_base/self.md` | L1 self‑portrait (core traits and reply style). |
| `knowledge_base/social_graph.md` | Social network of known persons (entities and properties). |
| `config/system_prompt.md` | Template for task‑mode prompts. |

These files are **Markdown with YAML frontmatter**. Example `self.md`:

```markdown
---
core_traits: ["helpful", "precise", "symbiotic"]
reply_style: "concise and professional"
---
# Helix Self Portrait
(Content can be expanded later)
```

---

## 🏗️ Architecture Overview

```
User Command
     │
     ▼
┌─────────────┐
│   ana CLI   │  (manual .env parsing, Settings construction)
└──────┬──────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────┐
│                       Agent Loop                             │
│  ┌──────────┐   ┌─────────┐   ┌─────────┐   ┌───────────┐  │
│  │ Perceive │ → │  Think  │ → │   Act   │ → │  Observe  │  │
│  │(Amygdala)│   │(LLM call)│   │(Synapse)│   │(Update ctx)│  │
│  └──────────┘   └─────────┘   └─────────┘   └───────────┘  │
└─────────────────────────────────────────────────────────────┘
       │                              │
       ▼                              ▼
┌─────────────┐              ┌─────────────────┐
│ Tuck Backend│              │  Tool Registry  │
│ (or Mock)   │              │ (YAML + Pydantic)│
└─────────────┘              └─────────────────┘
```

- **Amygdala**: Assesses priority, emotion, and intent.
- **Synapse**: Validates tool calls, checks gene lock, executes CLI commands.
- **HXR Logger**: Records every step in JSONL for full traceability.

---

## 🛠️ Development

```bash
# Format and lint
ruff format .
ruff check .

# Run tests (when available)
pytest
```

**Important**: Due to known issues with editable installs and `.env` loading, the project currently uses a manual `.env` parser in `cli.py`. When the project is deployed via standard `pip install`, the loader can be simplified to rely on `pydantic-settings` automatic loading. See comments in `ana/cli.py` for restoration instructions.

---

## 📚 Related Projects

- **Helix-Mind** – Memory microservice (DuckDB + vector search).
- **Helix-Tentacle** – External perception & search encapsulation.
- **Tuck** – Model gateway (reused, already stable).

---

## 📄 License

MIT
