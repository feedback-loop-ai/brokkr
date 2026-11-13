//! Decision 0103 slice 4d2b: a probe's rate-limit refusal appends the next
//! round at the terminal's own `(run_id, event_seq)`, extends the wait
//! without spending an attempt, and the aggregate is re-read after the
//! probes and before the retry.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{fold, Status};
use brokkr_protocol::{Cleanup, PreSessionRefusal};
use serde_json::json;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

use super::*;
use crate::engine::Engine;

/// The instant a test's clock reads when its run starts.
fn start_of(now: &Arc<Mutex<OffsetDateTime>>) -> OffsetDateTime {
    *now.lock().unwrap()
}

/// Run `sql` against a journal from a second connection.
fn alter_journal(db: &Path, sql: &'static str) {
    rusqlite::Connection::open(db)
        .unwrap()
        .execute_batch(sql)
        .unwrap();
}

/// Replace the engine's clock with one that acts once, after its first
/// wait slice.
fn after_first_slice(
    engine: &mut Engine,
    now: &Arc<Mutex<OffsetDateTime>>,
    act: impl FnMut() + Send + 'static,
) {
    engine.cooldown.clock = clock_acting(now, Acting::AfterFirstSlice, act);
}

/// Drive a run whose probe ended without an answer: one attempt, the given
/// driver log, and the park reason.
fn drove_one(dir: &Path, engine: &mut Engine, expected: &[&str]) -> String {
    engine.drive().unwrap();
    assert_eq!(logs(dir, "work"), expected);
    assert_eq!(starts(engine).len(), 1, "the probe spent an attempt");
    park_reason(engine)
}

/// Stop the run once, at `at`, during its wait.
fn stop_after(
    engine: &mut Engine,
    now: &Arc<Mutex<OffsetDateTime>>,
    db: PathBuf,
    run: String,
    at: OffsetDateTime,
) {
    engine.cooldown.clock = clock_acting(now, Acting::At(at), move || append_stop(&db, &run));
}

/// One site's stored rounds on `event_seq`, in round order.
fn rounds(engine: &Engine, event_seq: u64, site: &str) -> Vec<u64> {
    engine
        .store
        .cooldown_rounds(&engine.run_id, event_seq, site)
        .unwrap()
        .into_iter()
        .map(|row| row.rounds)
        .collect()
}

/// The fact both maximum parks share: the terminal's own row alone, no row
/// for the round past the maximum, and the maximum named.
fn only_the_own_row_and_the_maximum(engine: &Engine, expected: &str) {
    let event = failed(engine).remove(0);
    assert_eq!(
        rounds(engine, event.seq, ""),
        [1],
        "the round past the maximum wrote a row"
    );
    let reason = park_reason(engine);
    assert!(reason.contains(expected), "{reason}");
    assert!(reason.contains("declared 100s maximum"), "{reason}");
}

/// A second run in the same journal, ready for its own refusal row.
fn second_run(dir: &Path) -> String {
    let store = brokkr_store::Store::open(&dir.join("forge.db")).unwrap();
    let engine = Engine::start(
        store,
        crate::engine::tests::bundle(dir, single_body(vec!["other".into()])),
        "Another feature",
        None,
    )
    .unwrap();
    engine.run_id
}

/// Append a second run's own terminal refusal for claude/first, ending at
/// `until`: another run's aggregate row on the route run-A cools.
fn append_second_row(db: &Path, run_id: &str, until: OffsetDateTime) {
    let mut store = brokkr_store::Store::open(db).unwrap();
    store
        .append_with_cooldowns(
            brokkr_store::CooldownAppend {
                run_id,
                fenced: None,
                event_type: EventType::EffectFailed,
                payload: json!({
                    "effect_id": "b-effect",
                    "waits": [{
                        "provider": "claude", "route": "first",
                        "words": "run-b refused", "seconds": 5,
                        "until": until.format(&Rfc3339).unwrap(), "rounds": 1
                    }]
                }),
                causation_id: None,
                attempt_id: None,
            },
            &[brokkr_store::Cooling {
                site: String::new(),
                provider: "claude".into(),
                route: "first".into(),
                until_at: until,
                reason: "run-b refused".into(),
                retry_after_seconds: None,
                rounds: 1,
            }],
        )
        .unwrap();
}

