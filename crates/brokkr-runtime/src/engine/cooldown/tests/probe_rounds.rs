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

/// The fixture deadline for a test whose fake driver must outlast a loaded
/// host (this round's four load-flaky fixtures): the attempt deadline is
/// unrelated to what those tests prove, so it is generous, as 4d1's
/// neighbours are.
pub(super) fn patient(attempts: u64) -> Limits {
    Limits {
        max_attempts: attempts,
        timeout_seconds: 30,
    }
}

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

/// The maximum judged over a wait to `end` with nothing waited before it,
/// at the engine's own instant.
fn judged(
    engine: &Engine,
    events: &[brokkr_core::EventEnvelope],
    waits: &[brokkr_core::ProviderWait],
    end: OffsetDateTime,
    counted: OffsetDateTime,
) -> Result<super::super::wait::WaitBudget, super::super::CooldownError> {
    let now = engine.now();
    let span = super::super::wait::Span { end, counted, now };
    let charge = super::super::wait::Charge::default();
    engine.extension_park_reason(events, "effect", waits, span, charge)
}

/// A second run in the same journal, ready for its own refusal row.
pub(super) fn second_run(dir: &Path) -> String {
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

/// Append a second run's own terminal refusal for `provider`/`route`,
/// ending at `until`: another run's aggregate row on the route run-A cools.
pub(super) fn append_second_row_on(
    db: &Path,
    run_id: &str,
    provider: &str,
    route: &str,
    until: OffsetDateTime,
) {
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
                        "provider": provider, "route": route,
                        "words": "run-b refused", "seconds": 5,
                        "until": until.format(&Rfc3339).unwrap(), "rounds": 1
                    }]
                }),
                causation_id: None,
                attempt_id: None,
            },
            &[brokkr_store::Cooling {
                site: String::new(),
                provider: provider.into(),
                route: route.into(),
                until_at: until,
                reason: "run-b refused".into(),
                retry_after_seconds: None,
                rounds: 1,
                held: None,
            }],
        )
        .unwrap();
}

/// Append a second run's refusal on claude/first, ending at `until`.
fn append_second_row(db: &Path, run_id: &str, until: OffsetDateTime) {
    append_second_row_on(db, run_id, "claude", "first", until);
}

