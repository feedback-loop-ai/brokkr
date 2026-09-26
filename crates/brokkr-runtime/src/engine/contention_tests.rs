//! An engine must never exit on Busy.
//!
//! The bug these fence: two `brokkr run` processes on one shared realm
//! journal, one mid-review and one freshly started, and the second one
//! gone 2.5 minutes in with `sqlite: database is locked` after
//! `run/started` and eighteen more events had already landed cleanly. It
//! reached the CLI's bare `?` as an anonymous `StoreError::Sqlite`,
//! `main` printed `error: {e:#}` and returned 1, and the run was
//! indistinguishable from a defect.
//!
//! Contention is not a defect and it is not a refusal. It is an accident
//! of timing on a lock, it writes nothing, and the store now says so in
//! a type. What is proved here is the other half: that the ENGINE turns
//! that type into an ending. Where `brokkr_core::fold` admits a
//! `run/parked`, the run parks with the lock it lost named in its own
//! journal; where the fold does not, the engine hands the typed
//! contention back rather than forging an event the fold would refuse.
//! Both endings leave a journal that folds.

use super::tests::{bundle, single_body};
use super::*;
use brokkr_store::StoreError;

/// A store and an engine sharing one journal file, the engine started
/// and its `run/started` landed — the state the production engine was in
/// when it met the lock.
fn started(dir: &Path) -> Engine {
    std::fs::create_dir_all(dir.join("work")).unwrap();
    let store = Store::open(&dir.join("realm.db")).unwrap();
    Engine::start(
        store,
        bundle(dir, single_body(vec!["missing-driver".into()])),
        "Feature: contention",
        Some(dir.join("work")),
    )
    .unwrap()
}

/// The contention the store hands up, built without having to lose a
/// lock race to get one: the mapping under test is the engine's, and it
/// reads the type, never a clock.
fn contended(operation: &'static str) -> EngineError {
    EngineError::Store(StoreError::Contended {
        operation,
        waited_ms: 30_000,
    })
}

/// A peer connection holding this journal's write lock until it is told
/// to let go.
fn write_lock_on(db: &Path) -> rusqlite::Connection {
    let holder = rusqlite::Connection::open(db).unwrap();
    holder
        .busy_timeout(std::time::Duration::from_secs(30))
        .unwrap();
    holder.execute_batch("BEGIN IMMEDIATE").unwrap();
    holder
        .execute(
            "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at)
             VALUES ('peer', 'feat', 'self', '{}', '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
    holder
}

/// The reproduction, end to end and deterministic: a real engine driving
/// a real run against a peer that holds the write lock and does not let
/// go. Before the fix this came back as `StoreError::Sqlite` carrying
/// `database is locked` — the shape that reached `main()` as an
/// anonymous exit 1. It is typed contention now, the engine's own
/// journal is untouched, and what it wrote before the lock still folds.
#[test]
fn an_engine_driving_into_a_held_lock_ends_typed_and_never_on_a_bare_sqlite_error() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = started(dir.path());
    let run_id = engine.run_id.clone();
    let db = dir.path().join("realm.db");
    engine
        .store
        .set_patience(std::time::Duration::from_millis(150))
        .unwrap();

    let holder = write_lock_on(&db);
    let ended = engine
        .drive()
        .expect_err("a held lock cannot be driven past");
    let EngineError::Store(store_error) = &ended else {
        panic!("contention arrived as something other than a store error: {ended:?}");
    };
    assert!(
        store_error.is_contention(),
        "the engine met the lock as an anonymous sqlite failure: {store_error:?}"
    );
    holder.execute_batch("ROLLBACK").unwrap();

    // Nothing of the failed turn landed, and the run folds.
    let events = engine.store.load(&run_id).unwrap();
    assert_eq!(events.len(), 1, "a contended turn wrote something anyway");
    let state = fold(&events).unwrap();
    assert_eq!(state.status, Status::Running);
}

/// Where the fold admits a park, contention is SAID. The journal carries
/// the reason in the run's own words, the drive ends as an ending rather
/// than an error, and the folded state is a park an operator can act on.
#[test]
fn contention_where_a_park_is_lawful_parks_with_the_lock_it_lost_named() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = started(dir.path());
    let run_id = engine.run_id.clone();
    // Walk the journal to `ExecuteEffect`, one of the two cursors
    // `fold` admits a `run/parked` at.
    engine
        .store
        .append_next(
            &run_id,
            EventType::PhaseEntered,
            json!({"phase": "work"}),
            None,
            None,
        )
        .unwrap();
    engine
        .store
        .append_next(
            &run_id,
            EventType::EffectRequested,
            json!({"effect_id": "effect-1", "seat": "work"}),
            None,
            None,
        )
        .unwrap();

    let end = engine
        .lawful_end_under_contention(contended("append"))
        .expect("a lawful park is an ending, not an error");
    assert_eq!(end.state.status, Status::AwaitingOperator);
    let reason = end.state.park_reason.clone().unwrap();
    assert!(
        reason.contains("journal contention") && reason.contains("nothing was written"),
        "the park does not name the contention: {reason}"
    );

    let events = engine.store.load(&run_id).unwrap();
    assert_eq!(events.last().unwrap().event_type, EventType::RunParked);
    fold(&events).expect("a journal that parked on contention still folds");
}

