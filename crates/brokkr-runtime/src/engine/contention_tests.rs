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
    EngineError::Store(contention(operation))
}

fn contention(operation: &'static str) -> StoreError {
    StoreError::Contended {
        operation,
        waited_ms: 30_000,
    }
}

/// A peer connection holding this journal's write lock until it is told
/// to let go.
fn write_lock_on(db: &Path) -> rusqlite::Connection {
    let holder = rusqlite::Connection::open(db).unwrap();
    holder
        .busy_timeout(std::time::Duration::from_secs(30))
        .unwrap();
    holder.execute_batch("BEGIN IMMEDIATE").unwrap();
    brokkr_store::test_support::plant_run(&holder, "peer", "{}").unwrap();
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
        .expect("a lawful park is an ending, not an error")
        .expect("a park ends the drive");
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

/// The same park met through `drive()`: at a `Park` cursor the engine's
/// own `run/parked` meets a peer's lock and hands the contention up, and
/// the lock lets go within the lawful end's patience. The drive ends in
/// the lawful end's park, naming the lock, and the cursor's own reason is
/// never written.
// On macos-latest the peer's lock did not refuse this test's first park
// append in two CI runs, although the file's other lock tests pass there;
// #473 investigates. The arm it covers is counted by the Linux-only
// exact-coverage gate.
#[cfg(target_os = "linux")]
#[test]
fn a_drive_that_meets_the_lock_at_a_park_ends_in_the_lawful_ends_park() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = in_flight(dir.path());
    let run_id = engine.run_id.clone();
    let settled = json!({"effect_id": "effect-1", "attempt_id": "attempt-1", "reason": "gone"});
    engine
        .store
        .append_next(&run_id, EventType::EffectIndeterminate, settled, None, None)
        .unwrap();
    let before = engine.store.load(&run_id).unwrap().len();
    // The clock starts before `drive` reaches its append, so the release
    // must come after the engine's setup plus the park's one patience and
    // before the lawful end's park spends one more: at one and a half
    // patiences, any setup under half a patience (1 s here) is covered.
    // A 200 ms patience left a slow macOS runner 100 ms, and it missed.
    let patience = std::time::Duration::from_secs(2);
    engine.store.set_patience(patience).unwrap();

    let holder = write_lock_on(&dir.path().join("realm.db"));
    let end = drive_beside(&mut engine, move || {
        std::thread::sleep(patience * 3 / 2);
        drop(holder);
    })
    .expect("a park the lock lets go of in time is an ending");

    let reason = end.state.park_reason.clone().unwrap();
    let waited = reason
        .strip_prefix(
            "journal contention: contended: a peer still held the journal's write lock after ",
        )
        .and_then(|rest| rest.strip_suffix("ms of append; nothing was written"))
        .and_then(|ms| ms.parse::<u128>().ok());
    assert!(waited >= Some(patience.as_millis()), "{reason}");
    assert_eq!(end.state.status, Status::AwaitingOperator);
    let events = engine.store.load(&run_id).unwrap();
    assert_eq!(events.len(), before + 1);
    let last = events.last().unwrap();
    assert_eq!(
        (last.event_type, &last.payload),
        (
            EventType::RunParked,
            &json!({"reason": reason, "evidence": {}})
        )
    );
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

// ---------------------------------------------------------------------
// While an effect is in flight (#394). A seat's checkpoints are telemetry
// until its terminal event, so a peer's lock delays them and never ends
// the attempt; only its settlement keeps patience-then-park, with three
// patiences, and the park it ends in leaves the attempt settled.
// ---------------------------------------------------------------------

/// A started engine walked to `EffectInFlight`: its attempt is open and
/// its checkpoints would stream in now.
fn in_flight(dir: &Path) -> Engine {
    let mut engine = started(dir);
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
    engine
        .store
        .set_patience(std::time::Duration::from_millis(25))
        .unwrap();
    engine
}

/// The attempt's sink, holding `limit` bytes and calling `between` before
/// every append it tries.
fn sink<B: FnMut(&mut Store)>(engine: &mut Engine, limit: usize, between: B) -> Checkpoints<'_, B> {
    let attempt = checkpoints::Attempt {
        run_id: &engine.run_id,
        effect_id: "effect-1",
        attempt_id: "attempt-1",
    };
    Checkpoints::holding(
        &mut engine.store,
        &mut engine.current_cause,
        attempt,
        limit,
        between,
    )
}

