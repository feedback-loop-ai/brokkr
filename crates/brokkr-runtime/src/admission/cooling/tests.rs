//! Admission's cool-down leg (decision 0103 ruling 6, slice 4e): an entry
//! whose provider and route stand in a cool-down waits on it and is never
//! held, a route no row names is admitted, every unexpired row holds, an
//! expired row never does, a row the pass cannot read refuses it, and the
//! claim's fence refuses a row recorded after the pass read.

use std::path::{Path, PathBuf};

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_store::{Cooling as Row, QueueRefusal, Store, StoreError};
use serde_json::json;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

use crate::admission::capacity::tests::{
    ceilings, claimed, refused_rows as refuse, route, started_run, verdicts, Fixture, PLENTY, TEMP,
};
use crate::admission::capacity::{Capacity, Room};
use crate::admission::{pass_within_at, AdmissionError, Host, Judged, Reason, Standing};

/// A fixed instant after every row this file writes records at, so a row
/// is in force when the pass measures it.
fn base() -> OffsetDateTime {
    OffsetDateTime::parse("2099-01-01T00:00:00Z", &Rfc3339).expect("a fixed instant parses")
}

/// The instant a test's clock reads, `seconds` after [`base`].
fn at(seconds: i64) -> OffsetDateTime {
    base() + Duration::seconds(seconds)
}

/// One row a provider's refusal records for a lane.
fn row(provider: &str, route: &str, until: OffsetDateTime, words: &str) -> Row {
    Row {
        site: String::new(),
        provider: provider.into(),
        route: route.into(),
        until_at: until,
        reason: words.into(),
        retry_after_seconds: None,
        rounds: 1,
    }
}

/// Create `run`, record its start and its stop, and answer the stop.
fn stopped_run(store: &mut Store, run: &str) -> EventEnvelope {
    started_run(store, run);
    store
        .append_next(run, EventType::RunStopped, json!({}), None, None)
        .unwrap()
}

/// A fixture whose one entry seats beta's cloud route and is admitted, its
/// host file, and the room the pass found before any row.
fn room_before_rows() -> (Fixture, PathBuf, Room) {
    let mut fixture = Fixture::new();
    fixture.seat("cloudy", "cloudy");
    fixture.queue();
    let file = fixture.declare(&declared());
    let room = judged_at(&fixture, &file, base())
        .remove(0)
        .room
        .expect("the pass admits the entry before any row");
    (fixture, file, room)
}

/// The entry's reasons under a pass measuring at `now`, every filesystem
/// reporting plenty free.
fn reasons_at(fixture: &Fixture, file: &Path, now: OffsetDateTime) -> Vec<Vec<Reason>> {
    verdicts(judged_at(fixture, file, now))
}

/// The one entry's reasons at `now`, refused unless the pass admitted it.
fn admitted_at(fixture: &Fixture, file: &Path, now: OffsetDateTime) -> Vec<Reason> {
    let judged = judged_at(fixture, file, now);
    assert!(judged[0].room.is_some(), "the entry is admitted");
    judged[0].verdict.as_ref().unwrap().reasons.clone()
}

/// Every entry the pass judged at `now`.
fn judged_at(fixture: &Fixture, file: &Path, now: OffsetDateTime) -> Vec<Judged> {
    let probe = |_: &Path| Ok(PLENTY);
    let host = Host {
        file,
        temp: TEMP.as_ref(),
        free: &probe,
    };
    pass_within_at(&fixture.store, &host, now).unwrap().0
}

/// The capacity reason a lane stands at.
fn cooling(provider: &str, route: &str, until: OffsetDateTime, words: &str) -> Reason {
    Reason::Capacity(Capacity::CoolingDown {
        provider: provider.into(),
        route: route.into(),
        until,
        words: words.into(),
    })
}

/// The host this file declares: two places for each provider, beta's two
/// routes below their own ceilings.
fn declared() -> serde_json::Value {
    ceilings(
        2,
        2,
        json!({"cloud": route(2, "cloud"), "spark-glm": route(2, "cloud")}),
    )
}

/// The fixture, its one entry and the host file, with each of `rows`
/// written as one run before the pass.
fn refused(rows: &[Row]) -> (Fixture, PathBuf) {
    let mut fixture = Fixture::new();
    fixture.seat("cloudy", "cloudy");
    fixture.queue();
    let file = fixture.declare(&declared());
    for (index, row) in rows.iter().enumerate() {
        refuse(
            &mut fixture.store,
            &format!("run-{index}"),
            std::slice::from_ref(row),
        );
    }
    (fixture, file)
}

