//! The park projection (decision 0089 ruling 7): a park reads its phase,
//! its start, the operator's answer and the run's next move, only
//! parseable stamps are read, and its span class is the bit length of its
//! reach.

use super::*;
use crate::envelope::EventType;
use crate::envelope_builder::EnvelopeBuilder;
use serde_json::json;

/// The run's events at seq 1 onward, each stamped a second after the last
/// unless `at` names another, sealed from their content.
fn journal(steps: &[(EventType, serde_json::Value, &str)]) -> Vec<EventEnvelope> {
    steps
        .iter()
        .enumerate()
        .map(|(index, (event_type, payload, at))| {
            let seq = index as u64 + 1;
            EnvelopeBuilder::new(*event_type, payload.clone())
                .seq(seq)
                .event_id(format!("e{seq}"))
                .at(*at)
                .sealed()
        })
        .collect()
}

#[test]
fn a_park_reads_its_phase_start_answer_and_end() {
    let events = journal(&[
        (EventType::RunStarted, json!({}), "2026-01-01T00:00:00Z"),
        (
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            "2026-01-01T00:00:01Z",
        ),
        (
            EventType::RunParked,
            json!({"reason": "waiting"}),
            "2026-01-01T00:00:02Z",
        ),
        (
            EventType::OperatorCommanded,
            json!({"command": "go"}),
            "2026-01-01T00:00:03Z",
        ),
        // A second operator command is absorbed and does not move the
        // first, and a rejection answers nothing.
        (
            EventType::OperatorCommanded,
            json!({"command": "again"}),
            "2026-01-01T00:00:04Z",
        ),
        (
            EventType::OperatorRejected,
            json!({"reason": "no"}),
            "2026-01-01T00:00:05Z",
        ),
        // The run's next move ends the park at its own time.
        (
            EventType::PhaseEntered,
            json!({"phase": "review"}),
            "2026-01-01T00:00:06Z",
        ),
        (
            EventType::RunParked,
            json!({"reason": "review"}),
            "2026-01-01T00:00:07Z",
        ),
        // An operator rejection is not an answer: only a command answers.
        (
            EventType::OperatorRejected,
            json!({"reason": "no"}),
            "2026-01-01T00:00:08Z",
        ),
        // A stamp that does not parse is read by no park, so it neither
        // answers nor ends the open one.
        (
            EventType::PhaseEntered,
            json!({"phase": "stop"}),
            "not a stamp",
        ),
    ]);
    assert_eq!(
        parks(&events),
        [
            Park {
                seq: 3,
                phase: Some("implement".into()),
                at: 1_767_225_602_000,
                answered: Some(1_767_225_603_000),
                ended: Some(1_767_225_606_000),
            },
            Park {
                seq: 8,
                phase: Some("review".into()),
                at: 1_767_225_607_000,
                answered: None,
                ended: None,
            },
        ]
    );
}

/// A `run/parked` whose own stamp does not parse opens no park, and the
/// events after it do not close a park that was never opened.
#[test]
fn a_park_whose_stamp_does_not_parse_opens_nothing() {
    let events = journal(&[
        (
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            "2026-01-01T00:00:00Z",
        ),
        (EventType::RunParked, json!({}), "not a stamp"),
        (
            EventType::PhaseEntered,
            json!({"phase": "review"}),
            "2026-01-01T00:00:02Z",
        ),
    ]);
    assert_eq!(parks(&events), Vec::<Park>::new());
}

#[test]
fn a_park_s_span_class_is_its_reach_s_bit_length() {
    let at = 1_000;
    let park = |answered, ended| Park {
        seq: 1,
        phase: None,
        at,
        answered,
        ended,
    };
    let reach = |answered: i64, ended: i64| park(Some(answered), Some(ended)).span_class();
    // Open parks are class 64 whatever their start.
    assert_eq!(park(None, None).span_class(), 64);
    assert_eq!(park(Some(at), None).span_class(), 64);
    // A reach that is not positive is class 0.
    assert_eq!(reach(at, at), 0);
    assert_eq!(reach(at - 10, at - 5), 0);
    // Every boundary 2^c is class c + 1, and one below it is class c.
    for c in 1..=62 {
        let boundary = 1_i64 << c;
        assert_eq!(reach(at, at + boundary), c + 1, "2^{c}");
        assert_eq!(reach(at, at + boundary - 1), c, "2^{c} - 1");
    }
    // The greatest reach a 64-bit stamp difference takes is class 63.
    assert_eq!(reach(at, i64::MAX), 63);
    // A command stamped after the run's next move, across a clock stepped
    // back, is the park's later moment.
    assert_eq!(reach(at + 100, at + 10), 7);
}

/// The writer's account carries every park over each append as the run
/// read whole states it: an account carried over a split reads the
/// journal whole, and its parks are the free projection's.
#[test]
fn an_account_carries_the_parks_the_run_reads_whole() {
    use crate::summary::Account;

    let events = crate::summary::tests::parked();
    let expected = parks(&events);
    let whole = Account::whole("run", &events);
    assert_eq!(whole.parks(), expected);
    let carried = Account::whole("run", &events[..6]).onto(&events[6..]);
    assert_eq!(carried.parks(), expected);
    assert_eq!(
        carried
            .parks()
            .iter()
            .map(|park| (park.seq, park.phase.as_deref(), park.answered, park.ended))
            .collect::<Vec<_>>(),
        [(12, Some("review"), Some(1_767_225_613_000), None)]
    );
}
