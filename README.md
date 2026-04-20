# Anaphase-Helix

Execution orchestration core of Helix ecosystem.

## Overview

Anaphase-Helix is the execution layer of the Helix digital lifeform, responsible for:
- CLI entry point and command routing
- State-graph driven agent loop
- Brain region module orchestration
- Tool execution and validation
- Cross-service tracing and observability

## Quick Start

### 1. Clone the repository
```bash
git clone https://github.com/Jasonmilk/Anaphase-Helix.git
cd Anaphase-Helix
```

### 2. Setup virtual environment
```bash
python -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate
```

### 3. Install dependencies
```bash
pip install -e ".[dev]"
```

### 4. Configure environment
```bash
cp .env.example .env
# Edit .env with your configuration
```

### 5. Run your first task
```bash
ana run "Hello, Helix!"
```

## Documentation
- [Helix Ecosystem WhitePaper](./docs/WHITEPAPER.md)
- [Engineering Manual](./docs/ENGINEERING.md)

## License
MIT
