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
