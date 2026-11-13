//! Decision 0029's tail: each place the engine writes on a fold, with a
//! peer appending between the fold and the write. The write refuses with
//! the drift named, the peer's events stand, and the journal still folds.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use brokkr_core::fold::Status;

use super::super::tests::{bundle, single_body};
use super::super::*;

thread_local! {
    /// The peer a test puts between the engine's fold and its next append.
    static PEER: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
}

/// Run the peer a test put here, once, at the instant decision 0029's
/// fence exists for: the fold is taken and the append not yet made.
pub(super) fn peer_between() {
    if let Some(peer) = PEER.with(|peer| peer.borrow_mut().take()) {
        peer();
    }
}

/// Put `peer` between the engine's next fold and its next append.
pub(in crate::engine) fn after_the_fold(peer: Box<dyn FnOnce()>) {
    PEER.with(|slot| *slot.borrow_mut() = Some(peer));
}

fn db(dir: &Path) -> PathBuf {
    dir.join("realm.db")
}

fn body() -> SeatBody {
    single_body(vec!["missing-driver".into()])
}

/// A run started, with `events` journaled after its `run/started` as a
/// process that has since gone left them. Returns the run's id.
fn journaled(dir: &Path, events: Vec<(EventType, Value)>) -> String {
    std::fs::create_dir_all(dir.join("work")).unwrap();
    let store = Store::open(&db(dir)).unwrap();
    let work = Some(dir.join("work"));
    let mut engine = Engine::start(store, bundle(dir, body()), "Feature: fence", work).unwrap();
    for (event_type, payload) in events {
        let attempt = payload["attempt_id"].as_str().map(str::to_string);
        let run_id = &engine.run_id;
        engine
            .store
            .append_next(run_id, event_type, payload, None, attempt)
            .unwrap();
    }
    engine.run_id.clone()
}

/// The engine a fresh process resumes the run with.
fn resumed(dir: &Path, run_id: &str) -> Engine {
    let store = Store::open(&db(dir)).unwrap();
    Engine::resume(store, bundle(dir, body()), run_id, Some(dir.join("work"))).unwrap()
}

/// The run's phase entered and its effect requested: `Cursor::ExecuteEffect`.
fn requested() -> Vec<(EventType, Value)> {
    vec![
        (EventType::PhaseEntered, json!({"phase": "work"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "effect-1", "seat": "work"}),
        ),
    ]
}

/// The effect's attempt opened: `Cursor::EffectInFlight`.
fn in_flight() -> Vec<(EventType, Value)> {
    let mut events = requested();
    let started = json!({"effect_id": "effect-1", "attempt_id": "attempt-1"});
    events.push((EventType::EffectStarted, started));
    events
}

/// The attempt's live driver, journaling one more checkpoint from its
/// own process.
fn live_driver(dir: &Path, run_id: &str) -> Box<dyn FnOnce()> {
    let (db, run_id) = (db(dir), run_id.to_string());
    Box::new(move || {
        let checkpoint = json!({"effect_id": "effect-1", "attempt_id": "attempt-1"});
        let attempt = Some("attempt-1".to_string());
        Store::open(&db)
            .unwrap()
            .append_next(
                &run_id,
                EventType::EffectCheckpointed,
                checkpoint,
                None,
                attempt,
            )
            .unwrap();
    })
}

/// An operator at another terminal, whose stop — the command and its
/// acceptance — lands.
fn operator_stop(dir: &Path, run_id: &str) -> Box<dyn FnOnce()> {
    let (db, run_id) = (db(dir), run_id.to_string());
    Box::new(move || {
        let mut store = Store::open(&db).unwrap();
        let stopped = stop_as(&mut store, &run_id, "peer", "halt").unwrap();
        assert!(matches!(stopped, FencedCommandOutcome::Accepted { .. }));
    })
}

/// The refusal, exactly, and the journal it left: the peer's last event
/// is the head, nothing the engine decided is behind it, and the run
/// still folds.
fn refused(engine: &Engine, error: EngineError, folded: u64, found: u64) -> Vec<EventEnvelope> {
    assert!(
        matches!(
            &error,
            EngineError::JournalMoved { run_id, expected_seq, found_seq }
                if *run_id == engine.run_id && *expected_seq == folded && *found_seq == found
        ),
        "{error:?}"
    );
    let events = engine.store.load(&engine.run_id).unwrap();
    assert_eq!(events.last().unwrap().seq, found);
    fold(&events).unwrap();
    events
}

/// `resume`'s fresh-process branch: the attempt the journal shows open
/// is still held by a live driver in another process, which checkpoints
/// after the resuming process folded. The resume does not close it
/// indeterminate over the live work.
#[test]
fn a_fresh_process_does_not_close_an_attempt_its_live_driver_moved() {
    let dir = tempfile::tempdir().unwrap();
    let run_id = journaled(dir.path(), in_flight());
    let mut engine = resumed(dir.path(), &run_id);

    after_the_fold(live_driver(dir.path(), &run_id));
    let error = engine.drive().unwrap_err();

    let events = refused(&engine, error, 4, 5);
    let checkpointed = EventType::EffectCheckpointed;
    assert_eq!(events.last().unwrap().event_type, checkpointed);
}

/// A journal whose in-flight attempt was concluded beneath its driver,
/// and the fresh engine a resume builds over it: the shape (A) fixture
/// both late-append tests meet first.
fn concluded_beneath_driver(dir: &Path) -> (String, Engine) {
    let run_id = journaled(dir, in_flight());
    let mut store = Store::open(&db(dir)).unwrap();
    conclude_as(&mut store, &run_id, "operator", "believed dead").unwrap();
    let engine = resumed(dir, &run_id);
    (run_id, engine)
}

