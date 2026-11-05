use super::*;
use brokkr_core::ending::{ended, Citation, Ended, Ending};
use brokkr_core::fold::{fold, Cursor, ParkReason, Run};
use brokkr_store::StoreError;
use serde_json::Value;
use std::path::Path;

use super::super::ending::tests::{
    bytes, case_cause, cause_refusal, contract, CITATION_CASES, STOPPED,
};
use super::super::ending::{named_cause, Cause};
use crate::engine::conclude_as;

/// A run an operator stopped without naming a cause — the shape ruling 5
/// exists to classify.
fn finished_store(path: &Path, run_id: &str) -> Store {
    let mut store = crate::engine::tests::in_flight_store(path, run_id);
    store
        .append_next(
            run_id,
            EventType::RunStopped,
            json!({"reason": "operator 'operator' commanded stop: enough"}),
            None,
            None,
        )
        .unwrap();
    store
}

/// A run whose fold reached `completed`: its ending is its last review
/// ruling's, and no operator cause can name `shipped` or
/// `shipped-flagged` (ruling 6's ending-implies-status table).
fn completed_store(path: &Path, run_id: &str) -> Store {
    let mut store = crate::engine::tests::in_flight_store(path, run_id);
    store
        .append_next(
            run_id,
            EventType::EffectSucceeded,
            json!({"effect_id": "effect", "result": {"result": "complete"}}),
            None,
            None,
        )
        .unwrap();
    store
        .append_next(run_id, EventType::RunCompleted, json!({}), None, None)
        .unwrap();
    store
}

/// Append `events` to the run's journal in order: a fixture's whole
/// sequence, so no two fixtures restate the append loop.
fn journal(store: &mut Store, run_id: &str, events: Vec<(EventType, Value, Option<String>)>) {
    for (event_type, payload, attempt_id) in events {
        store
            .append_next(run_id, event_type, payload, None, attempt_id)
            .unwrap();
    }
}

/// A run `ruling` parked while it stood at `Decide`: the ruling reaches
/// the park and the engine journals `run/parked` with the reason the
/// fold's cursor holds, which `reason` pins. The run is still waiting for
/// the operator.
fn rule_park_store(path: &Path, run_id: &str, ruling: Value, reason: &str) -> Store {
    let mut store = decided_after_effect(path, run_id, ruling, "fail");
    parked_at_the_cursor(&mut store, run_id, reason);
    store
}

/// Journal `run/parked` as the engine's `Park` arm does: with the reason
/// the fold's cursor stands at, asserted to be `reason`.
fn parked_at_the_cursor(store: &mut Store, run_id: &str, reason: &str) {
    let (state, _) = read(store, run_id);
    assert_eq!(
        state.run,
        Run::Running(Cursor::Park {
            reason: ParkReason::read(reason.to_string()),
        }),
        "the fold opened this park with the reason the engine journals"
    );
    journal(
        store,
        run_id,
        vec![(
            EventType::RunParked,
            json!({"reason": reason, "evidence": {}}),
            None,
        )],
    );
}

/// The reason the fold opens [`rule_park`]'s park with.
const RULE_PARK_REASON: &str = "VERIFY-FAIL-EXHAUSTED for (verify, fail): the bounded loop ran out";

/// The run after a succeeded effect and the ruling `ruling` decided on
/// it, at the position that ruling reached.
fn decided_after_effect(path: &Path, run_id: &str, ruling: Value, result: &str) -> Store {
    let mut store = crate::engine::tests::in_flight_store(path, run_id);
    journal(
        &mut store,
        run_id,
        vec![
            (
                EventType::EffectSucceeded,
                json!({"effect_id": "effect", "attempt_id": "attempt",
                       "result": {"result": result}}),
                Some("attempt".to_string()),
            ),
            (EventType::TransitionDecided, ruling, None),
        ],
    );
    store
}

/// A finished run: the park `ruling` opened with `reason`, concluded by
/// the operator.
fn ruled_store(path: &Path, run_id: &str, ruling: Value, reason: &str) -> Store {
    let mut store = rule_park_store(path, run_id, ruling, reason);
    conclude_as(&mut store, run_id, "operator", "the rule parked it").unwrap();
    store
}

