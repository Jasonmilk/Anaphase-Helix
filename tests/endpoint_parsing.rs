//! ADR-0045 — one endpoint string, one interpretation.
//!
//! Measured 2026-09-24 on the live hub: `/v1/health` answered
//! `flowmodus configured=true ok=false detail="bad endpoint: grpc://127.0.0.1:60054"`
//! while `main.rs` **required** that same prefix to select the gRPC adapter and
//! `gloves::probe_endpoint` called the organ `Unavailable`. Three parsers, two
//! verdicts, one string.
//!
//! These criteria live here rather than beside the parser because `src/health.rs`
//! is ratcheted with zero headroom: criterion code is not what the ratchet is
//! there to bound.

use anaphase::health::{endpoint_addr, endpoint_authority};

#[test]
fn endpoint_authority_is_scheme_agnostic() {
    // The exact string the reasoning adapter required and the probe rejected.
    assert_eq!(endpoint_authority("grpc://127.0.0.1:60054"), "127.0.0.1:60054");
    assert_eq!(endpoint_authority("http://127.0.0.1:50052"), "127.0.0.1:50052");
    assert_eq!(endpoint_authority("https://example.test:443"), "example.test:443");
    // Bare (no scheme) and whitespace-padded both appear in real configs.
    assert_eq!(endpoint_authority("127.0.0.1:50051"), "127.0.0.1:50051");
    assert_eq!(endpoint_authority("  grpc://127.0.0.1:60054  "), "127.0.0.1:60054");
    // A path after the authority is dropped (`HTTP LLM endpoints carry /v1`).
    assert_eq!(endpoint_authority("http://127.0.0.1:11434/v1"), "127.0.0.1:11434");
    // A Unix socket keeps its leading `/` — it is a path, not an authority.
    assert_eq!(endpoint_authority("unix:///tmp/mind.sock"), "/tmp/mind.sock");
}

#[test]
fn endpoint_addr_accepts_every_scheme_the_ecosystem_writes() {
    assert!(endpoint_addr("grpc://127.0.0.1:60054").is_ok());
    assert!(endpoint_addr("http://127.0.0.1:50052").is_ok());
    assert!(endpoint_addr("127.0.0.1:50051").is_ok());
}

/// Negative control: "scheme-agnostic" must not degenerate into "accept
/// anything". Without this half, the three cases above could be satisfied by a
/// parser that stopped checking at all.
#[test]
fn endpoint_addr_still_refuses_a_broken_value() {
    assert!(endpoint_addr("not-an-endpoint").is_err());
    assert!(endpoint_addr("grpc://").is_err());
    assert!(endpoint_addr("").is_err());
}
