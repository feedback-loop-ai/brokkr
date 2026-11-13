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
    let retry = starts(&engine).pop().unwrap();
    assert_eq!(retry["probes"][0]["provider"], "claude");
}

/// Run-B's row on the same provider and route outlasts run-A's own `until`:
/// run-A waits until run-B's ends, not its own.
#[test]
fn two_overlapping_refusals_from_two_runs_hold_until_the_longer_ends() {
    let (dir, mut engine, now) = seated!(&["refuse", "succeed"], Probed::Answer, None, 2);
    let start = start_of(&now);
    let b = second_run(dir.path());
    append_second_row(
        &dir.path().join("forge.db"),
        &b,
        start + Duration::seconds(60),
    );
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
    let (dir, mut engine, now) = seated!(&["refuse", "succeed"], Probed::Answer, None, 2);
    let start = start_of(&now);
    let b = second_run(dir.path());
    let db = dir.path().join("forge.db");
    let marker = dir.path().join("work.done");
    let acted = Arc::new(Mutex::new(false));
    let read = Arc::clone(&now);
    let advance = Arc::clone(&now);
    let once = Arc::clone(&acted);
    engine.cooldown.clock = Clock {
        now: Box::new(move || {
            if marker.exists() && !*once.lock().unwrap() {
                *once.lock().unwrap() = true;
                append_second_row(&db, &b, start + Duration::seconds(120));
            }
            *read.lock().unwrap()
        }),
        sleep: Box::new(move |slice| {
            *advance.lock().unwrap() += Duration::seconds(slice.as_secs() as i64)
        }),
    };
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

/// A lost race that leaves the terminal's own round absent refuses the next
/// round by name: the engine parks naming the store, never silently.
#[test]
fn a_lost_round_race_parks_naming_the_store() {
    let (dir, mut engine, now) = seated!(&["refuse", "succeed"], Probed::RefuseThenAnswer, None, 2);
    engine.cooldown.host = Some(host(dir.path(), 30, 1000));
    let db = dir.path().join("forge.db");
    after_first_slice(&mut engine, &now, move || {
        alter_journal(
            &db,
            "DROP TRIGGER provider_cooldowns_append_only_delete;
             DELETE FROM provider_cooldowns;",
        )
    });
    let reason = drove_one(dir.path(), &mut engine, &["attempt:refuse", "probe"]);
    assert!(
        reason.contains("could not record its next round"),
        "{reason}"
    );
    assert!(
        reason.contains("stand at round 0; round 2 is not the next one"),
        "{reason}"
    );
}
