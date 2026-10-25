//! The run view's heap budgets (decision 0089 ruling 9): its first screen,
//! the run opened, its trail, its seat rows and its first journal page
//! read at the head the opening verified and the view composed from them,
//! holds the same live heap however many events the journal holds beside
//! the run, a small part of what the run holds, and grows with a parked
//! run's wide reason by the copies of it the screen holds and no more. A
//! readout's live heap is two shares,
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
use std::sync::{Mutex, PoisonError};

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_store::test_support::trail_plan;
use brokkr_store::Store;
use measure::{peak_of, sqlite_memory};
use serde_json::json;
use stretched::{median_micros, Stretch, SQLITE_SLACK, STRETCH};

/// The grown run parked at its head, as ruling 9 measures a park's reason:
/// cut after its last request, before that effect starts, and parked
/// there, where the engine parks a seat whose attempts are spent (decision
/// 0006). The park's `reason` is what the run view shows it parked for.
fn parked() -> Vec<EventEnvelope> {
    let mut events = grown::journal();
    let asked = events
        .iter()
        .rposition(|event| event.event_type == EventType::EffectRequested);
    let asked = asked.expect("the grown run asks for its verify");
    events.truncate(asked + 1);
    let park = EventEnvelope {
        seq: events[asked].seq + 1,
        event_id: "the-grown-run-parks".to_string(),
        event_type: EventType::RunParked,
        payload: json!({"evidence": {}, "reason": "the verify seat's attempts are spent"}),
        causation_id: Some(events[asked].event_id.clone()),
        attempt_id: None,
        ..events[asked].clone()
    };
    events.push(park);
    events
}

/// The grown run parked, its park's reason stretched as `stretch` says.
fn parked_run(stretch: Stretch) -> Vec<EventEnvelope> {
    let events = parked();
    let seq = events.len();
    stretched::run_stretched(events, seq, "reason", stretch)
}

/// A journal under `dir` named `name` of `events` events: `run` beside the
/// generated runs, its projections caught up by a writable open, as a
/// current writer leaves them.
fn journal(dir: &Path, name: &str, events: usize, run: &[EventEnvelope]) -> PathBuf {
    let db = stretched::journal(dir, name, events, run);
    drop(Store::open(&db).unwrap());
    db
}

/// The grown run, chained, and two journals under `dir` holding it beside
/// the generated runs, one of 10,000 events and one of 1,000,000.
fn grown_journals(dir: &Path) -> (Vec<EventEnvelope>, [PathBuf; 2]) {
    let run = grown::chained(grown::journal());
    let small = journal(dir, "small.db", 10_000, &run);
    let large = journal(dir, "large.db", 1_000_000, &run);
    (run, [small, large])
}

/// dhat allows one profiler per process, so this binary's measurements
/// are made one at a time.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// What the first screen cost, from its read-only open to its composed
/// view: Brokkr's peak heap under dhat, and SQLite's high-water above
/// where it stood before the open.
#[derive(Debug, Clone, Copy)]
struct Screen {
    brokkr: u64,
    sqlite: u64,
}

/// The bytes of `run`'s envelopes that `keep` keeps, as JSON text.
fn envelope_bytes(run: &[EventEnvelope], keep: impl Fn(&EventEnvelope) -> bool) -> u64 {
    let kept = run.iter().filter(|event| keep(event));
    kept.map(|event| serde_json::to_string(event).unwrap().len() as u64)
        .sum()
}

/// The bytes of `run`'s control envelopes, every one but a checkpoint.
fn control_bytes(run: &[EventEnvelope]) -> u64 {
    envelope_bytes(run, |event| {
        event.event_type != EventType::EffectCheckpointed
    })
}

/// What a first screen shows: its event count, how many journal rows it
/// holds, and how long the reason it shows the run parked for is.
type Shown = (usize, usize, Option<usize>);

/// What the first screen of `run` shows: its events, a page of them, and
/// its last event's reason where it parks there.
fn shows(run: &[EventEnvelope]) -> Shown {
    let reason = run[run.len() - 1].payload["reason"].as_str();
    (run.len(), 50, reason.map(str::len))
}

/// The grown run's first screen read from `store`: opened, read at its
/// head with its first journal page, and composed.
fn first_screen(store: &Store) -> Shown {
    let opened = store.open_run(grown::RUN).unwrap();
    let read = composed_read::page(store, grown::RUN, &opened, 0).unwrap();
    let (view, _) = read.expect("the grown run is read at its head");
    let summary = view.summary.expect("the grown run folds");
    let parked = summary.park_reason.map(|reason| reason.len());
    (view.event_count, view.journal.len(), parked)
}