/// The probes a retry's `effect/started` records, in probed order.
fn retry_probes(engine: &Engine) -> Vec<Value> {
    starts(engine)
        .pop()
        .and_then(|start| start["probes"].as_array().cloned())
        .unwrap_or_default()
}

/// A probe answer's `answered_at`, parsed.
fn answered_at(probe: &Value) -> OffsetDateTime {
    let at = probe["answered_at"]
        .as_str()
        .expect("an answer records when it was taken");
    OffsetDateTime::parse(at, &Rfc3339).expect("an answer's instant is RFC 3339")
}

/// A second run in the journal and its database, the row's destination.
fn second_run_route(dir: &Path) -> (String, PathBuf) {
    (second_run(dir), dir.join("forge.db"))
}

/// The single-seat fixture both cross-run tests share: a refused attempt
/// and a wide enough maximum that another run's row can be waited.
fn seated_for_two_runs() -> (tempfile::TempDir, Engine, Arc<Mutex<OffsetDateTime>>) {
    let (dir, mut engine, now) = seated!(&["refuse", "succeed"], Probed::Answer, None, 2);
    engine.cooldown.host = Some(host(dir.path(), 30, 1000));
    (dir, engine, now)
}

/// The one action a clock runs, taken once it fires.
type ProbeAction = Arc<Mutex<Option<Box<dyn FnMut() + Send>>>>;

/// A clock over the test's shared instant, whose reads do `read` and whose
/// slices advance it: the one sleep seam every planted action shares.
fn clock_from(
    now: &Arc<Mutex<OffsetDateTime>>,
    read: impl Fn() -> OffsetDateTime + Send + Sync + 'static,
) -> Clock {
    let advance = Arc::clone(now);
    Clock {
        now: Box::new(read),
        sleep: Box::new(move |slice| {
            *advance.lock().unwrap() += Duration::seconds(slice.as_secs() as i64)
        }),
    }
}

/// A clock that advances on each slice and runs `act` when the probe's own
/// `work.done` marker exists: once, or on every read for a test that plants
/// a row per probe.
fn clock_on_probe(
    now: &Arc<Mutex<OffsetDateTime>>,
    dir: &Path,
    once: bool,
    act: impl FnMut() + Send + 'static,
) -> Clock {
    let marker = dir.join("work.done");
    let done = Arc::new(Mutex::new(false));
    let read = Arc::clone(now);
    let fired = Arc::clone(&done);
    let action: ProbeAction = Arc::new(Mutex::new(Some(Box::new(act))));
    clock_from(now, move || {
        if marker.exists() && (!once || !*fired.lock().unwrap()) {
            *fired.lock().unwrap() = true;
            if let Some(action) = action.lock().unwrap().as_mut() {
                action();
            }
        }
        *read.lock().unwrap()
    })
}

/// A probe's rate-limit refusal writes the next round at the terminal's own
/// `(run_id, event_seq)`, the engine waits the extended end, the second
/// probe answers, and the retry runs with `failed_attempts` unchanged.
#[test]
fn a_probe_refusal_extends_the_wait_without_an_attempt() {
    let (dir, mut engine, now) = seated!(&["refuse", "succeed"], Probed::RefuseThenAnswer, None, 2);
    let start = start_of(&now);
    // A length no loaded host's event-recording delay outlasts, so the
    // terminal's own row lands before it records (as 4d1's fixtures do).
    engine.cooldown.host = Some(host(dir.path(), 30, 1000));
    engine.drive().unwrap();
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "probe", "attempt:succeed"]
    );
    assert_eq!(starts(&engine).len(), 2, "the retry is the second attempt");
    let mut failed_events = failed(&engine);
    let effect = failed_events[0].payload["effect_id"].clone();
    assert_eq!(
        failed_events
            .iter()
            .filter(|event| event.payload["effect_id"] == effect)
            .count(),
        1,
        "the probe spent an attempt"
    );
    assert!(
        *now.lock().unwrap() >= start + Duration::seconds(90),
        "the wait did not run to the extended end"
    );
    let event = failed_events.remove(0);
    assert_eq!(
        rounds(&engine, event.seq, ""),
        [1, 2],
        "the probe's round is the terminal's next one"
    );
    let probes = retry_probes(&engine);
    assert_eq!(
        probes.len(),
        1,
        "a probe answer was recorded twice: {probes:?}"
    );
    assert_eq!(probes[0]["provider"], "claude");
    assert!(
        answered_at(&probes[0]) >= start + Duration::seconds(90),
        "the retry recorded the first round's answer: {probes:?}"
    );
}

