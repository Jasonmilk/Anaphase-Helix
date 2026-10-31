//! Irreversible action families (I-1..I-7), classified **by consequence**.
//!
//! Why this exists: `hitl::HITLApprover::is_high_risk_call` judges the **call surface**
//! (a command-bearing field). The irreversible list judges **consequence** (money moved,
//! data gone, physical world touched). Those are different axes: the most dangerous
//! financial operation looks like a perfectly normal call. So the execution point needs
//! a consequence classifier, not the call-surface one.
//!
//! Definitions copied verbatim in intent from
//! `helix-mind/docs/helixECO/irreversible-actions.md` (v0). The list is DATA and it lives
//! in helix-mind; this module must not carry a second copy of the list. Until the
//! cross-repo reference is wired, the mapping is declared here and the criterion
//! `tests/irreversible_matrix.rs` prints it, so a drift between the two shows up as a
//! red line rather than as silent agreement.
//!
//! Rule of the gate: a call whose family is `Some` **must be accompanied by a human
//! authorisation** at the execution point. The family is not "forbidden" — it is
//! "requires someone to decide".

/// I-1..I-7. The numbers are part of the contract: they are what the list uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    FundsTransfer,      // I-1
    Purchase,           // I-2
    CredentialRotate,   // I-3
    Deletion,           // I-4
    Publish,            // I-5
    PhysicalActuation,  // I-6
    PrivilegeGrant,     // I-7
}

impl Family {
    pub fn id(&self) -> &'static str {
        match self {
            Family::FundsTransfer => "I-1",
            Family::Purchase => "I-2",
            Family::CredentialRotate => "I-3",
            Family::Deletion => "I-4",
            Family::Publish => "I-5",
            Family::PhysicalActuation => "I-6",
            Family::PrivilegeGrant => "I-7",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Family::FundsTransfer => "funds transfer",
            Family::Purchase => "purchase / order",
            Family::CredentialRotate => "credential rotation or revocation",
            Family::Deletion => "data deletion",
            Family::Publish => "external publication",
            Family::PhysicalActuation => "physical actuation",
            Family::PrivilegeGrant => "privilege grant / delegation",
        }
    }
}

/// Classify a tool call by the consequence it can have.
///
/// Deliberately keyed on the **capability** a call needs plus the verb the family is
/// defined by, never on free text alone (see the measured false-positive rate that made
/// `is_high_risk_call` stop scanning args as text). Returns `None` for anything that
/// cannot reach one of the seven consequences.
pub fn classify(tool: &str, args_json: &str) -> Option<Family> {
    let t = tool.to_ascii_lowercase();

    // I-1 / I-2: money. Both need a payment-shaped capability; the split is whether the
    // call moves funds out (transfer) or creates a payment obligation (order).
    if has(&t, &["transfer", "remit", "wire", "payout", "withdraw"]) {
        return Some(Family::FundsTransfer);
    }
    if has(&t, &["purchase", "order", "checkout", "subscribe", "billing"]) {
        return Some(Family::Purchase);
    }
    // I-3: credentials. Rotating or revoking cannot be undone once the peer has adopted it.
    if has(&t, &["rotate", "revoke", "rekey", "rotate_key"]) {
        return Some(Family::CredentialRotate);
    }
    // I-4: deletion. Irreversible under append-only semantics.
    if has(&t, &["delete", "purge", "destroy", "rm_"]) {
        return Some(Family::Deletion);
    }
    // I-5: publication. Once read by a third party it cannot be recalled.
    if has(&t, &["publish", "post_public", "announce", "send_external"]) {
        return Some(Family::Publish);
    }
    // I-6: the physical world. `execute:*` is the sandbox capability from CAPABILITY-13
    // section 4; treat it as actuation only when the target is not a sandboxed pure call.
    if has(&t, &["actuate", "servo", "motor", "valve", "robot"]) {
        return Some(Family::PhysicalActuation);
    }
    // I-7: privilege. Delegation and grants create facts the moment they are used.
    if has(&t, &["grant", "delegate", "authorize", "escalate", "sudo"]) {
        return Some(Family::PrivilegeGrant);
    }
    // Capability-based fallback: a call that declares the admin gene-lock capability is
    // privilege-shaped whatever its name; `network:outbound` alone is not enough to be
    // irreversible (the spec routes it through Tuck), so it is not classified here.
    if args_json.contains("admin:gene_lock") {
        return Some(Family::PrivilegeGrant);
    }
    if args_json.contains("filesystem:write") {
        return Some(Family::Deletion);
    }
    None
}

fn has(tool: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| tool.contains(n))
}

/// The gate: at the execution point, a classified call must carry a human authorisation.
/// `authorised` is true only when a human decision **for this call** exists.
pub fn refuse_without_hitl(tool: &str, args_json: &str, authorised: bool) -> Option<String> {
    match classify(tool, args_json) {
        Some(f) if !authorised => Some(format!(
            "irreversible family {} ({}): must be decided by a human, not executed by default \
             (tool={tool})",
            f.id(),
            f.label()
        )),
        _ => None,
    }
}