/// The `step` of every checkpoint journaled, in journal order.
fn landed(engine: &Engine) -> Vec<String> {
    engine
        .store
        .load(&engine.run_id)
        .unwrap()
        .iter()
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .map(|event| {
            event.payload["checkpoint"]["step"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect()
}

fn step(name: &str) -> Value {
    json!({"step": name})
}

fn succeeded() -> AttemptOutcome {
    AttemptOutcome::Succeeded {
        result: json!({"result": "complete"}),
    }
}

/// The fenced-race seam: `between` runs at the instant before each
/// append, and a peer takes the lock there again after the journal has
/// taken only part of what was held. Nothing is lost, nothing reorders,
/// and the terminal event's cause is the last checkpoint that landed.
#[test]
fn checkpoints_held_behind_a_peers_lock_land_in_order_once_it_lets_go() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("realm.db");
    let mut engine = in_flight(dir.path());
    let peer = std::rc::Rc::new(std::cell::RefCell::new(Some(write_lock_on(&db))));
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let retake = {
        let (peer, calls, db) = (peer.clone(), calls.clone(), db.clone());
        move |_: &mut Store| {
            calls.set(calls.get() + 1);
            // The fifth append: `a` has just landed, `b` is next.
            if calls.get() == 5 {
                *peer.borrow_mut() = Some(write_lock_on(&db));
            }
        }
    };
    let let_go = || drop(peer.borrow_mut().take().unwrap());
    let mut checkpoints = sink(&mut engine, checkpoints::HELD_BYTES, retake);

    for name in ["a", "b", "c"] {
        checkpoints.offer("", step(name));
    }
    let_go();
    checkpoints.offer("", step("d"));
    let_go();
    let settled = checkpoints.settle().expect("the lock was released");

    assert_eq!(calls.get(), 8, "three held, then a and b, then b, c and d");
    assert_eq!(landed(&engine), ["a", "b", "c", "d"]);
    let events = engine.store.load(&engine.run_id).unwrap();
    assert_eq!(
        engine.current_cause.as_deref(),
        Some(events.last().unwrap().event_id.as_str())
    );
    assert!(matches!(
        settled.outcome("", succeeded()),
        AttemptOutcome::Succeeded { result } if result == json!({"result": "complete"})
    ));
}

/// `a` and `b` offered behind a peer's lock, then the seat stops. The
/// peer lets go just before the `release`th append the sink tries (never,
/// at 0). Returns the engine, the appends tried, and what settled.
fn two_held_then_settled(
    release: usize,
) -> (tempfile::TempDir, Engine, usize, checkpoints::Settled) {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = in_flight(dir.path());
    let calls = std::cell::Cell::new(0);
    let mut holder = Some(write_lock_on(&dir.path().join("realm.db")));
    let between = |_: &mut Store| {
        calls.set(calls.get() + 1);
        if calls.get() == release {
            holder.take();
        }
    };
    let mut checkpoints = sink(&mut engine, checkpoints::HELD_BYTES, between);
    checkpoints.offer("", step("a"));
    checkpoints.offer("", step("b"));
    let settled = checkpoints.settle().expect("contention is not a failure");
    (dir, engine, calls.get(), settled)
}

/// Once the seat stops, held checkpoints get the settlement's patiences:
/// a lock that lets go within them lands every held row, in order, and
/// the attempt keeps its driver's outcome.
#[test]
fn held_checkpoints_land_when_the_lock_lets_go_within_the_settlement() {
    let (_dir, engine, calls, settled) = two_held_then_settled(4);
    assert_eq!(calls, 5, "a, then a again, then a patience, a and b");
    assert_eq!(landed(&engine), ["a", "b"]);
    assert!(matches!(
        settled.outcome("", succeeded()),
        AttemptOutcome::Succeeded { .. }
    ));
}

/// A lock that outlasts every patience of the settlement strands what is
/// held: nothing was written, and the attempt is indeterminate with the
/// count named, never read as its driver's success.
#[test]
fn held_checkpoints_the_lock_outlasts_through_the_settlement_are_counted_not_claimed() {
    let (_dir, engine, calls, settled) = two_held_then_settled(0);
    assert_eq!(calls, 5, "a, then a again, then three patiences");
    assert!(landed(&engine).is_empty());
    let AttemptOutcome::Indeterminate { reason } = settled.outcome("", succeeded()) else {
        panic!("an attempt with stranded checkpoints was reported as its driver's");
    };
    assert_eq!(
        reason,
        "2 checkpoint(s) were not journaled: a peer still held the journal's write lock \
         3 patiences after the seat stopped"
    );
}

/// While the seat works, no append waits on a peer's lock, not even the
/// one that finds the hold empty, so a burst behind the lock costs the
/// reader nothing: the seat is not stalled on its pipe.
#[test]
fn a_burst_behind_a_held_lock_never_waits_on_the_reader() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = in_flight(dir.path());
    let patience = std::time::Duration::from_millis(400);
    engine.store.set_patience(patience).unwrap();
    let holder = write_lock_on(&dir.path().join("realm.db"));
    let mut checkpoints = sink(&mut engine, checkpoints::HELD_BYTES, |_| {});
    let burst = std::time::Instant::now();
    for name in ["a", "b", "c", "d", "e"] {
        checkpoints.offer("", step(name));
    }
    assert!(burst.elapsed() < patience, "{:?}", burst.elapsed());
    drop(holder);
    checkpoints.settle().expect("the lock was released");
    assert_eq!(landed(&engine), ["a", "b", "c", "d", "e"]);
}

/// The hold is bounded. A burst that meets a full hold is counted, not
/// journaled, and the attempt it belonged to can never be read as a
/// success; another site's outcome is its own. What was held still lands
/// in order, and a freed hold takes checkpoints again.
#[test]
fn a_burst_past_a_full_hold_makes_the_attempt_indeterminate_and_says_how_many() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("realm.db");
    let mut engine = in_flight(dir.path());
    let limit = step("a").to_string().len();
    let holder = write_lock_on(&db);
    let mut checkpoints = sink(&mut engine, limit, |_| {});
    for name in ["a", "b", "c"] {
        checkpoints.offer("", step(name));
    }
    drop(holder);
    checkpoints.offer("", step("d"));
    let settled = checkpoints.settle().expect("the lock was released");

    assert_eq!(landed(&engine), ["a", "d"]);
    let AttemptOutcome::Indeterminate { reason } = settled.outcome("", succeeded()) else {
        panic!("an attempt with lost checkpoints was reported as its driver's");
    };
    assert_eq!(
        reason,
        "2 checkpoint(s) were not journaled: a peer held the journal's write lock past \
         this attempt's 12-byte checkpoint hold"
    );
    assert!(matches!(
        settled.outcome("another-member", succeeded()),
        AttemptOutcome::Succeeded { .. }
    ));
}