/// What `db`'s first screen cost, holding that it shows `shows`.
fn measured(db: &Path, shows: Shown) -> Screen {
    let (base, _) = sqlite_memory(true);
    let mut shown = None;
    let brokkr = peak_of(|| {
        let store = Store::open_read_only(db).unwrap();
        shown = Some(first_screen(&store));
    });
    let (_, high) = sqlite_memory(false);
    assert_eq!(shown, Some(shows), "the first screen");
    Screen {
        brokkr,
        sqlite: u64::try_from(high - base).unwrap(),
    }
}

/// Ruling 9 on the run view: Brokkr's peak for the grown run's first
/// screen is the same to the byte in a journal of 10,000 events and one of
/// 1,000,000, within its budget and a tenth of the run's envelopes, and
/// SQLite's within its own; and the trail is read by its partial index in
/// both journals. With the run parked and its park's reason stretched to a
/// mebibyte, plain or escaped, Brokkr's peak grows over the run parked
/// unstretched by more than the reason and by no more than the copies of
/// it the screen holds ([`HELD`]), ruling 9 as the operator amended it on
/// 2026-10-25; the plain peak is within its budget, and SQLite's growth
/// within the envelope and [`SQLITE_SLACK`]. SQLite's budget is the
/// fleet's cap on every host.
/// The first screen's wall time is reported against ruling 9's 50 ms, not
/// gated.
#[test]
fn the_run_view_s_first_screen_holds_its_control_events_and_a_page() {
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
    let dir = tempfile::tempdir().unwrap();
    let (run, [small_db, large_db]) = grown_journals(dir.path());
    let parks = [Stretch::Unstretched, Stretch::Plain, Stretch::Escaped].map(|stretch| {
        let parked = parked_run(stretch);
        let name = format!("parked-{stretch:?}.db");
        (journal(dir.path(), &name, 10_000, &parked), shows(&parked))
    });
    // Read once unmeasured, so what a process allocates once and keeps
    // is allocated before any screen is measured.
    let grown = shows(&run);
    measured(&small_db, grown);
    let (small, large) = (measured(&small_db, grown), measured(&large_db, grown));
    let [parked, plain, escaped] = parks.map(|(db, shown)| measured(&db, shown));
    eprintln!("small: {small:?}; large: {large:?}; parked: {parked:?}; plain: {plain:?}; escaped: {escaped:?}");
    let (run_bytes, control_bytes) = (envelope_bytes(&run, |_| true), control_bytes(&run));
    eprintln!("the run's envelopes: {run_bytes} bytes, its control envelopes {control_bytes}");
    // The records scripts/measure-budgets.sh reads, each a budget.
    println!("heap\trun-view\t{}", small.brokkr.max(large.brokkr));
    println!("heap\trun-view-sqlite\t{}", small.sqlite.max(large.sqlite));
    println!("heap\trun-view-stretched\t{}", plain.brokkr);
    let store = Store::open_read_only(&small_db).unwrap();
    let paged = composed_read::pages(&store, grown::RUN).unwrap();
    let pages = paged.expect("the grown run is read at its head").len();
    assert_eq!(pages, grown::EVENTS.div_ceil(50), "every page of the run");
    let screen = median_micros(|| assert_eq!(first_screen(&store), grown));
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
    assert_eq!(
        sqlite,
        measure::SQLITE_CAP,
        "SQLite's budget is the fleet's cap on every host"
    );
    assert!(
        brokkr <= 256 * 1024 + control_bytes,
        "the budget {brokkr} is past ruling 9's 256 KiB and the {control_bytes} bytes of the run's control envelopes"
    );
    for (name, screen) in [("10,000", small), ("1,000,000", large)] {
        measure::report_headroom(name, screen.sqlite);
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
    let stretched = budgets::budget_named("run-view-stretched");
    assert!(
        plain.brokkr <= stretched,
        "{} bytes with a mebibyte reason; the budget is {stretched}",
        plain.brokkr
    );
    for (name, screen) in [("plain", plain), ("escaped", escaped)] {
        let growth = screen.brokkr - parked.brokkr;
        eprintln!("{name}: the first screen grew by {growth} bytes");
        assert!(
            growth > STRETCH as u64,
            "a mebibyte of {name} reason grew the first screen's peak by {growth} bytes, within one reason: the screen no longer holds the copies ruling 9's amendment of 2026-10-25 counts"
        );
        assert!(
            growth <= HELD * STRETCH as u64,
            "a mebibyte of {name} reason grew the first screen's peak by {growth} bytes, past ruling 9's three reasons as amended 2026-10-25"
        );
    }
    let held = plain.sqlite - parked.sqlite;
    assert!(
        held <= STRETCH as u64 + SQLITE_SLACK,
        "a mebibyte reason grew SQLite's high-water by {held} bytes"
    );
}

/// The copies of a parked run's reason its first screen holds at once: the
/// trail's envelope, the fold of the trail, and the view's summary, which
/// `brokkr_view::summary` copies from the fold. Three is ruling 9's bound
/// as the operator amended it on 2026-10-25, as the opening's escaped
/// bound was amended to three envelopes on 2026-10-24. Holding it once
/// would move brokkr-core's fold and `brokkr_view::Summary`.
/// An escaped reason costs no more here:
/// the trail's envelopes parse from the text SQLite holds, so the parser's
/// scratch copy is gone before the fold's and the view's are made.
const HELD: u64 = 3;

/// What `brokkr watch --once` paints of the grown run in the journal at
/// `db`, with `whole` what `inspect --json` prints of it, each read as the
/// readout reads it (decision 0089 slice 3).
fn readout(db: &Path, whole: bool) -> String {
    brokkr_cli::run_readout_for_budget(db, grown::RUN, whole).unwrap()
}

/// The grown run with every checkpoint among its first 50 events, the
/// first journal page, a mebibyte of plain text: what a frame that read
/// that page would hold at once.
fn first_page_stretched() -> Vec<EventEnvelope> {
    let mut events = grown::journal();
    for event in events.iter_mut().take(50) {
        if event.event_type == EventType::EffectCheckpointed {
            let one = vec![event.clone()];
            *event = stretched::run_stretched(one, 1, "stretch", Stretch::Plain).remove(0);
        }
    }
    grown::chained(events)
}

/// What `db`'s terminal frame cost, from its read-only open to its painted
/// text, holding that it paints `shown`.
fn framed(db: &Path, shown: &str) -> Screen {
    let (base, _) = sqlite_memory(true);
    let mut painted = None;
    let brokkr = peak_of(|| painted = Some(readout(db, false)));
    let (_, high) = sqlite_memory(false);
    assert_eq!(painted.as_deref(), Some(shown), "the frame of {db:?}");
    Screen {
        brokkr,
        sqlite: u64::try_from(high - base).unwrap(),
    }
}

/// Ruling 9 on the run view, for the terminal frame `inspect`, `seats` and
/// `watch` paint (decision 0089 slice 3): from its read-only open to its
/// painted text, the run opened and one read's trail composed, Brokkr's
/// peak is the same to the byte in a journal of 10,000 events and one of
/// 1,000,000 and within its budget, itself within ruling 9's 256 KiB and
/// the run's control envelopes; SQLite's within the fleet's cap; the two
/// frames are one. With every checkpoint of the run's first journal page
/// a mebibyte, the frame is the same and its peak within the budget and
/// the one checkpoint the opening verifies at a time: the frame reads no
/// journal page. The frame's wall time is reported against ruling 9's
/// 50 ms, and the whole read `inspect --json` makes, every journal page
/// and checkpoint page, beside it; neither is gated.
#[test]
fn the_run_s_terminal_frame_holds_its_trail_and_no_checkpoint() {
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
    let dir = tempfile::tempdir().unwrap();
    let (run, [small_db, large_db]) = grown_journals(dir.path());
    // Painted once unmeasured, so what a process allocates once and keeps
    // is allocated before any frame is measured.
    let shown = readout(&small_db, false);
    let (small, large) = (framed(&small_db, &shown), framed(&large_db, &shown));
    let wide = first_page_stretched();
    let wide_db = journal(dir.path(), "first-page-stretched.db", 10_000, &wide);
    let paged = framed(&wide_db, &shown);
    eprintln!("the frame: small {small:?}; large {large:?}; its first page stretched {paged:?}");
    println!("heap\trun-frame\t{}", small.brokkr.max(large.brokkr));
    let frame = median_micros(|| assert_eq!(readout(&small_db, false), shown));
    println!("wall\trun-frame\t{frame}");
    let whole = median_micros(|| drop(readout(&small_db, true)));
    eprintln!("the whole read, every page, median of 11: {whole} us");
    assert_eq!(
        large.brokkr, small.brokkr,
        "1,000,000 events against 10,000"
    );
    let (brokkr, sqlite) = (
        budgets::budget_named("run-frame"),
        budgets::budget_named("run-view-sqlite"),
    );
    let control = control_bytes(&run);
    assert!(
        brokkr <= 256 * 1024 + control,
        "the budget {brokkr} is past ruling 9's 256 KiB and the {control} bytes of the run's control envelopes"
    );
    for (name, frame) in [("10,000", small), ("1,000,000", large)] {
        assert!(
            frame.brokkr <= brokkr,
            "{name}: {} bytes; the budget is {brokkr}",
            frame.brokkr
        );
        assert!(
            frame.sqlite <= sqlite,
            "{name}: SQLite's {} bytes; the budget is {sqlite}",
            frame.sqlite
        );
    }
    let first = wide[..50].iter();
    let checkpoints = first.filter(|event| event.event_type == EventType::EffectCheckpointed);
    assert!(
        paged.brokkr <= brokkr + STRETCH as u64,
        "{} bytes with {} checkpoints of a mebibyte among the first 50 events, past the budget {brokkr} and the one checkpoint the opening verifies at a time: the frame holds a journal page it does not paint",
        paged.brokkr,
        checkpoints.count()
    );
}
