//! Fix round 0109a1r3, M3, through the real engine path with no planted
//! checkpoint: every gate verdict the journal records reads back under
//! the seat that produced it, a non-final step's and a panel member's
//! the chief omits included, and nowhere else.

use serde_json::{json, Value};

use super::covered_tests::drift_refusing;
use super::tests::{begun, driven_with, driver, failing, reported};
use crate::bundle::{
    Aggregate, Bundle, GateAssessment, Invocation, PanelMember, SeatBody, SeatClass, SequenceStep,
    StepBody,
};
use crate::engine::heads::HeadReads;
use crate::engine::tests::{member, single_body, step_body};
use crate::engine::{Engine, FencedCommandOutcome, MemberRetryOutcome};
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
    panel_of(residual("mine", 5), b)
}

/// A run whose review is a top-level gate panel of members `a` and `b`.
fn panel_of(a: Vec<String>, b: Vec<String>) -> (tempfile::TempDir, Vec<EventEnvelope>) {
    reviewed_as(SeatBody::Invocation(Invocation::Panel {
        members: vec![member("a", a), member("b", b)],
        aggregate: Aggregate::ReviewPanel,
    }))
}

/// A run whose review is the all-gate body `review`.
fn reviewed_as(review: SeatBody) -> (tempfile::TempDir, Vec<EventEnvelope>) {
    driven_with(
        single_body(driver(&json!({"result": "complete"}))),
        review_shaped(review),
    )
}

/// The bundle whose review is the all-gate body `review`, and whose ship
/// answers `shipped`.
fn review_shaped(review: SeatBody) -> impl FnOnce(&mut Bundle) {
    move |compiled: &mut Bundle| {
        compiled.machine = drift_refusing();
        let seat = compiled.seats.get_mut("review").unwrap();
        seat.body = review;
        seat.gate = GateAssessment::AllGate;
        let ship = compiled.seats.get_mut("ship").unwrap();
        ship.body = single_body(driver(&json!({"result": "shipped"})));
    }
}

/// A run whose review is a top-level gate panel of `members`, driven until
/// it parks, then retried for `names` through the operator's command,
/// which the engine accepts, and driven again: its journal as it parked,
/// checked by `parked` before the retry, and as it ends.
fn retried(
    members: Vec<PanelMember>,
    names: &[&str],
    parked: impl FnOnce(&[EventEnvelope]),
) -> (tempfile::TempDir, Vec<EventEnvelope>) {
    let review = SeatBody::Invocation(Invocation::Panel {
        members,
        aggregate: Aggregate::ReviewPanel,
    });
    let work = single_body(driver(&json!({"result": "complete"})));
    let (dir, mut engine) = begun(work, review_shaped(review), |_| HeadReads::default());
    engine.drive().unwrap();
    parked(&engine.store.load(&engine.run_id).unwrap());
    let read = Engine::read_member_retry(&mut engine.store, &engine.run_id)
        .unwrap()
        .unwrap();
    let names: Vec<String> = names.iter().map(|name| name.to_string()).collect();
    let outcome = engine.retry_members(read, &names, "test", "again").unwrap();
    assert!(
        matches!(
            outcome,
            MemberRetryOutcome::Disposed(FencedCommandOutcome::Accepted { .. })
        ),
        "{outcome:?}"
    );
    engine.drive().unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    (dir, events)
}

/// The members each start of `events` carried, in order.
fn carried(events: &[EventEnvelope]) -> Vec<Vec<String>> {
    (events.iter())
        .filter(|event| event.event_type == EventType::EffectStarted)
        .map(|event| {
            let carried = event.payload["carried"].as_array().cloned();
            (carried.unwrap_or_default().iter())
                .map(|entry| entry["member"].as_str().unwrap().to_string())
                .collect()
        })
        .collect()
}