/// Where the fold does NOT admit a park, the engine says so by handing
/// the typed contention back — it does not forge a `run/parked` the fold
/// would refuse. An engine that ends on contention must leave a journal
/// that still folds, and at this cursor the only way to keep that
/// promise is to write nothing.
#[test]
fn contention_where_a_park_is_unlawful_returns_the_type_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = started(dir.path());
    let run_id = engine.run_id.clone();
    engine
        .store
        .append_next(
            &run_id,
            EventType::PhaseEntered,
            json!({"phase": "work"}),
            None,
            None,
        )
        .unwrap();
    let before = engine.store.load(&run_id).unwrap().len();

    let handed_back = engine
        .lawful_end_under_contention(contended("append"))
        .expect_err("a park the fold refuses is not an ending");
    assert!(
        matches!(&handed_back, EngineError::Store(e) if e.is_contention()),
        "the type was lost on the way back: {handed_back:?}"
    );
    assert_eq!(engine.store.load(&run_id).unwrap().len(), before);
    fold(&engine.store.load(&run_id).unwrap()).expect("an untouched journal folds");
}

/// A seat that hands over three checkpoints and then succeeds, echoing
/// whatever effect and attempt the engine started it on.
const THREE_CHECKPOINTS: &str = r#"
read -r hello
printf '%s\n' '{"proto":"forge-driver/v1","msg_id":"cap","type":"capabilities","driver":"test","version":"1","supports":[]}'
read -r start
effect_id=$(printf '%s' "$start" | sed -n 's/.*"effect_id":"\([^"]*\)".*/\1/p')
attempt_id=$(printf '%s' "$start" | sed -n 's/.*"attempt_id":"\([^"]*\)".*/\1/p')
printf '{"proto":"forge-driver/v1","msg_id":"accepted","type":"accepted","effect_id":"%s","attempt_id":"%s","session_ref":null}\n' "$effect_id" "$attempt_id"
for step in 1 2 3; do
  printf '{"proto":"forge-driver/v1","msg_id":"c%s","type":"checkpoint","effect_id":"%s","attempt_id":"%s","data":{"step":"%s"}}\n' "$step" "$effect_id" "$attempt_id" "$step"
done
printf '{"proto":"forge-driver/v1","msg_id":"result","type":"result","effect_id":"%s","attempt_id":"%s","status":"succeeded","result":{"result":"complete"},"error":null}\n' "$effect_id" "$attempt_id"
read -r done
"#;

/// #394, the reproduction: a peer takes the journal's write lock just as
/// the seat's first checkpoint tries it, and holds it past the whole
/// patience. Before the fix the engine ended there with the typed
/// contention, the seat died with it, and the next resume could only
/// call the attempt indeterminate. Now the rows wait in the attempt's
/// buffer: the seat runs to its result, every checkpoint lands once the
/// peer lets go — none before — and the attempt concludes as the seat
/// said, with no `effect/indeterminate` anywhere.
#[test]
fn a_lock_held_past_the_patience_while_a_seat_works_delays_its_checkpoints_and_never_the_attempt() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let db = dir.path().join("realm.db");
    let command = vec!["sh".into(), "-c".into(), THREE_CHECKPOINTS.into()];
    let mut engine = Engine::start(
        Store::open(&db).unwrap(),
        bundle(dir.path(), single_body(command)),
        "Feature: contention",
        Some(dir.path().join("work")),
    )
    .unwrap();
    let run_id = engine.run_id.clone();
    engine
        .store
        .set_patience(std::time::Duration::from_millis(150))
        .unwrap();

    // The fenced-race seam: the first try takes the peer's lock, the
    // next lets it go, after counting what the journal held meanwhile.
    let landed_while_held = std::sync::Arc::new(std::sync::Mutex::new(None));
    let seen = landed_while_held.clone();
    let mut holder: Option<rusqlite::Connection> = None;
    let mut tries = 0;
    engine.checkpoint_between = Box::new(move |_| {
        tries += 1;
        match tries {
            1 => holder = Some(write_lock_on(&db)),
            2 => {
                let peer = holder.take().expect("the lock was taken");
                let checkpoints: i64 = peer
                    .query_row(
                        "SELECT count(*) FROM events WHERE instr(envelope, 'effect/checkpointed') > 0",
                        [],
                        |row| row.get(0),
                    )
                    .unwrap();
                *seen.lock().unwrap() = Some(checkpoints);
                peer.execute_batch("ROLLBACK").unwrap();
            }
            _ => {}
        }
    });

    engine
        .drive()
        .expect("contention on a checkpoint is no ending");

    assert_eq!(
        *landed_while_held.lock().unwrap(),
        Some(0),
        "the seam never released the lock, or a row landed through it"
    );
    let events = engine.store.load(&run_id).unwrap();
    let steps: Vec<Value> = events
        .iter()
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .map(|event| event.payload["checkpoint"]["step"].clone())
        .collect();
    assert_eq!(steps, vec!["1", "2", "3"]);
    assert!(
        !events
            .iter()
            .any(|event| event.event_type == EventType::EffectIndeterminate),
        "the attempt was thrown away: {events:#?}"
    );
    let succeeded = events
        .iter()
        .find(|event| event.event_type == EventType::EffectSucceeded)
        .expect("the attempt concluded as the seat said");
    assert_eq!(succeeded.payload["result"]["result"], "complete");
    let last_checkpoint = events
        .iter()
        .rfind(|event| event.event_type == EventType::EffectCheckpointed)
        .unwrap();
    assert_eq!(
        succeeded.causation_id.as_deref(),
        Some(last_checkpoint.event_id.as_str()),
        "the conclusion names the last buffered row as its cause"
    );
    fold(&events).expect("the journal folds");
}

