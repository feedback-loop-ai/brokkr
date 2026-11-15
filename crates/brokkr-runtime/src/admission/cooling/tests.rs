//! Admission's cool-down leg (decision 0103 ruling 6, slice 4e): an entry
//! whose provider and route stand in a cool-down waits on it and is never
//! held, a route no row names is admitted, every unexpired row holds, an
//! expired row never does, a row the pass cannot read refuses it, and the
//! claim's fence refuses a row recorded after the pass read.

use std::path::{Path, PathBuf};

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_store::{CooldownAppend, Cooling as Row, Generation, QueueRefusal, Store, StoreError};
use serde_json::json;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

use crate::admission::capacity::tests::{ceilings, route, verdicts, Fixture, PLENTY, TEMP};
use crate::admission::capacity::Capacity;
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

/// Write `rows` as a new run's first terminal event, through the store's
/// own writer, and answer the event.
fn refuse(store: &mut Store, run: &str, rows: &[Row]) -> EventEnvelope {
    store.create_run(run, "f", "b", &json!({})).unwrap();
    store
        .append_next(
            run,
            EventType::RunStarted,
            json!({"feature": "f"}),
            None,
            None,
        )
        .unwrap();
    store
        .append_with_cooldowns(
            CooldownAppend {
                run_id: run,
                fenced: None,
                event_type: EventType::RunStopped,
                payload: json!({}),
                causation_id: None,
                attempt_id: None,
            },
            rows,
        )
        .unwrap()
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

#[test]
fn an_expired_row_never_holds() {
    let (fixture, file) = refused(&[row("beta", "cloud", at(-60), "expired")]);
    assert_eq!(admitted_at(&fixture, &file, base()), vec![]);
    // The pass writes nothing: the expired row is history still.
    assert_eq!(
        fixture.store.cooldowns_in_force(at(-120)).unwrap().len(),
        1,
        "the pass cleared an expired row"
    );
}

#[test]
fn an_unreadable_row_refuses_the_pass() {
    let mut fixture = Fixture::new();
    fixture.seat("cloudy", "cloudy");
    fixture.queue();
    let file = fixture.declare(&declared());
    fixture
        .store
        .create_run("run-bad", "f", "b", &json!({}))
        .unwrap();
    fixture
        .store
        .append_next(
            "run-bad",
            EventType::RunStarted,
            json!({"feature": "f"}),
            None,
            None,
        )
        .unwrap();
    let event = fixture
        .store
        .append_next("run-bad", EventType::RunStopped, json!({}), None, None)
        .unwrap();
    // A stored instant past the last second the reader can spell, on a
    // row the table's own constraints accept.
    rusqlite::Connection::open(fixture.root.join("forge.db"))
        .unwrap()
        .execute(
            "INSERT INTO provider_cooldowns
                (run_id, event_seq, site, provider, route, until_at, reason, rounds, recorded_at)
             VALUES (?1, ?2, '', 'beta', 'cloud', 253402300800, 'unreadable', 1, 1)",
            rusqlite::params!["run-bad", event.seq as i64],
        )
        .unwrap();

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

#[test]
fn the_claim_refuses_a_row_recorded_after_the_pass_read() {
    let mut fixture = Fixture::new();
    fixture.seat("cloudy", "cloudy");
    fixture.queue();
    let file = fixture.declare(&declared());
    let room = judged_at(&fixture, &file, base())
        .remove(0)
        .room
        .expect("the pass admits the entry before any row");
    let until = at(60);
    refuse(
        &mut fixture.store,
        "run-a",
        &[row("beta", "cloud", until, "recorded in between")],
    );

    match room
        .claim(&mut fixture.store, Generation(1), "admitted")
        .unwrap()
    {
        Err(reason) => assert_eq!(
            reason,
            cooling("beta", "cloud", until, "recorded in between")
        ),
        Ok(claim) => panic!("the claim took {claim:?} over a cool-down"),
    }
}
