//! Fix round 0106b1r4, F1 (the operator's ruling of 2026-11-18, amending
//! decision 0103 ruling 2): a probe holds its route while it is in flight.
//! A hold is written before the probe is reserved, and the pass and the
//! claim read it as cooling down through the one in-force read. A refusal's
//! renewed round ends it in the round's own transaction; an answer ends it
//! at once; a round the store does not take leaves it standing until the
//! probe's deadline, and then it holds nothing.
//!
//! No other row is planted on the route: the old held row has expired when
//! the probe dispatches, so whatever a second connection's pass and claim
//! read is the hold or the probe's own round. Both are the production
//! gates, the real pass and the real claim of an entry queued in the
//! engine's journal (fix round 0106b1r5, M1).

use std::path::{Path, PathBuf};
use std::thread::JoinHandle;

pub(super) use crate::admission::capacity::tests::gates::{Gates, Legs};
use crate::admission::Capacity;

use super::held_wait::{held_until, standing};
use super::*;

/// The gates a probe's tests share with the engine's clock and a watcher.
pub(super) type Shared = Arc<Mutex<Gates>>;

/// A clock over `now` whose every read runs `act` first, and whose slices
/// advance `now`.
pub(super) fn acting_on_read(
    now: &Arc<Mutex<OffsetDateTime>>,
    act: impl Fn(OffsetDateTime) + Send + Sync + 'static,
) -> Clock {
    let read = Arc::clone(now);
    Clock {
        now: Box::new(move || {
            let at = *read.lock().unwrap();
            act(at);
            at
        }),
        sleep: slicing(now),
    }
}

/// A test clock's sleep: each slice advances `now` by its whole seconds.
pub(super) fn slicing(
    now: &Arc<Mutex<OffsetDateTime>>,
) -> Box<dyn FnMut(std::time::Duration) + Send + Sync> {
    let advance = Arc::clone(now);
    Box::new(move |slice: std::time::Duration| {
        *advance.lock().unwrap() += time::Duration::seconds(slice.as_secs() as i64);
    })
}

/// Both legs refuse claude/first as cooling down until exactly `end`.
pub(super) fn held_to(legs: &Legs, end: OffsetDateTime) -> bool {
    legs.iter().all(|leg| {
        matches!(
            leg,
            Err(Capacity::CoolingDown { until, .. }) if *until == end
        )
    })
}

/// A single seat held five seconds past its two-second maximum, journaled
/// by `drive`, whose probes do what `probed` says; the clock set to the
/// stored end of that held row, so no row holds the route any more.
pub(super) fn held_at_its_end(
    probed: Probed,
    drive: impl FnOnce(&Path, &mut Engine),
) -> (tempfile::TempDir, Engine, Arc<Mutex<OffsetDateTime>>) {
    let (dir, mut engine, now) = seated!(&["refuse", "succeed"], probed, Some(5), 2);
    engine.cooldown.host = Some(host(dir.path(), 1, 2));
    drive(dir.path(), &mut engine);
    assert_eq!(standing(&engine), (Status::Running, 0));
    let until = held_until(&engine);
    let stored = engine
        .store
        .cooldown("claude", "first", until - time::Duration::seconds(1))
        .unwrap()
        .expect("the held row holds before its own end")
        .until;
    *now.lock().unwrap() = stored;
    assert_eq!(
        engine.store.cooldown("claude", "first", stored).unwrap(),
        None
    );
    (dir, engine, now)
}

/// The fixture driven plainly with probes that do what `probed` says, the
/// instant its probe will run at, and the production gates over its
/// journal, whose pass found room at that instant: nothing holds the route
/// before the probe is dispatched.
pub(super) fn probing(
    probed: Probed,
) -> (
    tempfile::TempDir,
    Engine,
    Arc<Mutex<OffsetDateTime>>,
    OffsetDateTime,
    Shared,
) {
    let (dir, engine, now) = held_at_its_end(probed, |_, engine| {
        engine.drive().unwrap();
    });
    let probe_at = *now.lock().unwrap();
    let gates = Gates::over(engine.store.path(), probe_at);
    (dir, engine, now, probe_at, Arc::new(Mutex::new(gates)))
}

