//! The host exact-coverage misses this fix round reaches (decision 0103
//! slice 4d2b, fix round 430s4d2d): a crash resume whose episode now reads
//! states the record's generic cause, the wait round's fold failure
//! propagates through `step`, and a panel's later member re-reads the
//! aggregate before its probe. The last test binds the record invariants
//! that make `maximum_reason`'s emptiness guard unreachable. Every test
//! takes a production path; none plants a test-only branch.

use std::sync::{Arc, Mutex};

use brokkr_core::fold::FoldError;
use serde_json::json;

use super::*;
use crate::engine::EngineError;

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
/// The assertion matches the typed refusal and its exact seq and field, not
/// the error's text (0071 ruling 8).
#[test]
fn a_fold_failure_in_the_wait_round_propagates_through_the_step() {
    own_engine!();
    let dir = tempfile::tempdir().unwrap();
    let engine = super::probe_rounds::wide_seat(dir.path(), &["refuse", "succeed"], Probed::Answer);
    let (mut engine, now) = with_clock(engine);
    let db = dir.path().join("forge.db");
    let run = engine.run_id.clone();
    let appended = Arc::new(Mutex::new(None));
    let appended_in_clock = Arc::clone(&appended);
    engine.cooldown.clock = clock_acting(&now, Acting::AfterFirstSlice, move || {
        let event = brokkr_store::Store::open(&db)
            .unwrap()
            .append_next(
                &run,
                brokkr_core::envelope::EventType::EffectStarted,
                json!({}),
                None,
                None,
            )
            .unwrap();
        *appended_in_clock.lock().unwrap() = Some(event.seq);
    });
    let error = engine
        .drive()
        .expect_err("a fold failure must not be read as a park");
    match error {
        EngineError::Fold(FoldError::BadPayload { seq, field }) => {
            assert_eq!(Some(seq), *appended.lock().unwrap(), "a different event");
            assert_eq!(field, "effect_id");
        }
        other => panic!("the wait round's fold failure did not come back typed: {other:?}"),
    }
    // This test appends an event its run's summary disagrees with, so the
    // store's own drop check would mask the typed refusal; the tempdir
    // still removes the file.
    std::mem::forget(engine);
}

/// A panel's first member probes and answers, then the cool-down table is
/// lost; the next read of the panel's aggregate refuses by the store's
/// words, so nothing probes or retries after it. This is `probe_each`'s
/// aggregate-read failure arm, reached through the panel's own round.
#[test]
fn a_panels_second_member_parks_when_its_probe_read_loses_the_aggregate() {
    own_engine!();
    let dir = tempfile::tempdir().unwrap();
    let plain = super::panel::two_route_engine(
        dir.path(),
        &["refuse", "succeed"],
        &["refuse", "succeed"],
        Probed::Answer,
        Probed::Answer,
        2,
    );
    let (mut engine, now) = with_clock(plain);
    let db = dir.path().join("forge.db");
    engine.cooldown.clock =
        super::probe_rounds::clock_on_marker(&now, &dir.path().join("a.done"), true, move || {
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
    assert!(
        reason.contains("the cool-down aggregate for claude/first cannot be read"),
        "{reason}"
    );
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

/// `recorded_waits` refuses a record that mixes a site-less wait with a
/// member's, so no wait with `site: None` stands beside a member's round.
/// With `NextRound.site` being the probing wait's own site, that is what
/// keeps `maximum_reason`'s `passed` from ever being `Some("")`; this test
/// is the removal control for the invariant that let its emptiness guard
/// go (0071 ruling 9, fix round 430s4d2d).
#[test]
fn recorded_waits_refuses_an_absent_site_beside_a_member() {
    let dir = tempfile::tempdir().unwrap();
    let engine = started(dir.path(), single_body(vec!["unused".into()]), work(1));
    let member = json!({"site": "a", "provider": "claude", "route": "first",
                        "words": "w", "seconds": 1,
                        "until": "2026-11-01T00:00:00Z", "rounds": 1});
    let own = json!({"provider": "codex", "route": "second", "words": "w",
                     "seconds": 1, "until": "2026-11-01T00:00:00Z", "rounds": 1});
    let event = crate::envelope_builder::EnvelopeBuilder::new(
        brokkr_core::envelope::EventType::EffectFailed,
        json!({"effect_id": "effect", "waits": [own, member]}),
    )
    .seq(4)
    .build();
    match engine.recorded_wait(&[event], "effect") {
        Err(super::super::CooldownError::TooManyWaits { seq, count }) => {
            assert_eq!((seq, count), (4, 2));
        }
        other => panic!("a mixed record read as {other:?}, not a refusal"),
    }
}