/// Shape (A) of #440: the run is concluded while its attempt is open,
/// and the attempt's settlement then meets the store's terminal gate.
/// Nothing is written, the driver ends on a typed error naming the
/// conclusion, and the run still folds as stopped.
#[test]
fn a_late_settlement_over_a_conclusion_ends_the_driver_and_the_run_still_folds() {
    let dir = tempfile::tempdir().unwrap();
    let (run_id, mut engine) = concluded_beneath_driver(dir.path());
    let settlement = json!({"effect_id": "effect-1", "result": {"result": "complete"}});
    let error = engine
        .append_raw(
            EventType::EffectSucceeded,
            settlement,
            Some("attempt-1".into()),
        )
        .unwrap_err();
    assert!(
        matches!(
            &error,
            EngineError::ConcludedBeneath {
                run_id: named,
                status: Status::Stopped,
                event_type: EventType::EffectSucceeded,
                attempt_id: Some(attempt),
            } if named == &run_id && attempt == "attempt-1"
        ),
        "{error:?}"
    );
    assert_eq!(
        error.to_string(),
        format!(
            "run '{run_id}' was concluded (stopped) while attempt 'attempt-1' was still working, \
             so its effect/succeeded could not be appended and nothing was written; the driver \
             stops here, and the conclusion already closed the attempt as effect/indeterminate — \
             the report this driver was carrying is lost to the journal"
        )
    );
    let events = engine.store.load(&run_id).unwrap();
    assert_eq!(events.last().unwrap().event_type, EventType::RunStopped);
    assert!(events.iter().any(|event| {
        event.event_type == EventType::EffectIndeterminate
            && event.attempt_id.as_deref() == Some("attempt-1")
    }));
    assert_eq!(fold(&events).unwrap().run, Run::Stopped);
}

/// The same ending for an event that names no attempt: the driver is
/// named as itself, and the typed value still says no attempt's report is
/// lost, so a late non-attempt append cannot be mistaken for a settlement.
#[test]
fn a_late_event_that_names_no_attempt_ends_concluded_beneath() {
    let dir = tempfile::tempdir().unwrap();
    let (run_id, mut engine) = concluded_beneath_driver(dir.path());
    let error = engine
        .append_raw(EventType::RunParked, json!({}), None)
        .unwrap_err();
    assert!(
        matches!(
            &error,
            EngineError::ConcludedBeneath {
                run_id: named,
                status: Status::Stopped,
                event_type: EventType::RunParked,
                attempt_id: None,
            } if named == &run_id
        ),
        "{error:?}"
    );
    assert!(
        error
            .to_string()
            .contains("while this driver was still working"),
        "{error}"
    );
    let events = engine.store.load(&run_id).unwrap();
    assert_eq!(events.last().unwrap().event_type, EventType::RunStopped);
}

/// A step the engine decides between effects: an operator's stop is
/// accepted after the engine folded `Cursor::Start` and before it enters
/// the phase. Unfenced, `phase/entered` landed at `Cursor::Stop` and the
/// journal no longer folded; fenced, it refuses, and the stop concludes
/// the run on the next drive.
#[test]
fn a_step_between_effects_does_not_land_over_an_operator_stop() {
    let dir = tempfile::tempdir().unwrap();
    let run_id = journaled(dir.path(), Vec::new());
    let mut engine = resumed(dir.path(), &run_id);

    after_the_fold(operator_stop(dir.path(), &run_id));
    let error = engine.drive().unwrap_err();

    let events = refused(&engine, error, 1, 3);
    let accepted = EventType::OperatorAccepted;
    assert_eq!(events.last().unwrap().event_type, accepted);
    let end = engine.drive().unwrap();
    assert_eq!(end.state.run, Run::Stopped);
}

/// The lawful end decides on a fold of its own (#394): it settles the
/// open attempt, and parks where the fold admits a park. A peer that
/// appends after that fold takes either write's head away.
#[test]
fn the_lawful_end_does_not_settle_or_park_over_a_peer() {
    type Peer = fn(&Path, &str) -> Box<dyn FnOnce()>;
    let cases: [(_, Peer, _, _); 2] = [
        (in_flight(), live_driver, 4, 5),
        (requested(), operator_stop, 3, 5),
    ];
    for (events, peer, folded, found) in cases {
        let dir = tempfile::tempdir().unwrap();
        let run_id = journaled(dir.path(), events);
        let mut engine = resumed(dir.path(), &run_id);

        after_the_fold(peer(dir.path(), &run_id));
        let contended = EngineError::Store(StoreError::Contended {
            operation: "append",
            waited_ms: 30_000,
        });
        let error = engine.lawful_end_under_contention(contended).unwrap_err();
        refused(&engine, error, folded, found);
    }
}

/// The refusal's text, pinned once: what the operator reads.
#[test]
fn the_refusal_names_the_drift_and_says_to_look_first() {
    let refusal = EngineError::JournalMoved {
        run_id: "r1".into(),
        expected_seq: 4,
        found_seq: 5,
    };
    assert_eq!(
        refusal.to_string(),
        "run 'r1': the journal moved beneath the fold this engine decided on (seq 4, now 5), \
         so something else may be driving the run; nothing was written — look with \
         `brokkr runs` before resuming (decision 0029)"
    );
}

/// Only the store's terminal refusal becomes the driver's typed ending: any
/// other store error stays a storage error, so a contended or moved head is
/// never reported as a conclusion beneath the driver (#440).
#[test]
fn a_store_error_other_than_the_terminal_refusal_stays_a_store_error() {
    let error = StoreError::HeadMoved {
        expected_seq: 3,
        found_seq: 4,
    };
    assert!(matches!(
        EngineError::concluded_beneath(error, Some("attempt")),
        EngineError::Store(StoreError::HeadMoved { .. })
    ));
}
