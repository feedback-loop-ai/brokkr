//! The branch-coverage round for decision 0103 slice 4c1 (#430): the parks
//! and guards this slice added, each reached by a named test. The fixtures
//! are the suite's own: the fake driver, the v2 host file, the injected
//! clock and `fail_event`'s forced append failure.

use brokkr_core::fold::Cursor;
use brokkr_core::ProviderWait;

use super::*;
use crate::engine::cooldown::Waited;
use crate::engine::tests::{fail_event, state};
use crate::engine::EngineError;

/// A second run in the same store: another engine's start appends its
/// `run/started`, whose row moves the store-wide high water.
fn second_run(dir: &Path) -> String {
    let store = brokkr_store::Store::open(&dir.join("forge.db")).unwrap();
    let engine = Engine::start(
        store,
        bundle(dir, single_body(vec!["other".into()])),
        "Another feature",
        None,
    )
    .unwrap();
    engine.run_id
}

/// The seat the last probe invocation was told it ran as.
fn probe_seat(dir: &Path, tag: &str) -> String {
    std::fs::read_to_string(dir.join(format!("{tag}.seat"))).unwrap_or_default()
}

/// The four refusal and wait parks share one `run/parked` write, so one
/// forced append failure reaches its error arm: the store error propagates
/// out of `drive()`, and the refused park writes no row.
#[test]
fn a_park_that_cannot_journal_propagates_the_store_error() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = resumed_payload(
        dir.path(),
        2,
        json!({"effect_id": "effect", "attempt_id": "a1", "error": "refused",
               "start_failure": true, "start_failure_sites": [null],
               "refused": {"cause": "past-maximum", "provider": "claude",
                           "route": "first", "words": "rate_limit"},
               "waits": wait_value(base(), 1)}),
    );
    fail_event(&dir.path().join("forge.db"), "run/parked");
    let error = engine.drive().unwrap_err();
    assert!(matches!(error, EngineError::Store(_)), "{error}");
    let events = engine.store.load(&engine.run_id).unwrap();
    assert!(
        !events
            .iter()
            .any(|event| event.event_type == EventType::RunParked),
        "a refused park wrote a row"
    );
}

/// `Terminal::journal`'s two appends: the failed event's error arm writes
/// nothing, and the park's error arm leaves the failed event, which is
/// already the attempt's record. Both propagate out of `drive()`.
#[test]
fn a_journal_append_that_cannot_journal_propagates_the_store_error() {
    for (event_type, journaled) in [("effect/failed", 0usize), ("run/parked", 1)] {
        let (dir, mut engine, _now) = seated!(&["refuse"], Probed::Answer, Some(5), 2);
        engine.cooldown.host = Some(host(dir.path(), 1, 2));
        fail_event(&dir.path().join("forge.db"), event_type);
        let error = engine.drive().unwrap_err();
        assert!(
            matches!(error, EngineError::Store(_)),
            "{event_type}: {error}"
        );
        assert_eq!(
            failed(&engine).len(),
            journaled,
            "{event_type}: the failed event's row count"
        );
    }
}

/// The no-`until` side of `past || wait.is_none()` with `past` false: a
/// clock within the maximum's reach of the last instant RFC 3339 can spell
/// forms an end the journal cannot carry. The record says `no-end`, no
/// `waits` entry is invented, the park names no end, and nothing retries.
#[test]
fn a_wait_with_no_representable_end_inside_the_maximum_is_no_end() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = started(
        dir.path(),
        single_body(standard_command(dir.path())),
        work(2),
    );
    engine.cooldown.host = Some(host(dir.path(), 1, 10));
    let at = OffsetDateTime::parse("9999-12-31T23:59:59Z", &Rfc3339).unwrap();
    let (mut engine, _now) = with_clock_at(engine, at);
    drove(&mut engine, dir.path(), &["attempt:refuse"]);
    assert_eq!(starts(&engine).len(), 1);
    let failed = failed(&engine);
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].payload["refused"]["cause"], "no-end");
    assert_eq!(failed[0].payload["refused"]["seconds"], 1);
    assert_eq!(failed[0].payload["refused"]["max_seconds"], 10);
    no_waits(&engine);
    let reason = park_reason(&engine);
    assert!(reason.contains("has no representable end"), "{reason}");
    assert!(reason.contains("declared 10s maximum"), "{reason}");
    assert!(!reason.contains("until"), "{reason}");
}