/// What the shared gates read of claude/first at `at`.
pub(super) fn legs(gates: &Shared, at: OffsetDateTime) -> Legs {
    gates.lock().unwrap().legs(at)
}

/// The probe's deadline, the instant its driver is killed by: the seat's
/// two-second timeout past the instant it was dispatched.
pub(super) fn deadline_of(probe_at: OffsetDateTime) -> OffsetDateTime {
    probe_at + time::Duration::seconds(2)
}

/// The probe's hold's end as the store keeps it: the first whole second
/// strictly after its deadline (fix round 0106b1r6, D1).
fn hold_end_of(probe_at: OffsetDateTime) -> OffsetDateTime {
    deadline_of(probe_at) + time::Duration::seconds(1)
}

/// While the barrier probe is in flight, the gates' legs at `at` and
/// whether any cool-down row stood then; the probe is released after the
/// look.
fn in_flight(
    dir: &Path,
    gates: &Shared,
    journal: PathBuf,
    at: OffsetDateTime,
) -> JoinHandle<(Legs, bool)> {
    let (started, release) = (dir.join("work.started"), dir.join("work.release"));
    let gates = Arc::clone(gates);
    std::thread::spawn(move || {
        let begun = std::time::Instant::now();
        while !started.exists() {
            assert!(begun.elapsed().as_secs() < 30, "the probe never started");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let seen = legs(&gates, at);
        let store = brokkr_store::Store::open(&journal).unwrap();
        let row = store.cooldown("claude", "first", at).unwrap().is_some();
        std::fs::write(release, "").unwrap();
        (seen, row)
    })
}

/// From dispatch until its refusal's row commits, and after it until the
/// row's own end, a second connection's pass and claim read the route as
/// cooling down: the hold while the probe runs and between the refusal and
/// the commit, then the renewed row.
#[test]
fn a_probe_holds_its_route_from_dispatch_until_its_refusals_row_commits() {
    let (dir, mut engine, now, probe_at, gates) = probing(Probed::BarrierRefuse);
    let journal = engine.store.path().to_path_buf();
    let flight = in_flight(dir.path(), &gates, journal.clone(), probe_at);
    // Every clock read after the probe returned: before its round's commit
    // (no second round yet) and after it.
    let seen: Arc<Mutex<Vec<(Legs, usize)>>> = Arc::default();
    let (watch, release, run_id) = (
        Arc::clone(&seen),
        dir.path().join("work.release"),
        engine.run_id.clone(),
    );
    let (seq, watching) = (failed(&engine)[0].seq, Arc::clone(&gates));
    engine.cooldown.clock = acting_on_read(&now, move |at| {
        if release.exists() {
            let store = brokkr_store::Store::open(&journal).unwrap();
            let rounds = store.cooldown_rounds(&run_id, seq, "").unwrap().len();
            watch.lock().unwrap().push((legs(&watching, at), rounds));
        }
    });
    engine.drive().unwrap();
    let (flying, row) = flight.join().unwrap();
    assert!(!row, "a cool-down row stood while the probe ran");
    let held = hold_end_of(probe_at);
    assert!(held_to(&flying, held), "{flying:?}");
    assert_eq!(standing(&engine), (Status::Running, 0));
    let end = engine
        .store
        .cooldown("claude", "first", probe_at)
        .unwrap()
        .expect("the probe's renewed round holds the route")
        .until;
    assert!(end >= probe_at + time::Duration::seconds(5), "{end}");
    let seen = seen.lock().unwrap().clone();
    // Between the refusal and the commit the hold still held the route.
    assert!(
        seen.iter()
            .any(|(legs, rounds)| *rounds == 1 && held_to(legs, held)),
        "{seen:?}"
    );
    for (legs, rounds) in &seen {
        let until = if *rounds == 1 { held } else { end };
        assert!(held_to(legs, until), "{seen:?}");
    }
    for at in [probe_at, held, end - time::Duration::seconds(1)] {
        assert!(held_to(&legs(&gates, at), end), "at {at}");
    }
    assert_eq!(legs(&gates, end), [Ok(()), Ok(())]);
    // The round's commit ended the hold with it: none stands unended.
    assert_eq!(unended_holds(engine.store.path()), 0);
}

/// How many probe holds the journal at `path` holds unended.
pub(super) fn unended_holds(path: &Path) -> i64 {
    rusqlite::Connection::open(path)
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM probe_holds WHERE ended_at IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

/// Force every insert into `table` to fail, drive `engine`, and see it
/// park once.
fn driven_failing(engine: &mut Engine, table: &str) {
    driven_raising(engine, table, &format!("ABORT, 'forced {table} failure'"));
}

/// Force every insert into `table` to `RAISE(raise)`, drive `engine`, and
/// see it park once.
pub(super) fn driven_raising(engine: &mut Engine, table: &str, raise: &str) {
    rusqlite::Connection::open(engine.store.path())
        .unwrap()
        .execute_batch(&format!(
            "CREATE TRIGGER forced_insert BEFORE INSERT ON {table}
             BEGIN SELECT RAISE({raise}); END;"
        ))
        .unwrap();
    engine.drive().unwrap();
    assert_eq!(standing(engine), (Status::AwaitingOperator, 1));
}

/// A renewed round the store does not take (every cool-down insert forced
/// to fail) parks the run and leaves the probe's hold standing: the route
/// reads cooling down until the probe's deadline, and is open at it.
#[test]
fn a_probe_round_the_store_cannot_take_leaves_its_route_held_until_the_probes_deadline() {
    let (dir, mut engine, _now, probe_at, gates) = probing(Probed::BarrierRefuse);
    std::fs::write(dir.path().join("work.release"), "").unwrap();
    driven_failing(&mut engine, "provider_cooldowns");
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse", "probe"]);
    held_to_its_deadline(&engine, &gates, deadline_of(probe_at));
}

/// A renewed round the table takes without writing (every cool-down insert
/// ignored by a trigger) refuses by name and ends no hold (fix round
/// 0106b1r5, H3): the run parks naming the unwritten row, and the route
/// reads cooling down until the probe's deadline.
#[test]
fn a_probe_round_the_table_does_not_write_ends_no_hold() {
    let (dir, mut engine, _now, probe_at, gates) = probing(Probed::BarrierRefuse);
    std::fs::write(dir.path().join("work.release"), "").unwrap();
    driven_raising(&mut engine, "provider_cooldowns", "IGNORE");
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse", "probe"]);
    let parked = (engine.store.load(&engine.run_id).unwrap().into_iter())
        .find(|event| event.event_type == EventType::RunParked)
        .expect("the run parked");
    let reason = parked.payload["reason"].as_str().unwrap();
    assert!(
        reason.contains("wrote no row for run") && reason.contains("hold on its route stands"),
        "{reason}"
    );
    held_to_its_deadline(&engine, &gates, deadline_of(probe_at));
}

/// The probe left no row on the route and its hold standing: the gates read
/// the route cooling down from the probe's instant through `deadline`, the
/// instant its driver is killed by, to the first whole second after it,
/// and open at that second (fix round 0106b1r6, D1).
pub(super) fn held_to_its_deadline(engine: &Engine, gates: &Shared, deadline: OffsetDateTime) {
    let probe_at = deadline - time::Duration::seconds(2);
    let end = deadline + time::Duration::seconds(1);
    assert_eq!(
        engine.store.cooldown("claude", "first", probe_at).unwrap(),
        None
    );
    for at in [probe_at, deadline] {
        assert!(held_to(&legs(gates, at), end), "at {at}");
    }
    assert_eq!(unended_holds(engine.store.path()), 1);
    assert_eq!(legs(gates, end), [Ok(()), Ok(())]);
}

/// An answered probe ends its own hold: the route reads cooling down while
/// the probe is in flight, and a second connection's pass and claim admit
/// on it at once after the answer, well before the probe's deadline.
#[test]
fn an_answered_probe_opens_its_route_at_once() {
    let (dir, mut engine, _now, probe_at, gates) = probing(Probed::Barrier);
    let flight = in_flight(
        dir.path(),
        &gates,
        engine.store.path().to_path_buf(),
        probe_at,
    );
    engine.drive().unwrap();
    let (flying, row) = flight.join().unwrap();
    assert!(!row, "a cool-down row stood while the probe ran");
    assert!(held_to(&flying, hold_end_of(probe_at)), "{flying:?}");
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
    // The real claim takes the entry on the route at once.
    assert_eq!(legs(&gates, probe_at), [Ok(()), Ok(())]);
}

/// A hold the store refuses (every hold insert forced to fail) admits no
/// probe: the run parks naming the unwritten hold, and nothing is probed.
#[test]
fn a_hold_the_store_refuses_parks_before_any_probe() {
    let (dir, mut engine, _now, _probe_at, _gates) = probing(Probed::BarrierRefuse);
    driven_failing(&mut engine, "probe_holds");
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
    let events = engine.store.load(&engine.run_id).unwrap();
    let reason = events
        .iter()
        .find(|event| event.event_type == EventType::RunParked)
        .and_then(|event| event.payload["reason"].as_str())
        .unwrap()
        .to_string();
    assert!(
        reason.contains(
            "the cool-down probe for claude/first was not admitted: its route's hold was not \
             written"
        ) && reason.contains("forced probe_holds failure"),
        "{reason}"
    );
}

/// A hold the table takes without writing (every hold insert ignored by a
/// trigger), after another hold, on codex/second, went in on the engine's
/// own connection, admits no probe (fix round 0106b1r6, D2): no driver starts,
/// the run parks with the store's typed refusal, the other hold stands,
/// and the route is as the store holds it, open.
#[test]
fn a_hold_the_table_does_not_write_parks_before_any_probe_and_ends_no_other() {
    let (dir, mut engine, _now, probe_at, gates) = probing(Probed::BarrierRefuse);
    let run_id = engine.run_id.clone();
    let other = brokkr_store::ProbeHold {
        run_id: &run_id,
        site: "",
        provider: "codex",
        route: "second",
        deadline: probe_at + time::Duration::hours(1),
    };
    engine.store.hold_probe(&other, probe_at).unwrap();
    driven_raising(&mut engine, "probe_holds", "IGNORE");
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
    let events = engine.store.load(&engine.run_id).unwrap();
    let parked = (events.iter())
        .find(|event| event.event_type == EventType::RunParked)
        .expect("the run parked");
    assert_eq!(
        parked.payload["reason"],
        format!(
            "the cool-down probe for claude/first was not admitted: its route's hold was not \
             written: the journal's probe holds wrote no row for run '{}' on claude/first, so \
             no probe runs on it and nothing was written",
            engine.run_id
        )
    );
    let unended: Vec<String> = rusqlite::Connection::open(engine.store.path())
        .unwrap()
        .prepare("SELECT route FROM probe_holds WHERE ended_at IS NULL")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(unended, ["second"]);
    assert_eq!(legs(&gates, probe_at), [Ok(()), Ok(())]);
}

/// Fix round 0106b1r9, E3: a probe on a queue this binary does not write
/// (its storage stamped an older version beneath the open engine, which
/// reads that version's waits) has no hold, and runs on none: the store
/// refuses the hold in its typed words, no driver starts, and the run
/// parks with them.
#[test]
fn a_hold_on_a_queue_this_binary_does_not_write_parks_before_any_probe() {
    let (dir, mut engine, _now, _probe_at, _gates) = probing(Probed::BarrierRefuse);
    rusqlite::Connection::open(engine.store.path())
        .unwrap()
        .execute("UPDATE meta SET value = '6' WHERE key = 'queue_schema'", [])
        .unwrap();
    engine.drive().unwrap();
    assert_eq!(standing(&engine), (Status::AwaitingOperator, 1));
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
    assert_eq!(
        park_reason(&engine),
        format!(
            "the cool-down probe for claude/first was not admitted: its route's hold was not \
             written: the journal's queue storage is not one this binary writes, so it takes no \
             probe hold for run '{}' on claude/first, no probe runs on it, and nothing was \
             written",
            engine.run_id
        )
    );
    assert_eq!(unended_holds(engine.store.path()), 0);
}
