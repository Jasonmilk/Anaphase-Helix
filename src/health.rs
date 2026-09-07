//! Self-check: Anaphase reports the physical readiness of its own organs.
//!
//! Every check is config-derived and probed — nothing is guessed (物理事实
//! 优先, 0 硬编码). The panel probes `/v1/health` instead of assuming; a
//! check that is not configured is not a failure (按需驱动: unconfigured
//! organs are simply not judged).
//!
//! Aggregation: `ok` is true iff every *configured* check is healthy.

use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use serde_json::json;
use serde_json::Value;

use crate::config::AnaphaseConfig;

/// Extract host:port from an endpoint that may carry a scheme or a path
/// (e.g. `http://127.0.0.1:11434/v1` -> `127.0.0.1:11434`).
fn endpoint_addr(endpoint: &str) -> Result<SocketAddr, String> {
    let bare = endpoint
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let hostport = bare.split('/').next().unwrap_or(bare);
    hostport
        .parse()
        .map_err(|_| format!("bad endpoint: {endpoint}"))
}

/// Deterministic reachability probe: a plain blocking connect on a helper
/// thread, with a hard 2s timeout on the caller side. `connect_timeout` is
/// flaky on macOS against loopback listeners (poll can stall ~1.7s then
/// still succeed), so we never rely on it — 确定性优先.
fn tcp_reachable(endpoint: &str) -> Result<(), String> {
    let addr = endpoint_addr(endpoint)?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(std::net::TcpStream::connect(addr).map(|_| ()));
    });
    match rx.recv_timeout(Duration::from_secs(2)) {
        Ok(r) => r.map_err(|e| e.to_string()),
        Err(_) => Err("connect timeout (2s)".to_string()),
    }
}

fn parent_dir_writable(path: &str) -> Result<(), String> {
    let p = Path::new(path);
    let parent = p
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    match std::fs::metadata(parent) {
        Ok(m) if m.is_dir() => Ok(()),
        Ok(_) => Err("parent exists but is not a directory".into()),
        Err(e) => Err(e.to_string()),
    }
}

fn check_path(name: &str, configured: Option<&String>) -> Value {
    // Empty string is the "unset" signal in config files — not a failure.
    match configured.filter(|s| !s.is_empty()) {
        Some(p) => {
            let (ok, detail) = match parent_dir_writable(p) {
                Ok(()) => (true, "write target reachable".to_string()),
                Err(e) => (false, e),
            };
            json!({ "name": name, "configured": true, "ok": ok, "detail": detail })
        }
        None => json!({ "name": name, "configured": false, "ok": true, "detail": "not configured" }),
    }
}

fn check_endpoint(name: &str, configured: Option<&String>) -> Value {
    match configured.filter(|s| !s.is_empty()) {
        Some(ep) => {
            let (ok, detail) = match tcp_reachable(ep) {
                Ok(()) => (true, "reachable".to_string()),
                Err(e) => (false, e),
            };
            json!({ "name": name, "configured": true, "ok": ok, "detail": detail })
        }
        None => json!({ "name": name, "configured": false, "ok": true, "detail": "not configured" }),
    }
}

/// Run every self-check over the config. Pure (no side effects beyond TCP
/// probes), deterministic for a given config + network state.
pub fn checks(cfg: &AnaphaseConfig) -> Value {
    let mut checks = vec![
        check_path("trace", cfg.reasoning_trace_path.as_ref()),
        check_path("ledger", cfg.events_log_path.as_ref()),
        check_path("session_notes", cfg.session_notes_path.as_ref()),
        check_endpoint("tentacle", cfg.tentacle_endpoint.as_ref()),
        check_endpoint("mind", cfg.mind_endpoint.as_ref()),
        check_endpoint("flowmodus", cfg.flowmodus_endpoint.as_ref()),
        check_endpoint("tuck", cfg.tuck_endpoint.as_ref()),
        check_endpoint("cellrix", cfg.cellrix_endpoint.as_ref()),
        check_endpoint("reasoning", cfg.reasoning_endpoint.as_ref()),
    ];
    // Scalar organs: report the deterministic source, no probe.
    checks.push(json!({
        "name": "judge",
        "configured": true,
        "ok": true,
        "detail": format!("backend={:?}", cfg.judge_backend)
    }));
    checks.push(json!({
        "name": "cap_http",
        "configured": cfg.cap_http_enabled,
        "ok": cfg.cap_http_enabled,
        "detail": format!("port={}", cfg.cap_http_port)
    }));
    checks.push(json!({
        "name": "reasoning_credentials",
        "configured": cfg.reasoning_api_key.is_some(),
        "ok": true,
        "detail": "presence only, value never reported"
    }));

    let ok = checks.iter().all(|c| {
        let configured = c["configured"].as_bool().unwrap_or(false);
        let ok = c["ok"].as_bool().unwrap_or(false);
        !configured || ok
    });
    json!({ "ok": ok, "checks": checks })
}

