//! Decision 0030's evidence, kept readable (design D8): the offer an
//! unstamped row of a single work seat may still supply, and the instance
//! comparison that guards it.

use brokkr_core::{EventEnvelope, EventType};
use serde_json::Value;

use super::{in_effects, started_attempt};
use crate::agents::kinds::v7_kind;

/// The narrow compatibility path: the last checkpoint of this seat that
/// carries an old-shaped session locator AND no site stamp, offered only
/// when the caller's predicate accepts the attempt that wrote it. The
/// predicate is decision 0030's instance comparison — the `driver` label
/// and the `provenance` of the opening attempt against this one's
/// (`legacy_instance_holds`); it holds no kind or version test, because
/// the evidence this path reads predates both. The two shapes it reads
/// are decision 0032's `codex-thread` locator and the flat `session_id`
/// a run opened before that ruling wrote — the flat id names no kind,
/// exactly as it did not when the shipped engine offered it. A row that
/// carries a site stamp is NEW evidence and was already judged by
/// `eligible_offer`; it may not fall back to legacy fields to evade the
/// confirmation it failed. A row whose transcript names a kind outside
/// v7's vocabulary yields no handle, not even its flat id, and no older
/// row stands in (the controller's ruling R27).
pub(super) fn legacy_offer(
    events: &[EventEnvelope],
    seat_effects: &[String],
    legacy: &impl Fn(&EventEnvelope) -> bool,
) -> Option<String> {
    let (attempt, session) = events
        .iter()
        .rev()
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .filter(|event| in_effects(event, seat_effects))
        .find_map(|event| {
            let checkpoint = event.payload.get("checkpoint")?;
            if checkpoint.get("site_ref").is_some() {
                return None;
            }
            let transcript = checkpoint.get("transcript");
            let kind = transcript.map(|transcript| transcript.get("kind").and_then(Value::as_str));
            if kind.is_some_and(|kind| !kind.is_some_and(v7_kind)) {
                return Some(None);
            }
            // Decision 0032's common shape first, then the old flat id a
            // run opened before that ruling still carries.
            let session = checkpoint
                .pointer("/transcript/locator")
                .and_then(Value::as_str)
                .filter(|locator| !locator.is_empty())
                .filter(|_| {
                    checkpoint
                        .pointer("/transcript/kind")
                        .and_then(Value::as_str)
                        == Some("codex-thread")
                })
                .or_else(|| checkpoint.get("session_id").and_then(Value::as_str))?;
            let attempt = event.attempt_id.clone()?;
            legacy(event).then(|| Some((attempt, session.to_string())))
        })
        .flatten()?;
    started_attempt(events, &attempt).map(|_| session)
}

/// The shipped instance comparison decision 0030 wrote, kept for the
/// legacy path alone: the `driver` label and the `provenance` this
/// attempt is about to journal against the ones the attempt that opened
/// the session did. New evidence uses `instance_ref`, which does not
/// move when a SIBLING member's chain moves.
pub(in crate::engine) fn legacy_instance_holds(opened_by: &Value, started: &Value) -> bool {
    ["driver", "provenance"]
        .iter()
        .all(|field| opened_by.get(field) == started.get(field))
}
