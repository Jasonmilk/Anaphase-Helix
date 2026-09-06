//! CI-144 transport layer for Anaphase (ADR-0017).
//!
//! Vendored protocol types (serde-compatible with Cellrix `cellrix-protocol`,
//! commit 21d13d4) + CIB/1.0 handshake + little-endian length-prefixed
//! MessagePack frames. This is the agent (server) side: the cockpit/client
//! (Cellrix `StdioTransport`) speaks the mirror contract.
//!
//! Event flow: handshake line -> `CIB/1.0 MSGPACK` -> Manifest (first frame)
//! -> Snapshot push loop (configurable cadence) + ActionRequest handling.
//!
//! Deterministic: clock is injected, projection is pure, no UUID.

pub mod server;

use serde::{Deserialize, Serialize};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Vendored protocol types (Cellrix cellrix-protocol @ 21d13d4, serde shapes
// must stay field-for-field identical for MessagePack interop).
// ---------------------------------------------------------------------------

/// Events an agent pushes to the cockpit over the transport.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum AgentEvent {
    /// Initial capability manifest (must be the first event after handshake).
    Manifest(CapabilityManifest),
    /// A full state snapshot for the UI to render.
    Snapshot(SemanticSnapshot),
    /// Periodic liveness signal.
    Heartbeat { epoch: u64 },
    /// An error that caused the event stream to terminate.
    StreamError(String),
}

/// Agent capability declaration (CAP protocol manifest endpoint).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityManifest {
    pub agent_name: String,
    pub version: String,
    pub actions: Vec<Action>,
    pub layout_hints: Option<LayoutHints>,
}

/// A single triggerable action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub label: String,
    pub security_class: SecurityClass,
    /// HITL approval window duration in milliseconds (present = lease enabled).
    pub lease_ms: Option<u64>,
    /// JSON Schema describing the parameter structure.
    pub parameters: serde_json::Value,
}

/// Security classification: normal vs critical (explicit human confirmation).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SecurityClass {
    Normal,
    Critical,
}

/// Layout hints (optional, higher priority than implicit heuristics).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutHints {
    pub preferred_panels: Vec<String>,
    pub grid: Option<GridDefinition>,
}

/// Explicit grid layout definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridDefinition {
    pub rows: Vec<GridSlot>,
}

/// Single layout slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridSlot {
    pub id: String,
    pub constraint: SlotConstraint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotConstraint {
    Percentage(f64),
    FixedLines(u16),
    Min(u16),
}

/// Agent's state projection (CAP protocol snapshot endpoint).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSnapshot {
    pub epoch_time: u64,
    pub status: String,
    pub metrics: serde_json::Value,
    pub semantic_tree: Vec<SemanticNode>,
    pub active_focus: Option<String>,
    pub layout_overrides: Option<LayoutHints>,
    /// PFP-xCF14 physical fingerprint (4 bytes, optional, CI-144 family).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pfp: Option<[u8; 4]>,
    /// SAP-xCF14 security proof (28 bytes, optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sap: Option<[u8; 28]>,
}

/// A single node in the semantic tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticNode {
    pub id: String,
    pub node_type: NodeType,
    pub label: String,
    pub content: serde_json::Value,
    pub slot_binding: Option<String>,
    pub focused: bool,
}

/// Node type (unknown types degrade safely).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    StateTree,
    TextPanel,
    ActionButton,
    ProgressBar,
    CodeDiff,
    Metrics,
    /// Unknown node type (tolerant parse).
    #[serde(other)]
    Unknown,
}

/// Action request triggered by the user or the system (CAPABILITY-13 endpoint).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequest {
    pub action_id: String,
    pub parameters: serde_json::Value,
    /// View hash of the last rendered state (CIC13 verification).
    pub view_hash: Option<ViewHash>,
}

/// Result of an action execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionResponse {
    Success { message: String },
    Failure { error: String, recoverable: bool },
    Pending { poll_id: String },
}

/// View hash: cryptographic anchor of "what you see is what you sign".
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ViewHash(pub [u8; 32]);

// ---------------------------------------------------------------------------
// Handshake + framing (CIB/1.0, little-endian length prefix).
// ---------------------------------------------------------------------------

/// Client handshake header we accept.
const HANDSHAKE_HEADER: &str = "CIB/1.0";
/// Our chosen wire format response (matches Cellrix `WireFormat::MessagePack`).
const FORMAT_RESPONSE: &str = "CIB/1.0 MSGPACK\n";
/// Default heartbeat interval (BIND-19 prime-number anti-resonance, 19s).
pub const HEARTBEAT_INTERVAL_SECS: u64 = 19;

