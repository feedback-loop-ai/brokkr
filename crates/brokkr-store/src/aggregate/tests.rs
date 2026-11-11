//! The aggregate reads (decision 0089 ruling 7): the window's runs and
//! parks, the preceding count and the medians' pass, each held to
//! brokkr-core's own reading over the same journal, each pinned to the index
//! it folds through, and the parks' ranges held to the window alone by their
//! VM steps.

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::ending::{Ending, UnknownEnding};
use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::fold::{concluded, Status};
use brokkr_core::stamp::parse_millis;
use brokkr_core::summary::{Account, Bounded, Listing, Summary, NAME_BYTES};
use serde_json::json;

use super::*;
use crate::test_support::{
    import_fixtures, lags, lane, plant_envelope, plant_older_writer, refolded, unmigrate,
};
use crate::tests::envelope_builder::EnvelopeBuilder;
use crate::tests::store;

/// One run as the reads hand it back, owned for comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
struct OwnedRun {
    run_id: String,
    realm: Option<String>,
    recipe: Option<String>,
    ending_rule: Option<String>,
    hard_stop: bool,
    started_ms: Option<i64>,
    finished_ms: Option<i64>,
    ending: Option<Ending>,
    implement_visits: u64,
    reviews: WindowReviews,
    open: bool,
}

impl OwnedRun {
    fn of(row: &WindowRun<'_>) -> OwnedRun {
        OwnedRun {
            run_id: row.run_id.to_string(),
            realm: row.realm.map(str::to_string),
            recipe: row.recipe.map(str::to_string),
            ending_rule: row.ending_rule.map(str::to_string),
            hard_stop: row.hard_stop,
            started_ms: row.started_ms,
            finished_ms: row.finished_ms,
            ending: row.ending,
            implement_visits: row.implement_visits,
            reviews: row.reviews,
            open: row.open,
        }
    }

    /// The run in `order`'s key order, the run's id for the tie.
    fn sort(rows: &mut [OwnedRun], order: RunOrder) {
        match order {
            RunOrder::Unordered => rows.sort_by(|left, right| left.run_id.cmp(&right.run_id)),
            RunOrder::ByEndingRule => rows.sort_by(|left, right| {
                left.ending_rule
                    .cmp(&right.ending_rule)
                    .then_with(|| left.run_id.cmp(&right.run_id))
            }),
            RunOrder::ByRealmThenRecipe => rows.sort_by(|left, right| {
                left.realm
                    .cmp(&right.realm)
                    .then_with(|| left.recipe.cmp(&right.recipe))
                    .then_with(|| left.run_id.cmp(&right.run_id))
            }),
        }
    }
}

/// One park as the reads hand it back, owned for comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
struct OwnedPark {
    run_id: String,
    realm: Option<String>,
    phase: Option<String>,
    at: i64,
    answered: Option<i64>,
    ended: i64,
    open: bool,
}

impl OwnedPark {
    fn of(row: &WindowPark<'_>) -> OwnedPark {
        OwnedPark {
            run_id: row.run_id.to_string(),
            realm: row.realm.map(str::to_string),
            phase: row.phase.map(str::to_string),
            at: row.at,
            answered: row.answered,
            ended: row.ended,
            open: row.open,
        }
    }
}

/// The cut a stored name keeps, as the writer cuts it.
fn cut(name: &str) -> String {
    Bounded::within(name, NAME_BYTES).cut
}

/// One run's stored aggregate columns, as brokkr-core reads the run whole.
fn expected_run(run_id: &str, events: &[EventEnvelope]) -> OwnedRun {
    let account = Account::whole(run_id, events);
    let summary = Summary::of(
        &account,
        Listing {
            feature: "",
            created_at: "",
        },
    );
    let aggregate = &summary.aggregate;
    OwnedRun {
        run_id: run_id.to_string(),
        realm: aggregate.realm.as_deref().map(cut),
        recipe: aggregate.recipe.as_deref().map(cut),
        ending_rule: aggregate
            .finished
            .as_ref()
            .and_then(|finished| finished.hard_stop.as_ref())
            .and_then(|stop| stop.rule.as_deref())
            .map(cut),
        hard_stop: aggregate
            .finished
            .as_ref()
            .is_some_and(|finished| finished.hard_stop.is_some()),
        started_ms: aggregate.started_ms,
        finished_ms: aggregate.finished.as_ref().map(|finished| finished.at),
        ending: summary.closing.as_ref().map(|closing| closing.ended.ending),
        implement_visits: aggregate.implement_visits,
        reviews: WindowReviews {
            first: aggregate.review_first,
            second: aggregate.review_second,
            back: aggregate.review_back,
        },
        open: account
            .state()
            .ok()
            .is_some_and(|state| concluded(state).is_none()),
    }
}

/// One run's parks, the open moments read as `now`, and only those that
/// reach `from` and start before `to`.
fn parks_of(
    run_id: &str,
    realm: Option<&str>,
    events: &[EventEnvelope],
    from: i64,
    to: i64,
    now: i64,
) -> Vec<OwnedPark> {
    Account::whole(run_id, events)
        .parks()
        .into_iter()
        .filter_map(|park| {
            let ended = park.ended.unwrap_or(now);
            let answered = park.answered.unwrap_or(ended);
            let open = park.ended.is_none();
            // A closed park of class 0 reaches nowhere, so no window holds
            // it, as the store's span-class arms read it. The class is
            // brokkr-core's own reading, not a second overlap rule.
            let reaches = park.span_class() > 0 && (open || answered.max(ended) > from);
            (park.at < to && reaches).then(|| OwnedPark {
                run_id: run_id.to_string(),
                realm: realm.map(str::to_string),
                phase: park.phase.as_deref().map(cut),
                at: park.at,
                answered: park.answered,
                ended,
                open,
            })
        })
        .collect()
}