/// Fix round 0109a1r7, X4: in a three-member top-level gate panel, `a`
/// returns a valid finding, `b` over-subset accounting and `c`'s driver
/// fails. The fence refuses `b`'s marked payload by name, journaled in the
/// failure's own words, and `a` keeps its payload, its verdict and its
/// retry eligibility: its finding shows under `a` and in the phase, and a
/// retry of `b` and `c` carries `a` and runs `b` and `c` only.
#[test]
fn one_refused_settled_member_keeps_every_admitted_sibling() {
    let over = json!({"result": "clean", "input_tokens": 1, "cache_read_tokens": 2});
    let members = vec![
        member("a", residual("mine", 5)),
        member("b", driver(&over)),
        member("c", failing()),
    ];
    let (_dir, events) = retried(members, &["b", "c"], |events| {
        assert_eq!(kept(events, Scope::Seat("review:a".into())), ["mine"]);
        assert_eq!(kept(events, Scope::Phase("review".into())), ["mine"]);
        assert!(kept(events, Scope::Seat("review:b".into())).is_empty());
        let failed = (events.iter())
            .find(|event| event.event_type == EventType::EffectFailed)
            .unwrap();
        let settled = failed.payload["settled_members"].as_object().unwrap();
        assert_eq!(settled.keys().collect::<Vec<_>>(), ["a"]);
        let error = failed.payload["error"].as_str().unwrap();
        let named = "; settled member 'b' was refused: seat record at journal seq";
        assert!(error.contains(named), "{error}");
        assert!(error.ends_with("at /settled_members/b/cache_read_tokens"));
    });
    // work's start, review's first attempt, and the retry.
    assert_eq!(carried(&events), [vec![], vec![], vec!["a".to_string()]]);
    let retry = (events.iter())
        .rfind(|event| event.event_type == EventType::EffectStarted)
        .unwrap();
    let ran = |member: &str| {
        events.iter().any(|event| {
            event.event_type == EventType::EffectCheckpointed
                && event.payload["attempt_id"] == retry.payload["attempt_id"]
                && event.payload["checkpoint"]["member"] == member
        })
    };
    assert_eq!((ran("a"), ran("b"), ran("c")), (false, true, true));
}

/// Fix round 0109a1r7, X5: W4's two members inside a non-final gate panel
/// step, `a` reporting a finding and `b`'s JSON carrying no result word.
/// The poisoned aggregate still refuses the step, failing the attempt, and
/// the failure keeps the step's panel evidence, so `a`'s admitted finding
/// shows under review:positions:a and in the phase; `b`'s wordless payload
/// is refused by name (X3, X4) and shows nowhere.
#[test]
fn a_poisoned_non_final_panel_steps_admitted_finding_stays_visible() {
    let positions = StepBody::Invocation(Invocation::Panel {
        members: vec![
            member("a", residual("mine", 5)),
            member("b", driver(&json!({"notes": "no word"}))),
        ],
        aggregate: Aggregate::ReviewPanel,
    });
    let words = ["clean", "residual"];
    let steps = vec![
        gate_step("positions", &words, positions),
        gate_step(
            "chief",
            &["clean"],
            step_body(driver(&json!({"result": "clean"}))),
        ),
    ];
    let (_dir, events) = reviewed_as(SeatBody::Sequence { steps });
    assert_eq!(
        kept(&events, Scope::Seat("review:positions:a".into())),
        ["mine"]
    );
    let failed = (events.iter())
        .find(|event| event.event_type == EventType::EffectFailed)
        .unwrap();
    let error = failed.payload["error"].as_str().unwrap();
    let refused = "sequence step 'positions': reported '__member-schema-invalid__'";
    assert!(error.starts_with(refused), "{error}");
    assert!(
        error.contains("; settled member 'positions:b' was refused"),
        "{error}"
    );
    let settled = failed.payload["settled_members"].as_object().unwrap();
    assert_eq!(settled.keys().collect::<Vec<_>>(), ["positions:a"]);
    assert!(!events
        .iter()
        .any(|event| event.event_type == EventType::EffectSucceeded
            && event.payload["effect_id"] == failed.payload["effect_id"]));
    assert!(kept(&events, Scope::Seat("review:positions:b".into())).is_empty());
    let by_a = vec![("mine".to_string(), "review:positions:a".to_string())];
    assert_eq!(producers(&events, Scope::Phase("review".into())), by_a);
}

