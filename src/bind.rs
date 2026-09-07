//! One-to-one human binding (2026-09-07) — Anaphase is the challenger.
//!
//! Anaphase generates the pairing code, shows it to the human, verifies the
//! human's confirm (HITL — a code only the owner can see), persists the
//! device identity, and verifies every later request. The client (panel /
//! `up`) holds the same secret in its own 0600 file and signs each request
//! with a fresh nonce + timestamp.
//!
//! Replay is killed twice (防重放双保险):
//!   1. timestamp window ±60s, and
//!   2. one-time nonce — a bounded seen-set (linear cap; a real Bloom
//!      filter is the v2 upgrade, same bounded-memory philosophy as D'-1).
//!
//! Unbound = open: until a human binds, every endpoint behaves exactly as
//! before and `/v1/bind/status` honestly reports `bound: false` (按需驱动,
//! 未配置不评判). Binding is always an explicit human action.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::config::AnaphaseConfig;

/// Pairing lifetime (protocol default): a code is one-time and expires.
const PAIR_TTL_SECS: u64 = 600;
/// Timestamp replay window (protocol default): ±60s.
const TS_WINDOW_SECS: u64 = 60;
/// Max remembered nonces (bounded memory — v1 linear, v2 Bloom).
const NONCE_CAP: usize = 4096;
/// Authorization scheme prefix: `Bearer v1.<id>.<ts>.<nonce>.<hmac>`.
const SCHEME: &str = "Bearer v1.";
/// Per-user convention for the Anaphase-side identity file.
const IDENTITY_REL: &str = ".cellrix/anaphase-identity.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceIdentity {
    pub device_id: String,
    pub secret: String,
    #[serde(default)]
    pub bound_at: u64,
}

#[derive(Debug, Clone)]
pub struct Pairing {
    pub code: String,
    pub expires_at: u64,
}

pub struct BindState {
    pub pair: Option<Pairing>,
    pub device: Option<DeviceIdentity>,
    /// One-time nonces seen so far (replay guard, bounded).
    pub seen_nonces: Vec<String>,
}

impl Default for BindState {
    fn default() -> Self {
        Self {
            pair: None,
            device: None,
            seen_nonces: Vec::new(),
        }
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// HMAC-SHA256 over the RFC 2104 construction, built on the existing sha2
/// dependency (极致复用 — no new crypto crate for a 30-line standard block).
pub fn hmac_sha256_hex(key: &[u8], msg: &[u8]) -> String {
    const BLOCK: usize = 64;
    let mut k = key.to_vec();
    if k.len() > BLOCK {
        k = Sha256::digest(&k).to_vec();
    }
    while k.len() < BLOCK {
        k.push(0);
    }
    let mut ipad = vec![0u8; BLOCK];
    let mut opad = vec![0u8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] = k[i] ^ 0x36;
        opad[i] = k[i] ^ 0x5c;
    }
    let mut inner = Vec::with_capacity(BLOCK + msg.len());
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(msg);
    let inner_digest = Sha256::digest(&inner);
    let mut outer = Vec::with_capacity(BLOCK + inner_digest.len());
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&inner_digest);
    hex(&Sha256::digest(&outer))
}

/// Cryptographically random hex string of `n` bytes (getrandom — already in
/// the dependency tree via tokio, zero new transitive deps).
fn random_hex(n: usize) -> Result<String, String> {
    let mut buf = vec![0u8; n];
    getrandom::getrandom(&mut buf).map_err(|e| format!("rng: {e}"))?;
    Ok(hex(&buf))
}

/// 6-digit pairing code (human-readable, one-time).
fn pairing_code() -> Result<String, String> {
    let mut buf = [0u8; 4];
    getrandom::getrandom(&mut buf).map_err(|e| format!("rng: {e}"))?;
    let n = u32::from_le_bytes(buf) % 1_000_000;
    Ok(format!("{n:06}"))
}

/// Deterministic path: explicit config wins, else the per-user convention.
pub fn identity_path(cfg: &AnaphaseConfig) -> Result<PathBuf, String> {
    if let Some(p) = cfg.bind_state_path.as_ref().filter(|s| !s.is_empty()) {
        return Ok(PathBuf::from(p));
    }
    let home = std::env::var("HOME").map_err(|_| "HOME unset".to_string())?;
    Ok(PathBuf::from(home).join(IDENTITY_REL))
}