/// Copy the fixture's compiled bundle under `/bundle-<name>`, for the
/// entries that must seat different routes.
fn copy_bundle(fixture: &Fixture, name: &str) {
    let from = fixture.root.join("bundle");
    let to = fixture.root.join(format!("bundle-{name}"));
    std::fs::create_dir_all(to.join("roles")).unwrap();
    for file in ["bundle.json", "policy.json", "roles/work.md"] {
        std::fs::copy(from.join(file), to.join(file)).unwrap();
    }
}

#[test]
fn a_cooling_entry_is_not_admitted_and_its_lane_never_latches() {
    // A deliberate short end, not `LONG_WAIT` (#857): it is five minutes on
    // the fixed injected clock, which lies decades past the journal's real
    // recording, so no lock stall can lose the row.
    let until = at(300);
    let words = "the provider's words";
    let (fixture, file) = refused(&[row("beta", "cloud", until, words)]);

    let judged = judged_at(&fixture, &file, base());
    let verdict = judged[0].verdict.as_ref().expect("a waiting entry");
    assert_eq!(
        verdict.reasons,
        vec![cooling("beta", "cloud", until, words)]
    );
    assert_eq!(verdict.standing(), Standing::Waiting);
    assert!(!verdict.reasons[0].holds(), "a cool-down clears by itself");

    let reason = &verdict.reasons[0];
    assert_eq!(reason.kind(), "cooling_down");
    assert_eq!(
        reason.to_string(),
        format!(
            "provider beta is cooling on route cloud until {}: {words}",
            until.format(&Rfc3339).unwrap()
        )
    );
}

#[test]
fn an_entry_on_a_route_with_no_row_is_admitted() {
    let mut fixture = Fixture::new();
    // The alpha lane takes no route, and no row holds a provider-wide lane.
    fixture.seat("one", "cloudy");
    fixture.queue();
    let file = fixture.declare(&declared());
    refuse(
        &mut fixture.store,
        "run-a",
        &[row("beta", "spark-glm", at(300), "another route")],
    );

    assert_eq!(admitted_at(&fixture, &file, base()), vec![]);
}

#[test]
fn two_runs_at_the_same_event_seq_hold_both_routes() {
    let mut fixture = Fixture::new();
    fixture.seat("cloudy", "cloudy");
    copy_bundle(&fixture, "cloud");
    fixture.seat("local", "local");
    copy_bundle(&fixture, "spark");
    fixture.queue_bundle("bundle-cloud");
    fixture.queue_bundle("bundle-spark");
    let file = fixture.declare(&declared());
    let cloud = at(300);
    let spark = at(600);
    let first = refuse(
        &mut fixture.store,
        "run-a",
        &[row("beta", "cloud", cloud, "run-a refused")],
    );
    let second = refuse(
        &mut fixture.store,
        "run-b",
        &[row("beta", "spark-glm", spark, "run-b refused")],
    );
    // Both runs record their first event, so the seq alone cannot tell
    // the rows apart; the run is what keeps both.
    assert_eq!(first.seq, second.seq);

    assert_eq!(
        reasons_at(&fixture, &file, base()),
        vec![
            vec![cooling("beta", "cloud", cloud, "run-a refused")],
            vec![cooling("beta", "spark-glm", spark, "run-b refused")],
        ]
    );
}

#[test]
fn two_overlapping_refusals_hold_until_the_longer_ends() {
    let longer = at(300);
    let (fixture, file) = refused(&[
        row("beta", "cloud", longer, "the earlier, longer"),
        row("beta", "cloud", at(60), "the later, shorter"),
    ]);

    // The later row is shorter, and never shortens the lane.
    assert_eq!(
        reasons_at(&fixture, &file, base()),
        vec![vec![cooling(
            "beta",
            "cloud",
            longer,
            "the earlier, longer"
        )]]
    );
    // The later row ends first; the earlier still holds the lane, so the
    // entry waits for the longer end, not the latest row.
    assert_eq!(
        reasons_at(&fixture, &file, base() + Duration::seconds(120)),
        vec![vec![cooling(
            "beta",
            "cloud",
            longer,
            "the earlier, longer"
        )]]
    );
    // The longer row alone holds the lane to its end, and no further.
    assert_eq!(reasons_at(&fixture, &file, longer), vec![vec![]]);
}

/// At one end, the latest row speaks for the lane (decision 0103 ruling 2):
/// the store reads rows by rising seq and keeps the latest row at the
/// latest `until_at`, and the pass and the claim both keep it.
#[test]
fn the_latest_row_at_the_latest_end_speaks_for_the_lane() {
    let until = at(300);
    let (fixture, file) = refused(&[
        row("beta", "cloud", until, "first"),
        row("beta", "cloud", until, "second"),
    ]);
    assert_eq!(
        reasons_at(&fixture, &file, base()),
        vec![vec![cooling("beta", "cloud", until, "second")]]
    );

    let (mut fixture, _file, room) = room_before_rows();
    refuse(
        &mut fixture.store,
        "run-first",
        &[row("beta", "cloud", until, "first")],
    );
    refuse(
        &mut fixture.store,
        "run-second",
        &[row("beta", "cloud", until, "second")],
    );
    match claimed(&mut fixture, &room) {
        Err(reason) => assert_eq!(reason, cooling("beta", "cloud", until, "second")),
        Ok(claim) => panic!("the claim took {claim:?} over a cool-down"),
    }
}

