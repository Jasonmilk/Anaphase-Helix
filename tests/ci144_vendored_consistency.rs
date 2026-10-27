//! K18 · CI-144 的 `vendored` 类型必须与 Cellrix 的上游定义**逐字段一致**。
//!
//! 为什么需要这条判据（不是"顺手加一个测试"）：
//! `anaphase-helix/src/ci144/mod.rs` 的协议类型是**从 Cellrix `cellrix-protocol` 复制来的**
//! （`mod.rs` 头注：@ 21d13d4，"serde shapes must stay field-for-field identical for MessagePack interop"）。
//! 复制品会漂：上游加一个字段、改名、或改**顺序**，anaphase 侧静默不知。
//! `ci144_anaphase_live.rs` 是**真往返**测试，但它只在两边**都启动**时跑，
//! 且它测的是"能通"，不是"形状相同"——上游**放宽**了类型、anaphase 用旧形状，
//! 往返照样通（旧字段少 ⇒ 解析时被忽略），**漂移不产生任何噪声**。
//!
//! ⇒ 本判据只做一件事：把两边的 `pub struct` / `pub enum` **形状**读出来对齐比较。
//!    **不作重构**（HANDOFF 批 1 K18 原文：只加判据）。
//!
//! 顺序敏感的**理由**：serde 的 struct 在 rmp（MessagePack）里默认序列化为**数组**（按声明序），
//! 所以字段**顺序**也是契约的一部分；把它当 set 比较会把真正的漂移判绿。
//!
//! 三态（与 `anaphase_live.rs` / `upstream_live.rs` 同一形状）：
//!   · 两侧都在且一致 ⇒ `CONSISTENT: N 个类型逐字段一致`（pass）
//!   · 上游源码不可得且未设 `CELLRIX_REQUIRED=1` ⇒ `NEEDS-INPUT: …`（pass，不假装验过）
//!   · 不可得但设了 ⇒ 具名 panic（这笔一致性必须红）
//!   · 两侧都在但不一致 ⇒ 具名 panic，逐条打印 `类型.字段` 差异
//!
//! 覆盖路径可用环境变量改（负样本要在**副本**上做，真件不动）：
//!   `CI144_VENDORED`（默认 `src/ci144/mod.rs`）· `CELLRIX_PROTOCOL_DIR`（默认 `../Cellrix/protocol/src`）

use std::path::{Path, PathBuf};

/// 一个类型的形状：`pub struct` 取字段名（有序），`pub enum` 取变体名（有序）。
#[derive(Debug, PartialEq, Eq)]
struct Shape {
    name: String,
    kind: &'static str, // "struct" | "enum"
    members: Vec<String>,
}

const VENDORED_DEFAULT: &str = "src/ci144/mod.rs";
const UPSTREAM_DEFAULT: &str = "../Cellrix/protocol/src";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn vendored_path() -> PathBuf {
    match std::env::var("CI144_VENDORED") {
        Ok(p) => root().join(p),
        Err(_) => root().join(VENDORED_DEFAULT),
    }
}

fn upstream_dir() -> PathBuf {
    match std::env::var("CELLRIX_PROTOCOL_DIR") {
        Ok(p) => root().join(p),
        Err(_) => root().join(UPSTREAM_DEFAULT),
    }
}

/// 从一段源码里抽出所有 `pub struct` / `pub enum` 的形状。
///
/// 刻意写得**笨**（逐行状态机，不引入任何解析库）：
/// 判据的可靠性来自"只认一种写法"，而不是"能读懂所有写法"。
/// 认不出的形状（tuple struct / 泛型 / 单行 `{}`）→ **具名报错**，不许静默跳过
/// （静默跳过 = 那个类型永远不会被判据看见 = 假覆盖）。
fn shapes_of(src: &str) -> Result<Vec<Shape>, String> {
    let mut out = Vec::new();
    let lines: Vec<&str> = src.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim();
        let (kind, name) = if let Some(rest) = t.strip_prefix("pub struct ") {
            ("struct", rest)
        } else if let Some(rest) = t.strip_prefix("pub enum ") {
            ("enum", rest)
        } else {
            i += 1;
            continue;
        };
        let name = name
            .trim_end_matches('{')
            .trim()
            .trim_end_matches(',')
            .trim()
            .to_string();
        if t.ends_with(';') {
            // tuple struct（`pub struct ViewHash(pub [u8; 32]);`）：形状 = 括号里的内层声明文本。
            // 不展开成字段名（没有名字），但**必须被看见**——它是 MessagePack 里的一个单元素数组。
            let inner = match (t.find('('), t.rfind(')')) {
                (Some(a), Some(b)) if b > a => t[a + 1..b].trim().to_string(),
                _ => return Err(format!("认不出的形状：`{t}`（以 `;` 结尾但没有括号体）")),
            };
            out.push(Shape {
                name,
                kind,
                members: vec![format!("tuple:{inner}")],
            });
            i += 1;
            continue;
        }
        if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(format!("认不出的形状：`{t}`（名字含泛型或约束）"));
        }
        if !t.ends_with('{') {
            return Err(format!("认不出的形状：`{t}`（声明行不以 `{{` 结尾）"));
        }
        let mut members = Vec::new();
        i += 1;
        while i < lines.len() && lines[i].trim() != "}" {
            let l = lines[i].trim();
            if kind == "struct" {
                if let Some(rest) = l.strip_prefix("pub ") {
                    if let Some((field, _)) = rest.split_once(':') {
                        members.push(field.trim().to_string());
                    }
                }
            } else if !l.is_empty() && !l.starts_with("//") && !l.starts_with("#[") {
                // 变体：`Name` | `Name(T)` | `Name { … }`
                let v: String = l
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !v.is_empty() && !matches!(v.as_str(), "pub" | "struct" | "enum") {
                    members.push(v);
                }
            }
            i += 1;
        }
        if i >= lines.len() {
            return Err(format!("{kind} {name} 没有闭合的 `}}`"));
        }
        out.push(Shape {
            name,
            kind,
            members,
        });
        i += 1;
    }
    Ok(out)
}

