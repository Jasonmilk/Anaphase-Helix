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
    let input = GcInput {
        objects,
        refs,
        /* C14b IS NOT LANDED: there is no pin store yet, so the collector sees ZERO holders. That is a
         * NAMED absence (§327.4), not a claim that nothing is pinned. */
        pins: vec![],
        vacancies,
        grace_secs,
    };
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
    })
}