/// The park a rule opened: `next` absent, the rule named.
fn rule_park() -> Value {
    json!({"from": "verify", "result": "fail", "rule_id": "VERIFY-FAIL-EXHAUSTED",
           "next": null, "problem": "the bounded loop ran out"})
}

/// A second run, so a `superseded` citation resolves. It is still going:
/// ruling 4 lets a re-commission be cited before it finishes.
fn later_store(path: &Path) -> Store {
    crate::engine::tests::in_flight_store(path, "later")
}

/// A stopped run and the journal a citation is read in, with the journal
/// length every refusal must leave alone.
fn held_fixture(dir: &Path, name: &str) -> (std::path::PathBuf, Store, Store, usize) {
    let db = dir.join(name);
    let store = finished_store(&db, "held");
    later_store(&db);
    let cited = Store::open_read_only(&db).unwrap();
    let before = store.load("held").unwrap().len();
    (db, store, cited, before)
}

/// The ask a test varies the cause of.
fn asking(cause: Cause<'static>) -> Named<'static> {
    Named {
        cause,
        reason: "the operator named the ending",
        operator: "operator",
    }
}

/// The ask naming `ending`, citing nothing.
fn ask(ending: Ending) -> Named<'static> {
    asking(Cause { ending, ..STOPPED })
}

/// The ask naming `superseded` by `by_run` in `by_realm`.
fn superseded(by_run: &'static str, by_realm: Option<&'static str>) -> Named<'static> {
    asking(Cause {
        ending: Ending::Superseded,
        by_run: Some(by_run),
        by_realm,
        by_ref: None,
    })
}

/// The one typed refusal `error` carries, so each test pins its text once:
/// the run's, or the cause's.
fn refusal(error: EngineError) -> String {
    match error {
        EngineError::ClassifyRefused(text) => text,
        cause @ EngineError::CauseRefused(_) => cause_refusal(cause, Verb::Classify).to_string(),
        other => panic!("classify refused with another error: {other}"),
    }
}

/// `exhausted` named on the stopped run `held`: accepted, and the one
/// event the verb writes is an annotation with no acceptance, whose
/// `args` satisfy operator-ending.v1.
fn assert_exhausted_accepted(db: &Path, store: &mut Store, what: &str) {
    let cited = Store::open_read_only(db).unwrap();
    let before = bytes(&store.load("held").unwrap());
    let written = operator_classify(store, &cited, "held", &ask(Ending::Exhausted)).unwrap();
    assert_eq!(written.event_type, EventType::OperatorCommanded, "{what}");
    assert_eq!(written.payload["command"], CLASSIFY, "{what}");
    assert_eq!(
        written.payload["args"],
        json!({"cause": "exhausted", "reason": "the operator named the ending"}),
        "{what}"
    );
    assert!(contract().is_valid(&written.payload["args"]), "{what}");
    let mut expected = before;
    expected.push(json!(written));
    assert_eq!(
        bytes(&store.load("held").unwrap()),
        expected,
        "{what}: the classify appends exactly one event, and no acceptance"
    );
}

/// The state and events a read after the writes uses.
fn read(store: &mut Store, run_id: &str) -> (brokkr_core::RunState, Vec<EventEnvelope>) {
    let events = store.load(run_id).unwrap();
    (fold(&events).unwrap(), events)
}

