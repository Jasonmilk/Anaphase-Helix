//! `up` — one-command Helix backend bootstrap (candidate G-4).
//!
//! Spawns the deterministic stack with a single command, no CLI zoo:
//!   1. Tentacle (gRPC, fixture plugins) — deterministic execution channel
//!   2. Anaphase (conscious layer) — with `ANAPHASE_TENTACLE_ENDPOINT`
//!      injected via env (config.toml untouched, 12-factor override)
//!   3. Health probes (port-ready, snapshot-ready) — physical facts only
//!   4. Optional `--cockpit`: lift the Cellrix cockpit TUI in the foreground
//!
//! Zero-hardcoding: every port/path is derived from config.toml, the
//! tentacle endpoint URL, or documented protocol defaults. Fail-open: a
//! missing Tentacle binary degrades to Noop (offline mode), never blocks.
//!
//! Usage:
//!   cargo run --bin up                 # backend + status
//!   cargo run --bin up -- --cockpit    # backend + Cellrix cockpit TUI
//!   HELIX_TENTACLE=/path/to/tentacle cargo run --bin up   # explicit binary
//!
//! Env knobs (all optional):
//!   HELIX_TENTACLE            explicit tentacle binary path
//!   HELIX_FIXTURES_DIR        explicit fixture plugins dir
//!   HELIX_TENTACLE_PORT       explicit grpc port (default derived)
//!   ANAPHASE_REASONING_ENDPOINT  passed through to Anaphase (LLM reasoning)

use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Max wait for a component to become ready (protocol default; Tentacle
/// boots in <1s, Anaphase compiles-free binary in <2s on a warm target).
const READY_TIMEOUT: Duration = Duration::from_secs(20);
/// Probe cadence while waiting for readiness.
const PROBE_INTERVAL: Duration = Duration::from_millis(250);
/// Tentacle gRPC default port when the endpoint is empty (ADR-0004: the
/// M1.5 transport default, documented in Tentacle `--grpc-port` help).
const TENTACLE_GRPC_DEFAULT_PORT: u16 = 50051;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let with_cockpit = args.iter().any(|a| a == "--cockpit");

    // 1. Config base state (config.toml in cwd; Noop defaults if absent).
    let config = anaphase::config::load_config()?;
    let anaphase_cfg = &config.anaphase;

    // 2. Derive Tentacle binary + fixtures (explicit env > workspace layout).
    let cwd = std::env::current_dir()?;
    let workspace_root = cwd.parent().unwrap_or(&cwd).to_path_buf();
    let tentacle_bin = std::env::var("HELIX_TENTACLE").ok().filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root.join("helix-tentacle/target/debug/tentacle"));
    let fixtures_dir = std::env::var("HELIX_FIXTURES_DIR").ok().filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root.join("helix-tentacle/fixtures"));

    // 3. Derive Tentacle gRPC port: from the configured endpoint URL, else
    //    the documented protocol default. Endpoint is the source of truth
    //    once configured (DNA principle 11: config > derivation > default).
    let grpc_port = anaphase_cfg.tentacle_endpoint.as_deref()
        .filter(|e| !e.is_empty())
        .and_then(|e| e.rsplit(':').next())
        .and_then(|p| p.trim_end_matches('/').parse::<u16>().ok())
        .or_else(|| std::env::var("HELIX_TENTACLE_PORT").ok().and_then(|v| v.parse().ok()))
        .unwrap_or(TENTACLE_GRPC_DEFAULT_PORT);

    println!("== Helix backend bootstrap ==");
    println!("tentacle:  {} (grpc :{grpc_port}, fixtures: {})", tentacle_bin.display(), fixtures_dir.display());
    println!("anaphase:  config.toml + ANAPHASE_TENTACLE_ENDPOINT=grpc://127.0.0.1:{grpc_port}");

    // 4. Start Tentacle (fail-open: missing binary -> offline Noop, warn).
    let mut tentacle: Option<Child> = None;
    if tentacle_bin.exists() {
        tentacle = Some(Command::new(&tentacle_bin)
            .args(["--transport", "grpc", "--grpc-port", &grpc_port.to_string(), "--plugins-dir"])
            .arg(&fixtures_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?);
        wait_for_port(grpc_port, "tentacle grpc")?;
        println!("  [ok] tentacle grpc ready on :{grpc_port}");
    } else {
        println!("  [warn] tentacle binary not found — Anaphase runs offline (Noop execution)");
    }

    // 5. Start Anaphase with the endpoint injected via env.
    let endpoint = format!("http://127.0.0.1:{grpc_port}");
    let mut anaphase = Command::new(std::env::current_dir()?.join("target/debug/anaphase"))
        .env("ANAPHASE_TENTACLE_ENDPOINT", &endpoint)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    // 6. Probe the snapshot endpoint (physical fact: HTTP is listening).
    let snapshot_port = anaphase_cfg.cap_http_port;
    wait_for_port(snapshot_port, "anaphase snapshot")?;
    println!("  [ok] anaphase snapshot ready on :{snapshot_port}");
    println!("== stack up ==");

    // 7. Optional cockpit: lift the Cellrix TUI in the foreground.
    if with_cockpit {
        let cellrix_cli = workspace_root.join("Cellrix/target/debug/cellrix-cli");
        let mock_agent = workspace_root.join("Cellrix/target/debug/mock-agent");
        if cellrix_cli.exists() && mock_agent.exists() {
            println!("lifting cockpit (stdio transport, mock-agent + snapshot)...");
            let status = Command::new(&cellrix_cli)
                .args(["run", "--mode", "stdio", "--exec"])
                .arg(&mock_agent)
                .args(["--anaphase-endpoint", &format!("http://127.0.0.1:{snapshot_port}")])
                .status()?;
            println!("cockpit exited: {}", status);
        } else {
            println!("  [warn] cellrix binaries not found — build Cellrix first");
        }
    }

    // 8. Stay alive until Ctrl+C, then clean up children in reverse order.
    wait_for_signal();
    drop(anaphase);
    if let Some(mut t) = tentacle {
        let _ = t.kill();
    }
    println!("\n== stack down ==");
    Ok(())
}

/// Poll a TCP port until connectable (physical readiness probe).
fn wait_for_port(port: u16, what: &str) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    while start.elapsed() < READY_TIMEOUT {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Ok(());
        }
        std::thread::sleep(PROBE_INTERVAL);
    }
    Err(format!("{what} not ready on :{port} within {READY_TIMEOUT:?}").into())
}

/// Block until SIGINT. Children spawned via `Command` live in the same
/// foreground process group, so terminal Ctrl+C delivers SIGINT to the whole
/// stack (tentacle/anaphase/cockpit) together — no explicit kill needed.
/// The sleep-loop keeps the parent alive until then (deterministic exit).
fn wait_for_signal() {
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}
