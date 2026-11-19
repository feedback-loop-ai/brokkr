//! The inner topology's event buckets: one fold over a run's events that
//! names every event type, so a new one does not compile until its
//! reading here is decided (decision 0071 ruling 2; #950).
use super::*;

/// Events bucketed by effect once, so `watch` does not pay an
/// O(phases × events) rescan on every redraw.
pub(super) struct Buckets<'a> {
    /// Phase name -> the NEWEST effect requested in it, and its seat
    /// (last write wins). The seat travels with the id because it is the
    /// label a structureless phase draws, and reading it here keeps that
    /// lookup total — a participant always exists for a requested effect,
    /// so a "no participant" fallback would be an uncoverable branch.
    pub(super) newest: BTreeMap<&'a str, (&'a str, &'a str)>,
    /// Effect id -> the sequence steps its checkpoints finished and the
    /// member tags they carry, each once, in first order.
    pub(super) listed: BTreeMap<&'a str, (Vec<String>, Vec<String>)>,
    /// Effect id -> every phase it was requested in (scope membership).
    pub(super) effect_phases: BTreeMap<&'a str, Vec<String>>,
}

/// A checkpoint's `payload` as its effect's lists read it: a finished
/// step's name, or else the member it tags, each listed once.
fn listed((finished, tags): &mut (Vec<String>, Vec<String>), payload: &Value) {
    let checkpoint = payload.get("checkpoint").unwrap_or(&Value::Null);
    let (list, name) = match field(checkpoint, "step") {
        Some("sequence-step-finished") => (finished, field(checkpoint, "step_name")),
        _ => (tags, field(checkpoint, "member")),
    };
    if let Some(name) = name.filter(|name| !list.iter().any(|known| known == name)) {
        list.push(name.to_string());
    }
}

pub(super) fn bucket(events: &[EventEnvelope]) -> Buckets<'_> {
    let mut buckets = Buckets {
        newest: BTreeMap::new(),
        listed: BTreeMap::new(),
        effect_phases: BTreeMap::new(),
    };
    for event in events {
        let payload = &event.payload;
        match event.event_type {
            EventType::EffectRequested => {
                if let (Some(effect_id), Some(phase)) =
                    (field(payload, "effect_id"), field(payload, "phase"))
                {
                    let seat = field(payload, "seat").unwrap_or("?");
                    buckets.newest.insert(phase, (effect_id, seat));
                    let names = buckets.effect_phases.entry(effect_id).or_default();
                    if !names.iter().any(|name| name == phase) {
                        names.push(phase.to_string());
                    }
                }
            }
            EventType::EffectCheckpointed => {
                if let Some(effect_id) = field(payload, "effect_id") {
                    listed(buckets.listed.entry(effect_id).or_default(), payload);
                }
            }
            EventType::PhaseEntered
            | EventType::EffectStarted
            | EventType::EffectSucceeded
            | EventType::EffectFailed
            | EventType::EffectIndeterminate
            | EventType::TransitionDecided
            | EventType::RunStarted
            | EventType::OperatorCommanded
            | EventType::OperatorAccepted
            | EventType::OperatorRejected
            | EventType::RunParked
            | EventType::RunCompleted
            | EventType::RunStopped => {}
        }
    }
    buckets
}
