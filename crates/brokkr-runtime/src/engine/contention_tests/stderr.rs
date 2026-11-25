use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use super::*;
use crate::engine::tests::{draft_then_finish, driver_stderr::Watch, member};

const SAID: &str = "the draft driver finished\n";

#[test]
fn a_step_checkpoint_the_lock_outlasts_settles_indeterminate_and_names_and_tells_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let draft = signalling_command(&[format!("printf '%s' '{SAID}' >&2")], &[]);
    let finish = member("finish", vec!["missing-driver".into()]);
    let steps = draft_then_finish("role.md", draft, &["complete"], vec![finish]);
    let body = SeatBody::Sequence { steps };
    let mut engine = driving(dir.path(), body, Duration::from_millis(25));

    let db = dir.path().join("realm.db");
    let watch = Watch::on_db(db.clone(), &mut engine);
    let peer = Rc::new(RefCell::new(None));
    let locking_peer = Rc::clone(&peer);
    fence::tests::after_the_fold_on(
        EventType::EffectCheckpointed,
        Box::new(move || *locking_peer.borrow_mut() = Some(write_lock_on(&db))),
    );
    let mut reason = None;
    let end = engine
        .drive_racing(|error| {
            let Some(
                contended @ StoreError::Contended {
                    operation: "append",
                    ..
                },
            ) = error.contention()
            else {
                panic!("expected append contention, got {error:?}");
            };
            reason = Some(format!("journal contention: {contended}"));
            drop(
                peer.borrow_mut()
                    .take()
                    .expect("the peer held the write lock"),
            );
        })
        .unwrap();

    let reason = reason.expect("the checkpoint met contention");
    assert_eq!(end.state.run.status(), Status::AwaitingOperator);
    assert_eq!(
        end.state.run.park_reason().map(ParkReason::as_str),
        Some(reason.as_str())
    );
    let events = engine.store.load(&engine.run_id).unwrap();
    let types: Vec<_> = events.iter().map(|event| event.event_type).collect();
    assert_eq!(
        types,
        [
            EventType::RunStarted,
            EventType::PhaseEntered,
            EventType::EffectRequested,
            EventType::EffectStarted,
            EventType::EffectIndeterminate,
            EventType::RunParked,
        ]
    );
    let attempt = events[3].attempt_id.as_deref().unwrap();
    let settled = &events[4];
    assert_eq!(settled.attempt_id.as_deref(), Some(attempt));
    assert_eq!(events[5].payload["reason"], reason);

    let digest = brokkr_core::canonical::sha256_bytes(SAID.as_bytes());
    let evidence = format!(
        "{} bytes (first byte: letter; sha256: {})",
        SAID.len(),
        &digest[..16]
    );
    let (told, held) = watch.told_and_held();
    assert_eq!(
        (settled.payload["reason"].as_str().unwrap(), told.as_str()),
        (
            format!("{reason}; driver stderr: {evidence}").as_str(),
            format!("{attempt}: driver stderr: {SAID}").as_str(),
        )
    );
    assert_eq!(held, types);
    assert_eq!(engine.stderr.take_contended(), None);
}

fn keep_stderr_for(engine: &mut Engine, attempt: &str) {
    engine.stderr.active = Some(SAID.into());
    engine.stderr.keep_contended(Some(attempt.into()));
}

#[test]
fn kept_stderr_names_only_its_own_attempt_and_is_spent_by_the_lawful_end() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = in_flight(dir.path());
    // The drive holds the run. Seed a stale attempt ID to exercise the
    // lawful end with an ID that production cannot reach through a peer.
    keep_stderr_for(&mut engine, "attempt-0");
    let reason = format!("journal contention: {}", contention("append"));
    let end = engine
        .lawful_end_under_contention(contended("append"))
        .unwrap()
        .unwrap();
    assert_eq!(end.state.run.status(), Status::AwaitingOperator);
    let events = engine.store.load(&engine.run_id).unwrap();
    let settled = &events[events.len() - 2];
    assert_eq!(
        (
            settled.event_type,
            settled.attempt_id.as_deref(),
            settled.payload["reason"].as_str()
        ),
        (
            EventType::EffectIndeterminate,
            Some("attempt-1"),
            Some(reason.as_str())
        )
    );
    assert_eq!(engine.stderr.take_contended(), None);

    keep_stderr_for(&mut engine, "attempt-0");
    assert!(matches!(
        engine.lawful_end_under_contention(contended("append")),
        Err(EngineError::Store(StoreError::Contended {
            operation: "append",
            waited_ms: 30_000
        }))
    ));
    assert_eq!(engine.stderr.take_contended(), None);

    let held_dir = tempfile::tempdir().unwrap();
    let mut engine = in_flight(held_dir.path());
    keep_stderr_for(&mut engine, "attempt-1");
    engine.held_outcome = Some(held_success("attempt-1", json!({"result": "complete"})));
    assert!(engine
        .lawful_end_under_contention(contended("append"))
        .unwrap()
        .is_none());
    assert_eq!(engine.stderr.take_contended(), None);
}
