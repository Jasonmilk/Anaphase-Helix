//! Criterion A for the irreversible list: matrix of `I-1..I-7 x {hit, miss}`.
//!
//! Case construction discipline: every case is built from the **definition of the family**
//! in `helix-mind/docs/helixECO/irreversible-actions.md`, never from what the classifier
//! happens to recognise. Using the subject under test to define its own test set makes
//! coverage 100% and proves nothing.
//!
//! Every family gets two cases: one obvious, one boundary (the same consequence reached
//! through a less threatening name). Tool names use shapes that exist in this ecosystem
//! (`snake_case` verbs, capability strings from CAPABILITY-13 section 4).
//!
//! The test also carries the necessary-to-be-red sample: an I-1 call with no human
//! authorisation must be refused. If that ever goes green while unauthorised, the gate has
//! stopped deciding and the criterion must not be trusted.

use anaphase::irreversible::{classify, refuse_without_hitl, Family};

struct Case {
    family: Family,
    defined_as: &'static str,
    tool: &'static str,
    args: &'static str,
}

fn cases() -> Vec<Case> {
    vec![
        // I-1 funds transfer -- "money leaves and cannot be recalled"
        Case { family: Family::FundsTransfer, defined_as: "money out, irreversible once settled",
               tool: "transfer_funds", args: r#"{"amount":"1000","to":"acct-1"}"# },
        Case { family: Family::FundsTransfer, defined_as: "same consequence, neutral name",
               tool: "send_payout", args: r#"{"amount":"5"}"# },
        // I-2 purchase -- "creates a payment obligation"
        Case { family: Family::Purchase, defined_as: "creates a payment obligation",
               tool: "purchase_item", args: r#"{"sku":"x"}"# },
        Case { family: Family::Purchase, defined_as: "same consequence, neutral name",
               tool: "checkout", args: r#"{"cart":[]}"# },
        // I-3 credential rotation -- "old credential dead, peer already updated"
        Case { family: Family::CredentialRotate, defined_as: "old credential cannot be restored",
               tool: "rotate_key", args: r#"{"id":"k1"}"# },
        Case { family: Family::CredentialRotate, defined_as: "revocation side",
               tool: "revoke_credential", args: r#"{"id":"k2"}"# },
        // I-4 deletion -- "append-only semantics broken"
        Case { family: Family::Deletion, defined_as: "record gone for good",
               tool: "delete_record", args: r#"{"id":"r1"}"# },
        Case { family: Family::Deletion, defined_as: "capability-shaped write",
               tool: "write_file", args: r#"{"capability":"filesystem:write"}"# },
        // I-5 publication -- "read by a third party, cannot recall"
        Case { family: Family::Publish, defined_as: "third party can read it",
               tool: "publish_release", args: r#"{"tag":"v1"}"# },
        Case { family: Family::Publish, defined_as: "same consequence, neutral name",
               tool: "send_external_message", args: r#"{"to":"vendor"}"# },
        // I-6 physical actuation -- "no undo in the physical world"
        Case { family: Family::PhysicalActuation, defined_as: "physical world has no undo",
               tool: "actuate_servo", args: r#"{"angle":10}"# },
        Case { family: Family::PhysicalActuation, defined_as: "same consequence, domain word",
               tool: "robot_move", args: r#"{"dx":1}"# },
        // I-7 privilege grant -- "fact the moment it is used"
        Case { family: Family::PrivilegeGrant, defined_as: "delegation creates a fact",
               tool: "grant_permission", args: r#"{"scope":"admin"}"# },
        Case { family: Family::PrivilegeGrant, defined_as: "capability-shaped admin",
               tool: "reload_gene_lock", args: r#"{"capability":"admin:gene_lock"}"# },
    ]
}

#[test]
fn matrix_every_irreversible_family_is_classified() {
    let mut miss = Vec::new();
    let mut hit = Vec::new();
    for c in cases() {
        match classify(c.tool, c.args) {
            Some(f) if f == c.family => hit.push(format!("  {} {:14} HIT  {}", c.family.id(), c.family.label(), c.tool)),
            Some(other) => miss.push(format!(
                "  {} {:14} MISS {} classified as {} ({})",
                c.family.id(), c.family.label(), c.tool, other.id(), c.defined_as
            )),
            None => miss.push(format!(
                "  {} {:14} MISS {} not classified ({})",
                c.family.id(), c.family.label(), c.tool, c.defined_as
            )),
        }
    }
    let mut per_family = std::collections::BTreeMap::new();
    for c in cases() {
        let e = per_family.entry(c.family.id()).or_insert((0usize, 0usize));
        if classify(c.tool, c.args) == Some(c.family) { e.0 += 1 } else { e.1 += 1 }
    }
    println!("I-x  family                             hit miss");
    for (id, (h, m)) in &per_family {
        println!("{}  {:<34} {:>3} {:>4}", id, "", h, m);
    }
    for l in &hit { println!("{l}"); }
    for l in &miss { println!("{l}"); }
    assert!(miss.is_empty(), "{} of {} cases missed:\n{}", miss.len(), cases().len(), miss.join("\n"));
}

/// The sample that must be red: an I-1 call with **no** human authorisation has to be
/// refused with a message naming the family. If this goes green the gate stopped deciding.
#[test]
fn unauthorised_funds_transfer_is_refused_by_name() {
    let r = refuse_without_hitl("transfer_funds", r#"{"amount":"1000"}"#, false);
    let msg = r.expect("an unauthorised I-1 call must be refused, not executed");
    assert!(msg.contains("I-1"), "the refusal must name the family: {msg}");
    assert!(msg.contains("funds transfer"), "and its label: {msg}");
}

/// Counter-sample: an authorised I-1 call is not refused (the family means "someone must
/// decide", not "forbidden").
#[test]
fn authorised_funds_transfer_passes_the_gate() {
    assert!(refuse_without_hitl("transfer_funds", r#"{"amount":"1000"}"#, true).is_none());
}

/// Counter-sample: a genuinely harmless call must not be classified (otherwise the gate
/// becomes so noisy it gets routed around).
#[test]
fn harmless_calls_are_not_classified() {
    for (tool, args) in [
        ("read_file", r#"{"path":"/tmp/x"}"#),
        ("web_search", r#"{"q":"weather"}"#),
        ("numbers", r#"{"series":[1,2]}"#),
    ] {
        assert!(classify(tool, args).is_none(), "{tool} must not be irreversible");
        assert!(refuse_without_hitl(tool, args, false).is_none(), "{tool} must pass");
    }
}

/// H1/H2 measurement: does the **call-surface** judge (`is_high_risk_call`) cover the
/// **consequence** families? Same 14 cases, second judge, printed side by side.
///
/// This is the reading that decides whether the execution point can reuse the existing
/// gate (branch A) or needs the consequence dimension (branch B/C). It is a *reporting*
/// test: it prints the matrix and only fails if the consequence judge misses its own
/// cases (already asserted above).
#[test]
fn measure_call_surface_coverage_against_consequence_families() {
    use anaphase::hitl::HITLApprover;
    let mut covered = 0;
    let mut missed = 0;
    println!("\nI-x  tool                        consequence  call-surface(is_high_risk_call)");
    for c in cases() {
        let surf = HITLApprover::is_high_risk_call(c.tool, c.args);
        if surf { covered += 1 } else { missed += 1 }
        println!("{}  {:<28} {}", c.family.id(), c.tool, if surf { "HIT" } else { "MISS" });
    }
    println!("call-surface coverage of consequence families: {}/{} hit, {} miss",
             covered, covered + missed, missed);
}