/// The attempt's conclusion is the one row that must land, and when a
/// peer's lock outlasts the patience for it too, the attempt is SETTLED
/// in the engine's own words — `effect/indeterminate` naming the lock —
/// and the run parks on it. What the next resume finds is a park an
/// operator can answer, never an attempt still in flight that it has to
/// call indeterminate on the engine's behalf.
#[test]
fn contention_on_an_attempt_in_flight_settles_it_and_parks_with_the_lock_named() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = started(dir.path());
    let run_id = engine.run_id.clone();
    for (event_type, payload) in [
        (EventType::PhaseEntered, json!({"phase": "work"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "effect-1", "seat": "work"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "effect-1", "attempt_id": "attempt-1"}),
        ),
    ] {
        engine
            .store
            .append_next(&run_id, event_type, payload, None, None)
            .unwrap();
    }

    let end = engine
        .lawful_end_under_contention(contended("append"))
        .expect("a settled attempt is an ending, not an error");
    assert_eq!(end.state.status, Status::AwaitingOperator);
    let lock = "journal contention: contended: a peer still held the journal's write lock \
                after 30000ms of append; nothing was written";
    assert_eq!(
        end.state.park_reason.as_deref(),
        Some(
            format!(
                "effect effect-1 indeterminate: {lock}; the attempt's seats had ended, \
                 and it is settled here rather than left in flight"
            )
            .as_str()
        )
    );

    let events = engine.store.load(&run_id).unwrap();
    let tail: Vec<EventType> = events[events.len() - 2..]
        .iter()
        .map(|event| event.event_type)
        .collect();
    assert_eq!(
        tail,
        vec![EventType::EffectIndeterminate, EventType::RunParked]
    );
    let settled = &events[events.len() - 2];
    assert_eq!(settled.payload["attempt_id"], "attempt-1");
    assert_eq!(settled.attempt_id.as_deref(), Some("attempt-1"));

    // The next resume finds the park, and writes nothing of its own.
    let again = engine.drive().unwrap();
    assert_eq!(again.state.status, Status::AwaitingOperator);
    assert_eq!(engine.store.load(&run_id).unwrap().len(), events.len());
}

/// Everything that is not contention leaves exactly as it arrived —
/// including the fenced-append refusal that lives next door. A
/// `HeadMoved` is a verdict about content: a peer legitimately moved the
/// head. Turning one into a park would be retrying a refusal into place
/// by another route, and it is not done here either.
#[test]
fn a_refusal_and_a_defect_both_pass_through_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = started(dir.path());
    let run_id = engine.run_id.clone();
    // At `ExecuteEffect`, where a park WOULD be lawful — so what stops
    // these is their type and nothing else.
    engine
        .store
        .append_next(
            &run_id,
            EventType::PhaseEntered,
            json!({"phase": "work"}),
            None,
            None,
        )
        .unwrap();
    engine
        .store
        .append_next(
            &run_id,
            EventType::EffectRequested,
            json!({"effect_id": "effect-1", "seat": "work"}),
            None,
            None,
        )
        .unwrap();
    let before = engine.store.load(&run_id).unwrap().len();

    let moved = engine
        .lawful_end_under_contention(EngineError::Store(StoreError::HeadMoved {
            expected_seq: 2,
            found_seq: 3,
        }))
        .expect_err("a moved head is not an ending to park on");
    assert!(
        matches!(&moved, EngineError::Store(StoreError::HeadMoved { .. })),
        "a fenced refusal was converted into something else: {moved:?}"
    );

    let defect = engine
        .lawful_end_under_contention(EngineError::Other("a real defect".into()))
        .expect_err("a defect is not an ending to park on");
    assert!(matches!(&defect, EngineError::Other(detail) if detail == "a real defect"));

    assert_eq!(
        engine.store.load(&run_id).unwrap().len(),
        before,
        "a pass-through wrote to the journal"
    );
}