/// `rows` read from the store in `order`, sorted as its figure folds.
fn window_runs(
    store: &Store,
    from: i64,
    to: i64,
    order: RunOrder,
    realm: Option<&str>,
) -> Vec<OwnedRun> {
    let mut rows = Vec::new();
    store
        .window_runs(from, to, order, realm, |row| rows.push(OwnedRun::of(row)))
        .unwrap();
    rows
}

/// The window's parks, in the read's order.
fn window_parks(
    store: &Store,
    from: i64,
    to: i64,
    now: i64,
    realm: Option<&str>,
) -> Vec<OwnedPark> {
    let mut rows = Vec::new();
    store
        .window_parks(from, to, now, realm, |row| rows.push(OwnedPark::of(row)))
        .unwrap();
    rows
}

/// Whether `run` is in `realm`'s part of the window.
fn in_realm(run: &OwnedRun, realm: Option<&str>) -> bool {
    run.realm.as_deref() == realm || realm.is_none()
}

/// Whether `run` finished inside the window.
fn finished_in(run: &OwnedRun, from: i64, to: i64) -> bool {
    run.finished_ms
        .is_some_and(|finished| finished >= from && finished < to)
}

/// The window's durations, as the read hands them back.
fn durations(store: &Store, from: i64, to: i64, realm: Option<&str>) -> Vec<(Option<String>, i64)> {
    let mut spans = Vec::new();
    store
        .durations(from, to, realm, |realm, span| {
            spans.push((realm.map(str::to_string), span))
        })
        .unwrap();
    spans
}

/// Every run's events, once the store has read it, or `None` where it does
/// not load.
fn readable(store: &Store, run_id: &str) -> Option<Vec<EventEnvelope>> {
    store.load(run_id).ok()
}

/// Hold every window read to brokkr-core's whole reading over `runs`.
fn compare_windows(
    store: &Store,
    runs: &[(String, Option<Vec<EventEnvelope>>)],
    windows: &[(i64, i64)],
    now: i64,
) {
    let expected: Vec<(OwnedRun, Vec<EventEnvelope>)> = runs
        .iter()
        .filter_map(|(run_id, events)| {
            let events = events.as_ref()?;
            Some((expected_run(run_id, events), events.clone()))
        })
        .collect();
    for &(from, to) in windows {
        for realm in [None, Some("realm")] {
            let mut wanted: Vec<OwnedRun> = expected
                .iter()
                .map(|(run, _)| run)
                .filter(|run| in_realm(run, realm) && (run.open || finished_in(run, from, to)))
                .cloned()
                .collect();
            let expected_parks: Vec<OwnedPark> = expected
                .iter()
                .flat_map(|(run, events)| {
                    parks_of(&run.run_id, run.realm.as_deref(), events, from, to, now)
                })
                .filter(|park| realm.is_none() || park.realm.as_deref() == realm)
                .collect();
            for order in [
                RunOrder::Unordered,
                RunOrder::ByEndingRule,
                RunOrder::ByRealmThenRecipe,
            ] {
                let mut actual = window_runs(store, from, to, order, realm);
                OwnedRun::sort(&mut wanted, order);
                if order == RunOrder::Unordered {
                    OwnedRun::sort(&mut actual, order);
                }
                assert_eq!(actual, wanted, "{order:?} {from}..{to} {realm:?}");
            }
            // The read's one order: phase, then run, then the park's own
            // start; a stable sort keeps the parks' seq order within a tie.
            let mut wanted_parks = expected_parks;
            wanted_parks.sort_by(|left, right| {
                left.phase
                    .cmp(&right.phase)
                    .then_with(|| left.run_id.cmp(&right.run_id))
                    .then_with(|| left.at.cmp(&right.at))
            });
            assert_eq!(
                window_parks(store, from, to, now, realm),
                wanted_parks,
                "{from}..{to} {realm:?}"
            );

            let mut spans: Vec<(Option<String>, i64)> = expected
                .iter()
                .map(|(run, _)| run)
                .filter(|run| run.realm.as_deref() == realm || realm.is_none())
                .filter_map(|run| {
                    let finished = run.finished_ms?;
                    (finished >= from && finished < to).then(|| {
                        (
                            run.realm.clone(),
                            (finished - run.started_ms.unwrap_or(finished)).max(0),
                        )
                    })
                })
                .collect();
            let mut read_spans = durations(store, from, to, realm);
            spans.sort();
            read_spans.sort();
            assert_eq!(read_spans, spans, "durations {from}..{to} {realm:?}");
            let wanted_count = expected
                .iter()
                .map(|(run, _)| run)
                .filter(|run| in_realm(run, realm) && finished_in(run, from, to))
                .filter(|run| {
                    run.ending
                        .is_some_and(|ending| ending.status() == Status::Completed)
                })
                .count() as u64;
            assert_eq!(
                store.finished_between(from, to, realm).unwrap(),
                wanted_count,
                "finished_between {from}..{to} {realm:?}"
            );
        }
    }
}

