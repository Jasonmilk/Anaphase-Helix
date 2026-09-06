//! Rails: external human-authored knowledge rails (心智外铁轨).
//!
//! Read-only, version-frozen, deterministic citation rails (ADR-0018).
//! Helix may only select an existing edge — it never synthesizes one:
//!   - `build_index` parses a `rails/<kb>/` markdown DAG into a
//!     deterministic node index (sorted traversal, no UUID; nodes carry
//!     SHA-256 file fingerprints for version freezing).
//!   - `navigate` retrieves entry nodes by deterministic term match and
//!     seeds a visited set (citation provenance).
//!   - `verify_reference` enforces the citation contract: the quote must be
//!     verbatim content of a node the navigation actually visited.
//!   - `NO_RAIL_CONTENT` is the deterministic graceful refusal — a rail
//!     miss is answered with this constant, never with synthesized text.
//!
//! All functions are pure (no IO, no LLM) — byte-identical replay under the
//! same input, same acceptance family as the ledger (ADR-0003).
//!
//! Philosophy: rails are a *human asset* (like statutes). Helix never
//! rewrites them — the write side does not exist in this module
//! (`RailScope` has no write variant, enforced at the type level).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Deterministic graceful refusal: the rail has no such content.
/// Answer is this constant, never a generated sentence (citation contract).
pub const NO_RAIL_CONTENT: &str = "rails: no matching content (refused to synthesize)";

/// Deterministic citation answer (ADR-0018): assembled verbatim from the
/// injected rail nodes — 0 tokens, no LLM, no synthesis possible by
/// construction. Helix only selects an existing rail edge (a node), never
/// generates one. Each entry carries the node id so the citation is
/// machine-checkable against the index.
pub fn assemble_rail_answer(nodes: &[Node], kb: &str) -> String {
    let mut out = String::from("[rail citation · ");
    out.push_str(kb);
    out.push_str("]\n");
    for node in nodes {
        out.push_str(&format!("- {} ({}):\n> {}\n", node.heading, node.id, node.content));
    }
    out
}

/// Capability scope for rails access. Type-level read-only: there is no
/// write variant — Helix can read (cite) a rail, never rewrite it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RailScope {
    Read,
}

/// A single knowledge node: one markdown heading + its verbatim body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Deterministic id: `{doc}#{heading}` (duplicate headings get a `~N`
    /// suffix). No UUID — replayable by construction.
    pub id: String,
    /// Doc path relative to the kb root, without `.md`.
    pub doc: String,
    /// Heading text (node title).
    pub heading: String,
    /// Heading level (1..=6).
    pub level: u32,
    /// Verbatim body text (原文, trimmed).
    pub content: String,
    /// Child node ids (heading-level containment).
    pub children: Vec<String>,
    /// Parent node id (nearest shallower heading).
    pub parent: Option<String>,
    /// Outbound citation edge ids (resolved markdown links).
    pub refs: Vec<String>,
}

/// Whole-kb index. Serialized deterministically: BTreeMap ordering, no
/// absolute paths, no timestamps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    /// Knowledge base name (root directory name).
    pub kb: String,
    /// id -> node.
    pub nodes: BTreeMap<String, Node>,
    /// Root node ids (no parent) in id order.
    pub roots: Vec<String>,
    /// doc path -> SHA-256 hex (version-frozen fingerprint).
    pub file_sha256: BTreeMap<String, String>,
}

/// Result of one navigation: ranked entry hits + visited provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Navigation {
    /// Entry node ids, ranked (deterministic: score desc, id asc).
    pub hits: Vec<String>,
    /// Provenance set — citation verification requires membership here.
    pub visited: BTreeSet<String>,
}

/// Citation check result — same fixed-field shape as criteria::CheckReport
/// (deterministic JSON, replayable).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RailCheck {
    pub check: String,
    pub passed: bool,
    pub detail: String,
}

/// Build a deterministic index for one knowledge rail root.
/// Returns an error (rather than a partial index) on any dangling link —
/// rails must be well-formed by construction (deterministic + strict).
pub fn build_index(root: &Path) -> Result<Index, String> {
    let kb = root
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .ok_or_else(|| format!("invalid rail root: {}", root.display()))?;

    let mut files: Vec<(String, PathBuf)> = Vec::new();
    collect_md(root, root, &mut files)?;
    files.sort(); // deterministic traversal order

    let mut index = Index {
        kb: kb.clone(),
        nodes: BTreeMap::new(),
        roots: Vec::new(),
        file_sha256: BTreeMap::new(),
    };

    // Pass 1: parse nodes per file + record SHA-256 fingerprints.
    for (doc, path) in &files {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("read {}: {e}", path.display()))?;
        let digest = hex(&Sha256::digest(content.as_bytes()));
        index.file_sha256.insert(doc.clone(), digest);
        parse_doc(&mut index, doc, &content)?;
    }

    // Pass 2: resolve citation edges (dangling link -> Err).
    let doc_heads: BTreeMap<String, String> = {
        let mut m = BTreeMap::new();
        for (id, node) in &index.nodes {
            m.entry(node.doc.clone()).or_insert_with(|| id.clone());
        }
        m
    };
    let ids: Vec<String> = index.nodes.keys().cloned().collect();
    for id in &ids {
        let node = index.nodes[id].clone();
        let refs = resolve_refs(&index, &node.doc, &node.content, &doc_heads)?;
        index.nodes.get_mut(id.as_str()).expect("node exists").refs = refs;
    }

    // Roots: nodes without a parent, in id order (deterministic).
    index.roots = index
        .nodes
        .values()
        .filter(|n| n.parent.is_none())
        .map(|n| n.id.clone())
        .collect();

    Ok(index)
}

