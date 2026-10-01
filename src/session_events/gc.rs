//! D1 — THE COLLECTOR'S PURE CORE (ADR-0048 §328).
//!
//! Criteria first: every invariant below is expressed as a PURE function of `(state, now)` so that each one
//! can be tested on its own and can go RED on its own. Three properties follow from the shape rather than
//! from discipline:
//!   · **C13** the plan is a pure function of `(replayed state, now)` — `now` is a PARAMETER, so a compaction
//!     cannot lose the anchor, and nothing reads a clock behind the caller's back;
//!   · **C16** reachability is computed from a replayed value, never from a store handle, so the write
//!     critical section cannot accidentally call it;
//!   · **C9** the grace anchor comes from a VACANCY FACT (recorded by the `ref/move` event), never from the
//!     collector's own bookkeeping; an absent anchor is NAMED, never guessed.
//!
//! Deletion has three knobs (`D0` tombstone · `D1` collect + `purge` event · `D2` content destruction); this
//! module decides only the middle one, and it decides nothing without a fact to point at.

use std::collections::{HashMap, HashSet};

/// One object as the collector sees it — replayed facts only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Object {
    pub id: String,
    pub parent: Option<String>,
    /// "Stamped" = marked for deletion (today: a `period/tombstone` row).
    pub stamped: bool,
}

/// The fact that a position was VACATED, and when. Recorded by the `ref/move` event (ADR-0048 §321).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vacancy {
    pub id: String,
    pub at: u64,
}