/// A window on each side of every event and park time, and the whole.
fn windows(runs: &[(String, Option<Vec<EventEnvelope>>)], now: i64) -> Vec<(i64, i64)> {
    let mut times: Vec<i64> = runs
        .iter()
        .filter_map(|(_, events)| events.as_ref())
        .flatten()
        .filter_map(|event| parse_millis(&event.recorded_at))
        .collect();
    times.sort_unstable();
    times.dedup();
    let lo = *times.first().expect("a journal holds an event");
    let span = (*times.last().unwrap() - lo).max(1);
    let mut windows = vec![
        (lo - 1, now + 1),
        (lo, lo + span / 3),
        (lo + span / 3, lo + 2 * span / 3),
        (lo + span / 2, now + 1),
        (lo + 2 * span / 3, now + 1),
    ];
    windows.dedup();
    windows
}

/// One run with `realm` named by its start's world pin.
fn manifest(realm: Option<&str>) -> serde_json::Value {
    match realm {
        Some(realm) => json!({
            "bundle_name": "self",
            "realms": {
                "source": "realm",
                "realm": realm,
                "map": {"realms": [{"name": realm, "path": "/realm"}]},
            },
        }),
        None => json!({"bundle_name": "self"}),
    }
}

/// A run started under `realm`, every step at a chosen second.
fn chain(
    run_id: &str,
    realm: Option<&str>,
    start: i64,
    steps: &[(EventType, serde_json::Value)],
) -> Vec<EventEnvelope> {
    let mut previous = ZERO_HASH.to_string();
    let mut events = Vec::new();
    for (at, (event_type, payload)) in steps.iter().enumerate() {
        let mut payload = payload.clone();
        if at == 0 {
            payload["manifest"] = manifest(realm);
        }
        let second = start + at as i64;
        let stamp = format!("2026-01-01T00:{:02}:{:02}Z", second / 60, second % 60);
        let event = EnvelopeBuilder::new(*event_type, payload)
            .run(run_id)
            .seq(at as u64 + 1)
            .event_id(format!("{run_id}-{at}"))
            .at(stamp)
            .previous(previous.clone())
            .sealed();
        previous = event.event_hash.clone();
        events.push(event);
    }
    events
}

/// A finished run's steps, from its start to `run/completed`.
fn finished_steps(feature: &str) -> Vec<(EventType, serde_json::Value)> {
    vec![
        (EventType::RunStarted, json!({"feature": feature})),
        (EventType::PhaseEntered, json!({"phase": "ship"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "fx", "phase": "ship", "seat": "ship"}),
        ),
        (
            EventType::EffectStarted,
            json!({"attempt_id": "attempt", "effect_id": "fx"}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "fx", "result": {"result": "shipped"}}),
        ),
        (
            EventType::TransitionDecided,
            json!({"from": "ship", "inputs": {}, "next": "done", "problem": null,
                   "result": "shipped", "rule_id": "SHIP-COMPLETE", "severity": "normal"}),
        ),
        (EventType::PhaseEntered, json!({"phase": "done"})),
        (EventType::RunCompleted, json!({})),
    ]
}

/// A stopped run's steps: an accepted stop with no named cause, which
/// brokkr-core classifies `Unclassified` (decision 0064 ruling 5).
fn stopped_steps() -> Vec<(EventType, serde_json::Value)> {
    vec![
        (EventType::RunStarted, json!({"feature": "stopped"})),
        (EventType::PhaseEntered, json!({"phase": "implement"})),
        (
            EventType::OperatorCommanded,
            json!({"command_id": "s", "command": "stop", "operator": "op", "args": {}}),
        ),
        (
            EventType::OperatorAccepted,
            json!({"command_id": "s", "operator": "op", "reason": "enough"}),
        ),
        (EventType::RunStopped, json!({})),
    ]
}

/// A run stopped before it entered any phase: its row's status is
/// `stopped` and its phase NULL, which is no hard stop (the comparison
/// must be NULL-safe).
fn stopped_before_any_phase_steps() -> Vec<(EventType, serde_json::Value)> {
    vec![
        (EventType::RunStarted, json!({"feature": "stopped at once"})),
        (EventType::RunStopped, json!({})),
    ]
}

/// A run the table's hard stop ended: a review ruling sent it to `stop`,
/// the phase was entered, and the run stopped. `rule` is the ruling's
/// `rule_id`, absent when it names none.
fn hard_stop_steps(rule: Option<&str>) -> Vec<(EventType, serde_json::Value)> {
    let mut decided = json!({"from": "review", "inputs": {}, "next": "stop",
                   "problem": null, "result": "ruled", "severity": "normal"});
    if let Some(rule) = rule {
        decided["rule_id"] = json!(rule);
    }
    vec![
        (EventType::RunStarted, json!({"feature": "hard stop"})),
        (EventType::PhaseEntered, json!({"phase": "review"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "fx", "seat": "builder"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "fx", "attempt_id": "a"}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "fx", "result": {"result": "complete"}}),
        ),
        (EventType::TransitionDecided, decided),
        (EventType::PhaseEntered, json!({"phase": "stop"})),
        (EventType::RunStopped, json!({})),
    ]
}

/// A parked run's steps, from its start to the open park, in `phase`.
fn parked_steps(phase: &str) -> Vec<(EventType, serde_json::Value)> {
    vec![
        (EventType::RunStarted, json!({"feature": "parked"})),
        (EventType::PhaseEntered, json!({"phase": phase})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "fx", "phase": "implement", "seat": "implement"}),
        ),
        (
            EventType::EffectStarted,
            json!({"attempt_id": "attempt", "effect_id": "fx"}),
        ),
        (
            EventType::EffectFailed,
            json!({"effect_id": "fx", "error": "the seat exited 1"}),
        ),
        (EventType::RunParked, json!({"reason": "waiting"})),
    ]
}

