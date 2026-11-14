//! Decision 0108 ruling 4: `brokkr operator repair` names the late tail of
//! a run that no longer folds, and the fold reads it as late — kept,
//! shown, never applied.

use super::*;
use crate::engine::tests::{in_flight_store, published_args};

use crate::engine::apply_fenced_operator_command;
use brokkr_core::fold::{fold, Run, Status};
use brokkr_store::test_support::{lags, plant_chained};
use brokkr_store::StoreError;
use serde_json::{json, Value};
use std::path::Path;

/// A run stopped at seq 5 by its operator, and nothing after it.
fn concluded(path: &Path, run: &str) -> Store {
    let mut store = in_flight_store(path, run);
    store
        .append_next(
            run,
            EventType::RunStopped,
            json!({"reason": "concluded"}),
            None,
            None,
        )
        .unwrap();
    store
}

/// `store` with `events` planted behind it, chained onto its head and
/// sealed as the store seals one: the shapes #440's two cases left.
fn planted(store: Store, path: &Path, run: &str, events: &[(EventType, Value)]) -> Store {
    let head = store.load(run).unwrap().pop().unwrap();
    drop(store);
    let connection = rusqlite::Connection::open(path).unwrap();
    plant_chained(&connection, run, &head, events).unwrap();
    drop(connection);
    Store::open(path).unwrap()
}

/// The repair verb's ask.
fn ask<'a>(operator: &'a str, reason: &'a str) -> Repair<'a> {
    Repair { operator, reason }
}

/// The events a journal holds, for the before-and-after of a refusal.
fn length(store: &Store, run: &str) -> usize {
    store.load(run).unwrap().len()
}

/// The operator's repair of shape (A): the settlement is named, the run
/// folds again as stopped, and the tail is kept and never applied.
#[test]
fn a_repair_names_a_late_settlement_and_the_run_folds_as_stopped() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("repair.db");
    let store = concluded(&db, "late");
    let mut store = planted(store, &db, "late", &[crate::late::settlement()]);
    assert_eq!(
        fold(&store.load("late").unwrap()).unwrap_err(),
        brokkr_core::FoldError::AfterTerminal { seq: 6 },
        "the shape does not fold before the repair"
    );

    let written =
        operator_repair(&mut store, "late", &ask("repairer", "the tail is evidence")).unwrap();
    let mut payload = written.payload.clone();
    let id = payload["command_id"].as_str().unwrap().to_string();
    assert_eq!(written.event_type, EventType::OperatorCommanded);
    assert_eq!(written.seq, 7);
    payload["command_id"] = json!("the id");
    assert_eq!(
        payload,
        json!({
            "command_id": "the id",
            "command": "repair",
            "operator": "repairer",
            "args": {
                "first_seq": 6,
                "last_seq": 6,
                "terminal_seq": 5,
                "event_types": ["effect/succeeded"],
                "reason": "the tail is evidence",
            },
        }),
        "the record names exactly the late tail"
    );
    assert!(!id.is_empty());

    // The args are the published contract, not this test's idea of it.
    published_args(
        "contracts/operator-repair.v1.schema.json",
        &written.payload["args"],
        &json!({"first_seq": 6, "last_seq": 6, "terminal_seq": 5}),
    );
    published_args(
        "contracts/operator-repair.v1.schema.json",
        &written.payload["args"],
        &json!({"first_seq": 6, "last_seq": 6, "terminal_seq": 5,
                "event_types": ["effect/succeeded"], "extra": true}),
    );

    let events = store.load("late").unwrap();
    let state = fold(&events).unwrap();
    assert_eq!(state.run, Run::Stopped);
    let tail = state.late_tail.expect("the fold honours the repair");
    assert_eq!((tail.first_seq, tail.last_seq), (6, 6));
    assert_eq!(tail.event_types, [EventType::EffectSucceeded]);
    assert_eq!(tail.operator, "repairer");
    assert!(
        !events
            .iter()
            .any(|event| event.event_type == EventType::OperatorAccepted && event.seq > 6),
        "no acceptance follows a repair"
    );
    lags(&mut store, &["late"]);
}

