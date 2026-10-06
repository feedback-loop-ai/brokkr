//! The codex seat's items (#550): the seat scan counts the
//! `item-completed` checkpoints the driver already journals, and the
//! turns cell — the one cell every surface paints — shows the count
//! beside the turn. Derived once in `brokkr-view`; these tests pin the
//! derivation through the public view: the concluded seat, the open
//! attempt whose count grows live at every checkpoint, the one-item
//! plural, and the retried seat's two folds.

use brokkr_core::envelope::EventType;
use brokkr_core::EventEnvelope;
use brokkr_view::{run_view, ABSENT};
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
fn seat_checkpoint(attempt: &str, step: &str, turn: u64, tool: Option<&str>) -> Value {
    let mut checkpoint = json!({"harness": "codex", "step": step, "turn": turn});
    if let Some(tool) = tool {
        checkpoint["tool"] = json!(tool);
    }
    json!({"effect_id": "eff1", "attempt_id": attempt, "checkpoint": checkpoint})
}

/// The seat's request line, where every journal of this fixture starts.
fn requested() -> Journal {
    let mut journal = Journal::default();
    journal.push(
        EventType::EffectRequested,
        json!({"effect_id": "eff1", "seat": "implement", "phase": "implement"}),
    );
    journal
}

/// One codex attempt's stream, as the driver journals it: the attempt
/// starts, each turn opens, one item is announced (`item-started`, the
/// neighbour step the reader emits beside `item-completed` from the
/// same arm) and the completed items arrive as `item-completed`
/// checkpoints, each completed turn closes. The attempt id is the
/// caller's, so a retried seat's second attempt is the same stream
/// again.
fn codex_attempt(
    journal: &mut Journal,
    attempt: &str,
    items: &[&str],
    turns_started: u64,
    turns_completed: u64,
) {
    journal.push(
        EventType::EffectStarted,
        json!({"effect_id": "eff1", "attempt_id": attempt}),
    );
    for turn in 1..=turns_started {
        journal.push(
            EventType::EffectCheckpointed,
            seat_checkpoint(attempt, "turn-started", turn, None),
        );
    }
    journal.push(
        EventType::EffectCheckpointed,
        seat_checkpoint(attempt, "item-started", 1, Some("command_execution")),
    );
    for tool in items {
        journal.push(
            EventType::EffectCheckpointed,
            seat_checkpoint(attempt, "item-completed", 1, Some(tool)),
        );
    }
    for turn in 1..=turns_completed {
        journal.push(
            EventType::EffectCheckpointed,
            seat_checkpoint(attempt, "turn-completed", turn, None),
        );
    }
}

/// A codex seat's journal with the attempt still working: requested,
/// started, the turns and their items — and nothing that concludes the
/// attempt. No `codex-session-finished`, no terminal event. The live
/// test folds it prefix by prefix, the way a reader watches.
fn open_codex_journal(
    items: &[&str],
    turns_started: u64,
    turns_completed: u64,
) -> Vec<EventEnvelope> {
    let mut journal = requested();
    codex_attempt(&mut journal, "a1", items, turns_started, turns_completed);
    journal.0
}