/// Ruling 5: one `operator/commanded` naming the latest reading, no
/// acceptance, and the fold derives nothing from it. The schema is the
/// published file, not this test's idea of the shape.
#[test]
fn a_classify_writes_one_annotation_and_no_acceptance() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("classify.db");
    let mut store = finished_store(&db, "held");
    later_store(&db);
    let before = fold(&store.load("held").unwrap()).unwrap();
    let cited = Store::open_read_only(&db).unwrap();
    let ask = superseded("later", None);
    let written = operator_classify(&mut store, &cited, "held", &ask).unwrap();
    assert_eq!(written.seq, before.seq + 1);
    assert_eq!(written.event_type, EventType::OperatorCommanded);
    assert_eq!(written.payload["command"], "classify");
    assert_eq!(written.payload["operator"], "operator");
    assert_eq!(
        written.payload["args"],
        json!({"cause": "superseded",
               "by": {"realm": null, "run_id": "later"},
               "reason": "the operator named the ending"})
    );
    assert!(
        contract().is_valid(&written.payload["args"]),
        "the written args satisfy operator-ending.v1"
    );

    let (after, events) = read(&mut store, "held");
    assert_eq!(
        events.len(),
        before.seq as usize + 1,
        "one event was appended"
    );
    assert!(
        !events
            .iter()
            .any(|event| event.event_type == EventType::OperatorAccepted),
        "no acceptance follows a classify"
    );
    assert_eq!(after.seq, before.seq + 1, "the journal advanced by one");
    assert_eq!(after.run, before.run, "an annotation is not a conclusion");
    assert_eq!(after.last_decision, before.last_decision);
    assert_eq!(after.phase, before.phase);
    assert!(
        after.pending_command.is_none(),
        "an annotation is never held as pending"
    );
    assert_eq!(
        ended(&events, &after),
        Some(Ended {
            ending: Ending::Superseded,
            cited: Some(Citation::Run {
                realm: None,
                run_id: "later".into(),
            }),
        })
    );
}

/// Choice 7: the schema is over `args` and covers each cause's exact
/// shape. The verb writes it, for all six causes, and nothing else.
#[test]
fn every_cause_writes_exactly_the_shape_operator_ending_v1_names() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("shapes.db");
    let mut store = ruled_store(&db, "held", rule_park(), RULE_PARK_REASON);
    later_store(&db);
    let cited = Store::open_read_only(&db).unwrap();
    let reason = "the operator named the ending";
    for (cause, by_run, by_ref, expected) in [
        (
            "superseded",
            Some("later"),
            None,
            json!({"cause": "superseded",
                   "by": {"realm": null, "run_id": "later"}, "reason": reason}),
        ),
        (
            "delivered-elsewhere",
            None,
            Some("abc123"),
            json!({"cause": "delivered-elsewhere", "ref": "abc123", "reason": reason}),
        ),
        (
            "infrastructure",
            None,
            None,
            json!({"cause": "infrastructure", "reason": reason}),
        ),
        (
            "withdrawn",
            None,
            None,
            json!({"cause": "withdrawn", "reason": reason}),
        ),
        (
            "exhausted",
            None,
            None,
            json!({"cause": "exhausted", "reason": reason}),
        ),
        (
            "operator-stopped",
            None,
            None,
            json!({"cause": "operator-stopped", "reason": reason}),
        ),
    ] {
        let ending = named_cause(Verb::Classify, Some(cause)).unwrap();
        let ask = asking(Cause {
            ending,
            by_run,
            by_ref,
            by_realm: None,
        });
        let written = operator_classify(&mut store, &cited, "held", &ask).unwrap();
        assert_eq!(written.payload["args"], expected, "{cause}");
        assert!(
            contract().is_valid(&written.payload["args"]),
            "{cause} must satisfy operator-ending.v1"
        );
    }
    // The schema refuses a shape outside the six, a misplaced citation,
    // and an extra field, so this is the one contract the verb writes.
    let contract = contract();
    for outside in [
        json!({"cause": "shipped", "reason": reason}),
        json!({"cause": "superseded", "reason": reason}),
        json!({"cause": "delivered-elsewhere", "by": {"realm": null, "run_id": "later"}, "reason": reason}),
        json!({"cause": "withdrawn", "ref": "abc123", "reason": reason}),
        json!({"cause": "withdrawn", "reason": reason, "extra": true}),
    ] {
        assert!(!contract.is_valid(&outside), "{outside}");
    }
}

