// Upstream wire token accounting (ADR-0038).
//
// This module owns one concern: what an upstream response says about the cost
// of ONE round trip, and how to read it off the wire. It lives apart from
// `mod.rs` — which holds adapter *contracts* — because parsing a payload is an
// implementation detail, not a contract; and apart from `http_reasoning.rs` so
// the accounting rules can be tested without an HTTP round trip.
//
// Two rules shape everything here. Both come from DNA principle 11 (no fake
// placeholders) and the physical-fact-first stance:
//
//   1. The upstream's `prompt_tokens` INCLUDES cache hits
//      (`prompt_tokens = cache_hit + cache_miss`), so the internal convention
//      is DISJOINT counts: `uncached input = prompt_tokens - cached_tokens`.
//   2. "absent" and "reported as 0" are different facts. Optional buckets stay
//      `Option` and are never coerced to 0.

use serde::{Deserialize, Serialize};

/// Wire token accounting of ONE upstream round trip (ADR-0038).
///
/// `total_tokens` is deliberately NOT stored: in the OpenAI-compatible shape it
/// equals `prompt + completion`, so keeping it would give one fact two sources
/// — and when an upstream contradicts itself there would be no way to decide
/// which value is true. Derive it instead, guarded (`total()`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageSnapshot {
    /// Prompt tokens as reported — INCLUDING cache reads. Never surface this
    /// raw: it overlaps `cached_tokens`.
    pub prompt_tokens: u64,
    /// Generated tokens (the private-thinking share is `reasoning_tokens`).
    pub completion_tokens: u64,
    /// Cache reads: `prompt_tokens_details.cached_tokens` (OpenAI-compatible
    /// spelling) or `prompt_cache_hit_tokens` (native spelling).
    /// None = the upstream did not report it; never coerced to 0.
    pub cached_tokens: Option<u64>,
    /// `completion_tokens_details.reasoning_tokens` — the private-thinking
    /// share of `completion_tokens`. None = not reported.
    pub reasoning_tokens: Option<u64>,
}

impl UsageSnapshot {
    /// Disjoint uncached input. None when the upstream never said whether its
    /// prompt count includes cache reads — answering `prompt - 0` would
    /// fabricate a cache miss. Contradictory data (cache reads exceeding the
    /// prompt count) is also refused rather than clamped to a plausible
    /// number.
    pub fn uncached_input(&self) -> Option<u64> {
        match self.cached_tokens {
            Some(c) if c > self.prompt_tokens => None,
            Some(c) => Some(self.prompt_tokens - c),
            None => None,
        }
    }

    /// Derived total, guarded against overflow — a wrapped sum is not a fact.
    pub fn total(&self) -> Option<u64> {
        self.prompt_tokens.checked_add(self.completion_tokens)
    }
}

/// Everything the upstream response told us about ONE round trip
/// (ADR-0036 + ADR-0038): which route served it, and what it cost.
///
/// Both ride the same response, are captured at the same three sites and are
/// consumed at the same place, so they are ONE piece of metadata — two
/// parallel fields would double every call site for no gain.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UpstreamMeta {
    /// The routed model (ADR-0036): the upstream response's `model` field, not
    /// the config's declared name. None = the adapter saw no response.
    pub model: Option<String>,
    /// Token accounting, when the upstream reported it. None = it did not;
    /// never estimated (ADR-0038 D11).
    pub usage: Option<UsageSnapshot>,
}

