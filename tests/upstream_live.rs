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
    // 端点名一律取自 health.rs 的 check_endpoint 声明（边普查 tools/edges-census.py 会核对）。
    // 与组件名不同的两处靠 chain.json 的 alias 归一（flowmodus→flowmodus-serve, reasoning→flowmodus-reason），
    // 端口仍只写在 chain.json 一处。
    const NAMES: [&str; 5] = ["tentacle", "mind", "tuck", "flowmodus", "reasoning"];
    NAMES.iter().filter_map(|name| {
        let list = v.get("components")?.as_array()?;
        let hit = list.iter().find(|c| {
            c.get("name").and_then(|n| n.as_str()) == Some(*name)
                || c.get("alias").and_then(|a| a.as_array())
                    .map(|a| a.iter().any(|x| x.as_str() == Some(*name)))
                    .unwrap_or(false)
        })?;
        Some((name.to_string(), hit.get("port")?.as_u64()? as u16))
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
    // ★ 端点缺失必须判红，不许被 filter_map 静默丢掉（丢掉的边永远绿 = 假覆盖）
    let all: Vec<String> = ports().into_iter().map(|(n, _)| n).collect();
    assert!(all.len() == 5,
        "chain.json 只解析到 {} 个端点 {:?}（期望 5：tentacle/mind/tuck/flowmodus/reasoning）⇒ 缺输入，绝不猜端口，也绝不静默跳过",
        all.len(), all);
    let mut missing: Vec<String> = vec![];
    for (name, port) in &ps {
        let ep = format!("http://127.0.0.1:{port}");
        let mut cfg = AnaphaseConfig::default();
        match name.as_str() {
            "tentacle" => cfg.tentacle_endpoint = Some(ep.clone()),
            "mind" => cfg.mind_endpoint = Some(ep.clone()),
            "tuck" => cfg.tuck_endpoint = Some(ep.clone()),
            "flowmodus" => cfg.flowmodus_endpoint = Some(ep.clone()),
            "reasoning" => cfg.reasoning_endpoint = Some(ep.clone()),
            _ => panic!("判据不认识端点 {name} ⇒ 判据自身坏了，不得静默跳过（否则这条边永远绿）"),
        }
        let v = checks(&cfg);
        // 引擎返回 {"ok":…,"governance":…,"checks":[…]}
        // —— 这里曾写成 `v.as_array()` ⇒ 永远取不到行 ⇒ 判据恒红（"恒红型假失败"）。
        // 故下方先断言"取到了数组"，取不到就具名失败，不许静默当成"不可达"。
        let arr = v.get("checks").and_then(|c| c.as_array());
        assert!(arr.is_some(), "checks() 返回里没有 checks 数组 ⇒ 判据自身坏了，不得当成『上游不可达』");
        let row = arr.and_then(|a| a.iter()
            .find(|c| c.get("name").and_then(|n| n.as_str()) == Some(name.as_str())));
        let ok = row.and_then(|c| c.get("ok")).and_then(|o| o.as_bool()) == Some(true);
        // R2：先出诊断（引擎原始 detail），再判红 —— 退出码只表结论，不表诊断。
        let detail = row.and_then(|c| c.get("detail")).and_then(|d| d.as_str()).unwrap_or("<无 detail>").to_string();
        if ok {
            println!("LIVE-OK: {name} reachable at {ep}");
        } else {
            missing.push(format!("{name}@{ep} · detail={detail}"));
        }
    }
    if !missing.is_empty() {
        let msg = format!("NEEDS-INPUT: upstream unreachable at {}", missing.join(" · "));
        if required() { panic!("{msg}（ANAPHASE_UPSTREAM_REQUIRED=1 ⇒ 这几条依赖必须红）") } else { println!("{msg}") }
    }
}