/// The driver's line that checkpoints `{"step": name}`.
fn checkpoint_line(name: &str) -> String {
    let line = format!(
        "{{\"proto\":\"forge-driver/v1\",\"msg_id\":\"{name}\",\"type\":\"checkpoint\",\
         \"effect_id\":\"%s\",\"attempt_id\":\"%s\",\"data\":{{\"step\":\"{name}\"}}}}"
    );
    format!("printf '{line}\\n' \"$effect_id\" \"$attempt_id\"")
}

/// The driver's line that raises the signal `name`, a file in `dir`.
fn signal_line(dir: &Path, name: &str) -> String {
    format!(": > '{}'", dir.join(name).display())
}

/// The driver's line that waits for the signal `name`, a file in `dir`.
fn wait_line(dir: &Path, name: &str) -> String {
    let signal = dir.join(name);
    format!("while [ ! -e '{}' ]; do sleep 0.01; done", signal.display())
}

/// The test's side of a signal: wait until `name` exists in `dir`.
fn arrive(dir: &Path, name: &str) {
    while !dir.join(name).exists() {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// The peer's side: once the driver raises `signal`, take `dir`'s journal
/// lock and raise `locked`.
fn lock_at(dir: &Path, signal: &str) -> rusqlite::Connection {
    arrive(dir, signal);
    let holder = write_lock_on(&dir.join("realm.db"));
    std::fs::write(dir.join("locked"), "").unwrap();
    holder
}

/// Drive `engine` while `peer` runs beside it on another thread.
fn drive_beside(engine: &mut Engine, peer: impl FnOnce() + Send) -> Result<DriveEnd, EngineError> {
    std::thread::scope(|scope| {
        scope.spawn(peer);
        engine.drive()
    })
}

/// A driver that accepts its effect, runs `middle`, succeeds, and runs
/// `after` once the engine is done with it.
fn signalling_driver(middle: &[String], after: &[String]) -> SeatBody {
    let head = r#"read -r hello
printf '%s\n' '{"proto":"forge-driver/v1","msg_id":"cap","type":"capabilities","driver":"test","version":"1","supports":[]}'
read -r start
effect_id=$(printf '%s' "$start" | sed -n 's/.*"effect_id":"\([^"]*\)".*/\1/p')
attempt_id=$(printf '%s' "$start" | sed -n 's/.*"attempt_id":"\([^"]*\)".*/\1/p')
printf '{"proto":"forge-driver/v1","msg_id":"accepted","type":"accepted","effect_id":"%s","attempt_id":"%s","session_ref":null}\n' "$effect_id" "$attempt_id""#;
    let tail = r#"printf '{"proto":"forge-driver/v1","msg_id":"result","type":"result","effect_id":"%s","attempt_id":"%s","status":"succeeded","result":{"result":"complete"},"error":null}\n' "$effect_id" "$attempt_id"
read -r done"#;
    let script = std::iter::once(head.to_string())
        .chain(middle.iter().cloned())
        .chain([tail.to_string()])
        .chain(after.iter().cloned())
        .collect::<Vec<_>>()
        .join("\n");
    single_body(vec!["sh".into(), "-c".into(), script])
}

/// A seat's deadline where the deadline is not what is under test.
const UNHURRIED_SECONDS: u64 = 60;

/// The bundle that seats `body`; the seat's deadline is not what is under
/// test.
fn seating(dir: &Path, body: SeatBody) -> Bundle {
    seating_within(dir, body, UNHURRIED_SECONDS)
}

/// The bundle that seats `body` with a deadline of `timeout_seconds`.
fn seating_within(dir: &Path, body: SeatBody, timeout_seconds: u64) -> Bundle {
    let mut compiled = bundle(dir, body);
    if let Some(seat) = compiled.seats.get_mut("work") {
        seat.limits.timeout_seconds = timeout_seconds;
    }
    compiled
}

/// A real engine on `dir`'s journal, about to drive `body` with the
/// store's `patience`.
fn driving(dir: &Path, body: SeatBody, patience: std::time::Duration) -> Engine {
    driving_within(dir, body, patience, UNHURRIED_SECONDS)
}

/// [`driving`], with the seat's deadline `timeout_seconds`.
fn driving_within(
    dir: &Path,
    body: SeatBody,
    patience: std::time::Duration,
    timeout_seconds: u64,
) -> Engine {
    std::fs::create_dir_all(dir.join("work")).unwrap();
    let compiled = seating_within(dir, body, timeout_seconds);
    let store = Store::open(&dir.join("realm.db")).unwrap();
    let work = Some(dir.join("work"));
    let mut engine = Engine::start(store, compiled, "Feature: contention", work).unwrap();
    engine.store.set_patience(patience).unwrap();
    engine
}

/// The event types the run's one effect journaled, in order.
fn work_events(engine: &Engine) -> Vec<EventType> {
    let events = engine.store.load(&engine.run_id).unwrap();
    fold(&events).expect("the journal folds");
    events
        .iter()
        .filter(|event| event.payload["effect_id"] == events[2].payload["effect_id"])
        .map(|event| event.event_type)
        .collect()
}

/// The acceptance, end to end: a real engine drives a real seat, and a
/// peer takes the journal's write lock while the seat is working and
/// holds it well past the engine's patience while the seat streams a
/// burst of checkpoints. The seat is never stopped; its checkpoints land,
/// in order, after the lock is released; its attempt succeeds; and no
/// `effect/indeterminate` is written.
#[test]
fn a_seat_working_through_a_peers_lock_keeps_its_attempt() {
    let dir = tempfile::tempdir().unwrap();
    let (signal, wait) = (
        |name| signal_line(dir.path(), name),
        |name| wait_line(dir.path(), name),
    );
    let middle = [
        checkpoint_line("first"),
        signal("working"),
        wait("locked"),
        checkpoint_line("burst-1"),
        checkpoint_line("burst-2"),
        checkpoint_line("burst-3"),
        signal("burst"),
        wait("released"),
        checkpoint_line("after"),
    ];
    let patience = std::time::Duration::from_millis(25);
    let mut engine = driving(dir.path(), signalling_driver(&middle, &[]), patience);

    let ended = drive_beside(&mut engine, || {
        let holder = lock_at(dir.path(), "working");
        arrive(dir.path(), "burst");
        // Well past the patience of every append the burst tries.
        std::thread::sleep(patience * 20);
        drop(holder);
        std::fs::write(dir.path().join("released"), "").unwrap();
    })
    .expect("a peer's lock on a checkpoint does not end the engine");

    assert_eq!(
        landed(&engine),
        ["first", "burst-1", "burst-2", "burst-3", "after"]
    );
    let events = engine.store.load(&engine.run_id).unwrap();
    assert_eq!(
        work_events(&engine),
        [
            EventType::EffectRequested,
            EventType::EffectStarted,
            EventType::EffectCheckpointed,
            EventType::EffectCheckpointed,
            EventType::EffectCheckpointed,
            EventType::EffectCheckpointed,
            EventType::EffectCheckpointed,
            EventType::EffectSucceeded,
        ]
    );
    assert!(!events
        .iter()
        .any(|event| event.event_type == EventType::EffectIndeterminate));
    fold(&events).expect("the journal folds");
    assert_eq!(ended.state.status, Status::AwaitingOperator);
}

/// The driver's line that writes more blank bytes than a pipe holds. The
/// engine skips a blank line, and the seat moves past it only once the
/// engine has read it all.
fn pipe_filling_line() -> String {
    "head -c 262144 /dev/zero | tr '\\0' ' '; echo".to_string()
}

/// The seat's deadline, end to end: a peer takes the lock as the seat
/// checkpoints and then writes more than its pipe holds. A reader that
/// waited out the lock on that checkpoint would leave the seat blocked on
/// its pipe past its deadline, where the watchdog kills it. The reader
/// never waits, so the seat finishes within its deadline, and its held
/// checkpoint lands once the lock lets go.
#[test]
fn a_seat_writing_behind_a_peers_lock_is_never_held_past_its_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let at = dir.path();
    let middle = [
        signal_line(at, "working"),
        wait_line(at, "locked"),
        checkpoint_line("held"),
        pipe_filling_line(),
        signal_line(at, "drained"),
        wait_line(at, "released"),
    ];
    let deadline = 3;
    // A patience that outlasts the seat's deadline many times over.
    let patience = std::time::Duration::from_secs(deadline * 10);
    let body = signalling_driver(&middle, &[]);
    let mut engine = driving_within(dir.path(), body, patience, deadline);

    let ended = drive_beside(&mut engine, || {
        let holder = lock_at(dir.path(), "working");
        // Once the seat has drained, or, had the reader stalled so that
        // it never could, once its deadline has passed.
        let given_up = std::time::Instant::now() + std::time::Duration::from_secs(deadline + 2);
        while !dir.path().join("drained").exists() && std::time::Instant::now() < given_up {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        drop(holder);
        std::fs::write(dir.path().join("released"), "").unwrap();
    })
    .expect("a peer's lock on a checkpoint does not end the engine");

    assert_eq!(landed(&engine), ["held"]);
    assert_eq!(
        work_events(&engine),
        [
            EventType::EffectRequested,
            EventType::EffectStarted,
            EventType::EffectCheckpointed,
            EventType::EffectSucceeded,
        ]
    );
    assert_eq!(ended.state.status, Status::AwaitingOperator);
}

/// The settlement, end to end: the seat has done its work when a peer
/// takes the lock, and holds it past the first patience of the terminal
/// event but not past all of them. The attempt's real outcome lands once
/// the lock lets go; nothing settles it as indeterminate.
#[test]
fn a_finished_seats_terminal_event_waits_out_a_peers_lock_and_lands_its_real_outcome() {
    let dir = tempfile::tempdir().unwrap();
    let middle = [
        signal_line(dir.path(), "finishing"),
        wait_line(dir.path(), "locked"),
    ];
    let patience = std::time::Duration::from_millis(200);
    let mut engine = driving(dir.path(), signalling_driver(&middle, &[]), patience);

    let ended = drive_beside(&mut engine, || {
        let holder = lock_at(dir.path(), "finishing");
        std::thread::sleep(patience * 3 / 2);
        drop(holder);
    })
    .expect("a peer's lock on a terminal event that lets go in time does not end the engine");

    assert_eq!(
        work_events(&engine),
        [
            EventType::EffectRequested,
            EventType::EffectStarted,
            EventType::EffectSucceeded,
        ]
    );
    assert_eq!(ended.state.status, Status::AwaitingOperator);
}

/// A seat that finishes behind a peer's lock: the peer takes the lock as
/// the seat finishes, and lets it go when told to.
fn finishing_behind_a_lock(dir: &Path) -> SeatBody {
    let middle = [signal_line(dir, "finishing"), wait_line(dir, "locked")];
    signalling_driver(&middle, &[signal_line(dir, "exited")])
}

/// The lawful end, end to end: the peer holds the lock past every
/// patience of the terminal event and lets go within the lawful end's.
/// The attempt's real outcome lands there, nothing names the lock, and
/// the drive goes on to the run's own ending.
#[test]
fn a_terminal_event_the_lock_outlasts_lands_its_real_outcome_in_the_lawful_end() {
    let dir = tempfile::tempdir().unwrap();
    let patience = std::time::Duration::from_millis(200);
    let body = finishing_behind_a_lock(dir.path());
    let mut engine = driving(dir.path(), body, patience);

    let ended = drive_beside(&mut engine, || {
        let holder = lock_at(dir.path(), "finishing");
        arrive(dir.path(), "exited");
        // Past the terminal event's three patiences, within the lawful
        // end's three.
        std::thread::sleep(patience * 9 / 2);
        drop(holder);
    })
    .expect("a held outcome that lands in the lawful end does not end the engine");

    assert_eq!(
        work_events(&engine),
        [
            EventType::EffectRequested,
            EventType::EffectStarted,
            EventType::EffectSucceeded,
        ]
    );
    let events = engine.store.load(&engine.run_id).unwrap();
    assert!(!events
        .iter()
        .any(|event| event.payload.to_string().contains("journal contention")));
    assert_eq!(ended.state.status, Status::AwaitingOperator);
}

/// A lock that outlasts both windows, end to end: nothing is writable, so
/// the drive hands the typed contention back with nothing written after
/// the attempt started, and the next resume settles the attempt as
/// restarted and parks.
#[test]
fn a_lock_that_outlasts_both_windows_hands_the_contention_back_with_the_attempt_open() {
    let dir = tempfile::tempdir().unwrap();
    let patience = std::time::Duration::from_millis(50);
    let mut engine = driving(dir.path(), finishing_behind_a_lock(dir.path()), patience);
    let run_id = engine.run_id.clone();

    let (release, released) = std::sync::mpsc::channel::<()>();
    let peer = dir.path();
    let ended = std::thread::scope(|scope| {
        scope.spawn(move || {
            let holder = lock_at(peer, "finishing");
            released.recv().unwrap();
            drop(holder);
        });
        let ended = engine.drive();
        release.send(()).unwrap();
        ended
    });
    let handed_back = ended.expect_err("nothing is writable behind the lock");
    assert!(
        matches!(
            &handed_back,
            EngineError::Store(StoreError::Contended {
                operation: "append",
                ..
            })
        ),
        "{handed_back:?}"
    );
    assert_eq!(
        work_events(&engine),
        [EventType::EffectRequested, EventType::EffectStarted]
    );

    let store = Store::open(&dir.path().join("realm.db")).unwrap();
    let bundle = seating(dir.path(), finishing_behind_a_lock(dir.path()));
    let mut resumed = Engine::resume(store, bundle, &run_id, Some(dir.path().join("work")))
        .expect("the pinned bundle resumes");
    let again = resumed.drive().unwrap();
    assert_eq!(again.state.status, Status::AwaitingOperator);
    let events = resumed.store.load(&run_id).unwrap();
    let settled = &events[events.len() - 2];
    assert_eq!(
        (settled.event_type, &settled.payload["reason"]),
        (
            EventType::EffectIndeterminate,
            &json!(
                "engine restarted while the attempt was in flight; completion cannot be \
                 established"
            )
        )
    );
}

/// A terminal event the lock outlasts: the drive ends lawfully, the
/// attempt settled as `effect/indeterminate` naming the lock and the run
/// parked with `run/parked` naming it too — so the next resume finds no
/// open attempt to call "engine restarted", and writes nothing.
#[test]
fn a_terminal_event_the_lock_outlasts_settles_the_attempt_and_parks() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = in_flight(dir.path());
    let run_id = engine.run_id.clone();
    let expected = "journal contention: contended: a peer still held the journal's write \
                    lock after 30000ms of append; nothing was written";

    let end = engine
        .lawful_end_under_contention(contended("append"))
        .expect("a settled attempt parks lawfully")
        .expect("with no outcome held, the park ends the drive");
    assert_eq!(end.state.status, Status::AwaitingOperator);
    assert_eq!(end.state.park_reason.as_deref(), Some(expected));
    let events = engine.store.load(&run_id).unwrap();
    let tail: Vec<_> = events[events.len() - 2..]
        .iter()
        .map(|event| (event.event_type, event.payload.clone()))
        .collect();
    assert_eq!(
        tail,
        [
            (
                EventType::EffectIndeterminate,
                json!({"effect_id": "effect-1", "attempt_id": "attempt-1", "reason": expected})
            ),
            (
                EventType::RunParked,
                json!({"reason": expected, "evidence": {}})
            ),
        ]
    );
    assert_eq!(
        events[events.len() - 2].attempt_id.as_deref(),
        Some("attempt-1")
    );

    let written = events.len();
    let mut resumed = resumed(dir.path(), &run_id);
    let again = resumed.drive().unwrap();
    assert_eq!(again.state.park_reason.as_deref(), Some(expected));
    assert_eq!(resumed.store.load(&run_id).unwrap().len(), written);
}

/// A fresh engine resuming `run_id` on `dir`'s journal, as the next
/// `brokkr resume` would.
fn resumed(dir: &Path, run_id: &str) -> Engine {
    let store = Store::open(&dir.join("realm.db")).unwrap();
    let body = single_body(vec!["missing-driver".into()]);
    Engine::resume(store, bundle(dir, body), run_id, Some(dir.join("work"))).unwrap()
}

/// `append`, or the lawful end, met the lock for all three of the
/// settlement's patiences and handed the typed contention up.
fn assert_outlasted(started: std::time::Instant, handed_up: &EngineError) {
    let three_patiences = std::time::Duration::from_millis(75);
    assert!(
        started.elapsed() >= three_patiences,
        "{:?}",
        started.elapsed()
    );
    assert!(
        matches!(
            handed_up,
            EngineError::Store(StoreError::Contended {
                operation: "append",
                ..
            })
        ),
        "{handed_up:?}"
    );
}

/// The same ending met for real: the engine's own terminal append spends
/// all three of the settlement's patiences on the attempt's real outcome
/// before it hands the contention up, and holds that outcome. The lawful
/// end spends its patiences on the same outcome: while the lock holds it
/// writes nothing, and once the lock is gone the real outcome lands, no
/// indeterminate is written, and the drive goes on.
#[test]
fn a_terminal_event_behind_a_held_lock_gets_every_settling_patience_first() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = in_flight(dir.path());
    let holder = write_lock_on(&dir.path().join("realm.db"));
    let result = json!({
        "effect_id": "effect-1",
        "attempt_id": "attempt-1",
        "result": {"result": "complete"},
    });
    let started = std::time::Instant::now();
    let unlanded = engine
        .append(
            EventType::EffectSucceeded,
            result.clone(),
            Some("attempt-1".into()),
        )
        .expect_err("a lock held throughout cannot be appended past");
    assert_outlasted(started, &unlanded);
    let before = engine.store.load(&engine.run_id).unwrap().len();

    let started = std::time::Instant::now();
    let outlasted = engine
        .lawful_end_under_contention(unlanded)
        .expect_err("nothing is writable behind the lock");
    assert_outlasted(started, &outlasted);
    assert_eq!(engine.store.load(&engine.run_id).unwrap().len(), before);

    drop(holder);
    let end = engine
        .lawful_end_under_contention(outlasted)
        .expect("the held outcome lands once the lock lets go");
    assert!(end.is_none(), "the drive ended instead of going on");
    let events = engine.store.load(&engine.run_id).unwrap();
    assert_eq!(events.len(), before + 1);
    let last = events.last().unwrap();
    assert_eq!(
        (last.event_type, &last.payload, last.attempt_id.as_deref()),
        (EventType::EffectSucceeded, &result, Some("attempt-1"))
    );
}