/// Parse one upstream `usage` object into disjoint wire facts (ADR-0038).
///
/// Only counts actually present are kept: `cached_tokens` and
/// `reasoning_tokens` stay `None` when the upstream omits them (or sends
/// `null`), because "absent" and "reported as 0" are different facts.
/// `total_tokens` is not read at all — it duplicates `prompt + completion`, so
/// reading it would create a second source for one fact.
///
/// An optional bucket whose value contradicts the count it is a subset of
/// (`cached > prompt`, `reasoning > completion`) is dropped rather than stored
/// — an internally inconsistent number is not a fact. Absent required counts
/// make the whole record unusable: no usage is better than half of one.
pub fn parse_usage(value: Option<&serde_json::Value>) -> Option<UsageSnapshot> {
    // Absent or `null` both mean "no update" — never "all zero" (D6).
    let u = value?.as_object()?;
    let prompt_tokens = u.get("prompt_tokens").and_then(|v| v.as_u64())?;
    let completion_tokens = u.get("completion_tokens").and_then(|v| v.as_u64())?;
    // Cache reads carry two spellings; the OpenAI-compatible one wins and the
    // native one is the fallback (D7). Absent under both = not reported.
    let cached_tokens = u
        .get("prompt_tokens_details")
        .and_then(|d| d.get("cached_tokens"))
        .and_then(|v| v.as_u64())
        .or_else(|| u.get("prompt_cache_hit_tokens").and_then(|v| v.as_u64()))
        .filter(|c| *c <= prompt_tokens);
    let reasoning_tokens = u
        .get("completion_tokens_details")
        .and_then(|d| d.get("reasoning_tokens"))
        .and_then(|v| v.as_u64())
        .filter(|r| *r <= completion_tokens);
    Some(UsageSnapshot {
        prompt_tokens,
        completion_tokens,
        cached_tokens,
        reasoning_tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_disjoint_wire_facts() {
        let v = serde_json::json!({
            "prompt_tokens": 288, "completion_tokens": 46, "total_tokens": 334,
            "prompt_tokens_details": { "cached_tokens": 256 },
            "completion_tokens_details": { "reasoning_tokens": 43 }
        });
        let u = parse_usage(Some(&v)).expect("usage");
        assert_eq!(u.prompt_tokens, 288);
        assert_eq!(u.completion_tokens, 46);
        assert_eq!(u.cached_tokens, Some(256));
        assert_eq!(u.reasoning_tokens, Some(43));
        // Disjoint derivation: prompt_tokens already INCLUDES the cache reads.
        assert_eq!(u.uncached_input(), Some(32));
        assert_eq!(u.total(), Some(334));
    }

    #[test]
    fn treats_absent_and_null_as_no_usage() {
        assert!(parse_usage(None).is_none());
        assert!(parse_usage(Some(&serde_json::Value::Null)).is_none());
        let u = parse_usage(Some(&serde_json::json!({
            "prompt_tokens": 10, "completion_tokens": 2
        })))
        .unwrap();
        // Absent buckets stay None — "absent" and "reported as 0" differ.
        assert_eq!(u.cached_tokens, None);
        assert_eq!(u.reasoning_tokens, None);
        // Without a cache-read count the uncached input is unknowable:
        // answering `prompt - 0` would fabricate a cache miss.
        assert_eq!(u.uncached_input(), None);
    }

    #[test]
    fn accepts_both_cache_spellings() {
        let native = serde_json::json!({
            "prompt_tokens": 100, "completion_tokens": 5, "prompt_cache_hit_tokens": 60
        });
        assert_eq!(parse_usage(Some(&native)).unwrap().cached_tokens, Some(60));
        let both = serde_json::json!({
            "prompt_tokens": 100, "completion_tokens": 5,
            "prompt_cache_hit_tokens": 60,
            "prompt_tokens_details": { "cached_tokens": 70 }
        });
        // The OpenAI-compatible spelling wins; the native one is the fallback.
        assert_eq!(parse_usage(Some(&both)).unwrap().cached_tokens, Some(70));
    }

    #[test]
    fn drops_contradictory_optional_buckets() {
        // Cache reads cannot exceed the prompt count, reasoning cannot exceed
        // the output count: an inconsistent number is not a fact.
        let v = serde_json::json!({
            "prompt_tokens": 10, "completion_tokens": 3,
            "prompt_tokens_details": { "cached_tokens": 99 },
            "completion_tokens_details": { "reasoning_tokens": 99 }
        });
        let u = parse_usage(Some(&v)).unwrap();
        assert_eq!(u.cached_tokens, None);
        assert_eq!(u.reasoning_tokens, None);
        assert_eq!(u.uncached_input(), None);
    }

    #[test]
    fn refuses_incomplete_records() {
        // Half a usage record is worse than none.
        assert!(parse_usage(Some(&serde_json::json!({ "prompt_tokens": 10 }))).is_none());
        assert!(parse_usage(Some(&serde_json::json!({ "completion_tokens": 10 }))).is_none());
        // `total_tokens` alone is not enough — it is deliberately not read.
        assert!(parse_usage(Some(&serde_json::json!({ "total_tokens": 10 }))).is_none());
    }

    #[test]
    fn derived_total_refuses_overflow() {
        let u = UsageSnapshot {
            prompt_tokens: u64::MAX,
            completion_tokens: 1,
            cached_tokens: None,
            reasoning_tokens: None,
        };
        assert_eq!(u.total(), None, "a wrapped sum is not a fact");
    }
}
