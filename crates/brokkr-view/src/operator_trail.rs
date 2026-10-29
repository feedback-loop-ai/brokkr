//! The operator's commands as the trail prints them: each event's
//! command and seq, and the `operator/commanded` a disposition answers,
//! read once so `inspect` and the run page say the same words.

use std::collections::BTreeMap;

use brokkr_core::{EventEnvelope, EventType};

use crate::{field, What};

/// Each operator command's word and the seq its `operator/commanded`
/// states, by command id: what an acceptance or rejection names to show
/// the command it disposed of.
pub(crate) fn commands_of(events: &[EventEnvelope]) -> BTreeMap<String, (String, u64)> {
    let mut commands = BTreeMap::new();
    for event in events {
        if event.event_type != EventType::OperatorCommanded {
            continue;
        }
        let (Some(id), Some(word)) = (
            field(&event.payload, "command_id"),
            field(&event.payload, "command"),
        ) else {
            continue;
        };
        commands.insert(id.to_string(), (word.to_string(), event.seq));
    }
    commands
}

/// An operator event's trail row: its command, its own seq, and — on a
/// disposition — the seq its `operator/commanded` stated.
pub(crate) fn operator_what(
    event: &EventEnvelope,
    commands: &BTreeMap<String, (String, u64)>,
) -> What {
    let payload = &event.payload;
    let empty = |text: String| What {
        text,
        badge: None,
        badge_class: None,
        arrow: None,
        problem: None,
    };
    if event.event_type == EventType::OperatorCommanded {
        let command = field(payload, "command").unwrap_or("?");
        return empty(format!("{command} · commanded at seq {}", event.seq));
    }
    let known = field(payload, "command_id").and_then(|id| commands.get(id));
    let (command, from) = match known {
        Some((command, seq)) => (command.as_str(), format!(" (command at seq {seq})")),
        None => ("?", String::new()),
    };
    match event.event_type {
        EventType::OperatorAccepted => {
            empty(format!("{command} · accepted at seq {}{from}", event.seq))
        }
        _ => empty(format!(
            "{command} · rejected at seq {}: {}{from}",
            event.seq,
            field(payload, "reason").unwrap_or("?")
        )),
    }
}