/// Ruling 5: classify is for a stopped run. A live run would hold the
/// annotation as a pending command, so it is refused before anything is
/// written.
#[test]
fn a_live_run_is_refused_and_its_journal_is_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("live.db");
    let mut store = crate::engine::tests::in_flight_store(&db, "going");
    later_store(&db);
    let cited = Store::open_read_only(&db).unwrap();
    let before = store.load("going").unwrap().len();
    let refused =
        operator_classify(&mut store, &cited, "going", &ask(Ending::Withdrawn)).unwrap_err();
    assert_eq!(
        refusal(refused),
        "run 'going' is running; classify names a stopped run's ending"
    );
    assert_eq!(
        store.load("going").unwrap().len(),
        before,
        "nothing written"
    );
}

/// Ruling 6: a completed run's ending is `shipped` or `shipped-flagged`,
/// derived from its last review, so naming one of ruling 4's six there
/// would journal an annotation the reading must ignore. Admission refuses
/// it, and reads stay status-implied.
#[test]
fn a_completed_run_is_refused_and_its_journal_is_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("completed.db");
    let mut store = completed_store(&db, "done");
    let cited = Store::open_read_only(&db).unwrap();
    let before = store.load("done").unwrap().len();
    let refused =
        operator_classify(&mut store, &cited, "done", &ask(Ending::Withdrawn)).unwrap_err();
    assert_eq!(
        refusal(refused),
        "run 'done' is completed; classify names a stopped run's ending"
    );
    assert_eq!(store.load("done").unwrap().len(), before, "nothing written");
}

/// Ruling 4's six are the causes: a vocabulary word that is not
/// operator-owned is refused where the run is classified, by exact text,
/// and writes nothing. A word outside the twelve is refused where it is
/// parsed, before any journal is opened (`ending`'s tests).
#[test]
fn a_cause_outside_the_operator_owned_six_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("causes.db");
    let mut store = finished_store(&db, "held");
    let cited = Store::open_read_only(&db).unwrap();
    let before = store.load("held").unwrap().len();
    let shipped = ask(Ending::Shipped);
    assert_eq!(
        refusal(operator_classify(&mut store, &cited, "held", &shipped).unwrap_err()),
        "'shipped' is not an operator-owned ending; an operator names one of exhausted, \
         superseded, infrastructure, delivered-elsewhere, withdrawn, operator-stopped"
    );
    assert_eq!(store.load("held").unwrap().len(), before, "nothing written");
}

/// Ruling 4 and choice 7: a citation belongs to its cause and nowhere
/// else. Each missing or misplaced flag is refused by its own exact text,
/// the one table `stop` and `conclude` are refused by too, and none
/// writes.
#[test]
fn a_missing_or_misplaced_citation_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (_db, mut store, cited, before) = held_fixture(dir.path(), "citations.db");
    for case in CITATION_CASES {
        let refused = operator_classify(&mut store, &cited, "held", &asking(case_cause(&case)));
        let refused = cause_refusal(refused.unwrap_err(), Verb::Classify);
        let (cause, _, variant, text) = case;
        assert!(variant(&refused), "{cause}: {refused:?}");
        assert_eq!(refused.to_string(), text.replace("{verb}", "classify"));
        assert_eq!(
            store.load("held").unwrap().len(),
            before,
            "{cause} wrote nothing"
        );
    }
}

/// Ruling 4: the cited run must exist in the journal the citation names,
/// and may be live; a journal that cannot show it is refused, never
/// guessed at.
#[test]
fn a_cited_run_must_exist_in_the_journal_it_names() {
    let dir = tempfile::tempdir().unwrap();
    let (_db, mut store, cited, before) = held_fixture(dir.path(), "cited.db");
    let absent = superseded("no-such-run", None);
    assert_eq!(
        refusal(operator_classify(&mut store, &cited, "held", &absent).unwrap_err()),
        "run 'no-such-run' in the workspace journal: run 'no-such-run' not found"
    );
    assert_eq!(store.load("held").unwrap().len(), before, "nothing written");
    let elsewhere = superseded("later", Some("neighbour"));
    let neighbour = Store::open(&dir.path().join("neighbour.db")).unwrap();
    let neighbour = Store::open_read_only(neighbour.path()).unwrap();
    assert_eq!(
        refusal(operator_classify(&mut store, &neighbour, "held", &elsewhere).unwrap_err()),
        "run 'later' in realm 'neighbour': run 'later' not found"
    );
    assert_eq!(store.load("held").unwrap().len(), before, "nothing written");
    // "finished or not": the cited run is still going, which ruling 4
    // admits because a re-commission is usually fired in the same minute.
    let live = superseded("later", None);
    operator_classify(&mut store, &cited, "held", &live).unwrap();
    assert_eq!(store.load("held").unwrap().len(), before + 1);
}

