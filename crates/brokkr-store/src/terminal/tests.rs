//! The store's terminal gate (#440): every non-annotation is refused
//! after a run's terminal event, every annotation still lands, and the
//! refusal writes nothing at all.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{lawful_after_terminal, Status};
use brokkr_core::summary::Account;
use serde_json::json;
use time::OffsetDateTime;

use crate::test_support::{append, lags};
use crate::{CooldownAppend, Cooling, Store, StoreError};

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    (dir, store)
}

/// A run that opened and then reached `terminal`, written exactly as a
/// writer that was handed no fold leaves it: the summary row is behind.
fn terminal_run(store: &mut Store, run_id: &str, terminal: EventType) {
    store.create_run(run_id, "f", "test", &json!({})).unwrap();
    append(
        store,
        run_id,
        EventType::RunStarted,
        json!({"feature": "f", "manifest": {}}),
    );
    append(store, run_id, terminal, json!({}));
}

fn events(store: &Store, run_id: &str) -> i64 {
    store
        .conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE run_id = ?1",
            [run_id],
            |row| row.get(0),
        )
        .unwrap()
}

fn cooldowns(store: &Store, run_id: &str) -> i64 {
    store
        .conn
        .query_row(
            "SELECT COUNT(*) FROM provider_cooldowns WHERE run_id = ?1",
            [run_id],
            |row| row.get(0),
        )
        .unwrap()
}

/// Decision 0071 ruling 9: the table is derived from the relation's own
/// answer for every type, so a new annotation or a changed column moves
/// the expectation with the code. Every non-annotation is refused after
/// each terminal status, and both annotations still land.
#[test]
fn every_non_annotation_is_refused_after_each_terminal_status() {
    for terminal in [EventType::RunCompleted, EventType::RunStopped] {
        let (_dir, mut store) = store();
        let run_id = format!("r-{}", terminal.as_str());
        terminal_run(&mut store, &run_id, terminal);
        let expected = if terminal == EventType::RunCompleted {
            Status::Completed
        } else {
            Status::Stopped
        };
        for event_type in EventType::ALL {
            // The core test `lawful_after_terminal_is_the_relations_terminal_column`
            // holds this pair to `ROWS`; here it holds the store's gate to the
            // pair, so a predicate that admits every type fails this test.
            let annotation = matches!(
                event_type,
                EventType::OperatorCommanded | EventType::OperatorRejected
            );
            assert_eq!(
                lawful_after_terminal(event_type),
                annotation,
                "{event_type:?}"
            );
            let before = store.head_hash(&run_id).unwrap();
            let appended = store.append_next(&run_id, event_type, json!({}), None, None);
            if annotation {
                let event = appended.unwrap();
                assert_eq!(event.event_type, event_type);
                assert_ne!(store.head_hash(&run_id).unwrap(), before);
            } else {
                let error = appended.unwrap_err();
                let refused = StoreError::AfterTerminal {
                    run_id: run_id.clone(),
                    status: expected.as_str(),
                    event_type: event_type.as_str(),
                };
                assert_eq!(error.to_string(), refused.to_string());
                assert!(
                    matches!(
                        error,
                        StoreError::AfterTerminal { run_id: named, status, event_type: kind }
                            if named == run_id
                                && status == expected.as_str()
                                && kind == event_type.as_str()
                    ),
                    "{event_type:?}"
                );
                assert_eq!(store.head_hash(&run_id).unwrap(), before, "{event_type:?}");
            }
        }
    }
}

/// The refusal is whole: the event, the cool-down rows it carried and the
/// head are exactly as they were. Only the run's terminal row is counted.
#[test]
fn a_refused_append_writes_no_event_cooldown_or_projection() {
    let (_dir, mut store) = store();
    terminal_run(&mut store, "r", EventType::RunStopped);
    let (before_events, before_cooldowns) = (events(&store, "r"), cooldowns(&store, "r"));
    let before_head = store.head_hash("r").unwrap();
    let cooling = Cooling {
        site: String::new(),
        provider: "p".into(),
        route: "r".into(),
        until_at: OffsetDateTime::UNIX_EPOCH + time::Duration::hours(1),
        reason: "rate".into(),
        retry_after_seconds: Some(1),
        rounds: 1,
    };
    let refused = store.append_with_cooldowns(
        CooldownAppend {
            run_id: "r",
            fenced: None,
            event_type: EventType::EffectStarted,
            payload: json!({"effect_id": "e", "attempt_id": "a"}),
            causation_id: None,
            attempt_id: None,
        },
        &[cooling],
    );
    let error = refused.unwrap_err();
    assert_eq!(
        error.to_string(),
        "run 'r' is already concluded (stopped); effect/started is not an operator annotation, \
         and nothing was written"
    );
    assert_eq!(events(&store, "r"), before_events);
    assert_eq!(cooldowns(&store, "r"), before_cooldowns);
    assert_eq!(store.head_hash("r").unwrap(), before_head);
}

