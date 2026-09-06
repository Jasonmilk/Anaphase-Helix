// Mind live end-to-end: REAL Helix-Mind binary + REAL Anaphase client.
//
// Proves the physical contract between the two codebases (ADR-0031 craft +
// ADR-0032 wake-up): the vendored proto is byte-compatible with what the
// Mind server actually serves, and craft / ana_wakeup / helix_consolidate
// round trip over the real wire.
//
// Requirements (manual run, hence #[ignore]):
//   1. Mind binary built:  cargo build -p helix-mind-cli  (in ../Helix-Mind)
//      (binary name is helix-mind-cli)
//   2. Run from anaphase-helix:
//        cargo test --test mind_live -- --ignored --nocapture
//   Override the binary path with MIND_BIN if the default relative path
//   (../Helix-Mind/target/debug/helix-mind) is not correct.
//
// The test writes a minimal config (serde defaults fill the rest) into a
// per-run temp dir, copies the repo gene_lock example, spawns the server on
// an ephemeral port, polls readiness, then drives the real adapters.

use anaphase::adapters::mind::GrpcMindAdapter;
use anaphase::adapters::MemoryAdapter;
use anaphase::config::MindConfig;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

fn mind_bin() -> String {
    std::env::var("MIND_BIN")
        .unwrap_or_else(|_| "../Helix-Mind/target/debug/helix-mind-cli".to_string())
}

fn gene_lock_example() -> String {
    std::env::var("GENE_LOCK_EXAMPLE")
        .unwrap_or_else(|_| "../Helix-Mind/gene_lock.md.example".to_string())
}

fn write_minimal_config(dir: &std::path::Path, port: u16) -> std::io::Result<std::path::PathBuf> {
    let config_path = dir.join("config.toml");
    let sqlite = dir.join("helix_mind.db");
    let gene_lock = dir.join("gene_lock.md");
    let content = format!(
        "[storage]\nsqlite_path = \"{}\"\nparquet_dir = \"{}\"\ndeep_cold_dir = \"{}\"\nhuman_view_dir = \"{}\"\n\n[gene_lock]\nfile_path = \"{}\"\n\n[api]\nlisten_addr = \"127.0.0.1:{}\"\ntransport = \"tcp\"\n",
        sqlite.display(),
        dir.join("parquet").display(),
        dir.join("deep_cold").display(),
        dir.join("human_views").display(),
        gene_lock.display(),
        port
    );
    std::fs::write(&config_path, content)?;
    std::fs::copy(gene_lock_example(), &gene_lock)?;
    Ok(config_path)
}

fn spawn_mind(config_path: &std::path::Path) -> std::io::Result<Child> {
    Command::new(mind_bin())
        .arg("-c")
        .arg(config_path)
        .arg("run")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

async fn wait_ready(port: u16) -> bool {
    for _ in 0..100 {
        if tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .is_ok()
        {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

fn free_port() -> u16 {
    // Binding then dropping gives a likely-free port; race is acceptable for
    // an ignored manual live test.
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

async fn kill_mind(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[tokio::test]
#[ignore = "manual live test — requires Mind binary (see file header)"]
async fn p10_live_craft_wakeup_consolidate_over_real_mind() {
    // ── assemble a per-run temp dir + minimal config ──
    let dir = std::env::temp_dir().join(format!(
        "helix-p10-live-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let port = free_port();
    let config_path = write_minimal_config(&dir, port).unwrap();
    let mut child = spawn_mind(&config_path).expect(
        "Mind binary not found — build it first: cargo build -p helix-mind-cli (../Helix-Mind)",
    );

    let ready = wait_ready(port).await;
    if !ready {
        let _ = child.kill();
        panic!("Mind server did not become ready on port {port}");
    }

    // ── real client over the real wire ──
    let addr = format!("http://127.0.0.1:{port}");
    let adapter = GrpcMindAdapter::new(&addr, MindConfig::default())
        .await
        .expect("adapter connects");

    // P10a: cognitive craft — synthesis must come back (0-token
    // DeterministicAdapter path, the production default).
    let note = adapter
        .craft("评估当前状态", "live-job-1")
        .await
        .expect("craft over real Mind");
    assert!(!note.synthesis.is_empty(), "synthesis non-empty");
    assert_eq!(note.trace_id, "craft#live-job-1", "deterministic trace id");

    // P10d: agenda poll — empty database, no alarms (never a failure).
    let alarms = adapter.wakeup(60).await.expect("wakeup over real Mind");
    assert!(alarms.is_empty(), "fresh database has no due alarms");

    // P10c chain entry: consolidate hibernate — no-op on a fresh L1 (fewer
    // than 2 L1 nodes) is the honest, documented behaviour.
    adapter
        .consolidate("hibernate")
        .await
        .expect("consolidate over real Mind");

    kill_mind(&mut child).await;
    let _ = std::fs::remove_dir_all(&dir);
}