/// Run-B's row on the same provider and route outlasts run-A's own `until`:
/// run-A waits until run-B's ends, not its own.
#[test]
fn two_overlapping_refusals_from_two_runs_hold_until_the_longer_ends() {
    let (dir, mut engine, now) = seated_for_two_runs();
    let start = start_of(&now);
    let (b, db) = second_run_route(dir.path());
    append_second_row(&db, &b, start + Duration::seconds(60));
    engine.drive().unwrap();
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
    assert!(
        *now.lock().unwrap() >= start + Duration::seconds(60),
        "run-A ended at its own until, before run-B's"
    );
    assert_eq!(starts(&engine).len(), 2);
}

/// A row recorded while the probe ran holds the route again: the re-read
/// after the probes waits it out and probes again before the retry.
#[test]
fn a_row_recorded_while_the_probe_ran_holds_the_route_again() {
    let (dir, mut engine, now) = seated_for_two_runs();
    let start = start_of(&now);
    let (b, db) = second_run_route(dir.path());
    engine.cooldown.clock = clock_on_probe(&now, dir.path(), true, move || {
        append_second_row(&db, &b, start + Duration::seconds(120));
    });
    engine.drive().unwrap();
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "probe", "attempt:succeed"],
        "the row recorded while the probe ran was not waited out and probed again"
    );
    assert!(
        *now.lock().unwrap() >= start + Duration::seconds(120),
        "the retry ran before the row recorded during the probe ended"
    );
    assert_eq!(starts(&engine).len(), 2);
    let probes = retry_probes(&engine);
    assert_eq!(
        probes.len(),
        1,
        "a probe answer was recorded twice: {probes:?}"
    );
    assert!(
        answered_at(&probes[0]) >= start + Duration::seconds(120),
        "the retry recorded the answer taken before the row landed: {probes:?}"
    );
}

/// The episode maximum parks the probe loop at the round that would pass
/// it, names the maximum, and writes no row for that round.
#[test]
fn the_episode_maximum_parks_the_probe_loop_and_writes_no_row_for_the_round() {
    let (dir, mut engine, _now) = seated!(&["refuse", "succeed"], Probed::Refuse, None, 3);
    engine.cooldown.host = Some(host(dir.path(), 30, 100));
    let reason = drove_one(
        dir.path(),
        &mut engine,
        &["attempt:refuse", "probe", "probe"],
    );
    let event = failed(&engine).remove(0);
    assert_eq!(
        rounds(&engine, event.seq, ""),
        [1, 2],
        "the round past the maximum wrote a row"
    );
    assert!(
        reason
            .contains("would wait 120s in round 3 (episode 210s), past the declared 100s maximum"),
        "{reason}"
    );
    assert!(reason.contains("rate_limit: slow down"), "{reason}");
}

/// An operator `stop` during the extended wait ends the run: no probe and
/// no retry follows it.
#[test]
fn a_stop_during_the_extended_wait_ends_the_run_without_a_probe_or_retry() {
    let (dir, mut engine, now) = seated!(&["refuse", "succeed"], Probed::RefuseThenAnswer, None, 2);
    let start = start_of(&now);
    engine.cooldown.host = Some(host(dir.path(), 30, 1000));
    let run_id = engine.run_id.clone();
    stop_after(
        &mut engine,
        &now,
        dir.path().join("forge.db"),
        run_id,
        start + Duration::seconds(45),
    );
    engine.drive().unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    assert_eq!(fold(&events).unwrap().run.status(), Status::Stopped);
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse", "probe"]);
    assert_eq!(starts(&engine).len(), 1);
    let event = failed(&engine).remove(0);
    assert_eq!(rounds(&engine, event.seq, ""), [1, 2]);
}

/// A version 1, 2 or 3 journal has no table for a probe's next round: the
/// refusal parks as 4c does and writes no row.
#[test]
fn a_version_three_journal_parks_a_probe_refusal_as_4c_and_writes_no_row() {
    let (dir, mut engine, _now) = seated!(&["refuse", "succeed"], Probed::Refuse, None, 2);
    let db = dir.path().join("forge.db");
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute("UPDATE meta SET value = '3' WHERE key = 'queue_schema'", [])
        .unwrap();
    let reason = drove_one(dir.path(), &mut engine, &["attempt:refuse", "probe"]);
    assert!(
        reason.contains("was refused again by the provider"),
        "{reason}"
    );
    assert!(reason.contains("rate_limit: slow down"), "{reason}");
    let count: i64 = rusqlite::Connection::open(&db)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM provider_cooldowns", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 0, "a version 3 journal wrote a probe round");
}