#[test]
fn an_expired_row_never_holds() {
    // A deliberate past end, not `LONG_WAIT` (#857): it is expired only on
    // the fixed injected clock, and still decades past the journal's real
    // recording, so the store keeps the row however long the append stalls.
    let (fixture, file) = refused(&[row("beta", "cloud", at(-60), "expired")]);
    assert_eq!(admitted_at(&fixture, &file, base()), vec![]);
    // The pass writes nothing: the expired row is history still.
    assert_eq!(
        fixture.store.cooldowns_in_force(at(-120)).unwrap().len(),
        1,
        "the pass cleared an expired row"
    );
}

/// A fixture whose one entry seats beta's cloud route, with a stopped run's
/// cool-down row planted by SQL at `until_at` and `words`, and the host
/// file. The row bypasses the writer's bound, as a journal written by
/// another build can.
fn planted(until_at: i64, words: &str) -> (Fixture, PathBuf) {
    let mut fixture = Fixture::new();
    fixture.seat("cloudy", "cloudy");
    fixture.queue();
    let file = fixture.declare(&declared());
    let event = stopped_run(&mut fixture.store, "run-planted");
    rusqlite::Connection::open(fixture.root.join("forge.db"))
        .unwrap()
        .execute(
            "INSERT INTO provider_cooldowns
                (run_id, event_seq, site, provider, route, until_at, reason, rounds, recorded_at)
             VALUES (?1, ?2, '', 'beta', 'cloud', ?3, ?4, 1, 1)",
            rusqlite::params!["run-planted", event.seq as i64, until_at, words],
        )
        .unwrap();
    (fixture, file)
}

#[test]
fn an_unreadable_row_refuses_the_pass() {
    let (fixture, file) = planted(i64::MAX, "unreadable");
    // The largest instant the table's own constraints accept: no `time`
    // range spells it. A second past 9999 is not enough, because the
    // workspace test build enables `time`'s `large-dates` through a
    // dev-dependency, and then year 10000 is readable.
    let probe = |_: &Path| Ok(PLENTY);
    let host = Host {
        file: &file,
        temp: TEMP.as_ref(),
        free: &probe,
    };
    let refused = pass_within_at(&fixture.store, &host, base()).unwrap_err();
    match refused {
        AdmissionError::Store(StoreError::Queue(QueueRefusal::CooldownTime { seq })) => {
            assert_eq!(seq, 1);
        }
        other => panic!("a row the pass cannot read was not refused: {other:?}"),
    }
}

/// An end RFC 3339 cannot spell still refuses, and printing its reason
/// never panics: a stored second of year 10000 is readable where `time`'s
/// `large-dates` is on (the workspace test build), and the fallback names
/// it in words; without it the reader refuses the row by name instead.
#[test]
fn a_reason_for_an_end_no_rfc_3339_text_can_spell_still_refuses_and_never_panics() {
    let (fixture, file) = planted(253_402_300_800, "the provider's words");
    let probe = |_: &Path| Ok(PLENTY);
    let host = Host {
        file: &file,
        temp: TEMP.as_ref(),
        free: &probe,
    };
    match pass_within_at(&fixture.store, &host, base()) {
        Ok((judged, _)) => {
            let reason = judged[0].verdict.as_ref().expect("a waiting entry").reasons[0].clone();
            assert_eq!(reason.kind(), "cooling_down");
            let text = reason.to_string();
            assert!(
                text.contains("a time RFC 3339 cannot spell (second 253402300800)"),
                "the reason names the end it cannot spell: {text}"
            );
            assert!(text.contains("the provider's words"), "{text}");
        }
        Err(AdmissionError::Store(StoreError::Queue(QueueRefusal::CooldownTime { seq }))) => {
            assert_eq!(seq, 1);
        }
        Err(other) => panic!("a row no time range spells was not refused: {other:?}"),
    }
}

#[test]
fn the_claim_refuses_a_row_recorded_after_the_pass_read() {
    let (mut fixture, _file, room) = room_before_rows();
    let until = at(60);
    refuse(
        &mut fixture.store,
        "run-a",
        &[row("beta", "cloud", until, "recorded in between")],
    );

    match claimed(&mut fixture, &room) {
        Err(reason) => assert_eq!(
            reason,
            cooling("beta", "cloud", until, "recorded in between")
        ),
        Ok(claim) => panic!("the claim took {claim:?} over a cool-down"),
    }
}