/// The same seat concluded as the driver leaves it: the thread closes
/// with the session checkpoint and the effect succeeds.
fn codex_journal(items: &[&str], turns_started: u64, turns_completed: u64) -> Vec<EventEnvelope> {
    let mut journal = requested();
    codex_attempt(&mut journal, "a1", items, turns_started, turns_completed);
    journal.at(
        T1,
        EventType::EffectCheckpointed,
        seat_checkpoint("a1", "codex-session-finished", 0, None),
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
fn a_working_codex_seat_counts_its_items_live_at_every_checkpoint() {
    // An OPEN attempt — no `codex-session-finished`, no terminal
    // event — folded prefix by prefix, the way a reader watches a seat
    // work: the cell is the absence mark until the first item lands,
    // an item that has only been announced does not count, the cell
    // grows at each `item-completed`, and it stands beside the turn
    // the moment it closes. The seat is working at every prefix, so
    // the count is live from the same checkpoints and no snapshot of
    // a concluded participant stands in for it.
    let full = open_codex_journal(&["command_execution", "reasoning"], 1, 1);
    let expected: &[(usize, &str, bool, Option<u64>)] = &[
        (1, ABSENT, true, None), // requested: the seat exists, nothing has arrived
        (2, ABSENT, true, None), // attempt one started
        (3, ABSENT, true, None), // turn-started opens the turn; it is not a completed item
        (4, ABSENT, true, None), // item-started announces the work; it does not count it
        (5, "1 items", false, None),
        (6, "2 items", false, None),
        (7, "1 · 2 items", false, Some(1)), // the turn closes behind the items
    ];
    for &(prefix, text, absent, turns) in expected {
        let view = run_view(&full[..prefix], None);
        assert_eq!(view.participants[0].status, "working", "prefix {prefix}");
        assert_eq!(
            view.participants[0].turns_cell.text, text,
            "prefix {prefix}"
        );
        assert_eq!(
            view.participants[0].turns_cell.absent, absent,
            "prefix {prefix}"
        );
        assert_eq!(view.participants[0].turns, turns, "prefix {prefix}");
    }
}

#[test]
fn a_codex_seat_whose_harness_journals_no_items_reads_as_before() {
    // The fixture plants an `item-started` that never completes beside
    // zero `item-completed` checkpoints: nothing counts, and the cell
    // keeps the turn count it always had.
    let view = run_view(&codex_journal(&[], 1, 1), None);
    assert_eq!(view.participants[0].turns, Some(1));
    assert_eq!(view.participants[0].turns_cell.text, "1");
}

#[test]
fn a_single_item_stays_the_fixed_plural() {
    // One item is a live case on every surface, and the issue's example
    // is plural. The house renders no singular anywhere — `over 2
    // attempts`, `k tok` — so the noun does not bend at one: the fixed
    // plural is the chosen reading, and the PR's ruling ask names it.
    let open = run_view(&open_codex_journal(&["agent_message"], 1, 0), None);
    assert_eq!(open.participants[0].turns_cell.text, "1 items");
    assert_eq!(open.participants[0].turns, None);
    let view = run_view(&codex_journal(&["agent_message"], 1, 1), None);
    assert_eq!(view.participants[0].turns_cell.text, "1 · 1 items");
    assert_eq!(view.participants[0].turns, Some(1));
}

#[test]
fn a_retried_seat_takes_its_turns_from_the_max_and_its_items_from_the_sum() {
    // Attempt one closed its turn with three items down and FAILED;
    // attempt two, still working, closed its own turn one with two
    // more. Neither fold keeps per-attempt state: the typed turns
    // number is the MAX over the attempts' turn numbers (a sum would
    // read 2) and the items are the SUM (a max would read 3), and the
    // cell carries no `over N attempts` mark — that is the cost cell's
    // mark. A failed attempt's items are journaled facts and stay
    // counted.
    let mut journal = requested();
    codex_attempt(
        &mut journal,
        "a1",
        &["command_execution", "reasoning", "agent_message"],
        1,
        1,
    );
    journal.push(
        EventType::EffectFailed,
        json!({"effect_id": "eff1", "attempt_id": "a1", "error": "the attempt failed"}),
    );
    let failed = journal.0.len();
    codex_attempt(
        &mut journal,
        "a2",
        &["command_execution", "reasoning"],
        1,
        1,
    );

    let view = run_view(&journal.0[..failed], None);
    assert_eq!(view.participants[0].attempts, 1);
    assert_eq!(view.participants[0].status, "failed");
    assert_eq!(view.participants[0].turns, Some(1));
    assert_eq!(view.participants[0].turns_cell.text, "1 · 3 items");

    let view = run_view(&journal.0, None);
    assert_eq!(view.participants[0].attempts, 2);
    assert_eq!(view.participants[0].status, "working");
    assert_eq!(view.participants[0].turns, Some(1));
    assert_eq!(view.participants[0].turns_cell.text, "1 · 5 items");
}