/// A store refusal while reading the aggregate parks by the store's words,
/// never as an absent aggregate, and retries nothing.
#[test]
fn a_store_refusal_while_reading_the_aggregate_parks_by_name() {
    let (dir, mut engine, now) = seated!(&["refuse", "succeed"], Probed::Answer, None, 2);
    let db = dir.path().join("forge.db");
    after_first_slice(&mut engine, &now, move || {
        alter_journal(&db, "DROP TABLE provider_cooldowns")
    });
    let reason = drove_one(dir.path(), &mut engine, &["attempt:refuse", "probe"]);
    assert!(
        reason.contains("the cool-down aggregate for claude/first cannot be read"),
        "{reason}"
    );
    assert!(
        reason.contains("no such table: provider_cooldowns"),
        "{reason}"
    );
}

/// The unchanged probe ends — a hang, an error, another typed refusal and a
/// malformed present refusal — each park and write no probe row.
#[test]
fn the_unchanged_probe_ends_write_no_row() {
    for (probed, expected) in [
        (Probed::Hang, "ended without an answer"),
        (Probed::Error, "probe exploded"),
        (Probed::OtherRefusal, "was refused: auth failed"),
        (Probed::MalformedRefusal, "driver refusal is unreadable"),
    ] {
        let (dir, mut engine, _now) = seated!(&["refuse", "succeed"], probed, None, 2);
        // Long enough that the terminal's own row lands however delayed the
        // host's recording is; only a probe row would be new.
        engine.cooldown.host = Some(host(dir.path(), 30, 1000));
        let reason = drove_one(dir.path(), &mut engine, &["attempt:refuse", "probe"]);
        assert!(reason.contains(expected), "{expected}: {reason}");
        let event = failed(&engine).remove(0);
        assert_eq!(
            rounds(&engine, event.seq, ""),
            [1],
            "{expected}: a non-rate-limit end wrote a probe row"
        );
    }
}

/// Cleanup's refusal speaks first even when the probe also reported a rate
/// limit, and writes no row: the tree is not proven gone, so nothing may
/// extend the wait.
#[test]
fn a_probes_cleanup_refusal_parks_first_and_writes_no_row() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = started(dir.path(), single_body(vec!["unused".into()]), work(2));
    let refusal: PreSessionRefusal = serde_json::from_value(json!({
        "class": "rate_limit", "provider": "claude", "route": "first",
        "words": "rate_limit: slow down"
    }))
    .unwrap();
    let report = report(
        Some(Box::new(refusal)),
        Cleanup::Unresolved {
            reason: brokkr_protocol::process::Unsettled::Group { group: 7 },
        },
        vec![json!({"step": "answer"})],
    );
    match engine.probe_end("effect", &wait(), report) {
        super::super::Probe::Parked(reason) => {
            assert!(reason.contains("cleanup failure"), "{reason}");
            assert!(reason.contains("group 7"), "{reason}");
            assert!(!reason.contains("rate_limit: slow down"), "{reason}");
        }
        _ => panic!("a cleanup refusal did not park"),
    }
    assert!(
        engine
            .store
            .cooldowns_in_force(OffsetDateTime::now_utc())
            .unwrap()
            .is_empty(),
        "a cleanup refusal wrote a row"
    );
}