fn read_upstream_shapes(dir: &Path) -> Result<Vec<Shape>, String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("读不到上游目录 {}：{e}", dir.display()))?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "rs").unwrap_or(false))
        .collect();
    files.sort();
    let mut all = Vec::new();
    for f in files {
        let src = std::fs::read_to_string(&f).map_err(|e| format!("读不到 {}：{e}", f.display()))?;
        all.extend(shapes_of(&src).map_err(|e| format!("{}：{e}", f.display()))?);
    }
    Ok(all)
}

#[test]
fn vendored_ci144_types_match_cellrix_protocol() {
    let vpath = vendored_path();
    let vsrc = std::fs::read_to_string(&vpath)
        .unwrap_or_else(|e| panic!("读不到 vendored 源码 {}：{e}", vpath.display()));
    let vendored = shapes_of(&vsrc).unwrap_or_else(|e| panic!("vendored 侧：{e}"));
    assert!(
        vendored.len() >= 10,
        "vendored 侧只解析出 {} 个类型 ⇒ 判据自己坏了（不是类型真的变少）",
        vendored.len()
    );

    let udir = upstream_dir();
    let upstream = match read_upstream_shapes(&udir) {
        Ok(s) => s,
        Err(e) => {
            // ★ 三态第二格：上游不可得 ≠ 一致。绝不静默 pass 成"验过"。
            if std::env::var("CELLRIX_REQUIRED").as_deref() == Ok("1") {
                panic!("上游协议源码不可得（{e}）⇒ CELLRIX_REQUIRED=1 ⇒ 这笔一致性必须红");
            }
            println!(
                "NEEDS-INPUT: Cellrix protocol sources not found at {} （{e}）\
                 —— 未验；设 CELLRIX_REQUIRED=1 可把这一格变成红",
                udir.display()
            );
            return;
        }
    };

    // 逐类型对齐。★ 顺序敏感（MessagePack 里 struct 按声明序编成数组）。
    let mut diffs = Vec::new();
    for v in &vendored {
        match upstream.iter().find(|u| u.name == v.name) {
            None => diffs.push(format!("{}：上游没有同名 {}（vendored 独有）", v.name, v.kind)),
            Some(u) => {
                if u.kind != v.kind {
                    diffs.push(format!(
                        "{}：kind 不同（vendored {} vs 上游 {}）",
                        v.name, v.kind, u.kind
                    ));
                    continue;
                }
                if u.members != v.members {
                    let only_v: Vec<&String> =
                        v.members.iter().filter(|m| !u.members.contains(m)).collect();
                    let only_u: Vec<&String> =
                        u.members.iter().filter(|m| !v.members.contains(m)).collect();
                    let order = only_v.is_empty() && only_u.is_empty();
                    diffs.push(format!(
                        "{}：{}（vendored {:?} vs 上游 {:?}）{}{}{}",
                        v.name,
                        if order { "成员相同但【顺序】不同" } else { "成员不同" },
                        v.members,
                        u.members,
                        if only_v.is_empty() { String::new() } else { format!(" · 只在 vendored: {only_v:?}") },
                        if only_u.is_empty() { String::new() } else { format!(" · 只在上游: {only_u:?}") },
                        if order { " ⇒ MessagePack 按声明序编成数组，顺序也是契约" } else { "" }
                    ));
                }
            }
        }
    }

    assert!(
        diffs.is_empty(),
        "vendored 的 CI-144 类型与 Cellrix `cellrix-protocol` 漂了 {} 处：\n  {}\n\
         ⇒ 要么同步 vendored 副本（改 anaphase），要么承认这是有意的分叉并在此处具名豁免。\n\
         ⇒ 【不许】把本判据放宽成只看类型名——那样它就不再判「形状」了。",
        diffs.len(),
        diffs.join("\n  ")
    );
    println!(
        "CONSISTENT: {} 个类型逐字段一致（vendored {} ← 上游 {}）",
        vendored.len(),
        vpath.display(),
        udir.display()
    );
}
