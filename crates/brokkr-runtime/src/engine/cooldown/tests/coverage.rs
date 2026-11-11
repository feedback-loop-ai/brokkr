//! The branch-coverage round for decision 0103 slice 4c1 (#430): the parks
//! and guards this slice added, each reached by a named test. The fixtures
//! are the suite's own: the fake driver, the v2 host file, the injected
//! clock and `fail_event`'s forced append failure.

use brokkr_core::fold::Cursor;
use brokkr_core::ProviderWait;

use super::panel::member;
use super::*;
#[cfg(target_os = "linux")]
use crate::engine::broker::admission::tests::{reservation_path, Coordinated};
use crate::engine::cooldown::Waited;
#[cfg(target_os = "linux")]
use crate::engine::tests::in_its_own_engine;
use crate::engine::tests::{fail_event, state};
use crate::engine::EngineError;

/// The wait these rows probe for claude/first, at `site`.
fn rate_limit_wait(site: Option<&str>) -> ProviderWait {
    ProviderWait {
        site: site.map(str::to_string),
        provider: "claude".into(),
        route: "first".into(),
        words: "rate_limit: slow down".into(),
        retry_after_seconds: None,
        seconds: 1,
        until: base() + time::Duration::seconds(1),
        rounds: 1,
    }
}

/// A seat-level panel member's probe with no phase refuses the engine
/// invariant rather than admitting: `phase_of` fails before the probe and
/// `probe_member` propagates that error arm.
#[test]
fn a_probe_member_without_a_phase_refuses_the_invariant() {
    let dir = tempfile::tempdir().unwrap();
    let body = SeatBody::Invocation(Invocation::Panel {
        members: vec![member("m", Vec::new())],
        aggregate: Aggregate::ReviewPanel,
    });
    let mut engine = started(dir.path(), body, work(2));
    engine.cooldown.host = Some(host(dir.path(), 1, 10));
    request_effect(&mut engine);
    let events = engine.store.load(&engine.run_id).unwrap();
    let mut current = fold(&events).unwrap();
    current.phase = None;
    let wait = rate_limit_wait(Some("m"));
    let error = match engine.probe_member(&events, &current, "effect", "work", &wait, "m") {
        Err(error) => error,
        Ok(_) => panic!("a phase-less probe was admitted"),
    };
    assert!(
        matches!(error, EngineError::Invariant("cursor has phase", _)),
        "{error}"
    );
}

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
    assert!(
        reason.contains("the computed wait is 1s, and the declared 10s maximum"),
        "{reason}"
    );
    assert!(
        !reason.contains("past the declared 10s maximum"),
        "a wait inside the maximum must not say it was passed: {reason}"
    );
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
        .take_cooldown(
            &events,
            &state,
            "effect",
            "work",
            std::slice::from_ref(&wait),
        )
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

