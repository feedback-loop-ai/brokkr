//! The long run the CPU and heap budgets size by (#342): a real exported
//! run grown to the largest run in the operator's journal when #342 was
//! filed, read by the benches and by the opening's heap budget.

use std::path::Path;

use brokkr_core::envelope::{EventEnvelope, EventType};

/// A real exported run: 105 events, 88 of them checkpoints.
const FIXTURE: &str =
    include_str!("../../../../fixtures/journals/tui-graph-the-selection-box-gets-80f98deb.ndjson");

/// The run the fixture is, which the grown run keeps.
pub(crate) const RUN: &str = "tui-graph-the-selection-box-gets-80f98deb";

/// The largest run in the operator's journal when #342 was filed. The fold
/// and the view are measured at the scale a long run makes them meet.
pub(crate) const EVENTS: usize = 7_510;

/// The fixture grown to `EVENTS` events by repeating each checkpoint in
/// place and renumbering: a long-running seat's journal is mostly its
/// checkpoints. Fold checks protocol shape, not the hash chain (callers
/// verify the chain first), so the grown journal folds as the fixture does.
pub(crate) fn journal() -> Vec<EventEnvelope> {
    let fixture: Vec<EventEnvelope> = FIXTURE
        .lines()
        .map(|line| serde_json::from_str(line).expect("the fixture is an exported journal"))
        .collect();
    let is_checkpoint = |event: &EventEnvelope| event.event_type == EventType::EffectCheckpointed;
    let checkpoints = fixture.iter().filter(|event| is_checkpoint(event)).count();
    let spare = EVENTS - (fixture.len() - checkpoints);
    let mut remainder = spare % checkpoints;
    let mut grown = Vec::with_capacity(EVENTS);
    for event in fixture {
        let copies = match is_checkpoint(&event) {
            true => spare / checkpoints + usize::from(remainder > 0),
            false => 1,
        };
        remainder = remainder.saturating_sub(usize::from(is_checkpoint(&event)));
        for _ in 0..copies {
            let mut copy = event.clone();
            copy.seq = grown.len() as u64 + 1;
            grown.push(copy);
        }
    }
    assert_eq!(
        grown.len(),
        EVENTS,
        "the grown journal has the budget's size"
    );
    grown
}

/// `events` sealed into one chain, in order: each continues the hash of
/// the one before it, so the run verifies as a journal's own run does.
pub(crate) fn chained(events: Vec<EventEnvelope>) -> Vec<EventEnvelope> {
    let mut previous = brokkr_core::canonical::ZERO_HASH.to_string();
    events
        .into_iter()
        .map(|event| {
            let sealed = EventEnvelope {
                previous_hash: previous.clone(),
                ..event
            }
            .sealed();
            previous.clone_from(&sealed.event_hash);
            sealed
        })
        .collect()
}

/// Plant `events`, one chained run, into the journal at `db` in one
/// transaction, past every writer, as an imported run's rows stand.
pub(crate) fn planted(db: &Path, events: &[EventEnvelope]) {
    let mut conn = rusqlite::Connection::open(db).expect("the journal opens");
    let tx = conn.transaction().expect("a transaction begins");
    brokkr_store::test_support::plant_run(&tx, &events[0].run_id, "{}").expect("the run plants");
    for event in events {
        brokkr_store::test_support::plant_envelope(&tx, event).expect("an event plants");
    }
    tx.commit().expect("the run commits");
}