/// A probe's rate-limit refusal parks by name when its bounds, its
/// journal, its rounds or its episode cannot be read, and writes no row.
#[test]
fn a_probe_refusal_parks_when_its_declaration_journal_or_episode_cannot_read() {
    let refusal = || a_rate_limit_with(Some("first"), None);
    let with_refusal = || {
        report(
            Some(Box::new(refusal())),
            Cleanup::Settled,
            vec![json!({"step": "answer"})],
        )
    };
    let parked = |engine: &mut Engine, expected: &str| match engine.probe_end(
        "effect",
        &wait(),
        with_refusal(),
    ) {
        super::super::Probe::Parked(reason) => {
            assert!(reason.contains(expected), "{reason} omits {expected}")
        }
        _ => panic!("a refused probe did not park for {expected}"),
    };

    // No declaration at all.
    let dir = tempfile::tempdir().unwrap();
    let mut engine = started(dir.path(), single_body(vec!["unused".into()]), work(2));
    parked(&mut engine, "no cool-down is declared");
    // A declaration that does not read.
    let invalid = dir.path().join("invalid.json");
    std::fs::write(&invalid, "not json").unwrap();
    engine.cooldown.host = Some(invalid);
    parked(&mut engine, "bounds cannot be read");

    // A journal whose events cannot be read.
    let dir = tempfile::tempdir().unwrap();
    let mut engine = started(dir.path(), single_body(vec!["unused".into()]), work(2));
    engine.cooldown.host = Some(host(dir.path(), 30, 1000));
    rusqlite::Connection::open(dir.path().join("forge.db"))
        .unwrap()
        .execute_batch("DROP TABLE provider_cooldowns; DROP TABLE events;")
        .unwrap();
    // The store's own drop check needs the events table, so this engine is
    // leaked after the call; the tempdir still removes its file.
    let reason = match engine.probe_end("effect", &wait(), with_refusal()) {
        super::super::Probe::Parked(reason) => reason,
        _ => panic!("a refused probe did not park for journal cannot be read"),
    };
    std::mem::forget(engine);
    assert!(reason.contains("journal cannot be read"), "{reason}");

    // No terminal event to extend.
    let dir = tempfile::tempdir().unwrap();
    let mut engine = started(dir.path(), single_body(vec!["unused".into()]), work(2));
    engine.cooldown.host = Some(host(dir.path(), 30, 1000));
    parked(&mut engine, "no terminal event to extend");

    // A version this binary does not know: the rounds cannot be read.
    let dir = tempfile::tempdir().unwrap();
    let mut engine = started(dir.path(), single_body(vec!["unused".into()]), work(2));
    engine.cooldown.host = Some(host(dir.path(), 30, 1000));
    append_waited(
        &mut engine,
        wait_value(OffsetDateTime::now_utc() + Duration::hours(1), 60),
    );
    rusqlite::Connection::open(dir.path().join("forge.db"))
        .unwrap()
        .execute("UPDATE meta SET value = '9' WHERE key = 'queue_schema'", [])
        .unwrap();
    parked(&mut engine, "rounds cannot be read");

    // An earlier wait that does not read makes the episode unreadable.
    let dir = tempfile::tempdir().unwrap();
    let mut engine = started(dir.path(), single_body(vec!["unused".into()]), work(2));
    engine.cooldown.host = Some(host(dir.path(), 30, 1000));
    append_waited(&mut engine, json!([{"route": "first"}]));
    engine
        .append(
            EventType::EffectFailed,
            json!({"effect_id": "effect", "attempt_id": "a2",
                   "waits": wait_value(OffsetDateTime::now_utc() + Duration::hours(1), 60)}),
            Some("a2".to_string()),
        )
        .unwrap();
    parked(&mut engine, "episode cannot be read");
}

/// A rows-bearing store refusal at the probe's own append — here the table
/// lost once the probe ran — parks naming the store's words, never silently.
#[test]
fn a_lost_table_at_the_probe_append_parks_naming_the_store() {
    let (dir, mut engine, now) = seated!(&["refuse", "succeed"], Probed::RefuseThenAnswer, None, 2);
    engine.cooldown.host = Some(host(dir.path(), 30, 1000));
    let db = dir.path().join("forge.db");
    engine.cooldown.clock = clock_on_probe(&now, dir.path(), true, move || {
        alter_journal(&db, "DROP TABLE provider_cooldowns");
    });
    let reason = drove_one(dir.path(), &mut engine, &["attempt:refuse", "probe"]);
    assert!(
        reason.contains("could not record its next round"),
        "{reason}"
    );
    assert!(
        reason.contains("no such table: provider_cooldowns"),
        "{reason}"
    );
}