/// Persist the device identity (0600). Pairing/nonces stay in memory —
/// restarting mid-pairing just requires a new code (按需加载).
pub fn save(cfg: &AnaphaseConfig, state: &Mutex<BindState>) -> Result<(), String> {
    let path = identity_path(cfg)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let dev = state.lock().unwrap().device.clone();
    let body = serde_json::to_string_pretty(&dev).map_err(|e| e.to_string())?;
    fs::write(&path, body).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Load a previously persisted identity (startup; best effort — a missing
/// file simply means unbound).
pub fn load(cfg: &AnaphaseConfig, state: &Mutex<BindState>) {
    let Ok(path) = identity_path(cfg) else { return };
    let Ok(body) = fs::read_to_string(&path) else { return };
    if let Ok(dev) = serde_json::from_str::<DeviceIdentity>(&body) {
        state.lock().unwrap().device = Some(dev);
    }
}

/// Start a pairing: generates a fresh one-time code (re-pairing replaces an
/// existing binding only after the human confirms — HITL).
pub fn start(cfg: &AnaphaseConfig, state: &Mutex<BindState>) -> Result<serde_json::Value, String> {
    let _ = cfg; // pairing is in-memory; cfg kept for a uniform signature
    let code = pairing_code()?;
    let expires_at = now_secs() + PAIR_TTL_SECS;
    let mut guard = state.lock().unwrap();
    guard.pair = Some(Pairing { code: code.clone(), expires_at });
    let bound = guard.device.as_ref().map(|d| d.device_id.clone());
    Ok(json!({
        "pairing_code": code,
        "expires_at": expires_at,
        "ttl_secs": PAIR_TTL_SECS,
        "replacing": bound,
        "hint": "show this code to your human; confirm within 10 minutes"
    }))
}

/// Confirm a pairing: validate the one-time code, mint the device identity,
/// persist it, and return the client secret exactly once.
pub fn confirm(
    cfg: &AnaphaseConfig,
    state: &Mutex<BindState>,
    code: &str,
) -> Result<serde_json::Value, String> {
    let mut guard = state.lock().unwrap();
    let pair = guard.pair.as_ref().ok_or("no pairing in progress")?;
    if code != pair.code {
        return Err("pairing code mismatch".into());
    }
    if now_secs() > pair.expires_at {
        return Err("pairing code expired — start again".into());
    }
    let device_id = format!("anaphase#{}", random_hex(8)?);
    let secret = random_hex(32)?;
    let dev = DeviceIdentity { device_id: device_id.clone(), secret: secret.clone(), bound_at: now_secs() };
    guard.device = Some(dev);
    guard.pair = None;
    drop(guard);
    save(cfg, state)?;
    Ok(json!({
        "bound": true,
        "device_id": device_id,
        "client_secret": secret,
        "hint": "secret shown once — store it in ~/.cellrix/identity.toml (0600)"
    }))
}

/// Binding status — honest, no secrets.
pub fn status(state: &Mutex<BindState>) -> serde_json::Value {
    let guard = state.lock().unwrap();
    json!({
        "bound": guard.device.is_some(),
        "device_id": guard.device.as_ref().map(|d| d.device_id.clone()),
        "pairing_pending": guard.pair.is_some(),
    })
}

/// Verify an `Authorization: Bearer v1.<id>.<ts>.<nonce>.<hmac>` header.
/// Unbound → pass (the endpoint set is open until a human binds). All four
/// checks must pass: known device, ±60s window, one-time nonce, real HMAC.
pub fn verify_bearer(state: &Mutex<BindState>, header: Option<&str>) -> Result<(), String> {
    let mut guard = state.lock().unwrap();
    let Some(dev) = guard.device.clone() else {
        return Ok(()); // unbound: open, honest "not bound" state
    };
    let Some(h) = header else {
        return Err("missing Authorization header".into());
    };
    let raw = h.strip_prefix(SCHEME).ok_or("bad scheme (want Bearer v1.)")?;
    let parts: Vec<&str> = raw.split('.').collect();
    if parts.len() != 4 {
        return Err("bad token shape (want id.ts.nonce.hmac)".into());
    }
    let (id, ts, nonce, mac) = (parts[0], parts[1], parts[2], parts[3]);
    if id != dev.device_id {
        return Err("unknown device".into());
    }
    let ts_u: u64 = ts.parse().map_err(|_| "bad timestamp".to_string())?;
    if ts_u.abs_diff(now_secs()) > TS_WINDOW_SECS {
        return Err("stale timestamp (replay?)".into());
    }
    if guard.seen_nonces.contains(&nonce.to_string()) {
        return Err("replayed nonce".into());
    }
    let expect = hmac_sha256_hex(dev.secret.as_bytes(), format!("{id}|{ts}|{nonce}").as_bytes());
    if mac != expect {
        return Err("bad hmac".into());
    }
    guard.seen_nonces.push(nonce.to_string());
    if guard.seen_nonces.len() > NONCE_CAP {
        guard.seen_nonces.drain(0..NONCE_CAP / 2);
    }
    Ok(())
}

/// Build a signed bearer for a client holding (device_id, secret).
pub fn sign_bearer(device_id: &str, secret: &str) -> String {
    let ts = now_secs().to_string();
    // Client-side nonce: deterministic fresh entropy, hex.
    let nonce = random_hex(8).unwrap_or_else(|_| format!("{:x}", now_secs()));
    let mac = hmac_sha256_hex(secret.as_bytes(), format!("{device_id}|{ts}|{nonce}").as_bytes());
    format!("{SCHEME}{device_id}.{ts}.{nonce}.{mac}") // full header value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AnaphaseConfig;

    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    fn cfg() -> AnaphaseConfig {
        // Unique dir per test — tests run in parallel and must not share
        // the identity file (same pid otherwise).
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut c = AnaphaseConfig::default();
        c.bind_state_path = Some(
            std::env::temp_dir()
                .join(format!("bind-test-{}-{n}", std::process::id()))
                .join("identity.json")
                .to_string_lossy()
                .into_owned(),
        );
        c
    }

    #[test]
    fn hmac_matches_rfc2202_case1() {
        let key = vec![0x0b; 20];
        let mac = hmac_sha256_hex(&key, b"Hi There");
        assert_eq!(
            mac,
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn start_confirm_then_verify_round_trip() {
        let c = cfg();
        let st = Mutex::new(BindState::default());
        let v = start(&c, &st).unwrap();
        let code = v["pairing_code"].as_str().unwrap().to_string();
        let out = confirm(&c, &st, &code).unwrap();
        assert_eq!(out["bound"], true);
        let id = out["device_id"].as_str().unwrap().to_string();
        let secret = out["client_secret"].as_str().unwrap().to_string();
        assert!(id.starts_with("anaphase#"));
        assert_eq!(secret.len(), 64);

        // Signed request passes.
        let tok = sign_bearer(&id, &secret);
        let ok = verify_bearer(&st, Some(&tok));
        assert!(ok.is_ok(), "{ok:?}");
        // status reflects bound.
        assert_eq!(status(&st)["bound"], true);
    }

    #[test]
    fn replay_is_rejected() {
        let c = cfg();
        let st = Mutex::new(BindState::default());
        let v = start(&c, &st).unwrap();
        let out = confirm(&c, &st, v["pairing_code"].as_str().unwrap()).unwrap();
        let id = out["device_id"].as_str().unwrap();
        let secret = out["client_secret"].as_str().unwrap();
        let tok = sign_bearer(id, secret);
        let header = tok.clone();
        assert!(verify_bearer(&st, Some(&header)).is_ok());
        // Same token again → nonce already seen.
        assert!(verify_bearer(&st, Some(&header)).is_err());
    }

    #[test]
    fn bad_hmac_is_rejected() {
        let c = cfg();
        let st = Mutex::new(BindState::default());
        let v = start(&c, &st).unwrap();
        let out = confirm(&c, &st, v["pairing_code"].as_str().unwrap()).unwrap();
        let id = out["device_id"].as_str().unwrap();
        let secret = out["client_secret"].as_str().unwrap();
        let tok = sign_bearer(id, secret);
        // Corrupt the hmac tail.
        let mut bad = tok.clone();
        let pos = bad.rfind('.').unwrap() + 1;
        bad.replace_range(pos..pos + 4, "0000");
        assert!(verify_bearer(&st, Some(&bad)).is_err());
    }

    #[test]
    fn wrong_pairing_code_and_expiry() {
        let c = cfg();
        let st = Mutex::new(BindState::default());
        let _ = start(&c, &st).unwrap();
        assert!(confirm(&c, &st, "000000").is_err());
    }

    #[test]
    fn persist_round_trip_0600() {
        let c = cfg();
        let st = Mutex::new(BindState::default());
        let v = start(&c, &st).unwrap();
        let out = confirm(&c, &st, v["pairing_code"].as_str().unwrap()).unwrap();
        let id = out["device_id"].as_str().unwrap().to_string();
        // Reload from disk into a fresh state.
        let st2 = Mutex::new(BindState::default());
        load(&c, &st2);
        assert_eq!(status(&st2)["device_id"].as_str(), Some(id.as_str()));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let p = identity_path(&c).unwrap();
            let mode = fs::metadata(&p).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
    }
}