#[derive(Debug, Clone, Default)]
pub struct GcInput {
    pub objects: Vec<Object>,
    pub refs: Vec<String>,
    /// **Refcounts, not a set** (C14): two holders releasing must not collect anything.
    pub pins: Vec<(String, u32)>,
    pub vacancies: Vec<Vacancy>,
    pub grace_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GcPlan {
    pub kept: Vec<String>,
    pub collected: Vec<String>,
    /// Stamped but STILL HELD ⇒ a ghost: never collected, always named (C6).
    pub ghosts: Vec<String>,
    /// Stamped and unheld, but no vacancy fact exists ⇒ the anchor is ABSENT and is named (C9).
    pub no_anchor: Vec<String>,
    /// Would have been collected, but a kept descendant still needs it (the theorem: no dangling).
    pub protected_by_descendant: Vec<String>,
}

/// Decide the collection — purely. `now` is passed in; nothing here reads a clock or a store.
pub fn plan(input: &GcInput, now: u64) -> GcPlan {
    let mut parent_of: HashMap<&str, Option<&str>> = HashMap::new();
    let mut stamped: HashSet<&str> = HashSet::new();
    for o in &input.objects {
        parent_of.insert(o.id.as_str(), o.parent.as_deref());
        if o.stamped {
            stamped.insert(o.id.as_str());
        }
    }

    /* C14 — a pin is a COUNT. `pins` may carry several entries for one id; they ADD. */
    let mut held: HashMap<&str, u32> = HashMap::new();
    for (id, n) in &input.pins {
        *held.entry(id.as_str()).or_insert(0) += *n;
    }

    /* C9 — the anchor is the vacancy FACT. Several facts ⇒ the LATEST one is the anchor. */
    let mut vacated_at: HashMap<&str, u64> = HashMap::new();
    for v in &input.vacancies {
        let e = vacated_at.entry(v.id.as_str()).or_insert(v.at);
        if v.at > *e {
            *e = v.at;
        }
    }

    let refs: HashSet<&str> = input.refs.iter().map(|s| s.as_str()).collect();

    let mut ghosts = Vec::new();
    let mut no_anchor = Vec::new();
    let mut candidates: Vec<&str> = Vec::new();
    for o in &input.objects {
        let id = o.id.as_str();
        if !stamped.contains(id) {
            continue; /* C5: only a stamped object is ever an INPUT to collection */
        }
        if held.get(id).copied().unwrap_or(0) > 0 {
            ghosts.push(o.id.clone()); /* C6: a stamped but held object is a ghost */
            continue;
        }
        match vacated_at.get(id) {
            Some(at) if now.saturating_sub(*at) >= input.grace_secs => candidates.push(id),
            Some(_) => { /* inside the grace window: protected by the anchor */ }
            None => no_anchor.push(o.id.clone()), /* C9: the absence is NAMED, not guessed */
        }
    }

    /* THE THEOREM (no dangling): whatever stays must keep its ancestors. So every ancestor of an object
     * that is NOT collected is protected — and a candidate that protects someone is kept, named as such. */
    let candidate_set: HashSet<&str> = candidates.iter().copied().collect();
    let mut protected: HashSet<&str> = HashSet::new();
    for o in &input.objects {
        let id = o.id.as_str();
        let stays = !candidate_set.contains(id) || refs.contains(id);
        if !stays {
            continue;
        }
        let mut cursor = parent_of.get(id).copied().flatten();
        let mut guard = 0;
        while let Some(p) = cursor {
            if !protected.insert(p) {
                break; /* already walked from here */
            }
            if guard > 4096 {
                break; /* a cycle is not a lineage; the chain reader names that separately */
            }
            guard += 1;
            cursor = parent_of.get(p).copied().flatten();
        }
    }

    let mut collected = Vec::new();
    let mut protected_by_descendant = Vec::new();
    for id in candidates {
        if refs.contains(id) {
            continue; /* a ref IS a root: nothing a ref points at is collectible */
        }
        if protected.contains(id) {
            protected_by_descendant.push(id.to_string());
        } else {
            collected.push(id.to_string());
        }
    }

    let collected_set: HashSet<&str> = collected.iter().map(|s| s.as_str()).collect();
    let kept = input
        .objects
        .iter()
        .filter(|o| !collected_set.contains(o.id.as_str()))
        .map(|o| o.id.clone())
        .collect();

    let mut sort = |v: &mut Vec<String>| v.sort();
    let (mut kept, mut collected, mut ghosts, mut no_anchor, mut protected_by_descendant) =
        (kept, collected, ghosts, no_anchor, protected_by_descendant);
    sort(&mut kept);
    sort(&mut collected);
    sort(&mut ghosts);
    sort(&mut no_anchor);
    sort(&mut protected_by_descendant);

    GcPlan { kept, collected, ghosts, no_anchor, protected_by_descendant }
}

/* ── THE STORE-FACING HALF (ADR-0048 §329) ─────────────────────────────────────────────────────────
 * The collector reads a REPLAYED state from the streams, decides with the pure `plan`, and records the
 * decision as a `period/purge` row in the object's OWN stream — one book, same writer lock, and no bytes
 * destroyed (that is `D2`). The state is read INSIDE the lock (P15): a dangling check that happens outside
 * it is a TOCTOU window where someone can attach between the check and the collection. */

/// RFC3339 (`…Z`) to epoch seconds. Pure, and tested against the inverse the ledger already owns.
pub fn rfc3339_to_secs(ts: &str) -> Option<u64> {
    let b = ts.as_bytes();
    if b.len() < 19 { return None; }
    let num = |a: usize, z: usize| -> Option<i64> { ts.get(a..z)?.parse::<i64>().ok() };
    let (y, mo, d) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let (h, mi, sec) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || sec > 60 { return None; }
    /* days_from_civil (Hinnant): civil date -> days since 1970-01-01, no lookup tables. */
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = if y2 >= 0 { y2 } else { y2 - 399 } / 400;
    let yoe = y2 - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    let secs = days * 86400 + h * 3600 + mi * 60 + sec;
    if secs < 0 { None } else { Some(secs as u64) }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GcOutcome {
    pub collected: Vec<String>,
    pub ghosts: Vec<String>,
    pub no_anchor: Vec<String>,
    pub protected_by_descendant: Vec<String>,
    /// C14b: holdings whose owner is not declared alive. They still PROTECT (conservatively) and are NAMED,
    /// because a silently collected object is worse than a named leak. Releasing them is an explicit act.
    pub orphan_pins: Vec<(String, String)>,
}

/// Read the replayed state, decide, and RECORD the decision as `period/purge` facts.
pub fn collect_garbage(dir: &std::path::Path, now: u64, grace_secs: u64) -> std::io::Result<GcOutcome> {
    let _writer = super::refs::WriterLock::acquire(dir)?; /* P15: one critical section for check + act */
    let mut objects: Vec<Object> = Vec::new();
    let mut vacancies: Vec<Vacancy> = Vec::new();
    let entries = std::fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let fname = entry.file_name().to_string_lossy().to_string();
        let Some(stem) = fname.strip_suffix(".events.jsonl") else { continue };
        if !super::identity::is_period_id(stem) { continue; }
        let body = match std::fs::read_to_string(entry.path()) { Ok(b) => b, Err(_) => continue };
        let mut parent: Option<String> = None;
        let mut stamped = false;
        for line in body.lines() {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
            match v.get("type").and_then(|t| t.as_str()) {
                Some("period/tombstone") => stamped = true,
                Some("context/inject") => {
                    if parent.is_none() {
                        if let Some(p) = v.get("data").and_then(|d| d.get("resume_from")).and_then(|r| r.as_str()) {
                            if super::identity::is_period_id(p) { parent = Some(p.to_string()); }
                        }
                    }
                }
                Some("ref/move") => {
                    let d = v.get("data").cloned().unwrap_or(serde_json::Value::Null);
                    let gone = d.get("new").map(|n| n.is_null()).unwrap_or(false);
                    if gone {
                        if let Some(old) = d.get("old").and_then(|o| o.as_str()) {
                            let at = v.get("time").and_then(|t| t.as_str()).and_then(rfc3339_to_secs).unwrap_or(0);
                            vacancies.push(Vacancy { id: old.to_string(), at });
                        }
                    }
                }
                _ => {}
            }
        }
        objects.push(Object { id: stem.to_string(), parent, stamped });
    }
    let refs: Vec<String> = super::refs::list_refs(dir)?.into_iter().map(|r| r.period_id).collect();
    /* C14b (ADR-0048 §330): holders come from the pin WAL, replayed — a restart cannot lose them, and an
     * owner that is no longer declared is NAMED below rather than silently protecting forever. */
    let (by_object, _) = super::pins::replay(dir)?;
    let pins: Vec<(String, u32)> = by_object.into_iter().collect();
    let orphans = super::pins::orphan_pins(dir)?;
    let input = GcInput { objects, refs, pins, vacancies, grace_secs };
    let decided = plan(&input, now);
    for id in &decided.collected {
        let mut stream = super::types_and_stream::SessionEventStream::open_append(
            dir.to_path_buf(), id, id, crate::trace::Redaction::default(),
        )?;
        stream.emit(
            &super::types_and_stream::now_ts(),
            super::types_and_stream::EventType::Purge,
            serde_json::json!({ "at": now }),
        )?;
    }
    Ok(GcOutcome {
        collected: decided.collected,
        ghosts: decided.ghosts,
        no_anchor: decided.no_anchor,
        protected_by_descendant: decided.protected_by_descendant,
        orphan_pins: orphans,
    })
}

/* ── REPLAY: TWO PREDICATES, NEVER ONE (ADR-0048 §331) ────────────────────────────────────────────
 * `D0` HIDES an object; `D1` makes it STOP EXISTING. Collapsing those into one predicate is how "the
 * reader does not show it" and "it is gone" become the same sentence — and then a replay that ignores the
 * tombstone silently revives something that was purged. So there are two, and the end-to-end criterion
 * checks both: a purged object is neither visible NOR existing, and removing the purge fact (the mutation)
 * brings it back — which is the reviewer's C8, measured in their sandbox and pinned here. */
fn scan(dir: &std::path::Path, ignore_purge: bool) -> std::io::Result<(Vec<String>, Vec<String>)> {
    let mut visible = Vec::new();
    let mut existing = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let fname = entry.file_name().to_string_lossy().to_string();
        let Some(stem) = fname.strip_suffix(".events.jsonl") else { continue };
        if !super::identity::is_period_id(stem) { continue; }
        let Ok(body) = std::fs::read_to_string(entry.path()) else { continue };
        let mut purged = false;
        let mut hidden = false;
        let mut rows = 0usize;
        for line in body.lines() {
            if line.trim().is_empty() { continue; }
            rows += 1;
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
            match v.get("type").and_then(|t| t.as_str()) {
                Some("period/purge") => purged = true,
                Some("period/tombstone") => hidden = true,
                _ => {}
            }
        }
        if rows == 0 { continue; }
        let gone = purged && !ignore_purge;
        if !gone { existing.push(stem.to_string()); }
        if !gone && !hidden { visible.push(stem.to_string()); }
    }
    visible.sort();
    existing.sort();
    Ok((visible, existing))
}