/// A terminal whose own row never landed (recorded on a version 3 queue, or
/// skipped because its end had passed) is filled at round one, the store's
/// next round: the probe's refusal extends the wait instead of parking on a
/// round the table refuses.
#[test]
fn a_probe_refusal_fills_a_terminal_whose_own_row_is_absent() {
    let dir = tempfile::tempdir().unwrap();
    let command = driver(
        dir.path(),
        "work",
        &["succeed"],
        Probed::RefuseThenAnswer,
        None,
        "claude",
        "first",
    );
    let mut engine = started(dir.path(), single_body(command), work(2));
    engine.cooldown.host = Some(host(dir.path(), 30, 1000));
    let (clock, now) = Clock::advancing(base());
    engine.cooldown.clock = clock;
    // The terminal records a wait whose end has already passed, so `insert`
    // writes no row for it and the site stands at round zero.
    append_waited(&mut engine, wait_value(base() - Duration::seconds(1), 30));
    engine.drive().unwrap();
    assert_eq!(
        logs(dir.path(), "work"),
        ["probe", "probe", "attempt:succeed"],
        "the probe's refusal did not fill round one and extend the wait"
    );
    let event = failed(&engine).remove(0);
    assert_eq!(
        rounds(&engine, event.seq, ""),
        [1],
        "the absent own row was not filled at round one"
    );
    assert!(
        *now.lock().unwrap() >= base() + Duration::seconds(30),
        "the filled round was not waited"
    );
}

/// A remainder another run's row already holds is judged before the first
/// probe: the episode's seconds and the remainder past its own end pass the
/// declared maximum together, and no probe runs.
#[test]
fn a_remainder_already_in_force_counts_toward_the_maximum_before_the_first_probe() {
    let (dir, mut engine, now) = seated!(&["refuse", "succeed"], Probed::Answer, None, 2);
    engine.cooldown.host = Some(host(dir.path(), 30, 100));
    let start = start_of(&now);
    let b = second_run(dir.path());
    append_second_row(
        &dir.path().join("forge.db"),
        &b,
        start + Duration::seconds(120),
    );
    engine.drive().unwrap();
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse"],
        "a probe ran past the maximum"
    );
    let reason = park_reason(&engine);
    assert!(reason.contains("100s maximum"), "{reason}");
    assert_eq!(starts(&engine).len(), 1, "the run retried past the maximum");
}

/// Another run's rows keep holding the route while this run probes: each
/// remainder is summed against the episode, so the loop parks at the round
/// that would pass the maximum rather than waiting rows forever.
#[test]
fn a_repeated_remainder_on_another_runs_row_cannot_outlast_the_maximum() {
    let (dir, mut engine, now) = seated!(&["refuse", "succeed"], Probed::Answer, None, 2);
    engine.cooldown.host = Some(host(dir.path(), 30, 150));
    let start = start_of(&now);
    let (b, db) = second_run_route(dir.path());
    let worklog = dir.path().join("work.log");
    let seen = Arc::new(Mutex::new(0usize));
    let probe_clock = Arc::clone(&now);
    engine.cooldown.clock = clock_on_probe(&now, dir.path(), false, move || {
        let probes = std::fs::read_to_string(&worklog)
            .unwrap_or_default()
            .lines()
            .filter(|line| *line == "probe")
            .count();
        let mut seen = seen.lock().unwrap();
        if probes > *seen {
            *seen = probes;
            let at = *probe_clock.lock().unwrap();
            append_second_row(&db, &b, at + Duration::seconds(100));
        }
    });
    engine.drive().unwrap();
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "probe"],
        "the loop did not stop at the maximum"
    );
    let reason = park_reason(&engine);
    assert!(reason.contains("150s maximum"), "{reason}");
    assert_eq!(starts(&engine).len(), 1, "the run retried past the maximum");
    assert!(
        *now.lock().unwrap() >= start + Duration::seconds(130),
        "the first remainder was not waited"
    );
}

/// The end of a round within the declared maximum is representable, the
/// invariant that leaves `extend_wait`'s expect with no error arm: the
/// maximum is a `u32`, and an RFC 3339 year holds every `u32` of seconds
/// from today.
#[test]
fn a_round_within_the_maximum_has_a_representable_end() {
    assert!(
        super::super::sites::wait_until(base(), u64::from(u32::MAX)).is_some(),
        "a round at the maximum u32 has no representable end"
    );
}