/// `result`, held as `attempt_id`'s success of effect-1.
fn held_success(attempt_id: &str, result: Value) -> checkpoints::HeldOutcome {
    checkpoints::HeldOutcome {
        event_type: EventType::EffectSucceeded,
        payload: json!({"effect_id": "effect-1", "attempt_id": attempt_id, "result": result}),
        attempt_id: Some(attempt_id.into()),
    }
}

/// A held result the seat-record fence refuses once the lock lets go is
/// not a success and does not end the engine: the attempt settles
/// indeterminate with the refusal named, and the drive goes on to park.
#[test]
fn a_held_result_the_fence_refuses_settles_the_attempt_indeterminate() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = in_flight(dir.path());
    engine.held_outcome = Some(held_success("attempt-1", json!({})));
    let end = engine
        .lawful_end_under_contention(contended("append"))
        .expect("a refusal of a held result is an outcome, not an error");
    assert!(end.is_none(), "the drive ended instead of going on");
    let events = engine.store.load(&engine.run_id).unwrap();
    let last = events.last().unwrap();
    assert_eq!(
        (last.event_type, &last.payload),
        (
            EventType::EffectIndeterminate,
            &json!({
                "effect_id": "effect-1",
                "attempt_id": "attempt-1",
                "reason": "the journal refused the result a peer's lock had held back: seat \
                           record at journal seq 5 violates contracts/seat-record.v5.schema.json \
                           at /",
            })
        )
    );
}