fn non_annotation(store: &mut Store) -> StoreError {
    store
        .append_next("r", EventType::EffectStarted, json!({}), None, None)
        .unwrap_err()
}

/// The summary is a projection, so it is not believed when it is behind or
/// gone: the walk finds the terminal event in the journal itself, and a
/// non-annotation is still refused.
#[test]
fn a_terminal_run_is_found_when_the_summary_is_behind_or_absent() {
    let (_dir, mut store) = store();
    terminal_run(&mut store, "r", EventType::RunStopped);
    assert!(matches!(
        non_annotation(&mut store),
        StoreError::AfterTerminal { .. }
    ));
    store
        .conn
        .execute("DELETE FROM run_summary WHERE run_id = 'r'", [])
        .unwrap();
    assert!(matches!(
        non_annotation(&mut store),
        StoreError::AfterTerminal { .. }
    ));
    append(
        &mut store,
        "r",
        EventType::OperatorCommanded,
        json!({"command_id": "c", "command": "classify"}),
    );
    // The annotation is now the head with no summary: the walk passes it
    // and still finds the terminal event beneath.
    assert!(matches!(
        non_annotation(&mut store),
        StoreError::AfterTerminal { .. }
    ));
    assert_eq!(super::walk(&store.conn, "r", 99).unwrap(), None);
    // The row was deliberately removed, so the drop check must not read
    // its absence as a writer's lag.
    lags(&mut store, &["r"]);
}

/// The summary's own fold answers when it stands at the head: both
/// branches of the bounded read are exercised, and the terminal word is
/// the one the refusal names.
#[test]
fn a_current_summary_answers_the_terminal_question() {
    let (_dir, mut store) = store();
    terminal_run(&mut store, "r", EventType::RunCompleted);
    let events = store.load("r").unwrap();
    store.carry(Account::whole("r", &events));
    append(
        &mut store,
        "r",
        EventType::OperatorCommanded,
        json!({"command_id": "c", "command": "classify"}),
    );
    let error = store
        .append_next("r", EventType::EffectStarted, json!({}), None, None)
        .unwrap_err();
    assert!(matches!(
        error,
        StoreError::AfterTerminal {
            status: "completed",
            ..
        }
    ));
}

/// The check is bounded: the terminal at the head is found in the same
/// instructions whether the run has ten events or ten thousand, so the
/// read is not a scan of the run's events.
#[test]
fn the_terminal_check_steps_the_same_on_ten_and_ten_thousand_events() {
    fn steps(total: usize) -> i64 {
        let (_dir, mut store) = store();
        let run_id = format!("r{total}");
        store.create_run(&run_id, "f", "test", &json!({})).unwrap();
        append(
            &mut store,
            &run_id,
            EventType::RunStarted,
            json!({"feature": "f", "manifest": {}}),
        );
        for _ in 0..total - 2 {
            append(
                &mut store,
                &run_id,
                EventType::PhaseEntered,
                json!({"phase": "work"}),
            );
        }
        append(&mut store, &run_id, EventType::RunStopped, json!({}));
        crate::terminal::take_steps();
        let found = super::terminal_status(&store.conn, &run_id, total as u64).unwrap();
        assert_eq!(found, Some(Status::Stopped));
        crate::terminal::take_steps()
    }
    let short = steps(10);
    let long = steps(10_000);
    println!("the terminal check takes {short} VM steps at 10 events and {long} at 10,000");
    assert_eq!(short, long, "the check scales with the run");
    assert!(short <= 100, "the check is bounded: {short} steps");
}