/// What a READER sees: rows, no tombstone, no purge.
pub fn replay_live(dir: &std::path::Path) -> std::io::Result<Vec<String>> {
    Ok(scan(dir, false)?.0)
}

/// What still EXISTS on disk: rows and no purge (a tombstoned object still exists — D0 destroys nothing).
pub fn replay_exists(dir: &std::path::Path) -> std::io::Result<Vec<String>> {
    Ok(scan(dir, false)?.1)
}

/// THE MUTATION, available to the criterion only: pretend the purge facts were never written.
pub fn replay_exists_ignoring_purge(dir: &std::path::Path) -> std::io::Result<Vec<String>> {
    Ok(scan(dir, true)?.1)
}

/* ── D2: CONTENT DESTRUCTION (ADR-0048 §332) ──────────────────────────────────────────────────────
 * The third knob. `D0` hides, `D1` records that the object is gone; `D2` removes the CONTENT of objects
 * that are staying (or already gone). Two rules decide the shape:
 *   · **ID AND PARENT SURVIVE** (C7). Deleting whole files breaks the chain — the child's `resume_from`
 *     would point at an id nothing can resolve any more, and the reader would (correctly) report a
 *     dangling parent. So the rows stay, with identity, lineage, `seq` and `time` untouched.
 *   · **DESTRUCTION NAMES ITSELF.** A row whose `text` was destroyed must not look like a row that never
 *     had text: that is the three-state law this project keeps meeting ("absent" ≠ "empty" ≠ "destroyed").
 *     Each affected row gains `data.content = "destroyed"`.
 * Rewriting is the one place this store is not append-only, so it happens under the WRITER LOCK. */