/// The loop's stop check when the store-wide high water moved but this run
/// is still on its effect: another run's append is not this run's stop, so
/// the wait goes on and probes.
#[test]
fn a_second_runs_append_does_not_stop_the_wait() {
    let (dir, mut engine, now) = resumed(&["succeed"], base() + time::Duration::seconds(2), 2);
    let second = second_run(dir.path());
    let db = dir.path().join("forge.db");
    engine.cooldown.clock = clock_acting(&now, Acting::AfterFirstSlice, move || {
        let mut store = brokkr_store::Store::open(&db).unwrap();
        store
            .append_next(
                &second,
                EventType::OperatorCommanded,
                json!({"command_id": "c", "command": "stop", "args": {},
                       "operator": "operator"}),
                None,
                None,
            )
            .unwrap();
    });
    let events = engine.store.load(&engine.run_id).unwrap();
    let state = fold(&events).unwrap();
    let wait = engine.recorded_wait(&events, "effect").unwrap().unwrap();
    match engine
        .take_cooldown(&events, &state, "effect", "work", &wait)
        .unwrap()
    {
        Waited::Proceed(_) => {}
        Waited::Stopped => panic!("a second run's append stopped this run's wait"),
        Waited::Parked(reason) => panic!("the wait parked: {reason}"),
    }
    assert_eq!(logs(dir.path(), "work"), ["probe"]);
}

/// The stop check after the loop: the operator's stop lands on the call
/// that first finds the wait ended, after the last slice, so nothing probes
/// or retries and the stop's own arm writes the ending.
#[test]
fn a_stop_after_the_last_slice_is_seen_before_the_probe() {
    let (dir, mut engine, now, db, run_id) = stop_fixture();
    // The wait's own end: the terminal formed it from this clock, so the
    // stop lands on the loop's exit read, never on the terminal's.
    let until = *now.lock().unwrap() + time::Duration::seconds(2);
    engine.cooldown.clock =
        clock_acting(&now, Acting::At(until), move || append_stop(&db, &run_id));
    engine.drive().unwrap();
    assert_stopped(dir.path(), &engine);
}

/// A cased seat's probe is told the same `<seat>:<case>` site name as its
/// attempt, so the probe's invocation is labelled as the retried one.
#[test]
fn the_probe_names_a_cased_site_as_the_attempt_does() {
    let dir = tempfile::tempdir().unwrap();
    let command = driver(
        dir.path(),
        "work",
        &["refuse", "succeed"],
        Probed::Answer,
        None,
        "claude",
        "first",
    );
    let body = SeatBody::Select {
        cases: [("chore".to_string(), single_body(command))].into(),
        default: None,
    };
    let mut engine = started(dir.path(), body, work(2));
    engine.cooldown.host = Some(host(dir.path(), 1, 10));
    let (mut engine, _now) = with_clock(engine);
    let mut current = state(
        Some("work"),
        Cursor::ExecuteEffect {
            effect_id: "effect".into(),
            seat: "work".into(),
            failed_attempts: 1,
        },
    );
    current.strategy = Some("chore".into());
    let events = engine.store.load(&engine.run_id).unwrap();
    let wait = ProviderWait {
        site: None,
        provider: "claude".into(),
        route: "first".into(),
        words: "rate_limit: slow down".into(),
        retry_after_seconds: None,
        seconds: 1,
        until: base() + time::Duration::seconds(1),
        rounds: 1,
    };
    match engine
        .probe(&events, &current, "effect", "work", &wait)
        .unwrap()
    {
        super::super::probe::Probe::Answered(answer) => {
            assert_eq!(answer.provider, "claude");
            assert_eq!(answer.route, "first");
        }
        super::super::probe::Probe::Parked(reason) => {
            panic!("the probe parked: {reason}")
        }
    }
    assert_eq!(probe_seat(dir.path(), "work"), "work:chore");
}

/// The probe under a held lease takes the door, rechecks it and answers;
/// the retry then runs. The lease is the admission suite's own fixture, so
/// the held branch of the probe's door is exercised without a new
/// mechanism.
#[test]
fn a_probe_under_a_held_lease_answers_and_retries() {
    let (dir, mut engine, _now) = seated!(&["refuse", "succeed"], Probed::Answer, None, 2);
    let home = dir.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let worktree = dir.path().join("work");
    engine.held = Some(crate::engine::broker::admission::tests::protecting(
        &home, &worktree,
    ));
    drove(
        &mut engine,
        dir.path(),
        &["attempt:refuse", "probe", "attempt:succeed"],
    );
    assert_eq!(probe_seat(dir.path(), "work"), "work");
}
