//! The run view read from the store a page at a time is today's (#753;
//! decision 0089 slice 2b's acceptance): every fixture journal imported
//! fresh, each run opened and read at its head, its view composed from its
//! trail, its seat rows and each journal page, with the fold and the causes
//! the read hands over, equals `run_view` of the run read whole and folded,
//! field for field and byte for byte, but each participant's checkpoints,
//! and its pages' journals together are today's rows. A run whose chain is
//! broken is refused at its opening. A run whose seat rows lag, and every
//! run of a journal whose index is not read, takes the kept read, which
//! reads it whole.

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::fold::fold;
use brokkr_store::test_support::{import_fixtures, lags, plant_summary_writer, unindex};
use brokkr_store::Store;
use brokkr_view::run_view;
use serde_json::json;

#[path = "../../../tests/support/composed_read.rs"]
mod composed_read;
#[path = "../../../tests/support/composed_view.rs"]
mod composed_view_support;

/// How a journal's runs read: what each composed view shows otherwise than
/// its view read whole, the runs read the kept way, the runs refused, each
/// with the store's words, and how many were composed.
#[derive(Debug, Default)]
struct Read {
    found: Vec<String>,
    kept: Vec<String>,
    refused: Vec<String>,
    composed: usize,
}

/// How `runs` read from the journal at `path`, opened read-only.
fn read(path: &std::path::Path, runs: &[String]) -> Read {
    let store = Store::open_read_only(path).unwrap();
    let mut read = Read::default();
    for run in runs {
        let pages = match composed_read::pages(&store, run) {
            Ok(Some(pages)) => pages,
            Ok(None) => {
                read.kept.push(run.clone());
                continue;
            }
            Err(refused) => {
                read.refused.push(format!("{run}: {refused}"));
                continue;
            }
        };
        let events = store.load(run).unwrap();
        let whole = run_view(&events, fold(&events).ok().as_ref());
        let found = composed_view_support::disagreements(run, &whole, &pages);
        read.found.extend(found);
        read.composed += 1;
    }
    read
}

/// Every fixture imported into a fresh journal at `path`, its runs folded
/// by a writable open: the fixture the import refuses is planted as an
/// older writer's, and that open folds its rows.
fn imported(path: &std::path::Path) -> (Store, Vec<String>) {
    let mut store = Store::open(path).unwrap();
    let (runs, _) = import_fixtures(&mut store);
    drop(store);
    (Store::open(path).unwrap(), runs)
}

/// The broken chain's refusal at its opening, in the store's words.
const BROKEN: &str =
    "conclude-broken-chain-hand-built: chain: event 11: event_hash does not match canonical content";

#[test]
fn every_fixture_s_run_view_read_a_page_at_a_time_is_its_run_view() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let (store, runs) = imported(&path);
    drop(store);
    let read = read(&path, &runs);
    assert_eq!(read.found, Vec::<String>::new());
    assert_eq!(read.kept, Vec::<String>::new());
    assert_eq!(read.refused, [BROKEN]);
    assert_eq!(read.composed, 5);
}

/// The checkpoint after the last of `events`, as a writer chains it: a
/// turn of the last attempt started, served by a model no other names.
fn next(events: &[EventEnvelope]) -> EventEnvelope {
    let head = events.last().unwrap();
    let started = events
        .iter()
        .rev()
        .find(|event| event.event_type == EventType::EffectStarted);
    let started = &started.unwrap().payload;
    let turn = json!({"step": "seat-turn", "turn": 9, "tool": "Read", "model": "lagged"});
    let payload = json!({
        "effect_id": started["effect_id"],
        "attempt_id": started["attempt_id"],
        "checkpoint": turn,
    });
    let next = EventEnvelope {
        seq: head.seq + 1,
        event_id: format!("{}-after", head.event_id),
        event_type: EventType::EffectCheckpointed,
        payload,
        previous_hash: head.event_hash.clone(),
        ..head.clone()
    };
    next.sealed()
}

#[test]
fn a_lagging_run_and_an_unindexed_journal_are_read_the_kept_way() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let (mut store, runs) = imported(&path);
    // A writer from before the seat rows moves the run's summary row on
    // and leaves its seat rows where they stood.
    let lagging = runs[0].clone();
    let events = store.load(&lagging).unwrap();
    lags(&mut store, &[&lagging]);
    plant_summary_writer(&store, &lagging, &[next(&events)]);
    drop(store);
    let lagged = read(&path, &runs);
    assert_eq!(lagged.found, Vec::<String>::new());
    assert_eq!((lagged.kept, lagged.composed), (vec![lagging], 4));
    unindex(&rusqlite::Connection::open(&path).unwrap()).unwrap();
    let unindexed = read(&path, &runs);
    let opened = runs.iter().filter(|run| !BROKEN.starts_with(run.as_str()));
    let opened: Vec<String> = opened.cloned().collect();
    assert_eq!((unindexed.kept, unindexed.composed), (opened, 0));
}
