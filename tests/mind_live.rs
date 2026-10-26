//! `anaphase ← helix-mind` 依赖边判据（**三态**）· 2026-10-27
//! · 可达 ⇒ 正向 query 必须成功（真连真 mind，不是 mock）
//! · 不可达 且 未设 `ANAPHASE_MIND_REQUIRED=1` ⇒ 打印 `NEEDS-INPUT:` 并 pass（CI 无 mind）
//! · 不可达 且 `ANAPHASE_MIND_REQUIRED=1` ⇒ panic 具名（ring 断上游时的红）
//! **端口不许字面量**（家族第 4 例）：读 `ecosystem/chain.json` 的 `components[name=mind].port`；
//! `MIND_ENDPOINT` 可覆盖。两者都取不到 ⇒ 缺输入，绝不猜端口。
use anaphase::adapters::mind::GrpcMindAdapter;
use anaphase::adapters::MemoryAdapter;
use std::time::Duration;

fn port_from_chain() -> Option<u64> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ecosystem/chain.json");
    let txt = std::fs::read_to_string(p).ok()?;
    let v: serde_json::Value = serde_json::from_str(&txt).ok()?;
    v.get("components")?.as_array()?.iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("mind"))?
        .get("port")?.as_u64()
}

fn required() -> bool {
    std::env::var("ANAPHASE_MIND_REQUIRED").map(|v| v.trim() == "1").unwrap_or(false)
}

fn needs_input(msg: String) {
    if required() { panic!("{msg}（ANAPHASE_MIND_REQUIRED=1 ⇒ 这笔依赖必须红）") } else { println!("{msg}") }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mind_live_query_roundtrip() {
    let base = match std::env::var("MIND_ENDPOINT").ok().filter(|s| !s.trim().is_empty()) {
        Some(b) => b,
        None => match port_from_chain() {
            Some(p) => format!("http://127.0.0.1:{p}"),
            None => { needs_input("NEEDS-INPUT: mind endpoint 未声明（MIND_ENDPOINT 未设，chain.json 无 mind 条目）".into()); return; }
        },
    };
    let connected = tokio::time::timeout(
        Duration::from_millis(4000),
        GrpcMindAdapter::new(&base, Default::default()),
    ).await;
    let adapter = match connected {
        Ok(Ok(a)) => a,
        Ok(Err(e)) => { needs_input(format!("NEEDS-INPUT: mind unreachable at {base}（{e}）")); return; }
        Err(_) => { needs_input(format!("NEEDS-INPUT: mind unreachable at {base}（连接超时 4000ms）")); return; }
    };
    let res = tokio::time::timeout(Duration::from_millis(8000), adapter.query("查询", false)).await
        .expect("query 超时 8000ms ⇒ mind 在线但无响应");
    assert!(res.is_ok(), "mind 在线 ⇒ query 必须成功，实际: {:?}", res.err());
}
