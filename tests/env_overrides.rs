//! ADR-0045 — the endpoint override surface must be **complete**, not merely
//! present for some fields.
//!
//! `anaphase-helix/config.toml` is gitignored, so a machine-rebuilt config
//! silently lost `tuck_endpoint` — which is both the audit leg and the
//! fail-closed gate — while the launcher had no channel to supply it. A launcher
//! can only be the chain's single source of truth if **every** endpoint can be
//! declared from outside the file.
//!
//! This drives the public `load_config()` path, so it also pins the
//! file-then-env precedence.
//!
//! **Both halves live in one test on purpose.** Environment variables are
//! process-global and this harness runs tests in parallel threads, so a second
//! test calling `remove_var` would race the first one's `set_var` (measured: the
//! first version of this file failed exactly that way). One test, sequential,
//! removes the hazard rather than documenting it.

use std::io::Write;

fn write_config(name: &str, body: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("anaphase-adr45-{}-{}", name, std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    let mut f = std::fs::File::create(&path).unwrap();
    write!(f, "{body}").unwrap();
    drop(f);
    path
}

#[test]
fn endpoints_are_declarable_from_outside_the_config_file() {
    // Minimal valid config: `Option` fields default to None, the two non-Option
    // ones must be present.
    let path = write_config(
        "a",
        "[anaphase]\ncap_http_enabled = true\ncap_http_port = 50061\n",
    );
    std::env::set_var("ANAPHASE_CONFIG", path.to_str().unwrap());

    // Half 1 — no override: the file's own value stands. Without this half, a
    // `load_config` that ignored the file entirely would still pass half 2.
    std::env::remove_var("ANAPHASE_TUCK_ENDPOINT");
    let c = anaphase::config::load_config().expect("the minimal config must load");
    assert_eq!(c.anaphase.tuck_endpoint, None, "the file says nothing about tuck");

    // Half 2 — every chain endpoint is declarable from outside the file.
    std::env::set_var("ANAPHASE_TUCK_ENDPOINT", "http://127.0.0.1:60052");
    std::env::set_var("ANAPHASE_FLOWMODUS_ENDPOINT", "grpc://127.0.0.1:60054");
    std::env::set_var("ANAPHASE_CELLRIX_ENDPOINT", "http://127.0.0.1:8080");
    std::env::set_var("ANAPHASE_MIND_ENDPOINT", "http://127.0.0.1:50052");

    let c = anaphase::config::load_config().expect("config loads");
    assert_eq!(
        c.anaphase.tuck_endpoint.as_deref(),
        Some("http://127.0.0.1:60052"),
        "the audit leg + fail-closed gate must be declarable from outside the file"
    );
    // M5：FlowModus 转发面已退役 ⇒ 该字段不再存在于结构体里。
    // 旧配置/环境变量里若仍写它，由 load_config 的"退役键提示"具名告知（不静默）。
    assert_eq!(c.anaphase.cellrix_endpoint.as_deref(), Some("http://127.0.0.1:8080"));
    assert_eq!(c.anaphase.mind_endpoint.as_deref(), Some("http://127.0.0.1:50052"));

    // Half 2b — `gene_lock_path`（d-machine-absolute-paths 的最后一条活配置绝对路径）
    // 可从环境声明：换机时不必改提交进仓的 config.toml。
    std::env::set_var("ANAPHASE_GENE_LOCK_PATH", "/tmp/other-machine/gene_lock.md");
    let c = anaphase::config::load_config().expect("config loads");
    assert_eq!(
        c.anaphase.gene_lock_path.as_deref(),
        Some("/tmp/other-machine/gene_lock.md"),
        "gene lock path must be declarable from the environment"
    );
    std::env::remove_var("ANAPHASE_GENE_LOCK_PATH");

    // Half 3 — a file value is overridden, and an empty env value is ignored.
    let path2 = write_config(
        "b",
        "[anaphase]\ncap_http_enabled = true\ncap_http_port = 50061\ntuck_endpoint = \"http://127.0.0.1:9999\"\n",
    );
    std::env::set_var("ANAPHASE_CONFIG", path2.to_str().unwrap());
    std::env::set_var("ANAPHASE_TUCK_ENDPOINT", "");
    let c = anaphase::config::load_config().expect("config loads");
    assert_eq!(
        c.anaphase.tuck_endpoint.as_deref(),
        Some("http://127.0.0.1:9999"),
        "an empty env value must not clear what the file said"
    );

    std::env::remove_var("ANAPHASE_TUCK_ENDPOINT");
    std::env::remove_var("ANAPHASE_FLOWMODUS_ENDPOINT");
    std::env::remove_var("ANAPHASE_CELLRIX_ENDPOINT");
    std::env::remove_var("ANAPHASE_MIND_ENDPOINT");
    std::env::remove_var("ANAPHASE_CONFIG");
}

// `d-machine-absolute-paths` 的一半：`gene_lock_path` 也必须可从环境声明。
// 并入**同一个测试**（env 是进程全局，另起一个测试会与上面的 set/remove 竞态 —— 文件头已写明）。
