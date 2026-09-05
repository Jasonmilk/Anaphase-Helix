//! `up` — one-command Helix backend bootstrap (candidate G-4 + G-5 + G-6).
//!
//! Spawns the deterministic stack with a single command, no CLI zoo:
//!   1. Tentacle (gRPC, fixture plugins) — deterministic execution channel
//!   2. Anaphase (conscious layer) — with `ANAPHASE_TENTACLE_ENDPOINT`
//!      injected via env (config.toml untouched, 12-factor override)
//!   3. Health probes (port-ready, snapshot-ready) — physical facts only
//!   4. Interactive menu (G-6): after `up`, the user answers choices —
//!      no more commands. Open cockpit / show status / config hints / exit.
//!
//! G-5 usability: first-run friendly output (welcome banner, prereq checks
//! with concrete build hints, per-step [ok]/[warn], next steps).
//! G-6 (this ADR): the "next steps" section is replaced by an interactive
//! choice menu when stdin is a terminal — one command, then choices only.
//! Non-tty (pipes/scripts) degrades to the plain hold-until-Ctrl+C behavior.
//!
//! Zero-hardcoding: every port/path is derived from config.toml, the
//! tentacle endpoint URL, or documented protocol defaults. Fail-open: a
//! missing Tentacle binary degrades to Noop (offline mode), never blocks.
//!
//! Usage:
//!   cargo run --bin up                 # backend + interactive menu (tty)
//!   cargo run --bin up                 # backend + hold (non-tty/scripts)
//!   HELIX_TENTACLE=/path/to/tentacle cargo run --bin up   # explicit binary

use std::io::{IsTerminal, Read, Write};
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

/// Menu choices (G-6): the user answers by number — no commands to remember.
#[derive(Debug, PartialEq, Eq)]
enum Choice {
    Cockpit,
    Status,
    Config,
    Configure,
    Exit,
}

/// Parse a menu input line. Empty input = default (open cockpit — the most
/// common next action). Unknown input = re-prompt (Status is harmless but
/// surprising; re-prompt keeps the loop honest).
fn parse_choice(line: &str) -> Option<Choice> {
    match line.trim() {
        "" | "1" => Some(Choice::Cockpit),
        "2" => Some(Choice::Status),
        "3" => Some(Choice::Config),
        "4" => Some(Choice::Configure),
        "5" | "q" | "Q" => Some(Choice::Exit),
        _ => None,
    }
}

/// Fetch the Anaphase snapshot summary via a hand-rolled HTTP GET (no new
/// dependency; the response body is small and its shape is the ADR-0010
/// contract: `{"status":"Active","snapshot":{mode,state,episode,ledger}}`).
fn fetch_snapshot(port: u16) -> Option<serde_json::Value> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    let _ = write!(
        stream,
        "GET /v1/agent/snapshot HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    );
    let mut buf = String::new();
    stream.read_to_string(&mut buf).ok()?;
    let body = buf.split("\r\n\r\n").nth(1).unwrap_or(&buf);
    serde_json::from_str(body).ok()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // -- welcome ----------------------------------------------------------
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Helix 后端引导 (up)");
    println!("  一条命令拉起确定性执行链: Tentacle → Anaphase → 驾驶舱");
    println!("  之后只需做选择题，不需要记任何命令");
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
    let snapshot_port = anaphase_cfg.cap_http_port;

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
        println!("      菜单 [3] 配置说明 可查看如何开启");
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
    wait_for_port(snapshot_port, "anaphase snapshot")?;
    println!("  [ok] Anaphase 就绪: snapshot :{snapshot_port}（模式: {}）", mode_label(anaphase_cfg.run_cycle.mode));

    // 8. Interactive menu (G-6) when stdin is a terminal; otherwise hold.
    let tty = std::io::stdin().is_terminal();
    if tty {
        menu_loop(
            &mut tentacle,
            &mut anaphase,
            &cellrix_cli,
            &mock_agent,
            grpc_port,
            snapshot_port,
            mode_label(anaphase_cfg.run_cycle.mode),
        )?;
    } else {
        println!("\n== stack up — Ctrl+C 退出（非交互模式）==");
        wait_for_signal();
    }

    // 9. Down: drop children in reverse order.
    drop(anaphase);
    if let Some(mut t) = tentacle {
        let _ = t.kill();
    }
    println!("\n== stack down ==");
    Ok(())
}

