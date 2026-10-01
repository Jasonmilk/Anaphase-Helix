//! D1's criteria, one per invariant (ADR-0048 §328). Pure: no store, no clock, no files — so a failure
//! names the RULE, not the environment. Each test states the rule and, where the rule is a guard, the
//! mutation that would slip past if the guard were removed.

use super::gc::{plan, GcInput, Object, Vacancy};

fn obj(id: &str, parent: Option<&str>, stamped: bool) -> Object {
    Object { id: id.to_string(), parent: parent.map(|p| p.to_string()), stamped }
}

/// C5 + C6 + the happy path: stamped, unheld, past grace ⇒ collected.
#[test]
fn a_stamped_unheld_object_past_grace_is_collected() {
    let input = GcInput {
        objects: vec![obj("a", None, true)],
        refs: vec![],
        pins: vec![],
        vacancies: vec![Vacancy { id: "a".into(), at: 100 }],
        grace_secs: 10,
    };
    let p = plan(&input, 111);
    assert_eq!(p.collected, vec!["a".to_string()]);
    assert!(p.kept.is_empty());
}

/// C6 — a stamped object WITH A HOLDER is a ghost: never collected, always named.
#[test]
fn a_stamped_object_with_a_holder_is_a_ghost_and_is_kept() {
    let input = GcInput {
        objects: vec![obj("a", None, true)],
        refs: vec![],
        pins: vec![("a".into(), 1)],
        vacancies: vec![Vacancy { id: "a".into(), at: 1 }],
        grace_secs: 10,
    };
    let p = plan(&input, 10_000);
    assert!(p.collected.is_empty(), "a held object is never collected");
    assert_eq!(p.ghosts, vec!["a".to_string()], "and the ghost is NAMED");
    assert_eq!(p.kept, vec!["a".to_string()]);
}

/// C14 — pins are COUNTS: two holders, one releases, nothing is collected.
#[test]
fn pin_refcounts_survive_one_release() {
    let base = |pins: Vec<(String, u32)>| GcInput {
        objects: vec![obj("a", None, true)],
        refs: vec![],
        pins,
        vacancies: vec![Vacancy { id: "a".into(), at: 1 }],
        grace_secs: 10,
    };
    assert!(plan(&base(vec![("a".into(), 2)]), 10_000).collected.is_empty(), "two holders");
    assert!(plan(&base(vec![("a".into(), 1)]), 10_000).collected.is_empty(), "one holder left");
    assert_eq!(plan(&base(vec![("a".into(), 0)]), 10_000).collected, vec!["a".to_string()], "no holders");
    /* MUTATION: a SET model would have collected after the first release — the count is what forbids it. */
    let counted = super::gc::GcInput { pins: vec![("a".into(), 1), ("a".into(), 1)], ..base(vec![]) };
    assert_eq!(plan(&counted, 10_000).ghosts, vec!["a".to_string()], "two separate holders ADD up");
}

/// C9 — the anchor is the VACANCY FACT; when it is absent the absence is NAMED, never guessed.
#[test]
fn an_absent_anchor_is_named_not_guessed() {
    let input = GcInput {
        objects: vec![obj("a", None, true)],
        refs: vec![],
        pins: vec![],
        vacancies: vec![],
        grace_secs: 10,
    };
    let p = plan(&input, 10_000);
    assert!(p.collected.is_empty(), "without a fact there is nothing to measure the grace against");
    assert_eq!(p.no_anchor, vec!["a".to_string()], "and the missing anchor is named");
}

/// C9 (second half) — inside the window it is PROTECTED; the window itself is the fact's age.
#[test]
fn the_grace_window_is_measured_from_the_vacancy_fact() {
    let input = GcInput {
        objects: vec![obj("a", None, true)],
        refs: vec![],
        pins: vec![],
        vacancies: vec![Vacancy { id: "a".into(), at: 100 }],
        grace_secs: 10,
    };
    assert!(plan(&input, 105).collected.is_empty(), "5 < 10 ⇒ still protected");
    assert_eq!(plan(&input, 110).collected, vec!["a".to_string()], "exactly at the window ⇒ collectible");
    assert_eq!(plan(&input, 999).collected, vec!["a".to_string()]);
}

/// C13 — a PURE function of `(state, now)`: same arguments, same answer; a later `now` only ever adds.
#[test]
fn the_plan_is_a_pure_function_of_state_and_now() {
    let input = GcInput {
        objects: vec![obj("a", None, true), obj("b", Some("a"), true)],
        refs: vec![],
        pins: vec![],
        vacancies: vec![Vacancy { id: "a".into(), at: 0 }, Vacancy { id: "b".into(), at: 50 }],
        grace_secs: 10,
    };
    assert_eq!(plan(&input, 20), plan(&input, 20), "same state, same now ⇒ identical plan");
    let early = plan(&input, 20).collected;
    let late = plan(&input, 100).collected;
    assert!(early.iter().all(|x| late.contains(x)), "a later clock never UN-collects: {early:?} ⊄ {late:?}");
}

/// The theorem (no dangling): a kept child protects its stamped parent, and the protection is NAMED.
#[test]
fn a_kept_child_protects_its_stamped_parent() {
    let input = GcInput {
        objects: vec![obj("parent", None, true), obj("child", Some("parent"), false)],
        refs: vec![],
        pins: vec![],
        vacancies: vec![Vacancy { id: "parent".into(), at: 0 }],
        grace_secs: 1,
    };
    let p = plan(&input, 1_000);
    assert!(p.collected.is_empty(), "collecting the parent would leave the child dangling");
    assert_eq!(p.protected_by_descendant, vec!["parent".to_string()], "and the reason is named");
}

/// A ref is a ROOT (C-fact, §321): a stamped, unheld, long-vacated object that a ref names is kept.
#[test]
fn a_ref_is_a_root_and_nothing_it_names_is_collected() {
    let input = GcInput {
        objects: vec![obj("a", None, true)],
        refs: vec!["a".into()],
        pins: vec![],
        vacancies: vec![Vacancy { id: "a".into(), at: 0 }],
        grace_secs: 0,
    };
    assert!(plan(&input, 10_000).collected.is_empty(), "the root set is not the collector's to empty");
}

/// C5 — an UNSTAMPED object is never an input, whatever else is true of it.
#[test]
fn an_unstamped_object_is_never_collected() {
    let input = GcInput {
        objects: vec![obj("a", None, false)],
        refs: vec![],
        pins: vec![],
        vacancies: vec![Vacancy { id: "a".into(), at: 0 }],
        grace_secs: 0,
    };
    let p = plan(&input, 10_000);
    assert!(p.collected.is_empty() && p.ghosts.is_empty() && p.no_anchor.is_empty(), "no stamp, no case");
    assert_eq!(p.kept, vec!["a".to_string()]);
}
