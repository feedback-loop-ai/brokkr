//! The run view's heap budgets (decision 0089 ruling 9): its first screen,
//! the run opened, its trail, its seat rows and its first journal page
//! read at the head the opening verified and the view composed from them,
//! holds the same live heap however many events the journal holds beside
//! the run, a small part of what the run holds, and grows by a wide control
//! envelope's text and no more. A readout's live heap is two shares,
//! Brokkr's under dhat and SQLite's from its own high-water. The budgets
//! live in `quality/heap-bytes.json`; dhat allows one profiler per process,
//! so this is a binary of its own.

#[path = "heap/budgets.rs"]
mod budgets;
#[path = "../../../tests/support/composed_read.rs"]
mod composed_read;
#[path = "heap/grown.rs"]
mod grown;
#[path = "heap/measure.rs"]
mod measure;
#[path = "heap/stretched.rs"]
mod stretched;

use std::path::{Path, PathBuf};

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_store::test_support::trail_plan;
use brokkr_store::Store;
use measure::{peak_of, sqlite_memory};
use stretched::{median_micros, Stretch, STRETCH};

/// The control envelope stretched: the operator's acceptance of the run's
/// stop, whose `reason` is the operator's own words. The grown run parks
/// nowhere, so this is the reason its trail carries.
fn accepted() -> usize {
    let events = grown::journal();
    let at = events
        .iter()
        .position(|event| event.event_type == EventType::OperatorAccepted);
    at.expect("the grown run's stop is accepted") + 1
}

/// The grown run, its acceptance's reason stretched as `stretch` says.
fn viewed_run(stretch: Stretch) -> Vec<EventEnvelope> {
    stretched::grown_run(accepted(), "reason", stretch)
}

/// A journal under `dir` named `name` of `events` events: the grown run,
/// stretched as `stretch` says, beside the generated runs, its projections
/// caught up by a writable open, as a current writer leaves them.
fn journal(dir: &Path, name: &str, events: usize, stretch: Stretch) -> PathBuf {
    let db = stretched::journal(dir, name, events, &viewed_run(stretch));
    drop(Store::open(&db).unwrap());
    db
}

/// What the first screen cost, from its read-only open to its composed
/// view: Brokkr's peak heap under dhat, and SQLite's high-water above
/// where it stood before the open.
#[derive(Debug, Clone, Copy)]
struct Screen {
    brokkr: u64,
    sqlite: u64,
}

/// The grown run's first screen read from `store`: opened, read at its
/// head with its first journal page, and composed; its event count and
/// how many journal rows it holds.
fn first_screen(store: &Store) -> (usize, usize) {
    let opened = store.open_run(grown::RUN).unwrap();
    let read = composed_read::page(store, grown::RUN, &opened, 0).unwrap();
    let (view, _) = read.expect("the grown run is read at its head");
    (view.event_count, view.journal.len())
}

/// What `db`'s first screen cost.
fn measured(db: &Path) -> Screen {
    let (base, _) = sqlite_memory(true);
    let mut shown = None;
    let brokkr = peak_of(|| {
        let store = Store::open_read_only(db).unwrap();
        shown = Some(first_screen(&store));
    });
    let (_, high) = sqlite_memory(false);
    assert_eq!(shown, Some((grown::EVENTS, 50)), "the first screen");
    Screen {
        brokkr,
        sqlite: u64::try_from(high - base).unwrap(),
    }
}

