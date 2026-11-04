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
#[path = "../../../tests/support/envelope.rs"]
mod envelope_builder;
#[path = "heap/grown.rs"]
mod grown;
#[path = "heap/measure.rs"]
mod measure;
#[path = "../../../tests/support/seat_steps.rs"]
mod seat_steps;
#[path = "heap/stretched.rs"]
mod stretched;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
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

/// A peak is the measuring thread's alone: `peak_of` measures the read and
/// no other thread's allocations, so libtest's own warning, allocated on
/// its thread at the 60-second mark (622 B on hosted CI, #965), cannot move
/// it. Another thread's 64 KiB is held through the window and moves it by
/// nothing; an allocator that counted every thread would show all of it.
/// The threads hand off through an atomic stage, never a `Barrier`: on
/// macOS std's `Mutex` boxes its pthread mutex on first use, so a barrier
/// first waited on inside the window is the measuring thread's own 64 B.
#[test]
fn a_peak_is_the_measuring_thread_s_alone() {
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
    let stage = AtomicU8::new(0);
    let reach = |want: u8| {
        while stage.load(Ordering::Acquire) != want {
            std::thread::yield_now();
        }
    };
    let peak = std::thread::scope(|scope| {
        scope.spawn(|| {
            reach(1);
            let held = vec![0u8; 64 * 1024];
            stage.store(2, Ordering::Release);
            reach(3);
            drop(held);
        });
        let peak = peak_of(|| {
            stage.store(1, Ordering::Release);
            reach(2);
        });
        stage.store(3, Ordering::Release);
        peak
    });
    assert_eq!(peak, 0, "another thread's 64 KiB was the read's peak");
}

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

/// What `inspect --json` printed of the grown run in the journal at `db`
/// before the pages: the run loaded, folded and viewed whole.
fn read_whole(db: &Path) -> String {
    let events = Store::open_read_only(db).unwrap().load(grown::RUN).unwrap();
    let state = brokkr_core::fold::fold(&events).unwrap();
    let view = brokkr_view::run_view(&events, Some(&state)).with_scopes();
    serde_json::to_string_pretty(&view).unwrap()
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
    framed_by(db, shown, |db| readout(db, false))
}