/* P42 (ADR-0048 §333): a CONTENT list is fail-OPEN — a new content key would silently survive while the UI
 * still said "destroyed". So the list is inverted: what SURVIVES is declared, everything else in `data` is
 * destroyed, and a key that is neither declared as surviving nor known as content is ALSO NAMED
 * (`unclassified`) so a new field cannot appear silently in either direction. */
/// Atomic rewrite (P40): write a temporary sibling, flush it, then rename over the target. A crash before the
/// rename leaves the PREVIOUS content fully intact, which is the whole point.
pub fn rewrite_atomically(path: &std::path::Path, body: &str) -> std::io::Result<()> {
    let tmp = std::path::PathBuf::from(format!("{}.tmp", path.display()));
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(body.as_bytes())?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

const SURVIVING_KEYS: [&str; 14] = [
    "resume_from", "model", "choice", "nodes", "chars", "injected_chars", "done", "success", "verdict",
    "impasse", "completion_tokens", "cached_tokens", "prompt_tokens", "at",
];
const KNOWN_CONTENT_KEYS: [&str; 8] =
    ["text", "reply", "reason", "summary", "args", "result", "preview", "content_body"];

/// Returns how many rows had content destroyed (`0` = a NAMED absence: nothing carried any).
/// What D2 did, and what it DECLARED rather than assumed (P41/P42/P43).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContentPurgeReport {
    pub destroyed_rows: usize,
    /// Keys that were neither declared as surviving nor known as content: destroyed AND named.
    pub unclassified: Vec<String>,
    /// What deliberately survived — a fact, so "destroyed" is never read as "everything is gone".
    pub retained: Vec<String>,
    /// The SCOPE of this operation (P43): the period streams. Snapshots and the audit chain are NOT covered,
    /// and saying so is part of the operation rather than a footnote.
    pub scope: Vec<String>,
}

pub fn purge_content(dir: &std::path::Path, ids: &[String]) -> std::io::Result<ContentPurgeReport> {
    let _writer = super::refs::WriterLock::acquire(dir)?;
    let mut destroyed_rows = 0usize;
    let mut unclassified: Vec<String> = Vec::new();
    for id in ids {
        let path = dir.join(format!("{id}.events.jsonl"));
        let body = match std::fs::read_to_string(&path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("cannot destroy content of {id}: no such stream ({})", path.display()),
                ))
            }
            Err(e) => return Err(e),
        };
        let mut out = String::new();
        for line in body.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let mut v: serde_json::Value = serde_json::from_str(line).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, format!("torn row in {id}: {e}"))
            })?;
            let mut hit = false;
            if let Some(data) = v.get_mut("data").and_then(|d| d.as_object_mut()) {
                let keys: Vec<String> = data.keys().cloned().collect();
                for key in keys {
                    if SURVIVING_KEYS.contains(&key.as_str()) {
                        continue; /* declared to survive: lineage, audit counters, decision facts */
                    }
                    if !KNOWN_CONTENT_KEYS.contains(&key.as_str()) {
                        unclassified.push(format!("{id}.{key}"));
                    }
                    data.remove(&key); /* everything else is destroyed (fail-CLOSED) */
                    hit = true;
                }
                if hit {
                    data.insert("content".to_string(), serde_json::Value::String("destroyed".to_string()));
                }
            }
            if hit {
                destroyed_rows += 1;
            }
            out.push_str(&serde_json::to_string(&v).unwrap_or_else(|_| line.to_string()));
            out.push('\n');
        }
        /* P40: A REWRITE IS THE ONLY NON-APPEND PATH, so a crash in the middle of one would leave the WHOLE
         * stream unparsable (measured elsewhere: ONE bad row took three suites down). `fs::write` truncated
         * the target first — exactly the failure mode this forbids. Temp file, flush, then RENAME. */
        rewrite_atomically(&path, &out)?;
    }
    unclassified.sort();
    Ok(ContentPurgeReport {
        destroyed_rows,
        unclassified,
        retained: SURVIVING_KEYS.iter().map(|k| k.to_string()).collect(),
        scope: vec!["period streams (<id>.events.jsonl)".to_string()],
    })
}
