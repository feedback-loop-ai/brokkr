//! Fix round 0109a1r3, M3, through the real engine path with no planted
//! checkpoint: every gate verdict the journal records reads back under
//! the seat that produced it, a non-final step's and a panel member's
//! the chief omits included, and nowhere else.

use serde_json::{json, Value};

use super::covered_tests::drift_refusing;
use super::tests::{driven_with, driver, failing, reported};
use crate::bundle::{
    Aggregate, Bundle, GateAssessment, Invocation, SeatBody, SeatClass, SequenceStep, StepBody,
};
use crate::engine::tests::{member, single_body, step_body};
use brokkr_core::envelope::{EventEnvelope, EventType};
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
    reviewed_by_a_chief(compiled, driver(&json!({"result": "clean"})));
}

/// [`reviewed`], with the chief's driver `chief`.
fn reviewed_by_a_chief(compiled: &mut Bundle, chief: Vec<String>) {
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
        gate_step("chief", &["clean"], step_body(chief)),
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
    (producers(events, scope).into_iter())
        .map(|(id, _)| id)
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
    // Each record is filed under the step that produced it, the final
    // chief's success included, which names its step (fix round
    // 0109a1r4, K6).
    let records = gate_records(&events, None);
    let keys: Vec<&str> = records.iter().map(|record| record.key.as_str()).collect();
    let review = keys[0].split(':').next().unwrap();
    assert_eq!(
        keys,
        [
            format!("{review}:judge"),
            format!("{review}:positions"),
            format!("{review}:chief"),
        ]
    );
    let markers: Vec<&Value> = (events.iter())
        .map(|event| &event.payload["checkpoint"])
        .filter(|checkpoint| checkpoint["step"] == "panel-member-finished")
        .collect();
    assert_eq!(markers.len(), 2);
    // The engine's own records carry its authorship, which is what a
    // reader admits a verdict by (fix round 0109a1r4, K1).
    let judge = (events.iter())
        .map(|event| &event.payload["checkpoint"])
        .find(|checkpoint| checkpoint["step"] == "sequence-step-finished")
        .unwrap();
    assert_eq!(judge["authored"], "engine");
    for marker in markers {
        assert_eq!(marker["authored"], "engine");
        assert!(
            marker["site_ref"]
                .as_str()
                .is_some_and(|site| site.len() == 64),
            "{marker}"
        );
    }
}

/// The producer of each finding `scope` keeps of the run's verdicts, by
/// id, in order.
fn producers(events: &[EventEnvelope], scope: Scope) -> Vec<(String, String)> {
    let state = brokkr_core::fold::fold(events).unwrap();
    let view = brokkr_view::run_view(events, Some(&state));
    let lens = lens_for(&view, Some(&scope)).unwrap();
    gate_records(events, Some(&state))
        .iter()
        .filter_map(|record| record.kept_by(lens.as_ref()))
        .flat_map(|record: GateRecord| record.findings)
        .map(|finding| (finding.id, finding.producer))
        .collect()
}

/// Fix round 0109a1r4, K6, through the real engine with no planted
/// checkpoint: a finding-bearing single gate step before the end of a
/// sequence and one at its end are each selected by their own seat, and
/// by no other; the phase keeps both and names each one's producer.
#[test]
fn a_gate_steps_finding_shows_under_its_own_seat_before_the_end_and_at_it() {
    let chief = driver(&json!({"result": "clean", "findings": [reported("late", 9)]}));
    let (_dir, events) = driven_with(
        single_body(driver(&json!({"result": "complete"}))),
        |compiled| reviewed_by_a_chief(compiled, chief),
    );
    let ids = |seat: &str| kept(&events, Scope::Seat(seat.into()));
    assert_eq!(ids("review:judge"), ["early"]);
    assert_eq!(ids("review:chief"), ["late"]);
    assert_eq!(ids("review"), Vec::<String>::new());
    let named = |id: &str, by: &str| (id.to_string(), by.to_string());
    assert_eq!(
        producers(&events, Scope::Phase("review".into())),
        [
            named("early", "review:judge"),
            named("omitted", "review:positions:adversarial"),
            named("theirs", "review:positions:correctness"),
            named("late", "review:chief"),
        ]
    );
}

/// A run whose review is a top-level gate panel of member `a`, which
/// reports `mine`, and member `b`, driven by `b`.
fn panel_reviewed(b: Vec<String>) -> (tempfile::TempDir, Vec<EventEnvelope>) {
    let review = SeatBody::Invocation(Invocation::Panel {
        members: vec![member("a", residual("mine", 5)), member("b", b)],
        aggregate: Aggregate::ReviewPanel,
    });
    driven_with(
        single_body(driver(&json!({"result": "complete"}))),
        |compiled| {
            compiled.machine = drift_refusing();
            let seat = compiled.seats.get_mut("review").unwrap();
            seat.body = review;
            seat.gate = GateAssessment::AllGate;
            let ship = compiled.seats.get_mut("ship").unwrap();
            ship.body = single_body(driver(&json!({"result": "shipped"})));
        },
    )
}