/// Ruling 9 on the run view: Brokkr's peak for the grown run's first
/// screen is the same to the byte in a journal of 10,000 events and one of
/// 1,000,000, within its budget and a tenth of the run's envelopes, and
/// SQLite's within its own; with the acceptance's reason stretched to a
/// mebibyte of plain text, Brokkr's peak grows by that envelope and no
/// more, as SQLite's does; and the trail is read by its partial index in
/// both journals. The stretch of escaped text is reported against the
/// three envelopes the opening's amended bound allows (see
/// [`the_escaped_stretch_meets_the_opening_s_bound`]). The first screen's
/// wall time is reported against ruling 9's 50 ms, not gated.
#[test]
fn the_run_view_s_first_screen_holds_its_control_events_and_a_page() {
    let dir = tempfile::tempdir().unwrap();
    let small_db = journal(dir.path(), "small.db", 10_000, Stretch::Unstretched);
    let large_db = journal(dir.path(), "large.db", 1_000_000, Stretch::Unstretched);
    let plain_db = journal(dir.path(), "plain.db", 10_000, Stretch::Plain);
    let escaped_db = journal(dir.path(), "escaped.db", 10_000, Stretch::Escaped);
    // Read once unmeasured, so what a process allocates once and keeps
    // is allocated before any screen is measured.
    measured(&small_db);
    let (small, large) = (measured(&small_db), measured(&large_db));
    let (plain, escaped) = (measured(&plain_db), measured(&escaped_db));
    eprintln!("small: {small:?}; large: {large:?}; plain: {plain:?}; escaped: {escaped:?}");
    let run = viewed_run(Stretch::Unstretched);
    let bytes = |event: &&EventEnvelope| serde_json::to_string(event).unwrap().len() as u64;
    let run_bytes: u64 = run.iter().map(|event| bytes(&event)).sum();
    let control = run
        .iter()
        .filter(|event| event.event_type != EventType::EffectCheckpointed);
    let control_bytes: u64 = control.map(|event| bytes(&event)).sum();
    eprintln!("the run's envelopes: {run_bytes} bytes, its control envelopes {control_bytes}");
    // The records scripts/measure-budgets.sh reads, each a budget.
    println!("heap\trun-view\t{}", small.brokkr.max(large.brokkr));
    println!("heap\trun-view-sqlite\t{}", small.sqlite.max(large.sqlite));
    println!("heap\trun-view-stretched\t{}", plain.brokkr);
    let store = Store::open_read_only(&small_db).unwrap();
    let paged = composed_read::pages(&store, grown::RUN).unwrap();
    let pages = paged.expect("the grown run is read at its head").len();
    assert_eq!(pages, grown::EVENTS.div_ceil(50), "every page of the run");
    let screen = median_micros(|| assert_eq!(first_screen(&store), (grown::EVENTS, 50)));
    println!("wall\trun-view-first-screen\t{screen}");
    let trail = ["SEARCH event_index USING INDEX event_index_trail (run_id=?)"];
    for db in [&small_db, &large_db] {
        let store = Store::open_read_only(db).unwrap();
        assert_eq!(trail_plan(&store, grown::RUN).unwrap(), trail, "{db:?}");
    }
    assert_eq!(
        large.brokkr, small.brokkr,
        "1,000,000 events against 10,000"
    );
    let (brokkr, sqlite) = (
        budgets::budget_named("run-view"),
        budgets::budget_named("run-view-sqlite"),
    );
    assert!(
        brokkr <= run_bytes / 10,
        "the budget {brokkr} is past a tenth of the run's {run_bytes} bytes"
    );
    assert!(
        brokkr <= 256 * 1024 + control_bytes,
        "the budget {brokkr} is past ruling 9's 256 KiB and the {control_bytes} bytes of the run's control envelopes"
    );
    for (name, screen) in [("10,000", small), ("1,000,000", large)] {
        assert!(
            screen.brokkr <= brokkr,
            "{name}: {} bytes; the budget is {brokkr}",
            screen.brokkr
        );
        assert!(
            screen.sqlite <= sqlite,
            "{name}: SQLite's {} bytes; the budget is {sqlite}",
            screen.sqlite
        );
    }
    let stretch = STRETCH as u64;
    let growth = plain.brokkr - small.brokkr;
    assert!(
        growth <= stretch,
        "a mebibyte reason grew the first screen's peak by {growth} bytes"
    );
    let stretched = budgets::budget_named("run-view-stretched");
    assert!(
        plain.brokkr <= stretched,
        "{} bytes with a mebibyte reason; the budget is {stretched}",
        plain.brokkr
    );
    let held = plain.sqlite - small.sqlite;
    assert!(
        held <= stretch,
        "a mebibyte reason grew SQLite's high-water by {held} bytes"
    );
    the_escaped_stretch_meets_the_opening_s_bound(small, escaped);
}

/// The 2026-10-24 amendment to ruling 9 bounds an opening whose envelope
/// carries escapes by three envelopes over the unstretched, since the
/// parser copies the text through its scratch buffer before the string it
/// keeps. The trail's envelopes parse the same way, so the first screen
/// with the acceptance's reason a mebibyte of escaped text grows by no
/// more than three envelopes either; its peak is reported, not budgeted,
/// until the opening's ruling applies to the run view too.
fn the_escaped_stretch_meets_the_opening_s_bound(unstretched: Screen, escaped: Screen) {
    let growth = escaped.brokkr - unstretched.brokkr;
    eprintln!("escaped: the first screen grew by {growth} bytes");
    assert!(
        growth <= 3 * STRETCH as u64,
        "an escaped mebibyte reason grew the first screen's peak by {growth} bytes"
    );
}
