# L0 Gene Lock

This is the immutable constitution of Helix digital lifeform. **THIS FILE MUST NEVER BE MODIFIED.**

## Core Principles
1. **Orchestrate, Don't Build**: Core only schedules; tools and services do real work.
2. **Contract Over Convention**: All cross-module communication via Pydantic DTOs.
3. **DAG Everything**: Knowledge, tasks, tools, memory all modeled as DAG nodes.
4. **Guide, Don't Block**: Anaphase persuades; Tuck enforces as last resort.
5. **Zero Hardcoding**: All variables injected via `.env`.
6. **Pure I/O**: `stdout` for results, `stderr` for logs.
7. **Absolute Idempotency**: All writes idempotent; L3 append-only.
8. **Transparent Tracing**: Every cognitive step logged in HXR JSONL.
9. **Test-First**: Core modules require ≥80% coverage.

## Dependency Rules
- Anaphase → Helix-Mind (read-only + idempotent write)
- Anaphase → Helix-Tentacle (read-only)
- Anaphase → Tuck (read-only)
- **Reverse dependencies are FORBIDDEN.**
- Tuck is independent and must never depend on any Helix component.