/// Every fixture journal, each run's events once the store migrated it, and
/// every window held to brokkr-core's whole reading.
#[test]
fn every_fixture_window_reads_as_the_whole_journal() {
    let (_dir, mut store) = store();
    let (runs, _refused) = import_fixtures(&mut store);
    let store = refolded(store);
    let runs: Vec<(String, Option<Vec<EventEnvelope>>)> = runs
        .into_iter()
        .map(|run_id| {
            let events = readable(&store, &run_id);
            (run_id, events)
        })
        .collect();
    let now = runs
        .iter()
        .filter_map(|(_, events)| events.as_ref())
        .flatten()
        .filter_map(|event| parse_millis(&event.recorded_at))
        .max()
        .expect("a fixture holds an event")
        + 1_000;
    let windows = windows(&runs, now);
    compare_windows(&store, &runs, &windows, now);

    // A behind run is never in a window's runs, and is in behind_runs.
    let mut behind: Vec<String> = Vec::new();
    store
        .behind_runs(|run_id| behind.push(run_id.to_string()))
        .unwrap();
    behind.sort();
    let mut wanted: Vec<String> = runs
        .iter()
        .map(|(run_id, _)| run_id.clone())
        .filter(|run_id| lane(&store, run_id).as_deref() == Some("behind"))
        .collect();
    wanted.sort();
    assert_eq!(behind, wanted);
    for &(from, to) in &windows {
        let in_window = window_runs(&store, from, to, RunOrder::Unordered, None);
        for run_id in &wanted {
            assert!(
                !in_window.iter().any(|run| &run.run_id == run_id),
                "behind run {run_id} in window {from}..{to}"
            );
        }
    }
}