/// Shape (B): the same steps over the stop command and acceptance 0.12.0
/// left after the conclusion.
#[test]
fn a_repair_names_a_late_command_and_acceptance() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("repair-b.db");
    let store = concluded(&db, "late-b");
    let mut store = planted(store, &db, "late-b", &crate::late::late_stop());
    let written =
        operator_repair(&mut store, "late-b", &ask("repairer", "0.12.0 left it")).unwrap();
    assert_eq!(written.seq, 8);
    assert_eq!(
        written.payload["args"],
        json!({
            "first_seq": 6,
            "last_seq": 7,
            "terminal_seq": 5,
            "event_types": ["operator/commanded", "operator/accepted"],
            "reason": "0.12.0 left it",
        })
    );
    let state = fold(&store.load("late-b").unwrap()).unwrap();
    assert_eq!(state.run, Run::Stopped);
    assert_eq!(state.late_tail.unwrap().event_types.len(), 2);
    lags(&mut store, &["late-b"]);
}

/// A run with no terminal event, a terminal run with no tail, and one
/// already repaired with nothing after the repair: each refused in words
/// naming the run, and nothing written.
#[test]
fn a_repair_is_refused_and_writes_nothing_where_it_cannot_help() {
    let dir = tempfile::tempdir().unwrap();

    let live_db = dir.path().join("live.db");
    let mut live = in_flight_store(&live_db, "live");
    let before = length(&live, "live");
    let refused = operator_repair(&mut live, "live", &ask("operator", "why")).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "repair refused, nothing was written: run 'live' has no terminal event; repair names the \
         late tail of a run that has already ended, and it is never automatic"
    );
    assert_eq!(length(&live, "live"), before);
    lags(&mut live, &["live"]);

    let plain_db = dir.path().join("plain.db");
    let mut plain = concluded(&plain_db, "plain");
    let before = length(&plain, "plain");
    let refused = operator_repair(&mut plain, "plain", &ask("operator", "why")).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "repair refused, nothing was written: run 'plain' is already concluded (stopped), with no \
         late tail; it folds as it stands and repair appends nothing"
    );
    assert_eq!(length(&plain, "plain"), before);
    lags(&mut plain, &["plain"]);

    let repaired_db = dir.path().join("repaired.db");
    let store = concluded(&repaired_db, "repaired");
    let mut repaired = planted(
        store,
        &repaired_db,
        "repaired",
        &[crate::late::settlement()],
    );
    operator_repair(&mut repaired, "repaired", &ask("operator", "first")).unwrap();
    let before = length(&repaired, "repaired");
    let refused =
        operator_repair(&mut repaired, "repaired", &ask("operator", "again")).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "repair refused, nothing was written: run 'repaired' is already repaired at seq 7; \
         it folds as it stands and repair appends nothing"
    );
    assert_eq!(length(&repaired, "repaired"), before);
    lags(&mut repaired, &["repaired"]);
}

/// A terminal run whose tail is only lawful annotations folds already:
/// repair refuses it, and writes nothing.
#[test]
fn a_repair_refuses_a_terminal_run_whose_tail_is_all_annotations() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("annotated.db");
    let store = concluded(&db, "annotated");
    let mut store = planted(
        store,
        &db,
        "annotated",
        &[(
            EventType::OperatorCommanded,
            json!({"command_id": "c-classify", "command": "classify",
                   "operator": "operator", "args": {"cause": "withdrawn"}}),
        )],
    );
    let before = length(&store, "annotated");
    let refused = operator_repair(&mut store, "annotated", &ask("operator", "why")).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "repair refused, nothing was written: run 'annotated' has no late tail after its terminal \
         event at seq 5; it folds as it stands and repair appends nothing"
    );
    assert_eq!(length(&store, "annotated"), before);
    lags(&mut store, &["annotated"]);
}

