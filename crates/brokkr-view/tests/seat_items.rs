//! The codex seat's items (#550): the seat scan counts the
//! `item-completed` checkpoints the driver already journals, and the
//! turns cell — the one cell every surface paints — shows the count
//! beside the turn. Derived once in `brokkr-view`; these tests pin the
//! derivation through the public view.

use brokkr_core::envelope::EventType;
use brokkr_core::EventEnvelope;
use brokkr_view::run_view;
use serde_json::{json, Value};

#[path = "../../../tests/support/envelope.rs"]
mod envelope_builder;
use envelope_builder::EnvelopeBuilder;

const T0: &str = "2026-01-01T00:00:00Z";
const T1: &str = "2026-01-01T00:02:03Z";

/// The journal under construction: the seq and the timestamp are this
/// type's to carry, so a test names only the events it is asking about.
#[derive(Default)]
struct Journal(Vec<EventEnvelope>);

impl Journal {
    fn push(&mut self, event_type: EventType, payload: Value) {
        self.at(T0, event_type, payload);
    }

    fn at(&mut self, at: &str, event_type: EventType, payload: Value) {
        let seq = self.0.len() as u64 + 1;
        self.0.push(
            EnvelopeBuilder::new(event_type, payload)
                .seq(seq)
                .at(at)
                .build(),
        );
    }
}

/// One checkpoint of the attempt, as the codex driver journals it: the
/// step, its turn, and the item's `tool` type when it is an item.
fn seat_checkpoint(step: &str, turn: u64, tool: Option<&str>) -> Value {
    let mut checkpoint = json!({"harness": "codex", "step": step, "turn": turn});
    if let Some(tool) = tool {
        checkpoint["tool"] = json!(tool);
    }
    json!({"effect_id": "eff1", "attempt_id": "a1", "checkpoint": checkpoint})
}

/// A codex seat's journal as the driver writes it: the attempt opens,
/// each turn opens, the work inside it arrives as `item-completed`
/// checkpoints, each completed turn closes, the thread closes with the
/// session checkpoint, and the effect concludes.
fn codex_journal(items: &[&str], turns_started: u64, turns_completed: u64) -> Vec<EventEnvelope> {
    let mut journal = Journal::default();
    journal.push(
        EventType::EffectRequested,
        json!({"effect_id": "eff1", "seat": "implement", "phase": "implement"}),
    );
    journal.push(
        EventType::EffectStarted,
        json!({"effect_id": "eff1", "attempt_id": "a1"}),
    );
    for turn in 1..=turns_started {
        journal.push(
            EventType::EffectCheckpointed,
            seat_checkpoint("turn-started", turn, None),
        );
    }
    for tool in items {
        journal.push(
            EventType::EffectCheckpointed,
            seat_checkpoint("item-completed", 1, Some(tool)),
        );
    }
    for turn in 1..=turns_completed {
        journal.push(
            EventType::EffectCheckpointed,
            seat_checkpoint("turn-completed", turn, None),
        );
    }
    journal.at(
        T1,
        EventType::EffectCheckpointed,
        seat_checkpoint("codex-session-finished", 0, None),
    );
    journal.push(
        EventType::EffectSucceeded,
        json!({"effect_id": "eff1", "attempt_id": "a1", "result": {"result": "complete"}}),
    );
    journal.0
}

#[test]
fn a_codex_seat_shows_its_items_beside_its_turn_count() {
    let view = run_view(
        &codex_journal(&["command_execution", "reasoning", "agent_message"], 1, 1),
        None,
    );
    assert_eq!(view.participants[0].label, "implement");
    assert_eq!(view.participants[0].turns, Some(1));
    assert_eq!(view.participants[0].turns_cell.text, "1 · 3 items");
    assert!(!view.participants[0].turns_cell.absent);
}

#[test]
fn a_working_codex_seat_counts_items_before_its_first_turn_completes() {
    // No `turn-completed` yet: the count still shows, and grows live
    // from the same checkpoints as the turn closes behind it.
    let view = run_view(
        &codex_journal(&["command_execution", "reasoning"], 1, 0),
        None,
    );
    assert_eq!(view.participants[0].turns, None);
    assert_eq!(view.participants[0].turns_cell.text, "2 items");
    assert!(!view.participants[0].turns_cell.absent);
}

#[test]
fn a_codex_seat_whose_harness_journals_no_items_reads_as_before() {
    // An item that has only started is not a completed item: nothing
    // counts, and the cell keeps the turn count it always had.
    let view = run_view(&codex_journal(&[], 1, 1), None);
    assert_eq!(view.participants[0].turns, Some(1));
    assert_eq!(view.participants[0].turns_cell.text, "1");
}
