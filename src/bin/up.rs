//! `up` — one-command Helix backend bootstrap (candidate G-4 + G-5).
//!
//! Spawns the deterministic stack with a single command, no CLI zoo:
//!   1. Tentacle (gRPC, fixture plugins) — deterministic execution channel
//!   2. Anaphase (conscious layer) — with `ANAPHASE_TENTACLE_ENDPOINT`
//!      injected via env (config.toml untouched, 12-factor override)
//!   3. Health probes (port-ready, snapshot-ready) — physical facts only
//!   4. Optional `--cockpit`: lift the Cellrix cockpit TUI in the foreground
//!
//! G-5 (usability guide): first-run friendly output — welcome banner,
//! prereq checks with concrete build hints, per-step [ok]/[warn] with
//! explanations, and a "next steps" section. A first-time user never stares
//! at a bare debug dump; they always know what is running, what is missing,
//! and what to do next.
//!
//! Zero-hardcoding: every port/path is derived from config.toml, the
//! tentacle endpoint URL, or documented protocol defaults. Fail-open: a
//! missing Tentacle binary degrades to Noop (offline mode), never blocks.
//!
//! Usage:
//!   cargo run --bin up                 # backend + guided status
//!   cargo run --bin up -- --cockpit    # backend + Cellrix cockpit TUI
//!   HELIX_TENTACLE=/path/to/tentacle cargo run --bin up   # explicit binary
//!
//! Env knobs (all optional):
//!   HELIX_TENTACLE            explicit tentacle binary path
//!   HELIX_FIXTURES_DIR        explicit fixture plugins dir
//!   HELIX_TENTACLE_PORT       explicit grpc port (default derived)
//!   ANAPHASE_REASONING_ENDPOINT  passed through to Anaphase (LLM reasoning)

use std::net::TcpStream;
use std::path::{Path, PathBuf};
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

/// A missing component with a concrete build hint (G-5 prereq guide).
struct Missing {
    what: &'static str,
    hint: &'static str,
}