/// Serialized manifest view: kb, node count, per-file SHA-256. Deterministic
/// output — a human may freeze a version by saving it (version-frozen rail).
pub fn manifest(index: &Index) -> serde_json::Value {
    serde_json::json!({
        "kb": index.kb,
        "nodes": index.nodes.len(),
        "roots": index.roots.len(),
        "file_sha256": index.file_sha256,
    })
}

/// Deterministic entry retrieval: whole-query substring (CJK-friendly) plus
/// token matches, ranked by hit quality. No embeddings — probability-free.
pub fn navigate(index: &Index, query: &str, max_hits: usize) -> Navigation {
    let q = query.trim().to_lowercase();
    let terms: Vec<String> = q
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(String::from)
        .collect();
    // CJK bigrams: tokenization-free term features for unspaced scripts
    // (deterministic, no external segmenter). E.g. 数据归属 -> 数据/据归/归属.
    let bigrams = cjk_bigrams(&q);

    let mut scored: Vec<(i64, String)> = Vec::new();
    for (id, node) in &index.nodes {
        let heading = node.heading.to_lowercase();
        let content = node.content.to_lowercase();
        let mut score: i64 = 0;
        let mut hit = false;
        if !q.is_empty() {
            if heading.contains(&q) {
                score += 4;
                hit = true;
            } else if content.contains(&q) {
                score += 2;
                hit = true;
            }
        }
        for t in &terms {
            if heading.contains(t) {
                score += 3;
                hit = true;
            } else if content.contains(t) {
                score += 1;
                hit = true;
            }
        }
        // Bigram hits count as a theme only when >= 2 distinct bigrams land
        // (a single stray bigram like 法的 is noise, not a topic).
        let bg_hits: Vec<&String> = bigrams
            .iter()
            .filter(|b| heading.contains(*b) || content.contains(*b))
            .collect();
        if bg_hits.len() >= 2 {
            score += bg_hits.len() as i64 * 2;
            hit = true;
        }
        if hit {
            scored.push((score, id.clone()));
        }
    }
    // Deterministic order: score desc, id asc (no HashMap iteration).
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    scored.truncate(max_hits);
    let hits: Vec<String> = scored.into_iter().map(|(_, id)| id).collect();
    let visited: BTreeSet<String> = hits.iter().cloned().collect();
    Navigation { hits, visited }
}

/// Expand one node into its neighbors (parent / children / refs), recording
/// them in the visited set. Returns the newly discovered ids.
pub fn expand(index: &Index, id: &str, visited: &mut BTreeSet<String>) -> Vec<String> {
    let mut fresh = Vec::new();
    if let Some(node) = index.nodes.get(id) {
        let mut neighbors: Vec<&String> = node
            .children
            .iter()
            .chain(node.parent.iter())
            .chain(node.refs.iter())
            .collect();
        neighbors.sort();
        for nid in neighbors {
            if visited.insert(nid.clone()) {
                fresh.push(nid.clone());
            }
        }
    }
    fresh
}

/// Citation contract verifier (pure): the quote must be verbatim content of
/// a node the navigation actually visited. Any other case fails — no
/// partial credit, no generated text.
pub fn verify_reference(
    index: &Index,
    quote: &str,
    node_id: &str,
    visited: &BTreeSet<String>,
) -> RailCheck {
    let check = "rail_reference".to_string();
    let Some(node) = index.nodes.get(node_id) else {
        return RailCheck {
            check,
            passed: false,
            detail: format!("unknown node {node_id}"),
        };
    };
    if !visited.contains(node_id) {
        return RailCheck {
            check,
            passed: false,
            detail: format!("node {node_id} not visited by navigation"),
        };
    }
    if !node.content.contains(quote) {
        return RailCheck {
            check,
            passed: false,
            detail: format!("quote is not verbatim content of {node_id}"),
        };
    }
    RailCheck {
        check,
        passed: true,
        detail: format!("verbatim cite of {node_id}"),
    }
}

fn is_cjk(c: char) -> bool {
    let u = c as u32;
    (0x4E00..=0x9FFF).contains(&u) || (0x3400..=0x4DBF).contains(&u)
}

fn cjk_bigrams(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    for w in chars.windows(2) {
        if is_cjk(w[0]) && is_cjk(w[1]) {
            out.push(format!("{}{}", w[0], w[1]));
        }
    }
    out
}