/// A hard stop whose last ruling names no rule is still a hard stop: the
/// flag comes from the fold's status and phase, never the rule column. A
/// run stopped before it entered a phase is no hard stop, and its NULL
/// phase must not turn the flag into SQL NULL.
#[test]
fn a_hard_stop_that_names_no_rule_is_still_a_stop() {
    let (_dir, store) = store();
    let no_rule = chain("no-rule", Some("realm"), 0, &hard_stop_steps(None));
    let named = chain(
        "named",
        Some("realm"),
        20,
        &hard_stop_steps(Some("STOP-RULE")),
    );
    let no_phase = chain(
        "no-phase",
        Some("realm"),
        40,
        &stopped_before_any_phase_steps(),
    );
    plant_older_writer(&store, "no-rule", &no_rule);
    plant_older_writer(&store, "named", &named);
    plant_older_writer(&store, "no-phase", &no_phase);
    let store = refolded(store);

    let mut runs = window_runs(&store, 0, i64::MAX, RunOrder::Unordered, None);
    runs.sort_by(|left, right| left.run_id.cmp(&right.run_id));
    assert_eq!(
        runs.iter()
            .map(|run| {
                (
                    run.run_id.as_str(),
                    run.hard_stop,
                    run.ending_rule.as_deref(),
                )
            })
            .collect::<Vec<_>>(),
        [
            ("named", true, Some("STOP-RULE")),
            ("no-phase", false, None),
            ("no-rule", true, None),
        ],
        "both hard stops kept; the stop that entered no phase is no hard stop"
    );
    // The flag reads the stored phase's cut, which is the whole `stop`: a
    // longer phase cannot cut to it, so `phase_whole` is absent.
    let whole: Option<String> = store
        .conn
        .query_row(
            "SELECT phase_whole FROM run_summary WHERE run_id = 'no-rule'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(whole, None);
    // The no-phase stop is stored exactly as the row reader sees it: a
    // stopped status and a NULL phase.
    let (status, phase): (String, Option<String>) = store
        .conn
        .query_row(
            "SELECT status, phase FROM run_summary WHERE run_id = 'no-phase'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((status.as_str(), phase), (Status::Stopped.as_str(), None));
}

/// A hand-built journal whose parks and runs cut the chosen windows.
#[test]
fn a_hand_built_window_reads_as_the_whole_journal() {
    let (_dir, store) = store();
    let parked = chain("parked", Some("realm"), 0, &parked_steps("review"));
    let finished = chain("finished", None, 20, &finished_steps("a finished feature"));
    let other = chain("zulu", None, 40, &parked_steps("implement"));
    plant_older_writer(&store, "parked", &parked);
    plant_older_writer(&store, "finished", &finished);
    plant_older_writer(&store, "zulu", &other);
    let store = refolded(store);
    let runs = vec![
        ("parked".to_string(), readable(&store, "parked")),
        ("finished".to_string(), readable(&store, "finished")),
        ("zulu".to_string(), readable(&store, "zulu")),
    ];
    let now = runs
        .iter()
        .filter_map(|(_, events)| events.as_ref())
        .flatten()
        .filter_map(|event| parse_millis(&event.recorded_at))
        .max()
        .unwrap()
        + 1_000;
    let windows = windows(&runs, now);
    compare_windows(&store, &runs, &windows, now);
}

/// The preceding count is the headline's `previous`: it counts the runs
/// shipped in the window, as `brokkr-view`'s `shipped_in` does (decision
/// 0089 ruling 7), while the medians' pass still measures every finished
/// run.
#[test]
fn the_previous_count_counts_only_shipped_runs() {
    let (_dir, store) = store();
    let shipped = chain("shipped", Some("realm"), 0, &finished_steps("a feature"));
    let stopped = chain("stopped", Some("realm"), 20, &stopped_steps());
    plant_older_writer(&store, "shipped", &shipped);
    plant_older_writer(&store, "stopped", &stopped);
    let store = refolded(store);

    let from = parse_millis("2026-01-01T00:00:00Z").unwrap();
    let to = parse_millis("2026-01-01T00:01:00Z").unwrap();
    let mut runs = window_runs(&store, from, to, RunOrder::Unordered, None);
    runs.sort_by(|left, right| left.run_id.cmp(&right.run_id));
    assert_eq!(
        runs.iter()
            .map(|run| (run.run_id.as_str(), run.ending))
            .collect::<Vec<_>>(),
        [
            ("shipped", Some(Ending::Shipped)),
            ("stopped", Some(Ending::Unclassified)),
        ],
        "both runs finished in the window, and only one shipped"
    );
    assert_eq!(store.finished_between(from, to, None).unwrap(), 1);
    assert_eq!(store.finished_between(from, to, Some("realm")).unwrap(), 1);

    // The medians still measure every finished run, shipped or not.
    let mut spans = durations(&store, from, to, None);
    spans.sort();
    assert_eq!(
        spans,
        [
            (Some("realm".to_string()), 4_000),
            (Some("realm".to_string()), 7_000),
        ]
    );
}

/// A run behind its head is never in a window and is in `behind_runs`: one
/// an older writer's append left behind, and one whose aggregate stamp is
/// not its folded head's.
#[test]
fn a_behind_run_is_read_whole_and_never_in_a_window() {
    let (_dir, mut store) = store();
    store
        .create_run("lagging", "a feature", "self", &json!({}))
        .unwrap();
    crate::test_support::append(
        &mut store,
        "lagging",
        EventType::RunStarted,
        json!({"feature": "a feature", "manifest": {}}),
    );
    let (seq, previous) = store.head_hash("lagging").unwrap();
    let moved = EnvelopeBuilder::new(EventType::PhaseEntered, json!({"phase": "review"}))
        .run("lagging")
        .seq(seq + 1)
        .event_id("lagging-peer")
        .at("2026-01-01T00:00:09Z")
        .previous(previous)
        .sealed();
    plant_envelope(&store.conn, &moved).unwrap();
    lags(&mut store, &["lagging"]);

    store
        .create_run("stale", "a feature", "self", &json!({}))
        .unwrap();
    crate::test_support::append(
        &mut store,
        "stale",
        EventType::RunStarted,
        json!({"feature": "a feature", "manifest": {}}),
    );
    crate::test_support::append(&mut store, "stale", EventType::RunCompleted, json!({}));
    store
        .conn
        .execute(
            "UPDATE run_summary SET aggregated_seq = 0 WHERE run_id = 'stale'",
            [],
        )
        .unwrap();
    lags(&mut store, &["stale"]);

    // A run an older writer planted, with no fold of its own.
    let older = chain("older", None, 60, &parked_steps("implement"));
    plant_older_writer(&store, "older", &older);

    let (from, to) = (0, i64::MAX);
    assert_eq!(
        window_runs(&store, from, to, RunOrder::Unordered, None),
        Vec::new()
    );
    let mut behind = Vec::new();
    store
        .behind_runs(|run_id| behind.push(run_id.to_string()))
        .unwrap();
    assert_eq!(behind, ["lagging", "older", "stale"]);
}

/// A finished row behind its head contributes to no indexed figure: the
/// caller reads it whole, so the count and the medians never see its stale
/// columns (ruling 8).
#[test]
fn a_behind_run_is_not_counted_or_measured() {
    let (_dir, store) = store();
    let finished = chain("stale", Some("realm"), 0, &finished_steps("a feature"));
    plant_older_writer(&store, "stale", &finished);
    let store = refolded(store);
    store
        .conn
        .execute(
            "UPDATE run_summary SET aggregated_seq = 0 WHERE run_id = 'stale'",
            [],
        )
        .unwrap();
    assert_eq!(lane(&store, "stale").as_deref(), Some("behind"));
    let mut behind = Vec::new();
    store
        .behind_runs(|run_id| behind.push(run_id.to_string()))
        .unwrap();
    assert_eq!(behind, ["stale"]);

    let (from, to) = (0, i64::MAX);
    assert_eq!(
        window_runs(&store, from, to, RunOrder::Unordered, None),
        Vec::new()
    );
    assert_eq!(store.finished_between(from, to, None).unwrap(), 0);
    assert_eq!(store.finished_between(from, to, Some("realm")).unwrap(), 0);
    let spans = durations(&store, from, to, None);
    assert!(spans.is_empty(), "{spans:?}");
}

/// The stored ending's word is read into decision 0064's twelve at the
/// edge; a word outside them is refused, never carried untyped.
#[test]
fn an_ending_outside_the_twelve_is_refused_at_the_row() {
    let (_dir, store) = store();
    let finished = chain("finished", None, 0, &finished_steps("a feature"));
    plant_older_writer(&store, "finished", &finished);
    let store = refolded(store);
    store
        .conn
        .execute(
            "UPDATE run_summary SET ending = 'running' WHERE run_id = 'finished'",
            [],
        )
        .unwrap();
    let error = store
        .window_runs(0, i64::MAX, RunOrder::Unordered, None, |_| {})
        .unwrap_err();
    let StoreError::Sqlite(rusqlite::Error::FromSqlConversionFailure(column, kind, source)) =
        &error
    else {
        panic!("not a conversion failure: {error:?}");
    };
    assert_eq!((*column, *kind), (6, rusqlite::types::Type::Text));
    assert_eq!(
        source.downcast_ref::<UnknownEnding>(),
        Some(&UnknownEnding {
            word: "running".to_string(),
        })
    );
    assert_eq!(
        error.to_string(),
        "sqlite: Conversion error from type Text at index: 6, 'running' is not an ending; an \
         ending is one of shipped, shipped-flagged, refused, failed, blocked, exhausted, \
         superseded, infrastructure, delivered-elsewhere, withdrawn, operator-stopped, \
         unclassified"
    );
}

/// The aggregates are unread on an unmigrated journal and while a park's
/// facts wait, and read after the next writable open.
#[test]
fn aggregates_are_ready_only_once_every_projection_is_backfilled() {
    let (_dir, store) = store();
    assert!(store.aggregates_ready().unwrap());
    unmigrate(&store.conn).unwrap();
    assert!(!store.aggregates_ready().unwrap());
    let store = refolded(store);
    assert!(store.aggregates_ready().unwrap());

    let parked = chain("parked", None, 0, &parked_steps("implement"));
    plant_older_writer(&store, "parked", &parked);
    let store = refolded(store);
    assert!(store.aggregates_ready().unwrap());
    store
        .conn
        .execute(
            "UPDATE event_index SET facts_seq = NULL WHERE type = 'run/parked'",
            [],
        )
        .unwrap();
    assert!(!store.aggregates_ready().unwrap());
    let store = refolded(store);
    assert!(store.aggregates_ready().unwrap());
}

/// Each readiness check refuses on its own: an event index that is not
/// intact beside an intact summary, and either backfill's mark unfinished
/// while every projection is intact.
#[test]
fn each_readiness_check_refuses_on_its_own() {
    let (_dir, store) = store();
    assert!(store.aggregates_ready().unwrap());
    store
        .conn
        .execute_batch("DROP INDEX event_index_park")
        .unwrap();
    assert!(
        summary_intact(&store.conn).unwrap(),
        "the summary stays intact"
    );
    assert!(!store.aggregates_ready().unwrap(), "an index not intact");
    let mut store = refolded(store);
    assert!(store.aggregates_ready().unwrap());
    for key in [SUMMARISED, INDEXED] {
        store
            .conn
            .execute("DELETE FROM meta WHERE key = ?1", [key])
            .unwrap();
        assert!(!store.aggregates_ready().unwrap(), "{key} unfinished");
        store = refolded(store);
        assert!(store.aggregates_ready().unwrap(), "{key} finished again");
    }
}

/// A park of class 63 at a window near `i64::MIN` is found without
/// overflow, and an open park reads its end as `now`.
#[test]
fn class_63_saturates_and_an_open_park_reads_as_now() {
    let (_dir, store) = store();
    crate::test_support::plant_run(&store.conn, "r", "{}").unwrap();
    let planted = |seq, phase, at, answered, ended, class| ParkRow {
        seq,
        phase,
        at,
        answered,
        ended,
        class,
    };
    plant_park(
        &store.conn,
        "r",
        planted(
            1,
            "old",
            i64::MIN + 100,
            Some(i64::MIN + 200),
            Some(i64::MIN + 300),
            63,
        ),
    );
    plant_park(
        &store.conn,
        "r",
        planted(2, "open", i64::MIN + 500, None, None, 64),
    );
    let from = i64::MIN + 100;
    let to = i64::MIN + 1_000;
    let now = 1_000_000;
    let parks = window_parks(&store, from, to, now, None);
    assert_eq!(
        parks,
        [
            OwnedPark {
                run_id: "r".into(),
                realm: None,
                phase: Some("old".into()),
                at: from,
                answered: Some(i64::MIN + 200),
                ended: i64::MIN + 300,
                open: false,
            },
            OwnedPark {
                run_id: "r".into(),
                realm: None,
                phase: Some("open".into()),
                at: i64::MIN + 500,
                answered: None,
                ended: now,
                open: true,
            },
        ]
    );
}

/// A closed park that reaches into its class's range but ends before the
/// window is visited and rejected, as brokkr-core's own reading rejects
/// it: the reach is part of the range's ruling, not a second pass.
#[test]
fn a_closed_park_that_ended_before_the_window_is_not_read() {
    let (_dir, store) = store();
    let events = closed_park("closed", "review", 500, 1_499);
    plant_older_writer(&store, "closed", &events);
    let store = refolded(store);
    let base = parse_millis("2026-01-01T00:00:00Z").unwrap();
    let now = base + 10_000;
    let (from, to) = (base + 1_500, base + 2_500);
    assert_eq!(
        window_parks(&store, from, to, now, None),
        parks_of("closed", None, &events, from, to, now),
        "the window reads the park as brokkr-core reads it"
    );
    assert!(
        window_parks(&store, from, to, now, None).is_empty(),
        "the park ended before the window"
    );
    // The same park is found where it reaches the window, so the empty
    // read is the reach's ruling and not a park no range ever read.
    assert_eq!(
        window_parks(&store, base + 500, to, now, None),
        parks_of("closed", None, &events, base + 500, to, now)
    );
    assert_eq!(window_parks(&store, base + 500, to, now, None).len(), 1);
}

/// A closed park of no reach is class 0: the store's arms never read it,
/// and brokkr-core's own span class gives it no window to overlap, so the
/// oracle excludes it by the same class rather than by its times alone.
#[test]
fn a_zero_reach_park_is_in_no_window() {
    let (_dir, store) = store();
    let events = closed_park("zero", "review", 500, 500);
    plant_older_writer(&store, "zero", &events);
    let store = refolded(store);
    let base = parse_millis("2026-01-01T00:00:00Z").unwrap();
    let now = base + 10_000;
    let (from, to) = (base + 100, base + 1_000);
    assert!(
        parks_of("zero", None, &events, from, to, now).is_empty(),
        "class 0 reaches nowhere"
    );
    assert_eq!(
        window_parks(&store, from, to, now, None),
        parks_of("zero", None, &events, from, to, now),
        "a zero-reach park is in no window"
    );
}

/// Every read's plan names its index and scans no projection whole.
#[test]
fn every_aggregate_read_is_indexed() {
    let (_dir, store) = store();
    for (order, index) in [
        (RunOrder::Unordered, "run_summary_finished"),
        (RunOrder::ByEndingRule, "run_summary_page"),
        (RunOrder::ByRealmThenRecipe, "run_summary_page"),
    ] {
        let plan = explain(
            &store,
            runs_sql(order),
            params_from_iter(run_bounds(0, 1, None)),
        );
        assert!(
            plan.iter().any(|step| step.contains(index)),
            "{order:?}: {plan:?}"
        );
        assert!(
            !plan.iter().any(|step| step.starts_with("SCAN run_summary")),
            "{order:?}: {plan:?}"
        );
    }
    let plan = explain(&store, &BEHIND, []);
    assert!(plan.iter().any(|step| step.contains("run_summary_page")));
    assert!(!plan.iter().any(|step| step.starts_with("SCAN run_summary")));
    for (sql, index) in [
        (&COUNT_FINISHED, "run_summary_finished"),
        (&DURATIONS, "run_summary_finished"),
    ] {
        let plan = explain(&store, sql, params![0, 1, None::<&str>]);
        assert!(plan.iter().any(|step| step.contains(index)), "{plan:?}");
        assert!(
            !plan.iter().any(|step| step.starts_with("SCAN run_summary")),
            "{plan:?}"
        );
    }
    let taken = explain(&store, &PARK_STALE, []);
    assert!(
        taken
            .iter()
            .any(|step| step.contains("event_index_park_stale")),
        "{taken:?}"
    );
    let taken = explain(
        &store,
        &PARK_SQL,
        params_from_iter(park_bounds(0, 1, 2, None)),
    );
    assert_eq!(
        taken
            .iter()
            .filter(|step| step.contains("SEARCH e USING INDEX event_index_park"))
            .count(),
        64,
        "each arm seeks the parks' index: {taken:?}"
    );
    assert_eq!(
        taken
            .iter()
            .filter(|step| step.contains("SEARCH s USING INDEX"))
            .count(),
        64,
        "each arm joins the run summary by its key: {taken:?}"
    );
    assert_eq!(
        taken
            .iter()
            .filter(|step| step.starts_with("SCAN "))
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["SCAN parks"],
        "only the materialized CTE is scanned: {taken:?}"
    );
    assert_eq!(
        taken
            .iter()
            .filter(|step| step.contains("USE TEMP B-TREE FOR ORDER BY"))
            .count(),
        1,
        "{taken:?}"
    );
}

/// One park row's dated facts, as the writer leaves it.
struct ParkRow {
    seq: i64,
    phase: &'static str,
    at: i64,
    answered: Option<i64>,
    ended: Option<i64>,
    class: i64,
}

/// A run whose park opens at `at` and ends at `ended`, both milliseconds
/// after 2026-01-01T00:00:00Z, so a window can cut it at a chosen distance.
fn closed_park(run_id: &str, phase: &str, at: i64, ended: i64) -> Vec<EventEnvelope> {
    let stamp = |ms: i64| format!("2026-01-01T00:00:{:02}.{:03}Z", ms / 1_000, ms % 1_000);
    let steps = [
        (EventType::RunStarted, json!({"feature": "closed"})),
        (EventType::PhaseEntered, json!({"phase": phase})),
        (EventType::RunParked, json!({"reason": "waiting"})),
        (EventType::PhaseEntered, json!({"phase": "implement"})),
    ];
    let mut previous = ZERO_HASH.to_string();
    let mut events = Vec::new();
    for (index, ((event_type, payload), at)) in steps.iter().zip([at, at, at, ended]).enumerate() {
        let mut payload = payload.clone();
        if index == 0 {
            payload["manifest"] = manifest(None);
        }
        let event = EnvelopeBuilder::new(*event_type, payload)
            .run(run_id)
            .seq(index as u64 + 1)
            .event_id(format!("{run_id}-{index}"))
            .at(stamp(at))
            .previous(previous.clone())
            .sealed();
        previous = event.event_hash.clone();
        events.push(event);
    }
    events
}

/// Insert one park row, as the writer leaves it.
fn plant_park(conn: &rusqlite::Connection, run_id: &str, row: ParkRow) {
    conn.execute(
        "INSERT INTO event_index
             (run_id, seq, type, phase, at_ms, answered_ms, ended_ms, span_class, facts_seq)
         VALUES (?1, ?2, 'run/parked', ?3, ?4, ?5, ?6, ?7, ?2)",
        params![
            run_id,
            row.seq,
            row.phase,
            row.at,
            row.answered,
            row.ended,
            row.class
        ],
    )
    .unwrap();
}

/// The plan SQLite takes for `sql`, each step's detail in order.
fn explain(store: &Store, sql: &str, bound: impl rusqlite::Params) -> Vec<String> {
    crate::test_support::plan(store, sql, bound).unwrap_or_else(|error| panic!("{sql}: {error}"))
}

/// The fixed window and its parks, for the ranges' VM-step count.
const FROM: i64 = 1_700_000_000_000;
const TO: i64 = FROM + 10 * DAY;
const DAY: i64 = 86_400_000;

/// One park with `reach` from `at`, its class read as the writer reads it.
fn class_of(reach: i64) -> i64 {
    if reach <= 0 {
        return 0;
    }
    i64::from(u64::BITS - (reach as u64).leading_zeros())
}

/// Plant `count` parks at `at` with `reach`, one seq each, from `next`.
fn plant_parks(
    conn: &rusqlite::Connection,
    parks: &mut i64,
    count: usize,
    at: impl Fn(usize) -> i64,
    reach: impl Fn(usize) -> i64,
) {
    for index in 0..count {
        let at = at(index);
        let reach = reach(index);
        *parks += 1;
        plant_park(
            conn,
            "r",
            ParkRow {
                seq: *parks,
                phase: "phase",
                at,
                answered: Some(at + reach),
                ended: Some(at + reach),
                class: class_of(reach),
            },
        );
    }
}

/// The parks a fixed window visits are the parks near it: a year of later
/// parks and a year of earlier ones, each shorter than half its distance
/// from the window, leave the step count equal; k parks inside the band
/// each add their own visit; and 10,000 class-0 parks add nothing.
#[test]
fn a_window_visits_only_the_parks_near_it() {
    let (_dir, store) = store();
    crate::test_support::plant_run(&store.conn, "r", "{}").unwrap();
    let mut next = 0;
    plant_near(&store.conn, &mut next);
    let step = store.window_parks_steps(FROM, TO, TO, None).unwrap();
    assert!(step > 0);

    plant_later(&store.conn, &mut next);
    assert_eq!(
        store.window_parks_steps(FROM, TO, TO, None).unwrap(),
        step,
        "later parks are in no range"
    );

    plant_earlier(&store.conn, &mut next);
    assert_eq!(
        store.window_parks_steps(FROM, TO, TO, None).unwrap(),
        step,
        "far earlier parks are in no range"
    );

    // k parks inside the band: started within the class's range and ended
    // before the window within their own length, so each is visited and
    // rejected, at one visit's cost.
    let k = 10;
    let at = FROM - 1_024;
    let before = store.window_parks_steps(FROM, TO, TO, None).unwrap();
    plant_park(
        &store.conn,
        "r",
        ParkRow {
            seq: next + 1,
            phase: "phase",
            at,
            answered: Some(FROM - 1),
            ended: Some(FROM - 1),
            class: 10,
        },
    );
    next += 1;
    let one = store.window_parks_steps(FROM, TO, TO, None).unwrap() - before;
    assert!(one > 0, "one band park is visited");
    for index in 1..k {
        plant_park(
            &store.conn,
            "r",
            ParkRow {
                seq: next + 1,
                phase: "phase",
                at: at + index as i64,
                answered: Some(FROM - 1),
                ended: Some(FROM - 1),
                class: 10,
            },
        );
        next += 1;
    }
    assert_eq!(
        store.window_parks_steps(FROM, TO, TO, None).unwrap() - before,
        one * (k as i64),
        "k band parks each cost one visit"
    );

    let banded = store.window_parks_steps(FROM, TO, TO, None).unwrap();
    plant_class_zero(&store.conn, &mut next);
    assert_eq!(
        store.window_parks_steps(FROM, TO, TO, None).unwrap(),
        banded,
        "class-0 parks are in no range"
    );
}

/// Two parks overlapping the window and two inside it, and one open park
/// of class 64 whose end reads as `now`.
fn plant_near(conn: &rusqlite::Connection, next: &mut i64) {
    plant_parks(
        conn,
        next,
        2,
        |index| FROM - 10_000 + index as i64,
        |_| 20_000,
    );
    plant_parks(conn, next, 1, |_| FROM + 1_000, |_| 0);
    plant_parks(conn, next, 2, |index| FROM + 100 + index as i64, |_| 1);
    *next += 1;
    plant_park(
        conn,
        "r",
        ParkRow {
            seq: *next,
            phase: "phase",
            at: FROM + 2_000,
            answered: None,
            ended: None,
            class: 64,
        },
    );
}

/// A year of parks after the window, so no range reads them.
fn plant_later(conn: &rusqlite::Connection, next: &mut i64) {
    plant_parks(
        conn,
        next,
        365,
        |index| TO + (index as i64 + 1) * DAY,
        |_| DAY,
    );
}

/// A year of parks before the window, each shorter than half its distance.
fn plant_earlier(conn: &rusqlite::Connection, next: &mut i64) {
    plant_parks(
        conn,
        next,
        365,
        |index| {
            let distance = (index as i64 + 1) * DAY;
            FROM - distance - distance / 4
        },
        |index| (index as i64 + 1) * DAY / 4,
    );
}

/// 10,000 class-0 parks at `from - 1` and inside the window, zero-length
/// and ended a year before they started: no range reads class 0.
fn plant_class_zero(conn: &rusqlite::Connection, next: &mut i64) {
    for index in 0..10_000 {
        let at = if index % 2 == 0 {
            FROM - 1
        } else {
            FROM + index as i64
        };
        *next += 1;
        plant_park(
            conn,
            "r",
            ParkRow {
                seq: *next,
                phase: "phase",
                at,
                answered: Some(at - 365 * DAY),
                ended: Some(at - 365 * DAY),
                class: 0,
            },
        );
    }
}