/// What `paint` cost painting `db`'s run, from its read-only open to its
/// painted text, holding that it paints `shown`.
fn framed_by(db: &Path, shown: &str, paint: impl Fn(&Path) -> String) -> Screen {
    let (base, _) = sqlite_memory(true);
    let mut painted = None;
    let brokkr = peak_of(|| painted = Some(paint(db)));
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
/// 50 ms, and the whole read `inspect --json` makes, the events its
/// opening kept composed once, beside it and today's read of the run
/// whole, whose bytes it prints; none is gated.
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
    assert_eq!(readout(&small_db, true), read_whole(&small_db));
    let whole = median_micros(|| drop(readout(&small_db, true)));
    let today = median_micros(|| drop(read_whole(&small_db)));
    eprintln!("the whole read, its opening kept, median of 11: {whole} us; read whole, as before the pages: {today} us");
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

/// The console's clock for its run level: a frame paints the run's age at
/// it, the same for both reads.
const CONSOLE_NOW: &str = "2026-10-26T00:00:00Z";

/// What the console's run level paints of the grown run in the journal at
/// `db`, first painted after the run is selected, read as the console
/// reads it, or with `whole` read whole as before the pages (decision 0089
/// slice 3).
fn console_level(db: &Path, whole: bool) -> String {
    brokkr_cli::console_run_level_for_budget(db, grown::RUN, CONSOLE_NOW, whole).unwrap()
}

/// What the console reads of the grown run in the journal at `db` for its
/// run level, as text: its event count, its journal's rows and each
/// participant's checkpoints, which no seat at work gives a page of.
fn console_read(db: &Path) -> String {
    let view = brokkr_cli::console_run_view_for_budget(db, grown::RUN).unwrap();
    let checkpoints = view.participants.iter().map(|part| part.checkpoints.len());
    let checkpoints: Vec<usize> = checkpoints.collect();
    format!(
        "{} {} {checkpoints:?}",
        view.event_count,
        view.journal.len()
    )
}

/// Ruling 9 on the console's run level (decision 0089 slice 3): selecting
/// the grown run, the console opens it and reads one read's trail and seat
/// rows, no seat at work, so no checkpoint page, and composes its view:
/// from its read-only open, Brokkr's peak is the same to the byte in a
/// journal of 10,000 events and one of 1,000,000, within ruling 9's 256
/// KiB and the run's control envelopes, and SQLite's within the run view's
/// cap. Painted at 160x48, the frame is today's, the run read whole; its
/// peak, ratatui's buffers on top, is reported, since two paints of one
/// view lay a table out by ratatui's solver, which moves its peak by a few
/// hundred bytes between them. The first paint's wall time, read and
/// painted, is reported against ruling 9's 50 ms and today's, read whole;
/// none is gated.
#[test]
fn the_console_s_run_level_holds_its_trail_and_no_checkpoint() {
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
    let dir = tempfile::tempdir().unwrap();
    let (run, [small_db, large_db]) = grown_journals(dir.path());
    let read = format!("{} 17 [0, 0, 0]", grown::EVENTS);
    assert_eq!(console_read(&small_db), read);
    let (small, large) = (
        framed_by(&small_db, &read, console_read),
        framed_by(&large_db, &read, console_read),
    );
    let shown = console_level(&small_db, false);
    assert_eq!(shown, console_level(&small_db, true), "today's frame");
    let level = |db: &Path| console_level(db, false);
    let painted = [&small_db, &large_db].map(|db| framed_by(db, &shown, level));
    eprintln!(
        "the console's run level read: small {small:?}; large {large:?}; painted: {painted:?}"
    );
    let paged = median_micros(|| assert_eq!(level(&small_db), shown));
    let whole = median_micros(|| drop(console_level(&small_db, true)));
    let alone = median_micros(|| assert_eq!(console_read(&small_db), read));
    eprintln!("the console's run level, read and painted, median of 11: {paged} us through its pages ({alone} us of it the read), {whole} us read whole as before; ruling 9's first screen is 50,000 us");
    println!("wall\tconsole-run-level\t{paged}");
    assert_eq!(
        large.brokkr, small.brokkr,
        "1,000,000 events against 10,000"
    );
    let control = control_bytes(&run);
    let sqlite = budgets::budget_named("run-view-sqlite");
    for (name, level) in [("10,000", small), ("1,000,000", large)] {
        assert!(
            level.brokkr <= 256 * 1024 + control,
            "{name}: {} bytes, past ruling 9's 256 KiB and the {control} bytes of the run's control envelopes",
            level.brokkr
        );
        assert!(
            level.sqlite <= sqlite,
            "{name}: SQLite's {} bytes; the budget is {sqlite}",
            level.sqlite
        );
    }
}

/// The grown run at work: cut where its second seat, the longest-running,
/// succeeds, so that seat is at work with every checkpoint the grown run
/// gives it, as a run the console selects mid-seat stands.
fn at_work() -> Vec<EventEnvelope> {
    let mut events = grown::journal();
    let mut succeeded = events
        .iter()
        .enumerate()
        .filter(|(_, event)| event.event_type == EventType::EffectSucceeded);
    let (cut, _) = succeeded
        .nth(1)
        .expect("the grown run's second seat succeeds");
    events.truncate(cut);
    grown::chained(events)
}

/// The checkpoints of `run`'s seat at work: every one after its last start.
fn stream_of(run: &[EventEnvelope]) -> &[EventEnvelope] {
    let started = run
        .iter()
        .rposition(|event| event.event_type == EventType::EffectStarted);
    &run[started.expect("the seat at work started") + 1..]
}

/// What the console's run level holds of a run at work (decision 0089
/// slice 3, as the operator ruled on 2026-10-26): selecting the grown run
/// cut mid-seat, its longest-running seat at work with 4,675 checkpoints,
/// the console opens it and reads one read's trail and seat rows, and no
/// checkpoint of that seat at all, and composes its view. From its
/// read-only open, Brokkr's peak is the same to the byte beside the
/// generated runs of a journal of 10,000 events and of one of 1,000,000,
/// within ruling 9's 256 KiB and the run's control envelopes, and never
/// with the seat's stream; SQLite's within the run view's cap. Its peak is
/// reported beside today's, the run read whole. Painted at 160x48 at the
/// seat's last checkpoint, the frame is today's; its wall time, read and
/// painted, is reported against ruling 9's 50 ms and today's; none is
/// gated.
#[test]
fn the_console_s_run_level_at_work_holds_its_seat_s_stream() {
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
    let dir = tempfile::tempdir().unwrap();
    let run = at_work();
    let stream = stream_of(&run);
    let [small_db, large_db] = [("small.db", 10_000), ("large.db", 1_000_000)]
        .map(|(name, events)| journal(dir.path(), name, events, &run));
    let trail = run
        .iter()
        .filter(|event| event.event_type != EventType::EffectCheckpointed);
    let read = format!("{} {} [0, 0]", run.len(), trail.count());
    assert_eq!(console_read(&small_db), read);
    let (small, large) = (
        framed_by(&small_db, &read, console_read),
        framed_by(&large_db, &read, console_read),
    );
    eprintln!("the console's run level at work: small {small:?}; large {large:?}");
    let now = &run[run.len() - 1].recorded_at;
    let level = |db: &Path, whole| {
        brokkr_cli::console_run_level_for_budget(db, grown::RUN, now, whole).unwrap()
    };
    let shown = level(&small_db, false);
    assert_eq!(shown, level(&small_db, true), "today's frame");
    let paged = median_micros(|| assert_eq!(level(&small_db, false), shown));
    let whole = median_micros(|| drop(level(&small_db, true)));
    eprintln!("the console's run level at work, read and painted, median of 11: {paged} us through the pages, {whole} us read whole as before; ruling 9's first screen is 50,000 us");
    println!("wall\tconsole-run-level-at-work\t{paged}");
    assert_eq!(
        large.brokkr, small.brokkr,
        "1,000,000 events against 10,000"
    );
    let control = control_bytes(&run);
    let sqlite = budgets::budget_named("run-view-sqlite");
    let (envelopes, rows) = (envelope_bytes(stream, |_| true), stream.len() as u64);
    let today = peak_of(|| drop(read_whole(&small_db)));
    eprintln!(
        "the seat's {rows} checkpoints: {envelopes} bytes of envelopes; the peak {} bytes, {} a checkpoint; read whole as before, {today} bytes",
        small.brokkr,
        small.brokkr / rows
    );
    for (name, level) in [("10,000", small), ("1,000,000", large)] {
        assert!(
            level.brokkr <= 256 * 1024 + control,
            "{name}: {} bytes, past ruling 9's 256 KiB and the {control} bytes of the run's control envelopes: the run level holds its seat's stream",
            level.brokkr
        );
        assert!(
            level.sqlite <= sqlite,
            "{name}: SQLite's {} bytes; the budget is {sqlite}",
            level.sqlite
        );
    }
}

/// A run at work with `checkpoints` checkpoints of its one seat, chained:
/// the journal the detail pane's merge is measured over.
fn working(checkpoints: u64) -> Vec<EventEnvelope> {
    let mut steps = vec![
        (
            EventType::RunStarted,
            json!({"feature": "the detail pane's run"}),
        ),
        (EventType::PhaseEntered, json!({"phase": "work"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "detail-eff", "seat": "detail", "phase": "measure"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "detail-eff", "attempt_id": "detail-att"}),
        ),
    ];
    for turn in 1..=checkpoints {
        steps.push(seat_steps::checkpoint(
            "detail-eff",
            "detail-att",
            json!({"step": "seat-turn", "turn": turn}),
        ));
    }
    grown::chained(seat_steps::journal(steps))
}

/// The detail pane's merge holds a window, not the depth it read (F4,
/// decision 0089 ruling 9): a fresh ask 200 rows deep and one 10,000 deep
/// over 10,000 checkpoints, and one 100,000 deep over 100,000, hold the
/// same Brokkr peak however far the scroll has gone; the rows above the
/// window are counted and dropped as they arrive, never held as a prefix.
#[test]
fn the_detail_pane_holds_a_window_however_deep_the_scroll() {
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
    let dir = tempfile::tempdir().unwrap();
    let small_events = working(10_000);
    let large_events = working(100_000);
    let small = stretched::journal(dir.path(), "detail-small.db", 10_000, &small_events);
    let large = stretched::journal(dir.path(), "detail-large.db", 1_000_000, &large_events);
    // A writable open catches the projections up, so the console reads the
    // pages the journal holds rather than falling back to the run whole.
    drop(Store::open(&small).unwrap());
    drop(Store::open(&large).unwrap());
    // Painted once unmeasured, so what a process allocates once and keeps
    // is allocated before any peak is measured.
    let warm = brokkr_cli::console_detail_view_for_budget(&small, "run", 200).unwrap();
    let warm_rows: usize = warm
        .participants
        .iter()
        .map(|part| part.checkpoints.len())
        .sum();
    eprintln!("the detail warm read holds {warm_rows} checkpoints");
    drop(warm);
    let shallow = peak_of(|| {
        drop(brokkr_cli::console_detail_view_for_budget(&small, "run", 200).unwrap());
    });
    let deep = peak_of(|| {
        drop(brokkr_cli::console_detail_view_for_budget(&small, "run", 10_000).unwrap());
    });
    let deepest = peak_of(|| {
        drop(brokkr_cli::console_detail_view_for_budget(&large, "run", 100_000).unwrap());
    });
    eprintln!("the detail pane: 200 rows {shallow} B; 10,000 {deep} B; 100,000 {deepest} B");
    println!("heap\tdetail-window\t{}", shallow.max(deep).max(deepest));
    assert!(
        deep <= shallow + 256 * 1024,
        "10,000 rows held {} bytes against {} at 200: the merge kept the depth",
        deep,
        shallow
    );
    assert!(
        deepest <= shallow + 256 * 1024,
        "100,000 rows held {} bytes against {} at 200: the merge kept the depth",
        deepest,
        shallow
    );
}