// --- internals -----------------------------------------------------------

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn collect_md(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) -> Result<(), String> {
    let mut entries: Vec<std::fs::DirEntry> = std::fs::read_dir(dir)
        .map_err(|e| format!("read dir {}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name()); // deterministic traversal
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_md(root, &path, out)?;
        } else if path.extension().map(|e| e == "md").unwrap_or(false) {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| format!("strip prefix {}: {e}", path.display()))?;
            let doc = rel.with_extension("").to_string_lossy().replace('\\', "/");
            out.push((doc, path));
        }
    }
    Ok(())
}

/// Parse one markdown file into nodes. Heading level defines the tree;
/// body text belongs to the nearest preceding heading (sub-heading bodies
/// are their own nodes — no content duplication between parent and child).
fn parse_doc(index: &mut Index, doc: &str, content: &str) -> Result<(), String> {
    let mut nodes: Vec<Node> = Vec::new();
    let mut stack: Vec<usize> = Vec::new(); // indices into `nodes`
    let mut seq: BTreeMap<String, u32> = BTreeMap::new(); // base id -> occurrences
    let mut body = String::new();
    let mut pending: Option<usize> = None; // node index awaiting its body

    for raw in content.lines() {
        let line = raw.trim_end();
        let t = line.trim_start();
        let hash_count = t.chars().take_while(|c| *c == '#').count();
        if hash_count == 0 {
            // Ordinary body line: belongs to the nearest preceding heading.
            body.push_str(line);
            body.push('\n');
            continue;
        }
        if hash_count > 6 {
            return Err(format!("{doc}: heading level > 6: {line}"));
        }
        let rest = &t[hash_count..];
        if !rest.is_empty() && !rest.starts_with(' ') {
            // Not an ATX heading (e.g. "#foo"); treat as body.
            body.push_str(line);
            body.push('\n');
            continue;
        }
        let heading = rest.trim();
        if heading.is_empty() {
            return Err(format!("{doc}: empty heading"));
        }
        let level = hash_count as u32;

        // Commit the previous node's body.
        if let Some(i) = pending {
            nodes[i].content = body.trim().to_string();
            body.clear();
        }

        // Deterministic id (duplicate headings get `~N` suffix).
        let base = format!("{doc}#{heading}");
        let n = seq.entry(base.clone()).or_insert(0);
        *n += 1;
        let id = if *n == 1 { base } else { format!("{base}~{n}") };

        // Parent = nearest shallower heading.
        while let Some(&top) = stack.last() {
            if nodes[top].level < level {
                break;
            }
            stack.pop();
        }
        let parent_id = stack.last().map(|&i| nodes[i].id.clone());
        if let Some(pidx) = stack.last().copied() {
            nodes[pidx].children.push(id.clone());
        }

        nodes.push(Node {
            id: id.clone(),
            doc: doc.to_string(),
            heading: heading.to_string(),
            level,
            content: String::new(),
            children: Vec::new(),
            parent: parent_id,
            refs: Vec::new(),
        });
        pending = Some(nodes.len() - 1);
        stack.push(nodes.len() - 1);
    }

    if let Some(i) = pending {
        nodes[i].content = body.trim().to_string();
    }

    for node in nodes {
        index.nodes.insert(node.id.clone(), node);
    }
    Ok(())
}

/// Extract markdown link targets `[text](target)` — lightweight scanner,
/// no regex dependency (deterministic, documented approximation).
fn extract_links(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    let bytes = content.as_bytes();
    while i < content.len() {
        if bytes[i] == b']' && i + 1 < content.len() && bytes[i + 1] == b'(' {
            let start = i + 2;
            let end = content[start..]
                .find(')')
                .map(|e| start + e)
                .unwrap_or(content.len());
            out.push(content[start..end].trim().to_string());
            i = end + 1;
        } else {
            i += 1;
        }
    }
    out
}

/// Resolve link targets to node ids; any dangling link is an error.
fn resolve_refs(
    index: &Index,
    doc: &str,
    content: &str,
    doc_heads: &BTreeMap<String, String>,
) -> Result<Vec<String>, String> {
    let mut refs = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for target in extract_links(content) {
        let (path, heading) = match target.split_once('#') {
            Some((p, h)) => (p.trim(), h.trim()),
            None => (target.as_str(), ""),
        };
        let doc2 = if path.is_empty() {
            doc.to_string()
        } else {
            path.strip_suffix(".md").unwrap_or(path).to_string()
        };
        let id = if !heading.is_empty() {
            let base = format!("{doc2}#{heading}");
            if index.nodes.contains_key(&base) {
                base
            } else {
                // Duplicate-heading fallback: first matching node in id order.
                index
                    .nodes
                    .values()
                    .find(|n| n.doc == doc2 && n.heading == heading)
                    .map(|n| n.id.clone())
                    .ok_or_else(|| format!("{doc}: dangling link [{target}]"))?
            }
        } else {
            doc_heads
                .get(&doc2)
                .cloned()
                .ok_or_else(|| format!("{doc}: dangling doc link [{target}]"))?
        };
        if seen.insert(id.clone()) {
            refs.push(id);
        }
    }
    Ok(refs)
}