/// Fail-closed gate: when Tuck is configured (audit/LLM gateway), the
/// engine refuses to reason while Tuck is unreachable — Tuck down = Helix
/// stops thinking (SPOF explicitly accepted, 网关可用性换审计完整性).
/// Unconfigured → pass (按需驱动: nothing to gate).
pub fn gate_ok(cfg: &AnaphaseConfig) -> Result<(), String> {
    match cfg.tuck_endpoint.as_ref().filter(|s| !s.is_empty()) {
        Some(ep) => tcp_reachable(ep).map_err(|e| format!("tuck unreachable ({ep}): {e}")),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AnaphaseConfig;
    use std::net::TcpListener;

    fn base() -> AnaphaseConfig {
        // Protocol defaults: every optional field None.
        AnaphaseConfig::default()
    }

    #[test]
    fn all_unconfigured_is_ok() {
        let c = base();
        let v = checks(&c);
        assert_eq!(v["ok"], true);
        let names: Vec<&str> = v["checks"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|c| c["name"].as_str())
            .collect();
        assert!(names.contains(&"trace"));
        assert!(names.contains(&"tentacle"));
        assert!(names.contains(&"judge"));
    }

    #[test]
    fn gate_unconfigured_passes() {
        let c = base();
        assert!(gate_ok(&c).is_ok());
    }

    #[test]
    fn gate_configured_unreachable_blocks() {
        let mut c = base();
        c.tuck_endpoint = Some("http://127.0.0.1:1".into()); // closed port
        assert!(gate_ok(&c).is_err());
    }

    #[test]
    fn gate_configured_reachable_passes() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let mut c = base();
        c.tuck_endpoint = Some(format!("http://127.0.0.1:{port}"));
        assert!(gate_ok(&c).is_ok());
    }

    #[test]
    fn missing_trace_parent_is_unhealthy() {
        let mut c = base();
        c.reasoning_trace_path = Some("/nonexistent-dir-xyz/trace.jsonl".into());
        let v = checks(&c);
        assert_eq!(v["ok"], false);
        let trace = v["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "trace")
            .unwrap();
        assert_eq!(trace["configured"], true);
        assert_eq!(trace["ok"], false);
    }

    #[test]
    fn endpoint_reachable_vs_refused() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        // Reachable while the listener lives.
        let mut c = base();
        c.tentacle_endpoint = Some(format!("http://{addr}"));
        assert_eq!(checks(&c)["ok"], true);
        // Refused after it is dropped (port freed, nothing listening).
        drop(listener);
        let v = checks(&c);
        assert_eq!(v["ok"], false);
        let tentacle = v["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "tentacle")
            .unwrap();
        assert_eq!(tentacle["ok"], false);
    }

    #[test]
    fn credentials_reported_as_presence_only() {
        let mut c = base();
        c.reasoning_api_key = Some("sk-super-secret".into());
        let v = checks(&c);
        let creds = v["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "reasoning_credentials")
            .unwrap();
        assert_eq!(creds["configured"], true);
        assert!(!v.to_string().contains("super-secret"));
    }

}
// SYNTAX ERROR TEST
// change marker 1788769106