/// A multi-event tail, repaired as one range: a settlement a driver left
/// and the stop command an older binary journaled after it. Both are late,
/// and neither is applied.
#[test]
fn a_repair_names_a_multi_event_late_tail_as_one_range() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("multi.db");
    let store = concluded(&db, "multi");
    let mut store = planted(
        store,
        &db,
        "multi",
        &[
            crate::late::settlement(),
            (
                EventType::OperatorCommanded,
                json!({"command_id": "c-stop", "command": "stop", "operator": "operator", "args": {}}),
            ),
        ],
    );
    let written =
        operator_repair(&mut store, "multi", &ask("operator", "kept as evidence")).unwrap();
    assert_eq!(written.seq, 8);
    assert_eq!(
        written.payload["args"],
        json!({
            "first_seq": 6,
            "last_seq": 7,
            "terminal_seq": 5,
            "event_types": ["effect/succeeded", "operator/commanded"],
            "reason": "kept as evidence",
        })
    );
    let state = fold(&store.load("multi").unwrap()).unwrap();
    assert_eq!(state.run, Run::Stopped);
    assert_eq!(state.late_tail.unwrap().event_types.len(), 2);
    lags(&mut store, &["multi"]);
}

/// A late command and its rejection are both annotations: the fold already
/// admits them, so the run folds and `repair` refuses it, writing nothing.
/// The literal pair the framing names is this case, not a broken tail.
#[test]
fn a_late_command_and_rejection_fold_already_and_repair_refuses_them() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("rejected.db");
    let store = concluded(&db, "rejected");
    let mut store = planted(
        store,
        &db,
        "rejected",
        &[
            (
                EventType::OperatorCommanded,
                json!({"command_id": "c-stop", "command": "stop", "operator": "operator", "args": {}}),
            ),
            (
                EventType::OperatorRejected,
                json!({"command_id": "c-stop", "operator": "operator", "reason": "after_terminal"}),
            ),
        ],
    );
    assert!(fold(&store.load("rejected").unwrap()).is_ok());
    let before = length(&store, "rejected");
    let refused = operator_repair(&mut store, "rejected", &ask("operator", "why")).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "repair refused, nothing was written: run 'rejected' has no late tail after its terminal \
         event at seq 5; it folds as it stands and repair appends nothing"
    );
    assert_eq!(length(&store, "rejected"), before);
    lags(&mut store, &["rejected"]);
}

/// The late settlement followed by a repair that names the wrong range:
/// the journal the fold refuses by name.
fn wrong_range() -> Vec<(EventType, Value)> {
    vec![
        crate::late::settlement(),
        (
            EventType::OperatorCommanded,
            json!({"command_id": "c-repair", "command": "repair", "operator": "operator",
                   "args": {"first_seq": 7, "last_seq": 6, "terminal_seq": 5,
                            "event_types": ["effect/succeeded"], "reason": "wrong"}}),
        ),
    ]
}

/// A repair planted by hand that names another range makes the fold
/// refuse, in words naming the repair.
#[test]
fn a_planted_repair_that_names_another_range_is_refused_by_the_fold() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("wrong.db");
    let store = concluded(&db, "wrong");
    let mut store = planted(store, &db, "wrong", &wrong_range());
    assert_eq!(
        fold(&store.load("wrong").unwrap()).unwrap_err(),
        brokkr_core::FoldError::BadRepair {
            seq: 7,
            detail: "it names seqs 7..=6, but the late tail is seqs 6..=6".to_string()
        }
    );
    lags(&mut store, &["wrong"]);
}

/// A repair the fold would refuse for another fault is not added to: the
/// verb reads the run's own refusal and writes nothing.
#[test]
fn a_repair_refuses_a_journal_that_faults_otherwise() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("faulted.db");
    let store = concluded(&db, "faulted");
    let mut store = planted(store, &db, "faulted", &wrong_range());
    let before = length(&store, "faulted");
    let refused = operator_repair(&mut store, "faulted", &ask("operator", "why")).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "repair refused, nothing was written: run 'faulted' does not fold for another reason: \
         event 7: operator repair does not name the late tail: it names seqs 7..=6, but the late \
         tail is seqs 6..=6; repair names a late tail, never another fault"
    );
    assert_eq!(length(&store, "faulted"), before);
    lags(&mut store, &["faulted"]);
}