/// Drive the engine and return the cooled seat's log, the two-run tests'
/// shared ending.
fn drive_and_log(engine: &mut Engine, dir: &Path) -> Vec<String> {
    engine.drive().unwrap();
    logs(dir, "work")
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

/// A two-run fixture: run-A's engine and clock, and run-B's row sink.
fn two_run_fixture() -> (
    tempfile::TempDir,
    Engine,
    Arc<Mutex<OffsetDateTime>>,
    OffsetDateTime,
    String,
    PathBuf,
) {
    let (dir, engine, now) = seated_for_two_runs();
    let start = start_of(&now);
    let (b, db) = second_run_route(dir.path());
    (dir, engine, now, start, b, db)
}

/// A single-seat engine with one journaled terminal wait, its wide host,
/// and the fixture's own temporary journal.
fn engine_with_wait(
    attempts: u64,
    until: OffsetDateTime,
    seconds: u64,
) -> (tempfile::TempDir, Engine) {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = super::bare_engine(dir.path(), attempts);
    append_waited(&mut engine, wait_value(until, seconds));
    engine.cooldown.host = Some(host(dir.path(), 30, 1000));
    (dir, engine)
}

/// Append one probe row at the single seat's terminal: the tests that plant
/// a written round share one row shape.
pub(super) fn append_round(
    engine: &mut Engine,
    event_seq: u64,
    until_at: OffsetDateTime,
    rounds: u64,
    seconds: u64,
) {
    let run_id = engine.run_id.clone();
    engine
        .store
        .append_cooldown_round(brokkr_store::CooldownRound {
            run_id: &run_id,
            event_seq,
            site: "",
            provider: "claude",
            route: "first",
            until_at,
            reason: "rate_limit: slow down",
            retry_after_seconds: None,
            rounds,
            seconds,
            ends: None,
        })
        .unwrap();
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
    clock_on_marker(now, &dir.join("work.done"), once, act)
}

/// [`clock_on_probe`] over a named marker: the panel's per-member `a.done`
/// lets a test plant a row while one member's probe runs and before the
/// next member's.
pub(super) fn clock_on_marker(
    now: &Arc<Mutex<OffsetDateTime>>,
    marker: &Path,
    once: bool,
    act: impl FnMut() + Send + 'static,
) -> Clock {
    let marker = marker.to_path_buf();
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
    let (dir, mut engine, now, start, b, db) = two_run_fixture();
    engine.cooldown.clock = clock_on_probe(&now, dir.path(), true, move || {
        append_second_row(&db, &b, start + Duration::seconds(120));
    });
    let log = drive_and_log(&mut engine, dir.path());
    assert_eq!(
        log,
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

/// A version 1, 2 or 3 journal has no table for a probe's hold or its next
/// round: the probe is refused before dispatch, unheld, and parks naming
/// the store's words (fix round 0106b1r9, E3), and no row is written.
#[test]
fn a_version_three_journal_parks_its_probe_unheld_and_writes_no_row() {
    let (dir, mut engine, _now) = seated!(&["refuse", "succeed"], Probed::Refuse, None, 2);
    let db = dir.path().join("forge.db");
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute("UPDATE meta SET value = '3' WHERE key = 'queue_schema'", [])
        .unwrap();
    let reason = drove_one(dir.path(), &mut engine, &["attempt:refuse"]);
    assert!(
        reason.contains("the journal's queue storage is not one this binary writes"),
        "{reason}"
    );
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
    // The wait's end is re-read before the probe (finding 8), so the lost
    // table parks this round before any probe runs.
    let reason = drove_one(dir.path(), &mut engine, &["attempt:refuse"]);
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
    engine.cooldown.host = Some(host(dir.path(), 30, 1000));
    // A terminal the refusal could extend: with the cleanup check gone, the
    // rate limit reaches `record_extension` and writes a row, so the row
    // count below can fail (finding F6).
    append_waited(
        &mut engine,
        wait_value(OffsetDateTime::now_utc() + Duration::hours(1), 60),
    );
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
    let extended = match engine.probe_end("effect", &wait(), report) {
        super::super::Probe::Parked(park) => {
            let reason = park.reason();
            assert!(reason.contains("cleanup failure"), "{reason}");
            assert!(reason.contains("group 7"), "{reason}");
            assert!(!reason.contains("rate_limit: slow down"), "{reason}");
            None
        }
        super::super::Probe::Extended(round) => Some(round),
        super::super::Probe::Answered(_) => panic!("a cleanup refusal answered"),
        super::super::Probe::Held => panic!("a probe that ran was held from its route"),
    };
    // Only the broken cleanup-first check reaches here; the write is the
    // row-writing path the row assertion is the removal control for.
    if let Some(round) = extended {
        let site = super::super::held::SiteMax {
            max: 1000,
            ordinary: true,
        };
        let _ = engine.record_extension(&[wait()], &[Some(site)], &round, 0);
    }
    let event = failed(&engine).remove(0);
    assert_eq!(
        rounds(&engine, event.seq, ""),
        [1],
        "a cleanup refusal wrote a probe row"
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
        super::super::Probe::Parked(park) => {
            let reason = park.reason();
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
        super::super::Probe::Parked(park) => park.reason().to_string(),
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
    let (dir, mut engine) = engine_with_wait(2, OffsetDateTime::now_utc() + Duration::hours(1), 60);
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

/// An engine whose cool-down table is dropped while its probe runs: the
/// probe end's own append and the post-probe re-read share this fixture.
fn engine_losing_the_table_on_probe(probed: Probed) -> (tempfile::TempDir, Engine) {
    let (dir, mut engine, now) = seated!(&["refuse", "succeed"], probed, None, 2);
    engine.cooldown.host = Some(host(dir.path(), 30, 1000));
    let db = dir.path().join("forge.db");
    engine.cooldown.clock = clock_on_probe(&now, dir.path(), true, move || {
        alter_journal(&db, "DROP TABLE provider_cooldowns");
    });
    (dir, engine)
}

/// A rows-bearing store refusal at the probe's own append — here the table
/// lost once the probe ran — parks naming the store's words, never silently.
#[test]
fn a_lost_table_at_the_probe_append_parks_naming_the_store() {
    let (dir, mut engine) = engine_losing_the_table_on_probe(Probed::RefuseThenAnswer);
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
    let mut engine = started(dir.path(), body, patient(2));
    engine.cooldown.host = Some(route_host(dir.path(), (2, 20), (3, 20)));
    let start = live_row_clock_start();
    let (mut engine, _) = with_clock_at(engine, start);
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
    let (dir, engine) = engine_with_wait(2, OffsetDateTime::now_utc() + Duration::seconds(60), 60);
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
    let mut charge = super::super::wait::Charge::default();
    let mut maxima = Vec::new();
    match engine
        .wait_round(&events, "effect", &waits, &mut charge, base(), &mut maxima)
        .unwrap()
    {
        super::super::wait::WaitRound::Parked(park) => {
            let reason = park.reason();
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
    let error = judged(&engine, &events, &waits, waits[0].until, base())
        .expect_err("unreadable bounds park");
    assert!(
        matches!(
            error,
            super::super::CooldownError::WaitBoundsUnreadable { .. }
        ),
        "{error}"
    );
    let reason = error.to_string();
    assert!(reason.contains("cannot read its bounds"), "{reason}");

    // A probe row whose own recorded instant does not read: the episode, not
    // the aggregate, is what refuses.
    let (dir, engine, events) = waited_round_fixture();
    alter_journal(
        &dir.path().join("forge.db"),
        "DROP TRIGGER provider_cooldowns_append_only_update;
         UPDATE provider_cooldowns SET recorded_at = -9223372036854775808;",
    );
    let error = judged(&engine, &events, &waits, waits[0].until, base())
        .expect_err("an unreadable round parks");
    assert!(
        matches!(
            error,
            super::super::CooldownError::WaitEpisodeUnreadable { .. }
        ),
        "{error}"
    );
    let reason = error.to_string();
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
    append_round(&mut engine, event.seq, start + Duration::seconds(90), 2, 60);
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
    only_the_own_row_and_the_maximum(&engine, "would wait 120s in round 3 (episode 210s)");
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
    only_the_own_row_and_the_maximum(&engine, "would wait 60s in round 2 (episode 110s)");
    assert_eq!(starts(&engine).len(), 1, "the run retried past the maximum");
}

/// The own-row window is exact only while a probe runs at or after every
/// own `until` and a round is at least a second. This fill lands after that
/// second, so the episode counts it; the removal control moves it inside
/// the window (a fill within a second of the own row's) and the episode
/// stops counting it, failing this test.
#[test]
fn a_fill_after_the_own_rows_second_is_counted_as_a_round() {
    let (_dir, mut engine) = engine_with_wait(2, base() - Duration::seconds(1), 30);
    let event = failed(&engine).remove(0);
    append_round(&mut engine, event.seq, base() + Duration::seconds(2), 1, 2);
    let events = engine.store.load(&engine.run_id).unwrap();
    let episode =
        super::super::episode_by_site(&engine.store, &engine.run_id, &events, "effect").unwrap();
    assert_eq!(
        episode.rounds.get(&None),
        Some(&2),
        "the fill after the own row's second was read as the own row"
    );
    assert_eq!(episode.seconds, 32);
}

/// A row recorded by another run during this run's slices holds the probe:
/// the wait's end is re-read before any probe, so the probe runs only once
/// the later row's end stands.
#[test]
fn a_row_recorded_during_the_wait_holds_the_probe() {
    let (dir, mut engine, now, start, b, db) = two_run_fixture();
    after_first_slice(&mut engine, &now, move || {
        append_second_row(&db, &b, start + Duration::seconds(120));
    });
    let log = drive_and_log(&mut engine, dir.path());
    assert!(
        *now.lock().unwrap() >= start + Duration::seconds(120),
        "the wait ended before the row recorded during the slices"
    );
    assert_eq!(
        log,
        ["attempt:refuse", "probe", "attempt:succeed"],
        "a probe ran inside another run's hold"
    );
    assert_eq!(starts(&engine).len(), 2);
}

/// A probe's round that passes any cooled site's maximum writes no row:
/// `a`'s own maximum is wide (200s), but the episode through the round
/// passes `b`'s 70s, so the round is judged before the append. The park
/// names `a`'s own round and length and `b`'s maximum as `b`'s (finding 5).
#[test]
fn a_round_that_passes_any_cooled_sites_maximum_writes_no_row() {
    use super::panel::{panel_engine, retried_two_route_panel, route_host};

    let dir = tempfile::tempdir().unwrap();
    let body = retried_two_route_panel(dir.path(), Probed::RefuseThenAnswer, Probed::Answer);
    let mut engine = panel_engine(
        dir.path(),
        body,
        2,
        route_host(dir.path(), (30, 200), (30, 70)),
    );
    engine.drive().unwrap();
    assert_eq!(logs(dir.path(), "a"), ["attempt:refuse", "probe"]);
    assert_eq!(
        logs(dir.path(), "b"),
        ["attempt:refuse"],
        "b was probed before a's round passed b's maximum"
    );
    let event = failed(&engine).remove(0);
    assert_eq!(
        rounds(&engine, event.seq, "a"),
        [1],
        "a row was written for a round past b's maximum"
    );
    let reason = park_reason(&engine);
    assert!(
        reason.contains("member a: cool-down for claude/first would wait 60s in round 2"),
        "{reason}"
    );
    assert!(
        reason.contains("member b's declared 70s maximum"),
        "{reason}"
    );
}

/// A resume parks after a written probe round with the episode's round and
/// length, not the terminal wait's own first round and length: the park says
/// round 2 at 60s, where the terminal recorded round 1 at 30s.
#[test]
fn a_resume_after_a_written_round_parks_with_the_episodes_round_and_length() {
    let start = OffsetDateTime::now_utc();
    let until = start + Duration::seconds(30);
    let (dir, mut engine, _now) = resumed_at(&["succeed"], until, 30, start, false);
    let event = failed(&engine).remove(0);
    append_round(&mut engine, event.seq, start + Duration::seconds(90), 2, 60);
    engine.cooldown.host = Some(host(dir.path(), 30, 50));
    engine.drive().unwrap();
    assert!(
        logs(dir.path(), "work").is_empty(),
        "a probe ran past the maximum"
    );
    let reason = park_reason(&engine);
    assert!(reason.contains("would wait 60s in round 2"), "{reason}");
    assert!(reason.contains("episode 90s"), "{reason}");
    assert!(reason.contains("declared 50s maximum"), "{reason}");
}

/// The terminal, resume and panel readers all carry a store refusal that
/// loses the episode into the park text: the store's own words, never the
/// generic "an earlier wait does not read" alone (finding 2).
#[test]
fn every_episode_store_refusal_parks_naming_the_store() {
    let (dir, mut engine) = engine_with_wait(3, OffsetDateTime::now_utc() + Duration::hours(1), 60);
    let events = engine.store.load(&engine.run_id).unwrap();
    alter_journal(
        &dir.path().join("forge.db"),
        "DROP TABLE provider_cooldowns",
    );
    let refusal = a_rate_limit_with(Some("first"), None);

    // The terminal path.
    match engine.rate_limit_terminal(
        "effect",
        json!({"effect_id": "effect", "error": "refused"}),
        Some(&refusal),
    ) {
        Ok(super::super::Terminal::Park { park, .. }) => {
            let reason = park.reason();
            assert!(
                reason.contains("the journal's cool-down rounds cannot be read"),
                "{reason}"
            );
            assert!(
                reason.contains("no such table: provider_cooldowns"),
                "{reason}"
            );
        }
        _ => panic!("a store refusal did not park the terminal"),
    }

    // The crash-before-the-park resume.
    let refused = brokkr_core::ProviderRefusal {
        cause: brokkr_core::RefusalCause::EpisodeUnreadable,
        site: None,
        provider: "claude".into(),
        route: Some("first".into()),
        words: "rate_limit: slow down".into(),
        retry_after_seconds: None,
        seconds: None,
        max_seconds: None,
    };
    let reason = engine.refused_park_reason(&events, "effect", &refused);
    assert!(
        reason.contains("no such table: provider_cooldowns"),
        "{reason}"
    );

    // The panel writer.
    let member = super::super::RefusedMember {
        site: "a".into(),
        refusal: a_rate_limit_with(Some("first"), None),
    };
    let panel = engine.panel_rate_limit_terminal(&events, "effect", json!({}), &[member], true);
    let park = panel.park.expect("a panel episode refusal parks");
    let reason = park.reason();
    assert!(
        reason.contains("no such table: provider_cooldowns"),
        "{reason}"
    );
}

/// A stop that lands while the probe runs stops the run before the append:
/// the probe's rate-limit refusal writes no row.
#[test]
fn a_stop_that_lands_during_the_probe_writes_no_probe_row() {
    let (dir, mut engine, _now) = seated!(&["refuse", "succeed"], Probed::BarrierRefuse, None, 2);
    engine.cooldown.host = Some(host(dir.path(), 30, 1000));
    let started = dir.path().join("work.started");
    let release = dir.path().join("work.release");
    let db = dir.path().join("forge.db");
    let run = engine.run_id.clone();
    let waiter = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while !started.exists() {
            assert!(
                std::time::Instant::now() < deadline,
                "the probe never started"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        append_stop(&db, &run);
        std::fs::write(&release, b"go").unwrap();
    });
    engine.drive().unwrap();
    waiter.join().unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    assert_eq!(fold(&events).unwrap().run.status(), Status::Stopped);
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse", "probe"]);
    let event = failed(&engine).remove(0);
    assert_eq!(
        rounds(&engine, event.seq, ""),
        [1],
        "a stop during the probe wrote a probe row"
    );
}

/// A declaration that appears after the round's wait read must not let the
/// probe write a round no maximum judged: the round parks naming the absent
/// declaration instead of panicking on a huge retry-after or writing past
/// the newly declared maximum (finding 1). The wait reads the host file
/// before its slices; the clock writes the declaration after the first.
#[test]
fn a_round_whose_wait_found_no_declaration_is_not_written() {
    for retry in [i64::MAX as u64, 5_000] {
        let dir = tempfile::tempdir().unwrap();
        let command = driver(
            dir.path(),
            "work",
            &["succeed"],
            Probed::Refuse,
            Some(retry),
            "claude",
            "first",
        );
        let mut engine = started(dir.path(), single_body(command), work(2));
        let (clock, now) = Clock::advancing(OffsetDateTime::now_utc());
        engine.cooldown.clock = clock;
        let start = start_of(&now);
        // The declaration is absent at the wait and written after its first
        // slice, the window the two reads opened.
        let host_path = host_with(
            dir.path(),
            "late-host.json",
            json!({"default_seconds": 30, "max_seconds": 100}),
        );
        let content = std::fs::read_to_string(&host_path).unwrap();
        std::fs::remove_file(&host_path).unwrap();
        engine.cooldown.host = Some(host_path.clone());
        let written = host_path.clone();
        engine.cooldown.clock = clock_acting(&now, Acting::AfterFirstSlice, move || {
            std::fs::write(&written, &content).unwrap();
        });
        append_waited(&mut engine, wait_value(start + Duration::seconds(30), 30));
        engine.drive().unwrap();
        assert_eq!(
            logs(dir.path(), "work"),
            ["probe"],
            "retry {retry}: a probe did not run once"
        );
        let event = failed(&engine).remove(0);
        assert_eq!(
            rounds(&engine, event.seq, ""),
            [1],
            "retry {retry}: the round the wait never bounded was written"
        );
        let reason = park_reason(&engine);
        assert!(
            reason.contains("its wait found no cool-down declared for"),
            "retry {retry}: {reason}"
        );
        assert_eq!(
            starts(&engine).len(),
            1,
            "retry {retry}: the run started another attempt"
        );
    }
}

/// The re-read after the probes and before the retry refuses by the store's
/// words when the table is lost while the probe runs: no retry follows
/// (finding 2).
#[test]
fn a_store_refusal_at_the_post_probe_re_read_parks_by_name() {
    let (dir, mut engine) = engine_losing_the_table_on_probe(Probed::Answer);
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

/// Another run's row for a later panel member's route lands while an
/// earlier member's probe runs: the aggregate is re-read before every probe,
/// so the later member is not probed inside the hold (finding 3).
#[test]
fn a_row_recorded_during_a_panels_probe_holds_the_next_member() {
    use super::panel::{retried_two_route_panel, route_host};

    let dir = tempfile::tempdir().unwrap();
    let body = retried_two_route_panel(dir.path(), Probed::Answer, Probed::Answer);
    let mut engine = started(dir.path(), body, work(2));
    engine.cooldown.host = Some(route_host(dir.path(), (30, 1000), (30, 1000)));
    let (mut engine, now) = with_clock(engine);
    let start = *now.lock().unwrap();
    let (b, db) = second_run_route(dir.path());
    engine.cooldown.clock = clock_on_marker(&now, &dir.path().join("a.done"), true, move || {
        append_second_row_on(&db, &b, "codex", "second", start + Duration::seconds(120));
    });
    engine.drive().unwrap();
    assert_eq!(
        logs(dir.path(), "a"),
        ["attempt:refuse", "probe", "probe", "attempt:succeed"],
        "a was not probed again after the hold ended"
    );
    assert_eq!(
        logs(dir.path(), "b"),
        ["attempt:refuse", "probe", "attempt:succeed"],
        "b was probed inside the hold, or not at all"
    );
    let probes = retry_probes(&engine);
    assert_eq!(probes.len(), 2, "an answer was recorded twice: {probes:?}");
    assert!(
        answered_at(&probes[1]) >= start + Duration::seconds(120),
        "b was probed before the row recorded during a's probe ended: {probes:?}"
    );
    assert!(
        answered_at(&probes[0]) >= start + Duration::seconds(120),
        "the retry kept a's answer taken inside the hold: {probes:?}"
    );
}

/// The maximum park carries its facts typed, not as prebuilt prose: the
/// variant's round, episode and maximum are the ones judged (finding 6).
#[test]
fn a_maximum_park_carries_the_rounds_typed_facts() {
    let (dir, mut engine, events) = waited_round_fixture();
    engine.cooldown.host = Some(host(dir.path(), 30, 50));
    let wait = engine
        .recorded_waits(&events, "effect")
        .unwrap()
        .expect("the fixture's terminal wait")
        .pop()
        .expect("one wait");
    let until = wait.until;
    let waits = [wait];
    // The stored row's end is rounded up to a whole second; starting the
    // extra wait there makes the judged remainder exactly 60s.
    let event = failed(&engine).remove(0);
    let row_until = engine
        .store
        .cooldown_rounds(&engine.run_id, event.seq, "")
        .unwrap()
        .pop()
        .expect("the terminal's own row")
        .until_at;
    let error = judged(
        &engine,
        &events,
        &waits,
        row_until + Duration::seconds(60),
        until,
    )
    .expect_err("a round past the maximum parks");
    match error {
        super::super::CooldownError::Maximum(facts) => {
            let super::super::MaximumFacts {
                wait,
                episode,
                max,
                passed,
            } = *facts;
            assert_eq!(wait.site, None);
            assert_eq!(wait.provider, "claude");
            assert_eq!(wait.route, "first");
            assert_eq!(wait.seconds, 60);
            assert_eq!(wait.rounds, 1);
            assert_eq!(episode, 120);
            assert_eq!(max, 50);
            assert_eq!(passed, None);
        }
        other => panic!("a past-maximum wait parked as {other:?}"),
    }
}

/// An unreadable earlier wait is named once: `EarlierWaitUnreadable`'s own
/// text already opens with the phrase, so the refusal must not prefix it
/// again (INFO, this fix round).
#[test]
fn an_unreadable_earlier_wait_is_named_once() {
    let (_dir, mut engine) = two_terminal_engine(4);
    match terminal(&mut engine, None) {
        super::super::Terminal::Park { park, .. } => {
            let reason = park.reason();
            assert_eq!(
                reason.matches("an earlier wait does not read").count(),
                1,
                "{reason}"
            );
        }
        super::super::Terminal::Plain(_) => {
            panic!("an unreadable earlier wait was read as nothing")
        }
    }
}