/// A held outcome is spent only on its own attempt. One held for another
/// attempt is dropped, and the attempt in flight is settled naming the
/// lock.
#[test]
fn an_outcome_held_for_another_attempt_is_never_journaled_for_this_one() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = in_flight(dir.path());
    engine.held_outcome = Some(held_success("attempt-0", json!({"result": "complete"})));
    let end = engine
        .lawful_end_under_contention(contended("append"))
        .expect("a settled attempt parks lawfully")
        .expect("with no outcome held for this attempt, the park ends the drive");
    assert_eq!(end.state.status, Status::AwaitingOperator);
    assert!(engine.held_outcome.is_none());
    let events = engine.store.load(&engine.run_id).unwrap();
    let tail: Vec<_> = events[events.len() - 2..]
        .iter()
        .map(|event| (event.event_type, event.payload["attempt_id"].clone()))
        .collect();
    assert_eq!(
        tail,
        [
            (EventType::EffectIndeterminate, json!("attempt-1")),
            (EventType::RunParked, Value::Null),
        ]
    );
}

/// A lock that outlasts the settlement too leaves nothing writable. The
/// engine hands the contention back with the attempt open and a held
/// checkpoint dies with it; the next resume settles the attempt as
/// restarted and parks, and never claims what was not journaled.
#[test]
fn a_lock_that_outlasts_the_settlement_leaves_the_attempt_to_the_next_resume() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = in_flight(dir.path());
    let run_id = engine.run_id.clone();
    let holder = write_lock_on(&dir.path().join("realm.db"));
    sink(&mut engine, checkpoints::HELD_BYTES, |_| {}).offer("", step("a"));
    let before = engine.store.load(&run_id).unwrap().len();
    let handed_back = engine
        .lawful_end_under_contention(contended("append"))
        .expect_err("nothing is writable behind the lock");
    assert!(
        matches!(
            &handed_back,
            EngineError::Store(StoreError::Contended {
                operation: "append",
                ..
            })
        ),
        "{handed_back:?}"
    );
    assert_eq!(engine.store.load(&run_id).unwrap().len(), before);

    drop(holder);
    let mut resumed = resumed(dir.path(), &run_id);
    let again = resumed.drive().unwrap();
    assert_eq!(again.state.status, Status::AwaitingOperator);
    assert!(landed(&resumed).is_empty());
    let events = resumed.store.load(&run_id).unwrap();
    let tail: Vec<_> = events[events.len() - 2..]
        .iter()
        .map(|event| (event.event_type, event.payload["reason"].clone()))
        .collect();
    let restarted = "engine restarted while the attempt was in flight; completion cannot be \
                     established";
    assert_eq!(
        tail,
        [
            (EventType::EffectIndeterminate, json!(restarted)),
            (
                EventType::RunParked,
                json!(format!("effect effect-1 indeterminate: {restarted}"))
            ),
        ]
    );
}