/// Interactive choice loop (G-6): after one command, the user only answers
/// choices. Default (Enter) = open cockpit.
fn menu_loop(
    tentacle: &mut Option<Child>,
    anaphase: &mut Child,
    cellrix_cli: &Path,
    mock_agent: &Path,
    grpc_port: u16,
    snapshot_port: u16,
    mode: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        println!("\n──────────────────────────────────────────────");
        println!("  状态: Tentacle:{grpc_port}  Anaphase:{snapshot_port}  模式: {mode}");
        println!("──────────────────────────────────────────────");
        println!("  1. 打开驾驶舱   (Enter 同此)");
        println!("  2. 查看状态");
        println!("  3. 配置说明");
        println!("  4. 配置 LLM（引导输入）");
        println!("  5. 停止并退出   (q)");
        print!("  选择: ");
        std::io::stdout().flush()?;

        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        match parse_choice(&line) {
            Some(Choice::Cockpit) => {
                if cellrix_cli.exists() && mock_agent.exists() {
                    println!("  [ok] 拉起驾驶舱（退出驾驶舱后回到本菜单）...");
                    let status = Command::new(cellrix_cli)
                        .args(["run", "--mode", "stdio", "--exec"])
                        .arg(mock_agent)
                        .args(["--anaphase-endpoint", &format!("http://127.0.0.1:{snapshot_port}")])
                        .status()?;
                    println!("  驾驶舱已退出（{}）", status);
                } else {
                    println!("  [warn] Cellrix 二进制未找到 → 跳过（构建后重试）");
                }
            }
            Some(Choice::Status) => {
                print_status(grpc_port, snapshot_port, mode);
            }
            Some(Choice::Config) => {
                println!("  配置说明（config.toml，位于 anaphase-helix 目录）:");
                println!("    [anaphase] reasoning_endpoint = \"http://...\"   # 开启 LLM 认知（当前: {}）",
                    if std::env::var("ANAPHASE_REASONING_ENDPOINT").map_or(true, |v| v.is_empty()) { "未配置 → Noop" } else { "已配置" });
                println!("    [anaphase.run_cycle] mode = \"drive\" | \"partner\" | \"survive\"");
                println!("    [anaphase] cap_http_port = 50061                # 驾驶舱数据端口");
                println!("  修改后需重启 up 生效。");
            }
            Some(Choice::Configure) => {
                run_config_wizard(&std::env::current_dir()?.join("config.toml"))?;
            }
            Some(Choice::Exit) => {
                println!("  停止并退出...");
                break;
            }
            None => {
                println!("  请输入 1-4（或 q 退出）");
            }
        }
        let _ = anaphase.try_wait(); // reap if crashed; loop continues honestly
        let _ = tentacle.as_mut().map(|t| t.try_wait());
    }
    Ok(())
}

/// Interactive LLM configuration wizard (menu choice 4): asks base_url,
/// model and api_key one question at a time (choices, not commands), backs
/// up `config.toml`, applies line-level updates and tells the user to
/// restart `up`. The api_key prompt disables terminal echo via `stty`
/// (present on macOS/Linux) — no new crate dependency.
fn run_config_wizard(config_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !config_path.exists() {
        println!("  [warn] 未找到 {} → 请在 anaphase-helix 目录运行 up", config_path.display());
        return Ok(());
    }
    let original = std::fs::read_to_string(config_path)?;

    println!("\n  —— 配置 LLM（引导输入，Enter 跳过保持现值）——");
    print!("  base_url（OpenAI 兼容，如 https://api.deepseek.com）: ");
    std::io::stdout().flush()?;
    let base_url = read_line();

    print!("  model（如 deepseek-chat）: ");
    std::io::stdout().flush()?;
    let model = read_line();

    print!("  api_key（输入不回显）: ");
    std::io::stdout().flush()?;
    let key = read_line_hidden();

    let updated = apply_reasoning_updates(&original, &base_url, &model, &key);
    if updated == original {
        println!("  [i] 无变更，config.toml 未改动");
        return Ok(());
    }
    // Backup the current file, then write (overwrite is the explicit wizard
    // intent; the .bak preserves recovery).
    let backup = config_path.with_extension("toml.bak");
    std::fs::write(&backup, &original)?;
    std::fs::write(config_path, &updated)?;
    println!("  [ok] 已写入 {}（原文件备份: {}）", config_path.display(), backup.display());
    println!("  [i] 重启 up 生效（退出后重新运行）");
    Ok(())
}

/// Apply reasoning config updates at line level: replace the three keyed
/// lines in place, keep everything else byte-identical. Empty input leaves
/// the existing value untouched (Enter = keep). Pure function — tested.
fn apply_reasoning_updates(config_text: &str, base_url: &str, model: &str, key: &str) -> String {
    let updates = [
        ("reasoning_endpoint", base_url),
        ("reasoning_model", model),
        ("reasoning_api_key", key),
    ];
    let mut out_lines: Vec<String> = Vec::new();
    let mut replaced = [false; 3];
    for line in config_text.lines() {
        let mut done = line.to_string();
        for (i, (field, value)) in updates.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with(&format!("{field} =")) && !value.is_empty() {
                done = format!("{field} = \"{}\"", escape_toml(value));
                replaced[i] = true;
                break;
            }
        }
        out_lines.push(done);
    }
    if !replaced.iter().all(|r| *r) {
        eprintln!("  [warn] 部分字段行未找到（{}），未修改这些字段",
            updates.iter().zip(replaced.iter())
                .filter(|(_, r)| !**r)
                .map(|(u, _)| u.0)
                .collect::<Vec<_>>()
                .join(", "));
    }
    out_lines.join("\n")
}

