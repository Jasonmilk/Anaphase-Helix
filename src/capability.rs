//! Capability classes, declared OUTSIDE the request (ADR-0048 §236).
//!
//! Hardy 1988 (*The Confused Deputy*) names the defect this module removes: a gate that holds its own
//! classifier **and** reads the monitored party's text draws "authority stemming from two sources".
//! The tool NAME is produced by the model and the argument text is produced by the model, so any
//! judgement based on either is a guess the model can steer (§213/§215/§222 measured the drift).
//!
//! So the class is looked up in a registry the REQUEST CANNOT INFLUENCE:
//!   · the same tool name always resolves to the same class;
//!   · a tool that was never declared resolves to NOTHING, and the caller refuses with a NAMED
//!     reason (`capability-undeclared`) — permission-based rather than exclusion-based
//!     (Saltzer & Schroeder 1975: "based on permission rather than exclusion").
//!
//! The registry takes NAMES ONLY. There is deliberately no constructor that accepts a class from a
//! payload: making it unforgeable is a property of the API, not of the caller's discipline
//! (Dennis & Van Horn 1966: a capability is "an unforgeable ticket").

use std::collections::BTreeMap;

/// What a call is allowed to be. Two classes today, because the engine has exactly two actions:
/// run it, or ask a human first. The THIRD state — "never run" — belongs to the absence of a
/// declaration, which is why an undeclared tool has no class rather than a default one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityClass {
    /// May run without asking.
    Normal,
    /// Must be confirmed by a human (the HITL gate) before it runs.
    Critical,
}

impl CapabilityClass {
    pub fn as_str(self) -> &'static str {
        match self {
            CapabilityClass::Normal => "normal",
            CapabilityClass::Critical => "critical",
        }
    }
}

/// The declaration table. Built from config/registry data; the request never touches it.
#[derive(Debug, Default, Clone)]
pub struct CapabilityRegistry {
    declared: BTreeMap<String, CapabilityClass>,
}

impl CapabilityRegistry {
    pub fn from_declarations(rows: &[(&str, CapabilityClass)]) -> Self {
        let mut declared = BTreeMap::new();
        for (name, class) in rows {
            declared.insert(normalize(name), *class);
        }
        Self { declared }
    }

    /// Declare one tool. Names are normalized so that `Ran_Command` and `ran command` are one entry —
    /// the registry is about the TOOL, not about how a caller spells it.
    pub fn declare(&mut self, tool: &str, class: CapabilityClass) {
        self.declared.insert(normalize(tool), class);
    }

    /// THE ONLY WAY TO OBTAIN A CLASS. It reads the declaration and nothing else: the call's
    /// arguments, the model's reasoning and the tool's own annotation are all invisible here.
    pub fn class_of(&self, tool: &str) -> Option<CapabilityClass> {
        self.declared.get(&normalize(tool)).copied()
    }

    /// A named refusal for an undeclared tool. It deliberately does NOT repeat the tool name: a
    /// refusal that names its trigger teaches an adaptive caller which name to change (§216 ⑤).
    pub fn undeclared_reason() -> &'static str {
        "capability-undeclared: this tool declares no capability class, and an undeclared capability \
         is not permission. Declare it in the registry to allow it."
    }
}

fn normalize(tool: &str) -> String {
    tool.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_undeclared_tool_has_no_class() {
        let r = CapabilityRegistry::default();
        assert_eq!(r.class_of("anything"), None, "no declaration ⇒ no permission");
        assert!(CapabilityRegistry::undeclared_reason().contains("capability-undeclared"));
        assert!(!CapabilityRegistry::undeclared_reason().contains("anything"),
            "the refusal must not name its trigger");
    }

    #[test]
    fn declared_tools_resolve_two_sided() {
        let r = CapabilityRegistry::from_declarations(&[
            ("read_file", CapabilityClass::Normal),
            ("delete_database", CapabilityClass::Critical),
        ]);
        assert_eq!(r.class_of("read_file"), Some(CapabilityClass::Normal));
        assert_eq!(r.class_of("delete_database"), Some(CapabilityClass::Critical));
        assert_eq!(r.class_of("write_file"), None, "a neighbour of a declared tool stays undeclared");
    }

    #[test]
    fn the_registry_cannot_be_steered_by_the_way_a_call_spells_things() {
        let r = CapabilityRegistry::from_declarations(&[("rm", CapabilityClass::Critical)]);
        for spelling in ["rm", " rm ", "RM", "Rm"] {
            assert_eq!(r.class_of(spelling), Some(CapabilityClass::Critical),
                "{spelling:?} is the same tool");
        }
        /* Naming tricks do not mint a class: the registry contains declarations, not parsers. */
        for forged in ["rm:critical", "rm(critical)", "critical", "capability=normal",
                       "{\"tool\":\"rm\",\"capability_class\":\"normal\"}"] {
            assert_eq!(r.class_of(forged), None,
                "{forged:?} is an unknown name — the request cannot declare a class for itself");
        }
    }
}