/// The settlement's retry: contention is tried again up to the patiences
/// given, and anything else returns at once.
#[test]
fn a_settlement_retries_contention_and_nothing_else() {
    let mut tries = 0;
    let landed = checkpoints::within_patiences(3, || {
        tries += 1;
        if tries < 3 {
            Err(contention("append"))
        } else {
            Ok(tries)
        }
    });
    assert_eq!(landed.unwrap(), 3);

    let mut tries = 0;
    let outlasted = checkpoints::within_patiences(3, || {
        tries += 1;
        Err::<(), _>(contention("append"))
    });
    assert!(matches!(
        outlasted,
        Err(StoreError::Contended {
            operation: "append",
            waited_ms: 30_000
        })
    ));
    assert_eq!(tries, 3);

    let mut tries = 0;
    let refused = checkpoints::within_patiences(3, || {
        tries += 1;
        Err::<(), _>(StoreError::RunNotFound("r".into()))
    });
    assert!(matches!(refused, Err(StoreError::RunNotFound(run)) if run == "r"));
    assert_eq!(tries, 1);
}

/// A site that lost checkpoints to a full hold and then met the fence's
/// refusal is indeterminate, and its reason names both: the refusal's
/// text is not dropped behind the count.
#[test]
fn a_site_that_lost_checkpoints_and_met_a_refusal_names_both() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = in_flight(dir.path());
    let limit = step("a").to_string().len();
    let holder = write_lock_on(&dir.path().join("realm.db"));
    let mut checkpoints = sink(&mut engine, limit, |_| {});
    checkpoints.offer("", step("a"));
    checkpoints.offer("", step("b"));
    drop(holder);
    checkpoints.offer("", json!({"step": "seat-turn", "turn": "one"}));
    let settled = checkpoints
        .settle()
        .expect("a refusal is an outcome, not a failure");

    let AttemptOutcome::Indeterminate { reason } = settled.outcome("", succeeded()) else {
        panic!("a site with lost checkpoints was reported as its driver's");
    };
    assert_eq!(
        reason,
        "1 checkpoint(s) were not journaled: a peer held the journal's write lock past this \
         attempt's 12-byte checkpoint hold; seat record at journal seq 6 violates \
         contracts/seat-record.v5.schema.json at /"
    );
    assert_eq!(landed(&engine), ["a"]);
}