/// Probe the component binaries; returns what is missing and how to build it.
/// Pure function — every path is a physical fact, no guessing.
fn check_prereqs(
    tentacle: &Path,
    anaphase: &Path,
    cellrix_cli: &Path,
    mock_agent: &Path,
) -> Vec<Missing> {
    let mut missing = Vec::new();
    if !tentacle.exists() {
        missing.push(Missing { what: "Tentacle", hint: "cd helix-tentacle && cargo build" });
    }
    if !anaphase.exists() {
        missing.push(Missing { what: "Anaphase", hint: "cargo build   (in anaphase-helix)" });
    }
    if !cellrix_cli.exists() {
        missing.push(Missing { what: "Cellrix cli", hint: "cd Cellrix && cargo build -p cellrix-cli" });
    }
    if !mock_agent.exists() {
        missing.push(Missing { what: "mock-agent", hint: "cd Cellrix && cargo build -p mock-agent" });
    }
    missing
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let with_cockpit = args.iter().any(|a| a == "--cockpit");

    // -- welcome ----------------------------------------------------------
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Helix 后端引导 (up)");
    println!("  一条命令拉起确定性执行链: Tentacle → Anaphase → 驾驶舱");
    println!("  退出: Ctrl+C（整栈一起停）   帮助: 本输出即引导");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 1. Config base state (config.toml in cwd; Noop defaults if absent).
    let config = anaphase::config::load_config()?;
    let anaphase_cfg = &config.anaphase;

    // 2. Derive binaries + fixtures (explicit env > workspace layout).
    let cwd = std::env::current_dir()?;
    let workspace_root = cwd.parent().unwrap_or(&cwd).to_path_buf();
    let tentacle_bin = std::env::var("HELIX_TENTACLE").ok().filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root.join("helix-tentacle/target/debug/tentacle"));
    let fixtures_dir = std::env::var("HELIX_FIXTURES_DIR").ok().filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root.join("helix-tentacle/fixtures"));
    let anaphase_bin = cwd.join("target/debug/anaphase");
    let cellrix_cli = workspace_root.join("Cellrix/target/debug/cellrix-cli");
    let mock_agent = workspace_root.join("Cellrix/target/debug/mock-agent");

    // 3. Derive Tentacle gRPC port: from the configured endpoint URL, else
    //    the documented protocol default. Endpoint is the source of truth
    //    once configured (DNA principle 11: config > derivation > default).
    let grpc_port = anaphase_cfg.tentacle_endpoint.as_deref()
        .filter(|e| !e.is_empty())
        .and_then(|e| e.rsplit(':').next())
        .and_then(|p| p.trim_end_matches('/').parse::<u16>().ok())
        .or_else(|| std::env::var("HELIX_TENTACLE_PORT").ok().and_then(|v| v.parse().ok()))
        .unwrap_or(TENTACLE_GRPC_DEFAULT_PORT);

    // 4. Prereq guide (G-5): every missing binary names its build command.
    println!("\n[前置检查]");
    let missing = check_prereqs(&tentacle_bin, &anaphase_bin, &cellrix_cli, &mock_agent);
    let mut blocked = false;
    for m in &missing {
        let fatal = m.what == "Anaphase"; // no Anaphase => nothing to run
        println!("  [{}] {} 未找到 → {}", if fatal { "✗" } else { "•" }, m.what, m.hint);
        if fatal { blocked = true; }
    }
    if missing.is_empty() {
        println!("  [ok] 全部组件已就绪");
    } else if !blocked {
        println!("  [i] 缺失项可按提示构建；Tentacle/Cellrix 缺失不会阻塞（fail-open）");
    }
    if blocked {
        println!("\n[停] 缺少 Anaphase 本体，无法启动。构建后重试:");
        println!("      cargo build");
        return Ok(());
    }
    if anaphase_cfg.reasoning_endpoint.as_deref().map_or(true, |s| s.is_empty()) {
        println!("  [i] reasoning 未配置 → Anaphase 运行在 Noop 模式（无 LLM 认知，ledger 为空）");
        println!("      配置方式: config.toml [anaphase] reasoning_endpoint = \"http://...\"");
    }

    // 5. Start Tentacle (fail-open: missing binary -> offline Noop, warn).
    println!("\n[启动]");
    let mut tentacle: Option<Child> = None;
    if tentacle_bin.exists() {
        tentacle = Some(Command::new(&tentacle_bin)
            .args(["--transport", "grpc", "--grpc-port", &grpc_port.to_string(), "--plugins-dir"])
            .arg(&fixtures_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?);
        wait_for_port(grpc_port, "tentacle grpc")?;
        println!("  [ok] Tentacle 就绪: gRPC :{grpc_port}（fixtures: {}）", fixtures_dir.display());
    } else {
        println!("  [warn] Tentacle 未找到 → Anaphase 离线执行（Noop）");
    }

    // 6. Start Anaphase with the endpoint injected via env.
    let endpoint = format!("http://127.0.0.1:{grpc_port}");
    let mut anaphase = Command::new(&anaphase_bin)
        .env("ANAPHASE_TENTACLE_ENDPOINT", &endpoint)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    // 7. Probe the snapshot endpoint (physical fact: HTTP is listening).
    let snapshot_port = anaphase_cfg.cap_http_port;
    wait_for_port(snapshot_port, "anaphase snapshot")?;
    println!("  [ok] Anaphase 就绪: snapshot :{snapshot_port}（模式: {}）", mode_label(anaphase_cfg.run_cycle.mode));

    // 8. Optional cockpit: lift the Cellrix TUI in the foreground.
    if with_cockpit {
        if cellrix_cli.exists() && mock_agent.exists() {
            println!("  [ok] 拉起驾驶舱 (stdio transport + mock-agent)...");
            let status = Command::new(&cellrix_cli)
                .args(["run", "--mode", "stdio", "--exec"])
                .arg(&mock_agent)
                .args(["--anaphase-endpoint", &format!("http://127.0.0.1:{snapshot_port}")])
                .status()?;
            println!("  驾驶舱退出: {}", status);
        } else {
            println!("  [warn] Cellrix 二进制未找到 → 跳过驾驶舱（构建后加 --cockpit 重试）");
        }
    }

    // 9. Next steps (G-5): the user always knows what to do next.
    println!("\n[下一步]");
    println!("  • 打开驾驶舱（需另起终端）: cellrix-cli run --mode stdio --exec ../Cellrix/target/debug/mock-agent --anaphase-endpoint http://127.0.0.1:{snapshot_port}");
    if !with_cockpit {
        println!("  • 或直接带驾驶舱重来:  Ctrl+C 后运行 cargo run --bin up -- --cockpit");
    }
    println!("  • 查看完整用法:          README.md（anaphase-helix）");
    println!("\n== stack up — Ctrl+C 退出 ==");

    // 10. Stay alive until Ctrl+C (children share the process group).
    wait_for_signal();
    drop(anaphase);
    if let Some(mut t) = tentacle {
        let _ = t.kill();
    }
    println!("\n== stack down ==");
    Ok(())
}

/// Chinese label for the configured interaction mode (ADR-0006).
fn mode_label(mode: anaphase::config::Mode) -> &'static str {
    match mode {
        anaphase::config::Mode::Drive => "drive 驾驶（无 Mind）",
        anaphase::config::Mode::Partner => "partner 伙伴（默认，带记忆协作）",
        anaphase::config::Mode::Survive => "survive 生存（保留）",
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_present_nothing_missing() {
        let dir = std::env::temp_dir().join(format!("up-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = |name: &str| -> PathBuf {
            let p = dir.join(name);
            std::fs::write(&p, b"x").unwrap();
            p
        };
        let missing = check_prereqs(&f("t"), &f("a"), &f("c"), &f("m"));
        assert!(missing.is_empty());
    }

    #[test]
    fn anaphase_missing_is_fatal() {
        let dir = std::env::temp_dir().join(format!("up-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = |name: &str| -> PathBuf {
            let p = dir.join(name);
            std::fs::write(&p, b"x").unwrap();
            p
        };
        let missing = check_prereqs(&f("t"), &dir.join("absent-anaphase"), &f("c"), &f("m"));
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].what, "Anaphase");
    }

    #[test]
    fn missing_names_build_hints() {
        let dir = std::env::temp_dir().join(format!("up-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let missing = check_prereqs(
            &dir.join("t"),
            &dir.join("a"),
            &dir.join("c"),
            &dir.join("m"),
        );
        assert_eq!(missing.len(), 4);
        assert!(missing.iter().all(|m| m.hint.contains("cargo build")));
    }
}
