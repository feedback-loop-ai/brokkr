//! Keep-refs' pure half: which git objects does a run's journal CITE?
//!
//! A journal names commits — the head recorded when the run entered its
//! protected phase, and the head each realm reported at ship. The chain
//! that proves the journal is hash-based, not git-based, so the journal
//! keeps verifying long after a squash-merge, a branch deletion and a
//! `git gc` have collected the objects it cites. The evidence goes; the
//! citation stays. Keep-refs plants a ref per cited object so the
//! exhibits outlive the branch that carried them (decision 0028); this
//! module answers only the reading half of that — no git, no filesystem,
//! no process (decision 0003, constitutional boundary 1).
//!
//! This is deliberately NOT [`crate::fold::fold`]'s `reviewed_heads`.
//! State keeps the LATEST head, correctly: it is state, not history. A
//! reforged run (decision 0022) revisits the protected phase and can
//! cite a different head each time, and every one of those commits is an
//! exhibit the journal named. Keep-refs needs the whole set across the
//! run's lifetime, so it folds the events itself.
//!
//! The fold reads structured payload fields off the event vocabulary and
//! nothing else — no text search over messages or blobs. An event that
//! is not `transition/decided`, an input that is not a head, and a head
//! that is not an object name are all simply not citations.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::envelope::{EventEnvelope, EventType};

/// The protected phase's record: realm name (or the legacy unkeyed
/// `repo`, per [`crate::realms::LEGACY_REALM_KEY`]) to observed head.
const REVIEWED_HEADS: &str = "reviewed_heads";
/// Ship's per-realm record; each realm's facts may carry a `head`.
const REALM_FACTS: &str = "realm_facts";
const HEAD: &str = "head";

/// Does this string name a git object? 40 hex for SHA-1 repositories,
/// 64 for SHA-256 ones, and nothing else.
///
/// The check is not fastidiousness: the collected string becomes part of
/// a ref name, so anything that is not an object name has no business
/// reaching git. A payload field that holds a branch name, a path or a
/// truncated head is not a citation this mechanism can keep, and saying
/// so here is cheaper than a refused `update-ref` later.
fn is_object_name(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Collect one candidate. Object names are canonically lowercase, so a
/// head recorded in upper case is the SAME exhibit and must not plant a
/// second ref for it.
fn cite(kept: &mut BTreeSet<String>, candidate: Option<&Value>) {
    if let Some(name) = candidate.and_then(Value::as_str) {
        if is_object_name(name) {
            kept.insert(name.to_ascii_lowercase());
        }
    }
}

/// Every distinct git object a run's journal cites, in sorted order.
///
/// Sorted and deduplicated because the result is a plan of refs to
/// plant: two decisions citing one head are one exhibit, and the same
/// journal must always yield the same plan.
///
/// The fold checks no protocol shape — that is [`crate::fold::fold`]'s
/// office. A journal too damaged to fold still names exhibits worth
/// keeping, and keeping them is exactly how the damage stays
/// investigable.
pub fn cited_shas(events: &[EventEnvelope]) -> BTreeSet<String> {
    events
        .iter()
        .filter(|event| event.event_type == EventType::TransitionDecided)
        .fold(BTreeSet::new(), |mut kept, event| {
            let Some(inputs) = event.payload.get("inputs").and_then(Value::as_object) else {
                return kept;
            };
            // Both shapes the contract names and no third one: keyed by
            // realm, or the single unkeyed head a pre-map journal wrote.
            // Reading the object's VALUES covers both without asking
            // which world this run believed in.
            if let Some(heads) = inputs.get(REVIEWED_HEADS).and_then(Value::as_object) {
                for head in heads.values() {
                    cite(&mut kept, Some(head));
                }
            }
            if let Some(realms) = inputs.get(REALM_FACTS).and_then(Value::as_object) {
                for facts in realms.values() {
                    cite(&mut kept, facts.get(HEAD));
                }
            }
            kept
        })
}

#[cfg(test)]
mod tests;