/// A stop riding the attempt keeps its own ending. The attempt is still
/// settled, but the fold concludes it into the stop, where a park would
/// be refused — so no `run/parked` is forged, the typed contention is
/// handed back, and the next resume stops the run.
#[test]
fn a_riding_stop_keeps_its_ending_once_the_attempt_is_settled() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = in_flight(dir.path());
    let run_id = engine.run_id.clone();
    for (event_type, payload) in [
        (
            EventType::OperatorCommanded,
            json!({"command_id":"ride","command":"stop","args":{},"operator":"operator"}),
        ),
        (
            EventType::OperatorAccepted,
            json!({"command_id":"ride","operator":"operator","reason":"enough"}),
        ),
    ] {
        engine
            .store
            .append_next(&run_id, event_type, payload, None, None)
            .unwrap();
    }

    let handed_back = engine
        .lawful_end_under_contention(contended("append"))
        .expect_err("a park the riding stop forbids is not an ending");
    assert!(
        matches!(&handed_back, EngineError::Store(e) if e.is_contention()),
        "{handed_back:?}"
    );
    let events = engine.store.load(&run_id).unwrap();
    assert_eq!(
        events.last().unwrap().event_type,
        EventType::EffectIndeterminate
    );
    assert_eq!(fold(&events).unwrap().cursor, Cursor::Stop);
}
