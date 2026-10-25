//! Participant facts read from an attempt's start, including carried verdicts (0093 ruling 5).
use super::*;

#[derive(Clone)]
pub(super) struct CarriedEntry {
    member: String,
    attempt_id: String,
}

impl CarriedEntry {
    /// The entry a start records of `member`, carried from `attempt_id`.
    pub(super) fn of(member: &str, attempt_id: &str) -> CarriedEntry {
        CarriedEntry {
            member: member.to_string(),
            attempt_id: attempt_id.to_string(),
        }
    }
}

/// The attempt where a carried verdict settled, and its place among the phase's attempts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CarriedFrom {
    pub attempt_id: String,
    pub ordinal: usize,
}

pub(super) fn sites(scan: &mut Scan, slot: usize, effect_id: &str, payload: &Value) {
    for part in scan
        .parts
        .iter_mut()
        .filter(|part| part.effect_index == slot)
    {
        part.boundary = None;
        part.stamp = None;
        part.carried = None;
    }
    for entry in payload
        .get("provenance")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let member = field(entry, "member");
        let part = ensure(scan, slot, effect_id, member);
        scan.parts[part].provenance = Some(entry.clone());
    }
    for entry in boundary_entries(payload) {
        let part = ensure(scan, slot, effect_id, entry.member.as_deref());
        scan.parts[part].boundary = Some(entry.word);
    }
    for entry in carried_entries(payload) {
        let part = ensure(scan, slot, effect_id, Some(&entry.member));
        scan.parts[part].carried = Some(entry);
    }
}

fn carried_entries(payload: &Value) -> Vec<CarriedEntry> {
    payload
        .get("carried")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            Some(CarriedEntry {
                member: field(entry, "member")?.to_owned(),
                attempt_id: field(entry, "attempt_id")?.to_owned(),
            })
        })
        .collect()
}

pub(super) fn status_of(part: &Build, effect: &EffectFacts) -> (&'static str, &'static str) {
    if part.carried.is_some() {
        return CARRIED;
    }
    let mut derived = if effect.open.is_some() {
        WORKING
    } else {
        match effect.terminal {
            Some(terminal) => terminal,
            None => WORKING,
        }
    };
    if part.member.is_some() {
        if let Some(outcome) = part.member_outcome.as_ref().and_then(Value::as_str) {
            if let Some(pinned) = outcome_status(outcome) {
                derived = pinned;
            }
        }
    }

    derived
}

pub(super) fn carried_from(
    events: &[EventEnvelope],
    scan: &Scan,
    part: &Build,
) -> Option<CarriedFrom> {
    let entry = part.carried.as_ref()?;
    let phase = &scan.effects[part.effect_index].phase;
    let mut ordinal = 0;
    for event in events {
        if event.event_type == EventType::EffectStarted {
            let effect = field(&event.payload, "effect_id")?;
            let belongs = events
                .iter()
                .find(|event| {
                    event.event_type == EventType::EffectRequested
                        && field(&event.payload, "effect_id") == Some(effect)
                })
                .is_some_and(|request| field(&request.payload, "phase") == phase.as_deref());
            if belongs {
                ordinal += 1;
            }
            if field(&event.payload, "attempt_id") == Some(entry.attempt_id.as_str()) {
                return Some(CarriedFrom {
                    attempt_id: entry.attempt_id.clone(),
                    ordinal,
                });
            }
        }
    }
    None
}

pub(super) fn activity(
    events: &[EventEnvelope],
    scan: &Scan,
    part: &Build,
    count: usize,
    source: Option<&CarriedFrom>,
) -> Activity {
    if let Some(source) = source {
        let phase = scan.effects[part.effect_index]
            .phase
            .as_deref()
            .unwrap_or("unknown");
        return Activity {
            text: format!("carried from {phase} attempt {}", source.ordinal),
            absent: false,
            note: Some(source.attempt_id.clone()),
            tool: None,
            target_short: None,
            target_full: None,
        };
    }
    activity_for(events, scan, part, count)
}

#[cfg(test)]
mod tests;
