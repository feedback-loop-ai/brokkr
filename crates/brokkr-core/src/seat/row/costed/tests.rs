use serde_json::{json, Value};

use super::*;
use crate::seat::row::tests::journals::{self, journal};
use crate::seat::row::Datum;
use crate::seat::seat_costs;
use crate::summary::Account;

/// Every hand-built and committed journal's rows, and the rows read back
/// from their data, cost as `seat_costs` gives the journal, a cost by its
/// bits: the seats compared.
#[test]
fn every_journal_s_rows_cost_as_its_journal() {
    let committed = crate::committed_journals::committed_journals();
    let committed = committed.into_iter().map(|(_, events)| events);
    let hand_built = journals::journals().into_iter().map(|(_, events)| events);
    let mut compared = 0;
    for events in hand_built.chain(committed) {
        let rows = Account::whole("run", &events).seats().to_vec();
        let data = rows.iter().map(SeatRow::data);
        let read: Vec<SeatRow> = data.map(|data| SeatRow::of_data(data).unwrap()).collect();
        let (seats, total) = seat_costs(&events);
        for kept in [costs(&rows), costs(&read)] {
            assert_eq!(kept.0, seats);
            assert_eq!(kept.1.to_bits(), total.to_bits());
        }
        compared += seats.len();
    }
    assert_eq!(compared, 76);
}

/// Each row's start count, as its `costs_attempts` column keeps it.
fn attempts(rows: &[SeatRow]) -> Vec<Datum> {
    let at = SeatRow::COLUMNS
        .iter()
        .position(|(name, _)| *name == "costs_attempts");
    rows.iter()
        .map(|row| row.data()[at.unwrap()].clone())
        .collect()
}

/// An effect asked with no seat is not seated, and its start is not
/// counted; asked by a seat it is, and asked again naming none its
/// accounting so far is taken onto the new request's own row, and asked
/// by another seat it is that seat's, all of it. Only a request's own row
/// keeps it, and an earlier one keeps what it had.
#[test]
fn an_effect_asked_again_is_counted_whole_under_the_last_seat_named() {
    let asked = |seat: Value| {
        let payload = json!({"effect_id": "e", "seat": seat});
        (EventType::EffectRequested, payload)
    };
    let started = (
        EventType::EffectStarted,
        json!({"effect_id": "e", "attempt_id": "a"}),
    );
    let priced = journals::checkpoint("e", "a", json!({"num_turns": 2, "total_cost_usd": 0.5}));
    let events = journal(vec![
        asked(Value::Null),
        started.clone(),
        asked(json!("first")),
        started.clone(),
        priced.clone(),
        asked(Value::Null),
        started,
        asked(json!("last")),
        priced,
    ]);
    let rows = Account::whole("run", &events).seats().to_vec();
    let (seats, total) = costs(&rows);
    assert_eq!(
        serde_json::to_value(&seats).unwrap(),
        json!({"last": {"attempts": 2, "turns": 4, "cost_usd": 1.0, "model": "not reported",
                        "boundary": "not recorded", "effort": "not reported"}})
    );
    assert_eq!(total, 1.0);
    assert_eq!((seats, total), seat_costs(&events));
    let (none, one, two) = (Datum::Null, Datum::Integer(1), Datum::Integer(2));
    let counted = [&none, &none, &one, &none, &two, &none, &two, &none].map(Datum::clone);
    assert_eq!(attempts(&rows), counted);
}

/// A result pricing at -0.0 an effect its records priced nothing moves
/// the request's own row, whose cost was 0.0, so the writer writes it
/// again: a cost is the same by its bits alone.
#[test]
fn a_result_priced_at_negative_zero_moves_its_own_row() {
    let asked = (
        EventType::EffectRequested,
        json!({"effect_id": "z", "seat": "verify"}),
    );
    let started = (
        EventType::EffectStarted,
        json!({"effect_id": "z", "attempt_id": "a"}),
    );
    let result = json!({"effect_id": "z", "attempt_id": "a", "result": {"total_cost_usd": -0.0}});
    let events = journal(vec![asked, started, (EventType::EffectSucceeded, result)]);
    let before = Account::whole("run", &events[..2]);
    let after = Account::whole("run", &events);
    assert_ne!(before.seats()[0], after.seats()[0]);
    let data = |account: &Account| account.seats()[0].data();
    let cost = SeatRow::COLUMNS
        .iter()
        .position(|(name, _)| *name == "costs_cost");
    let cost = cost.unwrap();
    let bits = |cost: f64| Datum::Integer(cost.to_bits() as i64);
    assert_eq!(
        [data(&before)[cost].clone(), data(&after)[cost].clone()],
        [bits(0.0), bits(-0.0)]
    );
}