/// A non-empty site's refusal extends its own round, leaves the answered
/// member unprobed for that round, and both are probed again before the
/// retry: the answered member's first answer is replaced, not duplicated.
#[test]
fn a_non_empty_sites_refusal_extends_and_reprobes_the_answered_member() {
    use super::panel::{retried_two_route_panel, route_host};

    let dir = tempfile::tempdir().unwrap();
    let body = retried_two_route_panel(dir.path(), Probed::Answer, Probed::RefuseThenAnswer);
    let mut engine = started(dir.path(), body, work(2));
    engine.cooldown.host = Some(route_host(dir.path(), (2, 20), (3, 20)));
    let (mut engine, now) = with_clock(engine);
    let start = *now.lock().unwrap();
    engine.drive().unwrap();
    let event = failed(&engine).remove(0);
    assert_eq!(rounds(&engine, event.seq, "a"), [1]);
    assert_eq!(
        rounds(&engine, event.seq, "b"),
        [1, 2],
        "the site's refusal wrote no next round"
    );
    assert_eq!(
        logs(dir.path(), "a"),
        ["attempt:refuse", "probe", "probe", "attempt:succeed"],
        "the answered member was not probed again"
    );
    assert_eq!(
        logs(dir.path(), "b"),
        ["attempt:refuse", "probe", "probe", "attempt:succeed"]
    );
    let probes = retry_probes(&engine);
    assert_eq!(probes.len(), 2, "an answer was recorded twice: {probes:?}");
    assert_eq!(probes[0]["site"], "a");
    assert_eq!(probes[1]["site"], "b");
    assert!(
        answered_at(&probes[0]) >= start + Duration::seconds(9),
        "the answered member's first answer stood in for its second: {probes:?}"
    );
}

/// The single-seat round fixture the unreadable-input cases share: a
/// journal with one future wait, a wide host, and its events.
fn waited_round_fixture() -> (tempfile::TempDir, Engine, Vec<brokkr_core::EventEnvelope>) {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = started(dir.path(), single_body(vec!["unused".into()]), work(2));
    append_waited(
        &mut engine,
        wait_value(OffsetDateTime::now_utc() + Duration::seconds(60), 60),
    );
    engine.cooldown.host = Some(host(dir.path(), 30, 1000));
    let events = engine.store.load(&engine.run_id).unwrap();
    (dir, engine, events)
}

/// The later rounds' two unreadable inputs park by name: a store that loses
/// the table between rounds, and bounds that stop reading; the probe rows'
/// own recorded instants make the episode unreadable too.
#[test]
fn the_wait_rounds_park_by_name_when_the_store_bounds_or_rounds_cannot_read() {
    // The table lost after the first round: the round's aggregate read
    // parks by the store's words.
    let (dir, mut engine, events) = waited_round_fixture();
    alter_journal(
        &dir.path().join("forge.db"),
        "DROP TABLE provider_cooldowns",
    );
    let waits = [wait()];
    let mut unrecorded = 0;
    match engine
        .wait_round(&events, "effect", &waits, &mut unrecorded, base())
        .unwrap()
    {
        super::super::wait::WaitRound::Parked(reason) => {
            assert!(
                reason.contains("the cool-down aggregate for claude/first cannot be read"),
                "{reason}"
            );
            assert!(reason.contains("no such table"), "{reason}");
        }
        _ => panic!("a lost table did not park the round"),
    }

    // Bounds that stop reading between rounds.
    let (dir, mut engine, events) = waited_round_fixture();
    invalid_host(dir.path(), &mut engine);
    let reason = engine
        .extension_park_reason(&events, "effect", &waits, waits[0].until, base(), 0)
        .expect_err("unreadable bounds park");
    assert!(reason.contains("cannot read its bounds"), "{reason}");

    // A probe row whose own recorded instant does not read: the episode, not
    // the aggregate, is what refuses.
    let (dir, engine, events) = waited_round_fixture();
    alter_journal(
        &dir.path().join("forge.db"),
        "DROP TRIGGER provider_cooldowns_append_only_update;
         UPDATE provider_cooldowns SET recorded_at = -9223372036854775808;",
    );
    let reason = engine
        .extension_park_reason(&events, "effect", &waits, waits[0].until, base(), 0)
        .expect_err("an unreadable round parks");
    assert!(reason.contains("cannot read its episode"), "{reason}");
}