/// Encode a message as a length-prefixed MessagePack frame (LE u32 + payload).
pub fn encode_frame<T: Serialize>(msg: &T) -> Result<Vec<u8>, std::io::Error> {
    let payload = rmp_serde::to_vec(msg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let mut out = Vec::with_capacity(4 + payload.len());
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(&payload);
    Ok(out)
}

/// Decode a length-prefixed MessagePack frame from a byte buffer.
/// Returns (frame, remaining_bytes) or an error on a truncated/invalid frame.
pub fn decode_frame<T: for<'de> Deserialize<'de>>(
    buf: &[u8],
) -> Result<(T, usize), std::io::Error> {
    if buf.len() < 4 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "frame header truncated",
        ));
    }
    let len = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    let total = 4 + len;
    if buf.len() < total {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "frame payload truncated",
        ));
    }
    let msg = rmp_serde::from_slice(&buf[4..total])
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok((msg, total))
}

/// Perform the CIB/1.0 handshake as the agent: read the client's header line,
/// reply with our format line. Pure byte-level contract (ADR-0017 D2).
pub fn handshake_response(first_line: &str) -> Result<&'static str, String> {
    if first_line.trim_start().starts_with(HANDSHAKE_HEADER) {
        Ok(FORMAT_RESPONSE)
    } else {
        Err(format!(
            "invalid CI-144 handshake header: {:?}",
            first_line.trim()
        ))
    }
}

// ---------------------------------------------------------------------------
// Projection: AgentSnapshot (ADR-0010) -> SemanticSnapshot (CI-144 cockpit).
// ---------------------------------------------------------------------------

/// Project the agent's live snapshot into the cockpit semantic tree
/// (ADR-0017 D4). Pure: clock injected, no hidden state.
pub fn project_snapshot(
    snap: &crate::run_cycle::AgentSnapshot,
    clock_secs: u64,
) -> SemanticSnapshot {
    use crate::states::HelixState;

    let mode_str = match snap.mode {
        crate::config::Mode::Drive => "driving",
        crate::config::Mode::Partner => "partner",
        crate::config::Mode::Survive => "survival",
    };
    let state_str = match snap.state {
        HelixState::Perception => "Perception",
        HelixState::PreAssessment => "PreAssessment",
        HelixState::MemoryRetrieval => "MemoryRetrieval",
        HelixState::Reasoning => "Reasoning",
        HelixState::ReflexCheck => "ReflexCheck",
        HelixState::Execution => "Execution",
        HelixState::Reflection => "Reflection",
    };

    let (ecosystem_up, ecosystem_total) = {
        let list = snap.ecosystem.clone();
        let up = list.iter().filter(|g| g.status != crate::gloves::GloveStatus::Unavailable).count();
        (up, list.len())
    };
    let ledger_len = snap.ledger.len();
    let episode_step = snap.episode.as_ref().map(|e| e.step).unwrap_or(0);

    let ecosystem_nodes: Vec<SemanticNode> = snap
        .ecosystem
        .iter()
        .map(|g| SemanticNode {
            id: format!("ecosystem-{}", g.name),
            node_type: NodeType::Metrics,
            label: g.name.clone(),
            content: serde_json::json!({ "status": format!("{:?}", g.status), "tier": format!("{:?}", g.tier) }),
            slot_binding: None,
            focused: false,
        })
        .collect();

    SemanticSnapshot {
        epoch_time: clock_secs,
        status: mode_str.to_string(),
        metrics: serde_json::json!({
            "ecosystem_up": ecosystem_up,
            "ecosystem_total": ecosystem_total,
            "ledger_entries": ledger_len,
            "episode_step": episode_step,
        }),
        semantic_tree: vec![
            SemanticNode {
                id: "cognitive-loop".to_string(),
                node_type: NodeType::StateTree,
                label: "Cognitive Loop".to_string(),
                content: serde_json::json!(
                    "Perception -> PreAssessment -> MemoryRetrieval -> Reasoning -> ReflexCheck -> Execution -> Reflection"
                ),
                slot_binding: None,
                focused: false,
            },
            SemanticNode {
                id: "status".to_string(),
                node_type: NodeType::TextPanel,
                label: "Status".to_string(),
                content: serde_json::json!({
                    "mode": mode_str,
                    "state": state_str,
                    "episode_step": episode_step,
                    "ledger_entries": ledger_len,
                }),
                slot_binding: None,
                focused: false,
            },
            SemanticNode {
                id: "ecosystem".to_string(),
                node_type: NodeType::Metrics,
                label: "Ecosystem".to_string(),
                content: serde_json::json!({
                    "up": ecosystem_up,
                    "total": ecosystem_total,
                }),
                slot_binding: None,
                focused: false,
            },
        ]
        .into_iter()
        .chain(ecosystem_nodes)
        .collect(),
        active_focus: Some(state_str.to_string()),
        layout_overrides: None,
        pfp: None,
        sap: None,
    }
}

/// The default snapshot push cadence (configurable via config, ADR-0017 D3).
pub const SNAPSHOT_PUSH_INTERVAL: Duration = Duration::from_secs(1);
