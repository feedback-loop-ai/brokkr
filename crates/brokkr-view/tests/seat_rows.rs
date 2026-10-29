//! The writer's seat rows, grouped by participant, read as the run view's
//! participants (#753; decision 0089 ruling 4 at the projection's level):
//! for every committed journal and every hand-built seat journal, the
//! facts `brokkr_core::seat::row::facts` derives from the rows of the
//! run's account equal what `run_view` shows of each participant, field
//! for field, and each phase's inner columns and each visit's models equal
//! what the rows list; a field that differs is named with its run. Each
//! count is of the participants, phases and visits compared.

use brokkr_core::summary::Account;
use brokkr_core::EventEnvelope;

use crate::{committed_journals, journals};

#[path = "../../../tests/support/seat_facts.rs"]
mod seat_facts;

/// What `seat_facts::disagreements` finds of `run`'s account read whole.
fn disagreements(run: &str, events: &[EventEnvelope]) -> (Vec<String>, usize) {
    let account = Account::whole(run, events);
    seat_facts::disagreements(run, events, account.seats(), account.seats_kept())
}

#[test]
fn every_committed_journal_s_seat_rows_read_as_its_participants() {
    let mut found = Vec::new();
    let mut compared = 0;
    for (path, events) in committed_journals::committed_journals() {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let (differ, count) = disagreements(&name, &events);
        found.extend(differ);
        compared += count;
    }
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(compared, 212);
}

#[test]
fn every_hand_built_journal_s_seat_rows_read_as_its_participants() {
    let mut found = Vec::new();
    let mut compared = 0;
    for (name, events) in journals::journals() {
        let (differ, count) = disagreements(name, &events);
        found.extend(differ);
        compared += count;
    }
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(compared, 68);
}

/// One seat's three effects asked in the order their ids do not sort in,
/// each priced once, so summed in another order than the effects' ids'
/// their cost differs by its bits; a fourth asked again naming no seat,
/// and then by another seat, which all of it is counted under; an effect
/// a request naming no seat never seats; and one whose turns and cost its
/// result alone reports.
fn reordered() -> Vec<EventEnvelope> {
    use brokkr_core::EventType::{
        EffectCheckpointed, EffectRequested, EffectStarted, EffectSucceeded,
    };
    use serde_json::json;
    let asked =
        |effect: &str, seat: &str| (EffectRequested, json!({"effect_id": effect, "seat": seat}));
    let started = |effect: &str| (EffectStarted, json!({"effect_id": effect}));
    let priced = |effect: &str, cost: f64| {
        let checkpoint = json!({"step": "seat-turn", "model": "m", "total_cost_usd": cost});
        (
            EffectCheckpointed,
            json!({"effect_id": effect, "checkpoint": checkpoint}),
        )
    };
    let result = json!({"effect_id": "z", "result": {"num_turns": 4, "total_cost_usd": 0.25}});
    journals::journal(vec![
        asked("c", "implement"),
        started("c"),
        priced("c", 0.1),
        asked("b", "implement"),
        started("b"),
        priced("b", 0.2),
        asked("a", "implement"),
        started("a"),
        priced("a", 0.3),
        asked("d", "implement"),
        started("d"),
        priced("d", 0.4),
        (EffectRequested, json!({"effect_id": "d"})),
        started("d"),
        asked("d", "review"),
        (EffectRequested, json!({"effect_id": "y"})),
        started("y"),
        asked("z", "verify"),
        started("z"),
        (EffectSucceeded, result),
    ])
}

/// Every journal's seat rows keep each seat's accounting as `seat_costs`
/// gives it of the journal, field for field and a cost by its bits
/// (decision 0089 slice 3): every committed and hand-built journal, and
/// one whose seat's cost reads otherwise summed in another order. The
/// count is of the seats compared.
#[test]
fn every_journal_s_seat_rows_keep_its_seat_costs() {
    let committed = committed_journals::committed_journals().into_iter();
    let committed = committed.map(|(path, events)| (path.display().to_string(), events));
    let hand_built = journals::journals().into_iter();
    let hand_built = hand_built.map(|(name, events)| (name.to_string(), events));
    let shaped = ("reordered".to_string(), reordered());
    let (mut found, mut compared) = (Vec::new(), 0);
    for (name, events) in committed.chain(hand_built).chain([shaped]) {
        let account = Account::whole(&name, &events);
        let (differ, count) = seat_facts::costs_differ(&name, &events, account.seats());
        found.extend(differ);
        compared += count;
    }
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(compared, 79);
    let (seats, total) = brokkr_view::seat_costs(&reordered());
    let attempted = |attempts: u64, turns: u64, cost: f64, model: &str| {
        serde_json::json!({"attempts": attempts, "turns": turns, "cost_usd": cost,
            "model": model, "boundary": "not recorded", "effort": "not reported"})
    };
    assert_eq!(
        serde_json::to_value(&seats).unwrap(),
        serde_json::json!({
            "implement": attempted(3, 0, 0.6, "m"),
            "review": attempted(2, 0, 0.4, "m"),
            "verify": attempted(1, 4, 0.25, "not reported"),
        })
    );
    assert_eq!(total.to_bits(), 1.25f64.to_bits());
}

/// The rows of a run whose live start names no effect or one never asked
/// do not keep its live block, so they read stale and the run view is read
/// whole; every other journal's rows keep it, every committed one's too.
#[test]
fn only_a_live_start_no_row_is_of_leaves_its_rows_unkept() {
    let committed = committed_journals::committed_journals();
    let committed = committed
        .into_iter()
        .map(|(_, events)| ("committed", events));
    let unkept: Vec<&str> = journals::journals()
        .into_iter()
        .chain(committed)
        .filter(|(_, events)| !Account::whole("run", events).seats_kept())
        .map(|(name, _)| name)
        .collect();
    assert_eq!(unkept, ["effectless", "never asked"]);
}