/// A driver that fails its first attempt and answers `then` after it,
/// `flag` recording that it has failed once.
fn failing_once(flag: &std::path::Path, then: &Value) -> Vec<String> {
    let (fail, answer) = (failing().remove(2), driver(then).remove(2));
    let flag = flag.display();
    let script = format!("if [ -e '{flag}' ]; then\n{answer}\nelse\ntouch '{flag}'\n{fail}\nfi\n");
    vec!["sh".into(), "-c".into(), script]
}

/// Fix round 0109a1r7, X2: a partial retry mints a new effect and runs no
/// carried member, so the carried member's site is journaled from the
/// engine's own carry record. Member `a` reports `mine` and `b` fails, so
/// the run parks; a retry of `b` carries `a`, `b` answers clean, and the
/// retry's aggregate holds `mine`. Every `mine`, the failed attempt's and
/// the retry's, reads under review:a, in its seat and in the phase.
#[test]
fn a_carried_members_finding_keeps_its_members_name_in_the_retry() {
    let flag = tempfile::tempdir().unwrap();
    let members = vec![
        member("a", residual("mine", 5)),
        member(
            "b",
            failing_once(&flag.path().join("failed"), &json!({"result": "clean"})),
        ),
    ];
    let (_dir, events) = retried(members, &["b"], |events| {
        assert_eq!(kept(events, Scope::Seat("review:a".into())), ["mine"]);
    });
    assert_eq!(carried(&events).last().unwrap(), &["a"]);
    let aggregate = (events.iter())
        .rfind(|event| event.event_type == EventType::EffectSucceeded)
        .unwrap();
    assert_eq!(aggregate.payload["result"]["findings"][0]["id"], "mine");
    let mine = vec!["mine".to_string(); 2];
    assert_eq!(kept(&events, Scope::Seat("review:a".into())), mine);
    let by_a = vec![("mine".to_string(), "review:a".to_string()); 2];
    assert_eq!(producers(&events, Scope::Phase("review".into())), by_a);
    assert!(kept(&events, Scope::Seat("review:b".into())).is_empty());
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

/// Fix round 0109a1r6, W3, through the real writer: member `a` settles
/// with `input_tokens: 1` and `cache_read_tokens: 2` while `b` fails. Its
/// marked payload is refused by the append fence, so the failed row is
/// journaled without `settled_members`, naming the refusal, and `a` shows
/// no verdict; the drive never errors.
#[test]
fn a_marked_settled_member_the_fence_refuses_is_no_settled_verdict() {
    let over = json!({"result": "clean", "input_tokens": 1, "cache_read_tokens": 2});
    let (_dir, events) = panel_of(driver(&over), failing());
    let failed: Vec<&Value> = (events.iter())
        .filter(|event| event.event_type == EventType::EffectFailed)
        .map(|event| &event.payload)
        .collect();
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert!(failed[0].get("settled_members").is_none(), "{}", failed[0]);
    let error = failed[0]["error"].as_str().unwrap();
    assert!(
        error.ends_with("at /settled_members/a/cache_read_tokens"),
        "{error}"
    );
    assert!(error.contains("; settled member 'a' was refused: seat record at journal seq"));
    assert!(gate_records(&events, None).is_empty());
}

/// Fix round 0109a1r6, W4: a top-level gate panel whose member `a`
/// reports a finding and whose sibling's JSON carries no result word. The
/// aggregate is still refused, and `a`'s admitted finding stays verdict
/// evidence under `a` and in the phase, never under `b`.
#[test]
fn an_admitted_finding_stays_visible_when_its_panels_aggregate_is_refused() {
    let (_dir, events) = panel_reviewed(driver(&json!({"notes": "no word"})));
    let succeeded = (events.iter())
        .find(|event| {
            event.event_type == EventType::EffectSucceeded
                && event.payload["result"]["notes"]["members"].is_object()
        })
        .unwrap();
    let result = &succeeded.payload["result"];
    assert_eq!(result["result"], "__member-schema-invalid__", "{result}");
    assert_eq!(result["notes"]["members"]["b"], json!({"notes": "no word"}));
    let decided = (events.iter())
        .find(|event| {
            event.event_type == EventType::TransitionDecided && event.payload["from"] == "review"
        })
        .unwrap();
    assert_eq!(
        decided.payload["result"], "__schema-invalid__",
        "{}",
        decided.payload
    );
    assert_eq!(kept(&events, Scope::Seat("review:a".into())), ["mine"]);
    assert_eq!(
        kept(&events, Scope::Seat("review:b".into())),
        Vec::<String>::new()
    );
    assert_eq!(
        producers(&events, Scope::Phase("review".into())),
        [("mine".to_string(), "review:a".to_string())]
    );
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
    let marker = |member: &str, outcome: &str| {
        json!({"effect_id": "fx", "attempt_id": "at-1", "checkpoint": {
            "step": "panel-member-finished", "member": member, "outcome": outcome,
            "model": "m", "session_ref": null, "inner_checkpoints": 0}})
    };
    let tail = [
        (EventType::EffectCheckpointed, marker("a", "succeeded")),
        (EventType::EffectCheckpointed, marker("b", "failed")),
        (
            EventType::EffectFailed,
            json!({"effect_id": "fx", "attempt_id": "at-1",
            "error": "b failed", "tree": {"at_start": "clean", "at_end": "clean"},
            "settled_members": {"a": planted()}}),
        ),
    ];
    assert_unverified_legacy(&released_journal(tail), "review:a");
}

/// Fix round 0109a1r6, W6: the adversarial position's probe. A released
/// engine's single-seat success, as a v0.15.0 journal holds it, whose
/// result carries a driver-written `covered` and a sited `planted`
/// finding and whose row carries no engine mark: it exports and verifies,
/// and inspect shows no verdict head and no finding for review, only
/// unverified legacy evidence. The current writer's success is marked,
/// and reads as review's verdict.
#[test]
fn a_released_engines_success_row_is_no_verdict() {
    let succeeded = json!({"effect_id": "fx", "attempt_id": "at-1", "result": planted()});
    assert_unverified_legacy(
        &released_journal([(EventType::EffectSucceeded, succeeded)]),
        "review",
    );
    let (_dir, events) = reviewed_as(single_body(residual("mine", 5)));
    let review = (events.iter())
        .find(|event| {
            event.event_type == EventType::EffectSucceeded
                && event.payload["result"]["findings"].is_array()
        })
        .unwrap();
    assert_eq!(review.payload["authored"], "engine");
    assert_eq!(kept(&events, Scope::Phase("review".into())), ["mine"]);
    let records = gate_records(&events, None);
    assert!(records.iter().all(|record| record.unverified.is_none()));
}

/// A stamped-looking result a released engine copied raw: a driver's
/// `covered` head and a sited finding.
fn planted() -> Value {
    json!({"result": "clean",
        "covered": {"kind": "head", "head": "a".repeat(40), "dirty": false},
        "findings": [{"site": "5a1e".repeat(16), "id": "planted", "file": "a.rs",
                      "summary": "s", "severity": "high", "security": true}]})
}

/// A v0.15.0 journal whose review attempt `at-1` of effect `fx` ends with
/// `tail`, appended through the fence, exported and verified offline.
fn released_journal<const N: usize>(tail: [(EventType, Value); N]) -> Vec<EventEnvelope> {
    let dir = tempfile::tempdir().unwrap();
    let mut store = brokkr_store::Store::open(&dir.path().join("forge.db")).unwrap();
    let (run, manifest) = ("legacy", json!({"engine": "0.15.0"}));
    store.create_run(run, "feat", "self", &manifest).unwrap();
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
    ];
    for (event_type, payload) in journal.into_iter().chain(tail) {
        (store.append_next(run, event_type, payload, None, None)).unwrap();
    }
    let exported = store.export_ndjson(run).unwrap();
    assert!(brokkr_store::verify_export(&exported).is_ok());
    store.load(run).unwrap()
}

/// `events` hold one record, `by`'s unverified legacy evidence: no
/// verdict head and no finding.
fn assert_unverified_legacy(events: &[EventEnvelope], by: &str) {
    let records = gate_records(events, None);
    assert_eq!(records.len(), 1, "{records:?}");
    assert_eq!(records[0].findings, []);
    assert_eq!(records[0].covered, None);
    assert_eq!(records[0].unverified.as_deref(), Some(by));
}
