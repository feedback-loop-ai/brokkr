//! The cool-down tests' support for #1110, apart from `tests.rs` so that
//! file stays within ruling 4's 2,000-line test ceiling: the seat limits a
//! fixture gives, the test of the witness a failed assertion prints, and
//! the guard that gives a process-launching test its own engine.

use super::*;

/// Re-execute the test alone before it can launch a driver. The nested
/// item's type name includes the test function; libtest drops the crate
/// prefix and ends at the test function itself.
macro_rules! own_engine {
    () => {
        if !$crate::engine::tests::in_its_own_engine(own_engine!(path)) {
            return;
        }
    };
    (path) => {{
        fn test_marker() {}
        std::any::type_name_of_val(&test_marker)
            .strip_prefix(concat!(env!("CARGO_CRATE_NAME"), "::"))
            .unwrap()
            .strip_suffix("::test_marker")
            .unwrap()
    }};
}

pub(super) use own_engine;

#[test]
fn the_own_engine_guard_derives_its_libtest_path() {
    own_engine!();
    assert_eq!(
        own_engine!(path),
        "engine::cooldown::tests::support::the_own_engine_guard_derives_its_libtest_path"
    );
}

/// The seat's limits where its deadline is not what the test proves: far
/// past any load, so only the clock the test drives decides the rounds
/// (#1110, as #1099's item 4 did for the sealing tests).
pub(super) fn work(attempts: u64) -> Limits {
    Limits {
        max_attempts: attempts,
        timeout_seconds: 60,
    }
}

/// The seat's limits where a probe's lateness is what the test proves: a
/// deadline the hanging probe's `sleep 30` outlives, but long enough that
/// admission and the probe's spawn cannot plausibly spend it under load
/// before the probe logs (#1110: at 2s the Hang row of
/// `the_unchanged_probe_ends_write_no_row` read `["attempt:refuse"]`).
fn late(attempts: u64) -> Limits {
    Limits {
        max_attempts: attempts,
        timeout_seconds: 10,
    }
}

/// The limits a seat whose probes do `probed` is given: the short one only
/// where a probe hangs to be ended by its deadline.
pub(super) fn limits_for(probed: Probed, attempts: u64) -> Limits {
    match probed {
        Probed::Hang => late(attempts),
        Probed::Answer
        | Probed::Refuse
        | Probed::Error
        | Probed::Unadvertised
        | Probed::Barrier
        | Probed::HangThenAnswer
        | Probed::RefuseThenAnswer
        | Probed::OtherRefusal
        | Probed::MalformedRefusal
        | Probed::BarrierRefuse
        | Probed::AnswerThenLinger
        | Probed::DescendantThenLinger => work(attempts),
    }
}

/// Wait for a barrier probe's `started` marker while `driver` drives the
/// engine, and hand the driver back. The wait is not what these tests
/// prove, so it is far past any load, as the seat's deadline is (`work`),
/// where each copy of it waited 20s (#1110). An engine that ends first
/// fails at once with the witness, so a park says what each member did
/// rather than timing out.
pub(super) fn barrier_awaited(
    started: &Path,
    driver: std::thread::JoinHandle<Engine>,
    dir: &Path,
    what: &str,
) -> std::thread::JoinHandle<Engine> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while !started.exists() {
        if driver.is_finished() {
            let engine = driver.join().unwrap();
            panic!(
                "the engine ended before {what} started:{}",
                witness(&engine, dir)
            );
        }
        assert!(std::time::Instant::now() < deadline, "{what} never started");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    driver
}

/// Drive `engine` until member `tag`'s barrier probe starts, append a stop
/// while it holds, release it, and hand back the engine, which the stop
/// ended.
pub(super) fn stopped_at_barrier(mut engine: Engine, dir: &Path, tag: &str) -> Engine {
    let db = dir.join("forge.db");
    let run = engine.run_id.clone();
    let driver = std::thread::spawn(move || {
        engine.drive().unwrap();
        engine
    });
    let started = dir.join(format!("{tag}.started"));
    let driver = barrier_awaited(&started, driver, dir, &format!("probe {tag}"));
    append_stop(&db, &run);
    std::fs::write(dir.join(format!("{tag}.release")), "").unwrap();
    let engine = driver.join().unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    assert_eq!(fold(&events).unwrap().run.status(), Status::Stopped);
    engine
}

/// An engine that ends before its barrier probe starts (this probe only
/// answers, so it never writes the marker) fails the wait at once with
/// what it journaled and what its shims did, not at the wait's end.
#[test]
fn a_barrier_wait_whose_engine_ends_first_says_what_it_did() {
    own_engine!();
    let (dir, mut engine, _now) = seated!(&["refuse", "succeed"], Probed::Answer, None, 2);
    let driver = std::thread::spawn(move || {
        engine.drive().unwrap();
        engine
    });
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        barrier_awaited(
            &dir.path().join("work.started"),
            driver,
            dir.path(),
            "the probe",
        )
    }))
    .expect_err("the barrier wait must panic");
    let message = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied());
    assert!(
        message.is_some_and(
            |text| text.contains("the engine ended before the probe started:\njournal:")
        ),
        "unexpected barrier panic: {message:?}"
    );
}

/// A failed fixture assertion says why (#1110): the shared witness names
/// the journal's events with the failure's words, every shim's log, and
/// the deadline the seat's attempt and probe were given.
#[test]
fn the_witness_names_the_journal_the_shims_and_the_deadlines() {
    own_engine!();
    let (dir, mut engine, _now) = seated!(&["refuse", "succeed"], Probed::Answer, None, 2);
    engine.drive().unwrap();
    let seen = witness(&engine, dir.path());
    for line in [
        "\n  5 effect/failed\n    error: \"provider refused; driver stderr: empty\"\n    waits: [{",
        "\n  6 effect/started\n  7 effect/succeeded\n",
        "\nshims:\n  all: [\"work:attempt:refuse\", \"work:probe\", \"work:attempt:succeed\"]\n  \
         work: [\"attempt:refuse\", \"probe\", \"attempt:succeed\"]\n",
        "\n  work: 60s per attempt and probe, 2 attempts",
    ] {
        assert!(seen.contains(line), "{line:?} is not in:{seen}");
    }
}