/// Fix round 0109a1r4, K7: in a top-level gate panel whose member `b`
/// fails through its last retry, member `a`'s finding is durable in each
/// failed attempt's settled members, and shows under `a` and in the
/// phase, never under `b`.
#[test]
fn a_settled_members_finding_shows_when_its_sibling_fails() {
    let (_dir, events) = panel_reviewed(failing());
    assert!(!events.iter().any(|event| {
        event.event_type == EventType::EffectSucceeded
            && event.payload["result"]["findings"].is_array()
    }));
    let failed = events
        .iter()
        .filter(|event| event.event_type == EventType::EffectFailed);
    let attempts = failed.count();
    assert_eq!(attempts, 1);
    assert!(events
        .iter()
        .any(|event| event.event_type == EventType::RunParked));
    let mine = vec!["mine".to_string(); attempts];
    assert_eq!(kept(&events, Scope::Seat("review:a".into())), mine);
    assert_eq!(kept(&events, Scope::Phase("review".into())), mine);
    assert_eq!(
        kept(&events, Scope::Seat("review:b".into())),
        Vec::<String>::new()
    );
    let named = producers(&events, Scope::Phase("review".into()));
    assert!(named.iter().all(|(_, by)| by == "review:a"), "{named:?}");
}

/// A top-level gate panel where one member reports: its seat shows the
/// finding, its sibling's seat does not, and the phase does, naming it.
#[test]
fn a_top_level_panel_members_finding_shows_under_that_member_alone() {
    let (_dir, events) = panel_reviewed(driver(&json!({"result": "clean"})));
    assert_eq!(kept(&events, Scope::Seat("review:a".into())), ["mine"]);
    // Neither the sibling's seat nor the panel's own keeps a member's.
    for seat in ["review:b", "review"] {
        assert_eq!(
            kept(&events, Scope::Seat(seat.into())),
            Vec::<String>::new()
        );
    }
    assert_eq!(
        producers(&events, Scope::Phase("review".into())),
        [("mine".to_string(), "review:a".to_string())]
    );
}

/// Fix round 0109a1r5, V2: a released engine's failed panel row, as the
/// tagged v0.15.0 engine journaled it. Member `a` settled with a
/// driver-written `covered` head and a stamped-looking finding, copied raw
/// into `settled_members`; member `b` failed; the member markers carry no
/// `authored`. The row exports and verifies, and inspect shows no verdict
/// head and no finding for `review:a`, only unverified legacy evidence.
#[test]
fn a_released_engines_settled_member_payload_is_no_verdict() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = brokkr_store::Store::open(&dir.path().join("forge.db")).unwrap();
    let (run, manifest) = ("legacy", json!({"engine": "0.15.0"}));
    store.create_run(run, "feat", "self", &manifest).unwrap();
    let planted = json!({"result": "clean",
        "covered": {"kind": "head", "head": "a".repeat(40), "dirty": false},
        "findings": [{"site": "5a1e".repeat(16), "id": "planted", "file": "a.rs",
                      "summary": "s", "severity": "high", "security": true}]});
    let marker = |member: &str, outcome: &str| {
        json!({"effect_id": "fx", "attempt_id": "at-1", "checkpoint": {
            "step": "panel-member-finished", "member": member, "outcome": outcome,
            "model": "m", "session_ref": null, "inner_checkpoints": 0}})
    };
    let journal = [
        (
            EventType::RunStarted,
            json!({"feature": "feat", "manifest": manifest}),
        ),
        (EventType::PhaseEntered, json!({"phase": "review"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "fx", "phase": "review", "seat": "review"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "fx", "attempt_id": "at-1"}),
        ),
        (EventType::EffectCheckpointed, marker("a", "succeeded")),
        (EventType::EffectCheckpointed, marker("b", "failed")),
        (
            EventType::EffectFailed,
            json!({"effect_id": "fx", "attempt_id": "at-1",
            "error": "b failed", "tree": {"at_start": "clean", "at_end": "clean"},
            "settled_members": {"a": planted}}),
        ),
    ];
    for (event_type, payload) in journal {
        (store.append_next(run, event_type, payload, None, None)).unwrap();
    }
    let exported = store.export_ndjson(run).unwrap();
    assert!(brokkr_store::verify_export(&exported).is_ok());
    let records = gate_records(&store.load(run).unwrap(), None);
    assert_eq!(records.len(), 1, "{records:?}");
    assert_eq!(records[0].findings, []);
    assert_eq!(records[0].covered, None);
    assert_eq!(records[0].unverified.as_deref(), Some("review:a"));
}