/// Addendum ruling 3, as amended on 2026-11-04 (choice 3 (c)):
/// `exhausted` is named as the other five causes are, and nothing refuses
/// it. A run concluded from a rule's park and one concluded from the
/// engine's park (an indeterminate effect) each take it.
#[test]
fn exhausted_is_named_on_any_stopped_park() {
    let dir = tempfile::tempdir().unwrap();
    let rule = dir.path().join("rule.db");
    let mut store = ruled_store(&rule, "held", rule_park(), RULE_PARK_REASON);
    assert_exhausted_accepted(&rule, &mut store, "a rule's park");
    let engine = dir.path().join("engine.db");
    let mut store = crate::engine::tests::parked_store(&engine, "held");
    conclude_as(&mut store, "held", "operator", "the effect was lost").unwrap();
    assert_exhausted_accepted(&engine, &mut store, "the engine's park");
}

/// L1, the review chief's regression: a journal whose events do not fold
/// (an `operator/accepted` that matches no command, then `run/stopped`)
/// is this verb's typed refusal, naming the run and the fold's words, and
/// the journal is left byte for byte as it was.
#[test]
fn chief_nonfolding_journal_returns_typed_classify_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("nonfolding.db");
    let mut store = crate::engine::tests::in_flight_store(&db, "held");
    journal(
        &mut store,
        "held",
        vec![
            (
                EventType::OperatorAccepted,
                json!({"command_id": "nobody", "operator": "operator", "reason": "asked"}),
                None,
            ),
            (
                EventType::RunStopped,
                json!({"reason": "operator 'operator' commanded stop: enough"}),
                None,
            ),
        ],
    );
    let cited = Store::open_read_only(&db).unwrap();
    let before = bytes(&store.load("held").unwrap());
    let refused =
        operator_classify(&mut store, &cited, "held", &ask(Ending::Withdrawn)).unwrap_err();
    assert_eq!(
        refusal(refused),
        "run 'held' does not fold, so its ending cannot be named: event 5: operator/accepted \
         without a matching command"
    );
    assert_eq!(
        bytes(&store.load("held").unwrap()),
        before,
        "nothing written"
    );
}

/// Ruling 5's fenced append (decision 0029): a journal that grew while the
/// annotation was being verified takes the head away, nothing is written,
/// and the operator is sent back to read it again.
#[test]
fn a_classify_racing_a_growing_journal_refuses_rather_than_writing() {
    let dir = tempfile::tempdir().unwrap();
    let (_db, mut store, cited, before) = held_fixture(dir.path(), "raced.db");
    let refused = operator_classify_racing(
        &mut store,
        &cited,
        "held",
        &superseded("later", None),
        |store: &mut Store| {
            store
                .append_next(
                    "held",
                    EventType::OperatorRejected,
                    json!({"command_id": "peer", "operator": "peer", "reason": "mine first"}),
                    None,
                    None,
                )
                .unwrap();
        },
    )
    .unwrap_err();
    match refused {
        EngineError::Store(StoreError::HeadMoved {
            expected_seq,
            found_seq,
        }) => assert_eq!(
            (expected_seq, found_seq),
            (5, 6),
            "the fence is the store's own refusal, with its two heads"
        ),
        other => panic!("the fence refused with another error: {other}"),
    }
    assert_eq!(
        store.load("held").unwrap().len(),
        before + 1,
        "the peer's event is there and the classify is not"
    );
}
