//! The host exact-coverage misses this fix round reaches (decision 0103
//! slice 4d2b, fix round 430s4d2d): a crash resume whose episode now reads
//! states the record's generic cause, the wait round's store failure
//! propagates through `step`, and a panel's later member re-reads the
//! aggregate before its probe. Each test drives the production path; none
//! plants a test-only branch.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::json;
use time::{Duration, OffsetDateTime};

use super::*;

/// The one action a marker clock runs, taken once its marker exists.
type MarkerAction = Arc<Mutex<Option<Box<dyn FnMut() + Send>>>>;

/// A clock over the test's shared instant that advances on each slice and
/// runs `act` once, on its first read after `marker` exists. The panel test
/// uses it to lose the cool-down table while an earlier member probes.
fn clock_when_marker(
    now: &Arc<Mutex<OffsetDateTime>>,
    marker: PathBuf,
    act: impl FnMut() + Send + 'static,
) -> Clock {
    let read = Arc::clone(now);
    let advance = Arc::clone(now);
    let action: MarkerAction = Arc::new(Mutex::new(Some(Box::new(act))));
    let action_read = Arc::clone(&action);
    Clock {
        now: Box::new(move || {
            if marker.exists() {
                if let Some(mut act) = action_read.lock().unwrap().take() {
                    act();
                }
            }
            *read.lock().unwrap()
        }),
        sleep: Box::new(move |slice| {
            *advance.lock().unwrap() += Duration::seconds(slice.as_secs() as i64)
        }),
    }
}

/// A crash between the failed event and its park recorded the episode as
/// unreadable; by the resume it reads, so the record's own generic cause is
/// the park text. This is the `EpisodeUnreadable` refusal's no-error arm:
/// the store's words are absent because there were none to re-read.
#[test]
fn a_resume_over_an_episode_unreadable_record_states_the_generic_cause() {
    let dir = tempfile::tempdir().unwrap();
    let payload = json!({
        "effect_id": "effect", "attempt_id": "a1", "error": "refused",
        "start_failure": true, "start_failure_sites": [null],
        "refused": {"cause": "episode-unreadable", "provider": "claude",
                    "route": "first", "words": "rate_limit: slow down"}
    });
    let mut engine = super::resumed_payload(dir.path(), 2, payload);
    engine.drive().unwrap();
    assert!(
        super::logs(dir.path(), "work").is_empty(),
        "a crash resume invoked the driver"
    );
    assert_eq!(
        super::park_reason(&engine),
        "refusing to retry claude/first: an earlier wait does not read: rate_limit: slow down"
    );
}

/// A journal made unreadable by the fold while the wait round runs: the
/// error comes back through `wait_round`'s `?` and `step`'s `?`, so the
/// drive reports it rather than parking as a maximum or waiting on. The
/// wait's first slice is the seam that appends an event `fold` cannot read.
#[test]
fn a_fold_failure_in_the_wait_round_propagates_through_the_step() {
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
    let mut engine = started(
        dir.path(),
        single_body(command),
        super::probe_rounds::patient(2),
    );
    engine.cooldown.host = Some(host(dir.path(), 30, 1000));
    let (mut engine, now) = with_clock(engine);
    let db = dir.path().join("forge.db");
    let run = engine.run_id.clone();
    engine.cooldown.clock = clock_acting(&now, Acting::AfterFirstSlice, move || {
        brokkr_store::Store::open(&db)
            .unwrap()
            .append_next(
                &run,
                brokkr_core::envelope::EventType::EffectStarted,
                json!({}),
                None,
                None,
            )
            .unwrap();
    });
    let error = engine
        .drive()
        .expect_err("a fold failure must not be read as a park");
    assert!(
        error.to_string().contains("payload missing or mistyped"),
        "{error}"
    );
    std::mem::forget(engine);
}

/// A panel's first member probes and answers, then the cool-down table is
/// lost; the next member's pre-probe aggregate read refuses by the store's
/// words, so nothing probes or retries after it. This is `probe_each`'s
/// aggregate-read failure arm, reached through the panel's own round.
#[test]
fn a_panels_second_member_parks_when_its_probe_read_loses_the_aggregate() {
    let dir = tempfile::tempdir().unwrap();
    let mut plain = super::panel::two_route_engine(
        dir.path(),
        &["refuse", "succeed"],
        &["refuse", "succeed"],
        Probed::Answer,
        Probed::Answer,
        2,
    );
    plain.bundle.seats.get_mut("work").unwrap().limits = super::probe_rounds::patient(2);
    let (mut engine, now) = with_clock(plain);
    let db = dir.path().join("forge.db");
    engine.cooldown.clock = clock_when_marker(&now, dir.path().join("a.done"), move || {
        rusqlite::Connection::open(&db)
            .unwrap()
            .execute_batch("DROP TABLE provider_cooldowns")
            .unwrap();
    });
    engine.drive().unwrap();
    let logged = std::fs::read_to_string(dir.path().join("all.log")).unwrap_or_default();
    assert!(
        logged.lines().any(|line| line == "a:probe"),
        "the first member did not probe: {logged}"
    );
    assert!(
        !logged.lines().any(|line| line == "b:probe"),
        "the second member probed after the aggregate failed: {logged}"
    );
    let reason = super::park_reason(&engine);
    assert!(reason.contains("the cool-down aggregate for"), "{reason}");
    assert!(
        reason.contains("no such table: provider_cooldowns"),
        "{reason}"
    );
    assert_eq!(
        super::starts(&engine).len(),
        1,
        "the round retried after the aggregate failed"
    );
}