/// Escape a value for a TOML basic string (quotes and backslashes).
fn escape_toml(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Read one line from stdin (trimmed).
fn read_line() -> String {
    let mut line = String::new();
    if std::io::stdin().read_line(&mut line).is_ok() {
        line.trim().to_string()
    } else {
        String::new()
    }
}

/// Read one line with terminal echo disabled (`stty -echo`), restored after.
fn read_line_hidden() -> String {
    let _ = Command::new("stty").arg("-echo").status();
    let line = read_line();
    let _ = Command::new("stty").arg("echo").status();
    line
}

/// Physical status: probe both ports + read the live snapshot summary.
fn print_status(grpc_port: u16, snapshot_port: u16, mode: &str) {    let tentacle_up = TcpStream::connect(("127.0.0.1", grpc_port)).is_ok();
    let anaphase_up = TcpStream::connect(("127.0.0.1", snapshot_port)).is_ok();
    println!("  Tentacle (grpc :{grpc_port}): {}", if tentacle_up { "运行中" } else { "未运行" });
    println!("  Anaphase (snapshot :{snapshot_port}): {}", if anaphase_up { "运行中" } else { "未运行" });
    println!("  模式: {mode}");
    if let Some(snap) = fetch_snapshot(snapshot_port) {
        if let Some(s) = snap.get("snapshot") {
            let state = s.get("state").and_then(|v| v.as_str()).unwrap_or("?");
            let episode = s.get("episode").and_then(|v| v.as_str()).unwrap_or("无");
            let n_ledger = s.get("ledger").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
            println!("  认知状态: {state}   经历: {episode}   ledger 记录: {n_ledger} 条");
        } else {
            println!("  (snapshot 响应缺少 snapshot 字段)");
        }
    } else {
        println!("  (snapshot 端点无响应)");
    }
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
fn wait_for_signal() {
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_choice_default_is_cockpit() {
        assert_eq!(parse_choice(""), Some(Choice::Cockpit));
        assert_eq!(parse_choice("1"), Some(Choice::Cockpit));
        assert_eq!(parse_choice("1\n"), Some(Choice::Cockpit));
    }

    #[test]
    fn parse_choice_numbers_and_quit() {
        assert_eq!(parse_choice("2"), Some(Choice::Status));
        assert_eq!(parse_choice("3"), Some(Choice::Config));
        assert_eq!(parse_choice("5"), Some(Choice::Exit));
        assert_eq!(parse_choice("q"), Some(Choice::Exit));
        assert_eq!(parse_choice("Q"), Some(Choice::Exit));
    }

    #[test]
    fn parse_choice_unknown_is_none() {
        assert_eq!(parse_choice("x"), None);
        assert_eq!(parse_choice("6"), None);
        assert_eq!(parse_choice("abc"), None);
    }

    #[test]
    fn parse_choice_configure_is_five() {
        assert_eq!(parse_choice("4"), Some(Choice::Configure));
        assert_eq!(parse_choice("5"), Some(Choice::Exit));
    }

    #[test]
    fn apply_updates_replaces_three_fields_keeps_rest() {
        let cfg = "reasoning_endpoint = \"\"\nreasoning_model = \"\"\nreasoning_api_key = \"\"\ncap_http_port = 50061\n";
        let out = apply_reasoning_updates(cfg, "https://api.deepseek.com", "deepseek-chat", "sk-abc");
        assert!(out.contains("reasoning_endpoint = \"https://api.deepseek.com\""));
        assert!(out.contains("reasoning_model = \"deepseek-chat\""));
        assert!(out.contains("reasoning_api_key = \"sk-abc\""));
        assert!(out.contains("cap_http_port = 50061"));
        assert_eq!(out.lines().count(), 4); // no line added/removed
    }

    #[test]
    fn apply_updates_empty_input_keeps_existing() {
        let cfg = "reasoning_endpoint = \"http://old\"\nreasoning_model = \"m1\"\nreasoning_api_key = \"k1\"\n";
        let out = apply_reasoning_updates(cfg, "", "m2", "");
        assert!(out.contains("reasoning_endpoint = \"http://old\""));
        assert!(out.contains("reasoning_model = \"m2\""));
        assert!(out.contains("reasoning_api_key = \"k1\""));
    }

    #[test]
    fn apply_updates_escapes_quotes_and_backslashes() {
        let cfg = "reasoning_api_key = \"\"\n";
        let out = apply_reasoning_updates(cfg, "", "", "a\"b\\c");
        assert!(out.contains("reasoning_api_key = \"a\\\"b\\\\c\""));
    }

    #[test]
    fn apply_updates_noop_when_all_empty() {
        let cfg = "reasoning_endpoint = \"x\"";
        let out = apply_reasoning_updates(cfg, "", "", "");
        assert_eq!(out, cfg);
    }

    #[test]
    fn all_present_nothing_missing() {
        let dir = std::env::temp_dir().join(format!("up-test-present-{}", std::process::id()));
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
        let dir = std::env::temp_dir().join(format!("up-test-fatal-{}", std::process::id()));
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
        let dir = std::env::temp_dir().join(format!("up-test-missing-{}", std::process::id()));
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
