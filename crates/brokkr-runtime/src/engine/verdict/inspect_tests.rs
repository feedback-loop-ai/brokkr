//! Fix round 0109a1r3, M3, through the real engine path with no planted
//! checkpoint: every gate verdict the journal records reads back under
//! the seat that produced it, a non-final step's and a panel member's
//! the chief omits included, and nowhere else.

use serde_json::{json, Value};

use super::covered_tests::drift_refusing;
use super::tests::{driven_with, driver, reported};
use crate::bundle::{
    Aggregate, Bundle, GateAssessment, Invocation, SeatBody, SeatClass, SequenceStep, StepBody,
};
use crate::engine::tests::{member, single_body, step_body};
use brokkr_core::envelope::EventEnvelope;
use brokkr_view::{gate_records, lens_for, GateRecord, Scope};

/// A gate step `name` of `body` answering one of `results`.
fn gate_step(name: &str, results: &[&str], body: StepBody) -> SequenceStep {
    uncompiled_sequence_step! {
        name: name.into(),
        class: SeatClass::Gate,
        results: results.iter().map(|word| word.to_string()).collect(),
        body,
    }
}

/// A residual answer carrying one finding `id`.
fn residual(id: &str, line: u64) -> Vec<String> {
    driver(&json!({"result": "residual", "findings": [reported(id, line)]}))
}

/// A review of three gate steps: a judge that reports `early`, a panel of
/// positions whose adversarial member reports `omitted` and whose
/// correctness member reports `theirs`, and a chief that reports none.
fn reviewed(compiled: &mut Bundle) {
    compiled.machine = drift_refusing();
    let positions = StepBody::Invocation(Invocation::Panel {
        members: vec![
            member("adversarial", residual("omitted", 3)),
            member("correctness", residual("theirs", 4)),
        ],
        aggregate: Aggregate::ReviewPanel,
    });
    let words = ["clean", "residual"];
    let steps = vec![
        gate_step("judge", &words, step_body(residual("early", 2))),
        gate_step("positions", &words, positions),
        gate_step(
            "chief",
            &["clean"],
            step_body(driver(&json!({"result": "clean"}))),
        ),
    ];
    let review = compiled.seats.get_mut("review").unwrap();
    review.body = SeatBody::Sequence { steps };
    review.gate = GateAssessment::AllGate;
    review.results = vec!["clean".into()];
    let ship = compiled.seats.get_mut("ship").unwrap();
    ship.body = single_body(driver(&json!({"result": "shipped"})));
}

/// The ids of the findings `scope` keeps of the run's verdicts, in order.
fn kept(events: &[EventEnvelope], scope: Scope) -> Vec<String> {
    let state = brokkr_core::fold::fold(events).unwrap();
    let view = brokkr_view::run_view(events, Some(&state));
    let lens = lens_for(&view, Some(&scope)).unwrap();
    gate_records(events)
        .iter()
        .filter_map(|record| record.kept_by(lens.as_ref()))
        .flat_map(|record: GateRecord| record.findings)
        .map(|finding| finding.id)
        .collect()
}

/// The finding the chief omits is read from the positions checkpoint and
/// shows under its position's seat and in the review phase; the judge's
/// non-final verdict shows in its own scope; and each member's finding
/// shows under that member alone.
#[test]
fn every_gate_verdict_the_journal_records_reads_back_under_its_own_seat() {
    let (_dir, events) = driven_with(
        single_body(driver(&json!({"result": "complete"}))),
        reviewed,
    );
    let ids = |scope| kept(&events, scope);
    assert_eq!(
        ids(Scope::Phase("review".into())),
        ["early", "omitted", "theirs"]
    );
    let seat = |member: &str| Scope::Seat(format!("review:positions:{member}"));
    assert_eq!(ids(seat("adversarial")), ["omitted"]);
    assert_eq!(ids(seat("correctness")), ["theirs"]);
    assert!(ids(Scope::Phase("work".into())).is_empty());
    // The judge's step, a single driver with no participant of its own,
    // is filed under its step, and the chief's success records none.
    let records = gate_records(&events);
    let keys: Vec<&str> = records.iter().map(|record| record.key.as_str()).collect();
    let review = keys[0].split(':').next().unwrap();
    assert_eq!(
        keys,
        [
            format!("{review}:judge"),
            format!("{review}:positions"),
            review.into()
        ]
    );
    let markers: Vec<&Value> = (events.iter())
        .map(|event| &event.payload["checkpoint"])
        .filter(|checkpoint| checkpoint["step"] == "panel-member-finished")
        .collect();
    assert_eq!(markers.len(), 2);
    for marker in markers {
        assert!(
            marker["site_ref"]
                .as_str()
                .is_some_and(|site| site.len() == 64),
            "{marker}"
        );
    }
}
