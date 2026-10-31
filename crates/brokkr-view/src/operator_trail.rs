//! The operator's commands as the trail prints them: each event's
//! command and seq, and the `operator/commanded` a disposition answers,
//! read once so `inspect` and the run page say the same words.

use std::collections::BTreeMap;

use brokkr_core::{EventEnvelope, EventType};

use crate::{field, What};

/// The three operator events a trail row words, named so each disposition
/// is matched explicitly rather than by its event type's fallthrough.
#[derive(Clone, Copy)]
pub(crate) enum OperatorKind {
    Commanded,
    Accepted,
    Rejected,
}

impl OperatorKind {
    /// The disposition an event of this type records, where it records
    /// one.
    pub(crate) fn of(event_type: EventType) -> Option<OperatorKind> {
        match event_type {
            EventType::OperatorCommanded => Some(OperatorKind::Commanded),
            EventType::OperatorAccepted => Some(OperatorKind::Accepted),
            EventType::OperatorRejected => Some(OperatorKind::Rejected),
            _ => None,
        }
    }
}

/// Each operator disposition's command word and the seq of the
/// `operator/commanded` that stood when it answered, by the disposition's
/// own seq: a command id reused later never overwrites an earlier answer.
pub(crate) fn commands_of(events: &[EventEnvelope]) -> BTreeMap<u64, (String, u64)> {
    let mut commanded: BTreeMap<&str, (String, u64)> = BTreeMap::new();
    let mut answered = BTreeMap::new();
    for event in events {
        if event.event_type == EventType::OperatorCommanded {
            let (Some(id), Some(word)) = (
                field(&event.payload, "command_id"),
                field(&event.payload, "command"),
            ) else {
                continue;
            };
            commanded.insert(id, (word.to_string(), event.seq));
            continue;
        }
        if event.event_type != EventType::OperatorAccepted
            && event.event_type != EventType::OperatorRejected
        {
            continue;
        }
        let Some(id) = field(&event.payload, "command_id") else {
            continue;
        };
        if let Some((word, seq)) = commanded.get(id) {
            answered.insert(event.seq, (word.clone(), *seq));
        }
    }
    answered
}

/// An operator event's trail row: its command, its own seq, and — on a
/// disposition — the seq its `operator/commanded` stated.
pub(crate) fn operator_what(
    kind: OperatorKind,
    event: &EventEnvelope,
    commands: &BTreeMap<u64, (String, u64)>,
) -> What {
    let payload = &event.payload;
    let empty = |text: String| What {
        text,
        badge: None,
        badge_class: None,
        arrow: None,
        problem: None,
    };
    match kind {
        OperatorKind::Commanded => {
            let command = field(payload, "command").unwrap_or("?");
            empty(format!("{command} · commanded at seq {}", event.seq))
        }
        OperatorKind::Accepted => {
            let (command, from) = answered(commands, event.seq);
            empty(format!("{command} · accepted at seq {}{from}", event.seq))
        }
        OperatorKind::Rejected => {
            let (command, from) = answered(commands, event.seq);
            empty(format!(
                "{command} · rejected at seq {}: {}{from}",
                event.seq,
                field(payload, "reason").unwrap_or("?")
            ))
        }
    }
}

/// The command word a disposition at `seq` answers and the clause that
/// names its seq, `?` and no clause where it answers none.
fn answered(commands: &BTreeMap<u64, (String, u64)>, seq: u64) -> (&str, String) {
    match commands.get(&seq) {
        Some((command, at)) => (command.as_str(), format!(" (command at seq {at})")),
        None => ("?", String::new()),
    }
}

#[cfg(test)]
mod tests;