/// A stop journaled between the fold and the first slice is seen before
/// the wait sleeps: the resume's own guard, exercised through the round.
#[test]
fn a_stop_journaled_before_the_first_slice_is_honoured() {
    let (dir, mut engine, now) = seated!(&["refuse", "succeed"], Probed::Answer, None, 2);
    engine.cooldown.host = Some(host(dir.path(), 30, 1000));
    let db = dir.path().join("forge.db");
    let run = engine.run_id.clone();
    let worklog = dir.path().join("work.log");
    let calls = Arc::new(Mutex::new(0usize));
    let read = Arc::clone(&now);
    let seen = Arc::clone(&calls);
    engine.cooldown.clock = clock_from(&now, move || {
        let log = std::fs::read_to_string(&worklog).unwrap_or_default();
        if log.lines().any(|line| line == "attempt:refuse") && !log.contains("probe") {
            let mut calls = seen.lock().unwrap();
            *calls += 1;
            // The terminal's own end is the first reading; this is the
            // round's, taken between its fold and its first slice.
            if *calls == 2 {
                append_stop(&db, &run);
            }
        }
        *read.lock().unwrap()
    });
    drive_to_stop(&mut engine);
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse"],
        "a probe ran after the stop"
    );
}

/// A resume over a round already written counts that round's remainder
/// once: the episode through it is within the maximum, the engine waits the
/// written end, the probe answers and the retry runs, spending no attempt.
#[test]
fn a_resume_over_a_written_round_counts_it_once() {
    let start = OffsetDateTime::now_utc();
    let (dir, mut engine, now) = resumed_at(
        &["succeed"],
        start + Duration::seconds(30),
        30,
        start + Duration::seconds(45),
        false,
    );
    engine.cooldown.host = Some(host(dir.path(), 30, 100));
    let event = failed(&engine).remove(0);
    let run_id = engine.run_id.clone();
    engine
        .store
        .append_cooldown_round(brokkr_store::CooldownRound {
            run_id: &run_id,
            event_seq: event.seq,
            site: "",
            provider: "claude",
            route: "first",
            until_at: start + Duration::seconds(90),
            reason: "rate_limit: slow down",
            retry_after_seconds: None,
            rounds: 2,
            seconds: 60,
        })
        .unwrap();
    engine.drive().unwrap();
    assert_eq!(
        logs(dir.path(), "work"),
        ["probe", "attempt:succeed"],
        "a written round was counted twice and parked the resume"
    );
    assert!(
        *now.lock().unwrap() >= start + Duration::seconds(90),
        "the write's end was not waited"
    );
    assert_eq!(starts(&engine).len(), 2, "the resume spent an attempt");
    let effect = event.payload["effect_id"].clone();
    assert_eq!(
        failed(&engine)
            .iter()
            .filter(|failed| failed.payload["effect_id"] == effect)
            .count(),
        1,
        "the resume wrote a second failure"
    );
}

/// A terminal whose own row never landed is filled at round one; that
/// filled round is the episode's, so the next round would pass the declared
/// maximum and is neither written nor waited.
#[test]
fn a_filled_round_is_counted_so_the_next_round_parks_at_the_maximum() {
    let dir = tempfile::tempdir().unwrap();
    let command = driver(
        dir.path(),
        "work",
        &["succeed"],
        Probed::Refuse,
        None,
        "claude",
        "first",
    );
    let mut engine = started(dir.path(), single_body(command), work(3));
    engine.cooldown.host = Some(host(dir.path(), 30, 100));
    let (clock, _now) = Clock::advancing(base());
    engine.cooldown.clock = clock;
    append_waited(&mut engine, wait_value(base() - Duration::seconds(1), 30));
    engine.drive().unwrap();
    assert_eq!(logs(dir.path(), "work"), ["probe", "probe"]);
    only_the_own_row_and_the_maximum(&engine, "would wait 60s in round 2 (episode 120s)");
    assert_eq!(starts(&engine).len(), 1, "the run started another attempt");
}

/// A remainder already waited on another run's row counts before a probe's
/// next round is written: the round would pass the maximum, so it writes no
/// row and parks naming the maximum.
#[test]
fn a_remainder_waited_on_another_run_counts_before_the_extension_writes() {
    let (dir, mut engine, now) = seated!(&["refuse"], Probed::Refuse, None, 3);
    engine.cooldown.host = Some(host(dir.path(), 30, 100));
    let start = start_of(&now);
    let (b, db) = second_run_route(dir.path());
    append_second_row(&db, &b, start + Duration::seconds(50));
    engine.drive().unwrap();
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse", "probe"]);
    only_the_own_row_and_the_maximum(&engine, "would wait 80s in round 2 (episode 110s)");
    assert_eq!(starts(&engine).len(), 1, "the run retried past the maximum");
}
