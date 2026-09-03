# Fixture Data Shapes

> Contract family member (alongside `tt_job.schema.json`). ADR-0003 decision 6 +
> v2.3 Δ7: the M1 inlined fixtures and the M1.5 plugin outputs MUST share these
> shapes verbatim, so the M1.5 swap is seamless. M1.5 plugin implementations take
> THIS document as their authoritative shape reference.

## Shapes

| expect | Tool return shape | Applicable criteria |
|---|---|---|
| `numbers` | `{"series":[f64]}` | `sequence_length` / `sample_size` / `threshold` |
| `rate` | `{"numerator":f64,"denominator":f64}` | `ratio_band` / `cross_check` / `threshold` |
| `text` | `{"trend_a":f64,"trend_b":f64}` | `divergence` |

## Derivation rules

- `trace_id` / `evidence_id`: derived from `{job_id}#{call_index}` (deterministic,
  no randomness; ADR-0003 decision 6).
- Evidence/ledger records MUST NOT contain endpoint/port or any machine-dependent
  coordinate (ADR-0003 decision 6, determinism clamps).
- JSON construction MUST use fixed-field structs or `BTreeMap` — never `HashMap`
  (serde_json key order is unstable across processes; byte-identical replay would
  false-negative).

## Rule/data separation

- Criteria thresholds (low/high/min_len/min_n/tolerance/min) come from
  `knowledge_base/fixture-codex.json`, NEVER from tool return values.
- Tool returns carry only the data shapes above.