/// The cased engine the two `work:chore` rows drive: a selector whose one
/// `chore` case is claude/first, the standard host and the test clock.
fn chore_engine(dir: &Path) -> Engine {
    let command = driver(
        dir,
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
    let mut engine = started(dir, body, work(2));
    engine.cooldown.host = Some(host(dir, 1, 10));
    with_clock(engine).0
}

/// A cased seat's probe is told the same `<seat>:<case>` site name as its
/// attempt, so the probe's invocation is labelled as the retried one.
#[test]
fn the_probe_names_a_cased_site_as_the_attempt_does() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = chore_engine(dir.path());
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
    let wait = rate_limit_wait(None);
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

/// F1: the cleanup label's case half is the selected case or the
/// selector's default, never the bare seat name; only an uncased seat's
/// label is the seat alone.
#[test]
fn selected_label_names_the_selected_case_or_default() {
    let selecting = tempfile::tempdir().unwrap();
    let body = SeatBody::Select {
        cases: [("chore".to_string(), single_body(vec!["chore".into()]))].into(),
        default: Some(Box::new(single_body(vec!["fallback".into()]))),
    };
    let engine = started(selecting.path(), body, work(2));
    assert_eq!(engine.selected_label("work", Some("chore")), "work:chore");
    assert_eq!(engine.selected_label("work", None), "work:default");
    assert_eq!(
        engine.selected_label("work", Some("absent")),
        "work:default"
    );

    let uncased = tempfile::tempdir().unwrap();
    let engine = started(uncased.path(), single_body(vec!["unused".into()]), work(2));
    assert_eq!(engine.selected_label("work", None), "work");
    assert_eq!(engine.selected_label("work", Some("chore")), "work");
}

/// The selector's own first attempt: the phase, the requested effect and
/// the driver's one typed refusal, leaving the journal's site-less wait.
/// The effect id is the one the cursor executes from.
#[cfg(target_os = "linux")]
fn refused_cased_attempt(engine: &mut Engine, dir: &Path) -> String {
    engine
        .append(EventType::PhaseEntered, json!({"phase": "work"}), None)
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let mut current = state(Some("work"), Cursor::RequestEffect);
    current.strategy = Some("chore".into());
    engine
        .advance_running(&events, Cursor::RequestEffect, current)
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let effect = events
        .iter()
        .find(|event| event.event_type == EventType::EffectRequested)
        .map(|event| event.payload["effect_id"].as_str().unwrap().to_string())
        .expect("the requested effect");
    let cursor = Cursor::ExecuteEffect {
        effect_id: effect.clone(),
        seat: "work".into(),
        failed_attempts: 0,
    };
    let mut current = state(Some("work"), cursor.clone());
    current.strategy = Some("chore".into());
    engine.advance_running(&events, cursor, current).unwrap();
    assert_eq!(
        logs(dir, "work"),
        ["attempt:refuse"],
        "the first attempt did not refuse"
    );
    effect
}

/// The crashed probe's reservation: attempt `a2`, no journal event naming
/// it, its driver at the selected label unsettled.
#[cfg(target_os = "linux")]
fn plant_crashed_probe(engine: &mut Engine, home: &Path) {
    let events = engine.store.load(&engine.run_id).unwrap();
    let observed = crate::engine::broker::observed();
    let seat_body = engine.bundle.seats["work"].body.clone();
    let selected = seat_body.selected(Some("chore")).unwrap().0;
    engine
        .admit_for_probe(
            &events,
            ["a2", "work:chore"],
            selected,
            &crate::engine::Selection::new(),
            (std::time::Instant::now(), std::time::Duration::from_secs(5)),
            &observed,
        )
        .unwrap();
    drop(engine.held.take());
    let at = reservation_path(home, &engine.workdir(), &engine.run_id);
    let mut sealed: Value = serde_json::from_slice(&std::fs::read(&at).unwrap()).unwrap();
    let reserved = sealed["attempts"]
        .as_object_mut()
        .expect("the planted reservation stands")
        .get_mut("a2")
        .expect("the planted probe attempt");
    reserved["cleanup"] = json!("settling");
    reserved["drivers"] = json!([{"site": "work:chore", "begun": null, "cleanup": "settling"}]);
    std::fs::write(&at, serde_json::to_vec(&sealed).unwrap()).unwrap();
}

/// F1: the cased label is bound end to end. A selector's `chore` case runs
/// as `work:chore`; a crashed probe's reservation holds a driver at that
/// label, unsettled and named by no journal event, beside the attempt's one
/// site-less claude/first wait. Driven from the recorded wait, the cleanup
/// refusal names claude/first only if the strategy reached the label; a
/// bare `work` leaves it neutral.
#[cfg(target_os = "linux")]
#[test]
fn a_cased_seats_crashed_probe_reservation_parks_naming_its_route() {
    let test = "engine::cooldown::tests::coverage::a_cased_seats_crashed_probe_reservation_parks_naming_its_route";
    if !in_its_own_engine(test) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let home = super::lease::home(dir.path());
    let _stand = Coordinated::at(&home);
    let mut engine = chore_engine(dir.path());
    let effect = refused_cased_attempt(&mut engine, dir.path());
    plant_crashed_probe(&mut engine, &home);
    // The cursor from the recorded wait: the refusal names claude/first only
    // if the selected case reaches the label the reserved driver carries.
    let events = engine.store.load(&engine.run_id).unwrap();
    let cursor = Cursor::ExecuteEffect {
        effect_id: effect,
        seat: "work".into(),
        failed_attempts: 1,
    };
    let mut current = state(Some("work"), cursor.clone());
    current.strategy = Some("chore".into());
    engine.advance_running(&events, cursor, current).unwrap();
    let reason = park_reason(&engine);
    assert!(
        reason.starts_with(
            "the cool-down probe for claude/first did not prove its process tree gone"
        ),
        "{reason}"
    );
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
    assert_eq!(starts(&engine).len(), 1, "an effect attempt was spent");
    assert_eq!(failed(&engine).len(), 1, "an effect attempt was spent");
}

/// F2: a driven probe's spawn is witnessed on its own lease at the door
/// before it starts, so the door is proved and not merely passed. The probe
/// is held open at its barrier while the test reads the reservation's one
/// driver record: site `work`, `settling`. With the door removed the probe
/// still answers, but no driver record is sealed, and this fails by name.
/// It runs in an engine of its own: one test binary is one process, and the
/// engine adopts every direct child outside its group, so a co-scheduled
/// test's live probe would read as a stray and park this one before its
/// probe starts.
#[cfg(target_os = "linux")]
#[test]
fn a_probes_spawn_witness_is_sealed_at_its_door_before_it_answers() {
    let test = "engine::cooldown::tests::coverage::a_probes_spawn_witness_is_sealed_at_its_door_before_it_answers";
    if !in_its_own_engine(test) {
        return;
    }
    let (dir, engine, _now) = seated!(&["refuse", "succeed"], Probed::Barrier, None, 2);
    let home = super::lease::home(dir.path());
    let worktree = dir.path().join("work");
    let run = engine.run_id.clone();
    let (release, driver) = super::lease::barrier_started(&home, engine);
    let at = reservation_path(&home, &worktree, &run);
    let sealed: Value = serde_json::from_slice(&std::fs::read(&at).unwrap()).unwrap();
    let attempts = sealed["attempts"]
        .as_object()
        .expect("the probe's attempt stands");
    assert_eq!(attempts.len(), 1, "the probe's one attempt: {sealed}");
    let reserved = attempts.values().next().unwrap();
    let drivers = reserved
        .get("drivers")
        .and_then(Value::as_array)
        .map(|drivers| {
            drivers
                .iter()
                .map(|driver| (driver["site"].clone(), driver["cleanup"].clone()))
                .collect::<Vec<_>>()
        });
    assert_eq!(
        drivers,
        Some(vec![(json!("work"), json!("settling"))]),
        "the door sealed no spawn witness for the probe: {sealed}"
    );
    std::fs::write(&release, "").unwrap();
    let engine = driver.join().unwrap();
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
    assert_eq!(starts(&engine).len(), 2);
}
