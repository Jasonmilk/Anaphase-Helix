//! 三条辐条的**真实上游链路**判据（参数化 · 一份机制三边共用）· 2026-10-27
//!
//! 与既有测试的分工（这是本轮最重要的区分）：
//!   `health.rs` 里的测试问的是「健康检查逻辑对任意 endpoint 是否正确」——**保留不动**；
//!   本文件问的是「anaphase 与**真实上游**的链路是否通」——这才是"环"。
//!   断言在 ≠ 断言连的是真上游。
//!
//! 三态：可达 ⇒ 必须 ok / 不可达且未设 `ANAPHASE_UPSTREAM_REQUIRED=1` ⇒ NEEDS-INPUT 并 pass /
//!       不可达且设了 ⇒ 具名 panic。
//! 端口**一律来自 `ecosystem/chain.json`**，无字面量（写死端口 = 瞎掉的仪器）。

use anaphase::config::AnaphaseConfig;
use anaphase::health::checks;

fn ports() -> Vec<(String, u16)> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ecosystem/chain.json");
    let txt = match std::fs::read_to_string(p) { Ok(t) => t, Err(_) => return vec![] };
    let v: serde_json::Value = match serde_json::from_str(&txt) { Ok(v) => v, Err(_) => return vec![] };
    ["tentacle", "mind", "tuck"].iter().filter_map(|name| {
        let c = v.get("components")?.as_array()?.iter()
            .find(|c| c.get("name").and_then(|n| n.as_str()) == Some(*name))?;
        Some((name.to_string(), c.get("port")?.as_u64()? as u16))
    }).collect()
}

fn required() -> bool {
    std::env::var("ANAPHASE_UPSTREAM_REQUIRED").map(|v| v.trim() == "1").unwrap_or(false)
}

#[tokio::test]
async fn upstream_live_links() {
    let only = std::env::var("ANAPHASE_UPSTREAM_ONLY").ok().filter(|s| !s.trim().is_empty());
    let ps: Vec<(String, u16)> = ports().into_iter()
        .filter(|(n, _)| only.as_ref().map(|o| o == n).unwrap_or(true)).collect();
    assert!(!ps.is_empty(), "chain.json 读不到 tri-spoke 端口 ⇒ 缺输入，绝不猜端口");
    let mut missing: Vec<String> = vec![];
    for (name, port) in &ps {
        let ep = format!("http://127.0.0.1:{port}");
        let mut cfg = AnaphaseConfig::default();
        match name.as_str() {
            "tentacle" => cfg.tentacle_endpoint = Some(ep.clone()),
            "mind" => cfg.mind_endpoint = Some(ep.clone()),
            "tuck" => cfg.tuck_endpoint = Some(ep.clone()),
            _ => {}
        }
        let v = checks(&cfg);
        let ok = v.as_array().map(|a| a.iter().any(|c|
            c.get("name").and_then(|n| n.as_str()) == Some(name.as_str())
            && c.get("ok").and_then(|o| o.as_bool()) == Some(true))).unwrap_or(false);
        if ok {
            println!("LIVE-OK: {name} reachable at {ep}");
        } else {
            missing.push(format!("{name}@{ep}"));
        }
    }
    if !missing.is_empty() {
        let msg = format!("NEEDS-INPUT: upstream unreachable at {}", missing.join(" · "));
        if required() { panic!("{msg}（ANAPHASE_UPSTREAM_REQUIRED=1 ⇒ 这几条依赖必须红）") } else { println!("{msg}") }
    }
}