/// The store's terminal gate still admits the annotation, and the target
/// status the readouts name is the terminal one.
#[test]
fn a_repair_leaves_the_terminal_status_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("status.db");
    let store = concluded(&db, "status");
    let mut store = planted(store, &db, "status", &[crate::late::settlement()]);
    let before = fold(&store.load("status").unwrap()).unwrap_err();
    assert!(matches!(
        before,
        brokkr_core::FoldError::AfterTerminal { .. }
    ));
    operator_repair(&mut store, "status", &ask("operator", "kept")).unwrap();
    let after = fold(&store.load("status").unwrap()).unwrap();
    assert_eq!(after.run.status(), Status::Stopped);
    lags(&mut store, &["status"]);
}

/// F4: a repaired run with a later lawful annotation still carries the
/// fold's late tail, so repair refuses it as repaired rather than as a
/// run with no late tail.
#[test]
fn a_repair_after_a_later_annotation_is_still_already_repaired() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("annotated-after-repair.db");
    let store = concluded(&db, "late");
    let mut store = planted(store, &db, "late", &[crate::late::settlement()]);
    operator_repair(&mut store, "late", &ask("operator", "first")).unwrap();
    store
        .append_next(
            "late",
            EventType::OperatorCommanded,
            json!({"command_id": "c-classify", "command": "classify",
                   "operator": "operator", "args": {"cause": "withdrawn", "reason": "later"}}),
            None,
            None,
        )
        .unwrap();
    let before = length(&store, "late");
    let refused = operator_repair(&mut store, "late", &ask("operator", "again")).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "repair refused, nothing was written: run 'late' is already repaired at seq 7; it folds \
         as it stands and repair appends nothing"
    );
    assert_eq!(length(&store, "late"), before);
    lags(&mut store, &["late"]);
}

/// F9: the fenced append's error arm. A peer moves the head between the
/// read and the write, so the repair lands nowhere and the store's
/// `HeadMoved` is the answer.
#[test]
fn a_repair_that_loses_the_head_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("moved.db");
    let store = concluded(&db, "late");
    let mut store = planted(store, &db, "late", &[crate::late::settlement()]);
    let before = length(&store, "late");
    let refused = repair_with(
        &mut store,
        "late",
        &ask("operator", "why"),
        |store| {
            store
                .append_next(
                    "late",
                    EventType::OperatorCommanded,
                    json!({"command_id": "peer", "command": "stop", "operator": "other", "args": {}}),
                    None,
                    None,
                )
                .unwrap();
        },
        |_| {},
    )
    .unwrap_err();
    assert!(
        matches!(refused, EngineError::Store(StoreError::HeadMoved { .. })),
        "{refused:?}"
    );
    let events = store.load("late").unwrap();
    assert_eq!(events.len(), before + 1, "only the peer's event landed");
    assert!(
        !events
            .iter()
            .any(|event| event.payload.get("command").and_then(Value::as_str) == Some(REPAIR)),
        "the repair wrote nothing"
    );
    lags(&mut store, &["late"]);
}

/// INFO a: the read-back's fold arm. A peer's annotation lands between
/// the repair and the read, and makes the journal refuse; the verb
/// reports the fold's error, not a success over a journal that does not
/// fold.
#[test]
fn a_repair_whose_read_back_refuses_reports_the_fold_error() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("read-back.db");
    let store = concluded(&db, "late");
    let mut store = planted(store, &db, "late", &[crate::late::settlement()]);
    let refused = repair_with(
        &mut store,
        "late",
        &ask("operator", "why"),
        |_| {},
        |store| {
            store
                .append_next(
                    "late",
                    EventType::OperatorCommanded,
                    json!({"command_id": "peer-repair", "command": "repair",
                           "operator": "other",
                           "args": {"first_seq": 6, "last_seq": 6, "terminal_seq": 5,
                                    "event_types": ["effect/failed"], "reason": "bad"}}),
                    None,
                    None,
                )
                .unwrap();
        },
    )
    .unwrap_err();
    assert!(
        matches!(
            &refused,
            EngineError::Fold(brokkr_core::fold::FoldError::BadRepair { seq: 8, .. })
        ),
        "{refused:?}"
    );
}

