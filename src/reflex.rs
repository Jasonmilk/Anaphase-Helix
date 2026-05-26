/// Fear adapter trait reference (import required for soft reflex)
use crate::adapters::FearAdapter;

/// Somatic reflex arc for immune system
/// Handles hard (fast) and soft (predictive) safety checks
pub struct ReflexArc {
    pub safety_rules: Vec<String>, // Safety constraints loaded from L2
}

impl ReflexArc {
    /// Hard reflex: O(1) hash matching for forbidden actions
    /// Returns true if action is SAFE, false if BLOCKED
    pub fn hard_reflex(&self, action: &str) -> bool {
        !self.safety_rules.iter().any(|rule| action.contains(rule))
    }

    /// Soft reflex: Calls fear prediction model
    /// Returns p_death (0.0 = no risk, 1.0 = critical risk)
    pub async fn soft_reflex(&self, fear: &dyn FearAdapter, context: &str) -> Result<f64, String> {
        // Call fear prediction and return death probability
        fear.predict_death(context).await
    }
}