/// C1(a), S1: the bridge refuses every annotation word before journaling
/// it, ANSWERING it as a refused command with a rejected receipt, and the
/// terminal run still folds. The `dance` control still journals the word
/// it was sent and refuses it by name, exactly as before.
#[test]
fn the_bridge_refuses_annotation_words_and_the_run_still_folds() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("reserved.db");
    let mut store = concluded(&db, "late");
    let before = length(&store, "late");
    for word in ["repair", "settle", "classify", "supersede"] {
        let (seq, hash) = store.head_hash("late").unwrap();
        let wire = crate::engine::FencedCommand {
            command_id: "reserved",
            command: crate::engine::CommandWord::parse(word),
            operator: "operator",
            reason: "please",
            expected_seq: seq,
            expected_hash: &hash,
        };
        let refused = apply_fenced_operator_command(&mut store, "late", &wire).unwrap();
        assert!(
            matches!(&refused, crate::engine::FencedCommandOutcome::Rejected { reason, .. }
                if reason == crate::engine::operator::RESERVED_COMMAND_REASON),
            "{word}: {refused:?}"
        );
        assert_eq!(length(&store, "late"), before, "{word} wrote nothing");
        assert_eq!(
            fold(&store.load("late").unwrap()).unwrap().run,
            Run::Stopped,
            "{word} left the run foldable"
        );
    }

    let (seq, hash) = store.head_hash("late").unwrap();
    let dance = crate::engine::FencedCommand {
        command_id: "control",
        command: crate::engine::CommandWord::parse("dance"),
        operator: "operator",
        reason: "please",
        expected_seq: seq,
        expected_hash: &hash,
    };
    let refused = apply_fenced_operator_command(&mut store, "late", &dance).unwrap();
    assert!(
        matches!(&refused, crate::engine::FencedCommandOutcome::Rejected { reason, .. }
            if reason == "command_not_allowed"),
        "{refused:?}"
    );
    assert_eq!(
        fold(&store.load("late").unwrap()).unwrap().run,
        Run::Stopped,
        "the control word is refused and the run still folds"
    );
}

/// C1(b): a `repair` word journaled before the terminal event is the
/// pending command it is, never a repair record; the run folds as stopped
/// and the store's summary row says the same status.
#[test]
fn a_pre_terminal_repair_folds_as_stopped_and_the_summary_agrees() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("pre-terminal.db");
    let mut store = in_flight_store(&db, "live");
    store
        .append_next(
            "live",
            EventType::OperatorCommanded,
            json!({"command_id": "remote", "command": "repair", "operator": "looper",
                   "args": {}}),
            None,
            None,
        )
        .unwrap();
    store
        .append_next(
            "live",
            EventType::OperatorRejected,
            json!({"command_id": "remote", "operator": "looper",
                   "reason": "command_not_allowed"}),
            None,
            None,
        )
        .unwrap();
    store
        .append_next(
            "live",
            EventType::RunStopped,
            json!({"reason": "concluded"}),
            None,
            None,
        )
        .unwrap();
    drop(store);
    let store = Store::open(&db).unwrap();
    let state = fold(&store.load("live").unwrap()).unwrap();
    assert_eq!(state.run, Run::Stopped);
    assert!(
        state.late_tail.is_none(),
        "the pre-terminal word named no late tail"
    );

    let rows = store
        .section(
            &[brokkr_core::page::Range {
                lane: brokkr_core::page::Lane::Finished,
                bound: None,
            }],
            None,
            10,
        )
        .unwrap();
    let row = rows
        .into_iter()
        .find_map(|row| row.ok().filter(|row| row.run_id == "live"))
        .expect("the run's summary row");
    assert_eq!(
        row.status,
        Some(Status::Stopped),
        "the summary row and the fold state the same status"
    );
}
