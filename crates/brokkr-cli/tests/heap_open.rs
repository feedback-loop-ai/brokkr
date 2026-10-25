//! Opening a run's heap budgets (decision 0089 ruling 9): a readout of one
//! run opens it first, its whole chain read and verified one envelope at a
//! time, so the opening holds the same live heap however many events the
//! journal holds beside the run, a small part of what the run holds, and
//! grows only by what one envelope costs to verify when one is wide. A
//! readout's live heap is two shares, Brokkr's under dhat and SQLite's from
//! its own high-water. The budgets live in `quality/heap-bytes.json`; dhat
//! allows one profiler per process, so this is a binary of its own.

#[path = "heap/budgets.rs"]
mod budgets;
#[path = "heap/grown.rs"]
mod grown;
#[path = "heap/measure.rs"]
mod measure;

use std::path::{Path, PathBuf};

use brokkr_core::envelope::EventEnvelope;
use brokkr_store::test_support::{self, Generated, Width};
use brokkr_store::Store;
use measure::{peak_of, sqlite_memory};

/// The clock the generated runs beside the opened one are stamped by.
const NOW: &str = "2026-10-20T00:00:00Z";

/// How long the stretched envelope's text is: a mebibyte.
const STRETCH: usize = 1 << 20;

/// What SQLite's high-water may grow past the stretched envelope itself:
/// its allocator and page cache around one large row, within ruling 9's
/// "the page cache, its statements and the largest envelope". CI's macOS
/// leg measured the envelope plus 13,296 B (1,061,872 B); Linux measures
/// within the envelope.
const SQLITE_SLACK: u64 = 64 * 1024;

/// The seq of the envelope stretched: the run's middle.
const STRETCHED: usize = grown::EVENTS / 2;

/// The generated runs that bring a journal holding the grown run to
/// `events` events: the fleet's 2,000, the remainder on the first.
fn beside(events: usize) -> Vec<Generated> {
    let others = events - grown::EVENTS;
    let mut runs = test_support::fleet(NOW, (others / 2_000) as u64, Width::Narrow);
    runs[0].events += (others % 2_000) as u64;
    runs
}

/// How the grown run's middle envelope is stretched, if it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stretch {
    /// As the grown run holds it.
    Unstretched,
    /// A run of `s`, which JSON writes as it stands, so the parser copies
    /// it once, into the string it keeps.
    Plain,
    /// A quote and a backslash, a run of `s`, and a newline, the three
    /// written as escapes, so the parser copies the text through its
    /// scratch buffer before the string it keeps. The shape costs the
    /// parser most: the run lands in the buffer whole and the newline
    /// after it grows the buffer past the text, where many short escapes
    /// grow it a little at a time.
    Escaped,
}

impl Stretch {
    /// Text that JSON writes in exactly `room` bytes between its quotes.
    fn text(self, room: usize) -> String {
        match self {
            Stretch::Unstretched => String::new(),
            Stretch::Plain => "s".repeat(room),
            // The quote and the backslash are written in four bytes, the
            // newline in two.
            Stretch::Escaped => format!("\"\\{}\n", "s".repeat(room - 6)),
        }
    }
}

/// The grown run, chained, its middle envelope's text stretched to
/// exactly [`STRETCH`] bytes unless it is unstretched.
fn opened_run(stretch: Stretch) -> Vec<EventEnvelope> {
    let stretched = stretch != Stretch::Unstretched;
    let mut events = grown::journal();
    if stretched {
        let event = &mut events[STRETCHED - 1];
        event.payload["stretch"] = "".into();
        let text = serde_json::to_string(event).unwrap().len();
        event.payload["stretch"] = stretch.text(STRETCH - text).into();
    }
    let events = grown::chained(events);
    let text = serde_json::to_string(&events[STRETCHED - 1]).unwrap().len();
    assert_eq!(text == STRETCH, stretched, "the stretched envelope's text");
    events
}

/// A journal under `dir` named `name` of `events` events: the grown run,
/// stretched as `stretch` says, and the generated runs beside it, written
/// in seconds.
fn journal(dir: &Path, name: &str, events: usize, stretch: Stretch) -> PathBuf {
    let db = dir.join(name);
    let started = std::time::Instant::now();
    let store = Store::open(&db).unwrap();
    test_support::generate(&store, &beside(events)).unwrap();
    drop(store);
    grown::planted(&db, &opened_run(stretch));
    eprintln!("{name}: {events} events in {:?}", started.elapsed());
    db
}

/// What opening the grown run cost, from its read-only open to its
/// verified head: Brokkr's peak heap under dhat, and SQLite's high-water
/// above where it stood before the open.
#[derive(Debug, Clone, Copy)]
struct Cost {
    brokkr: u64,
    sqlite: u64,
}

/// The grown run in `db` opened whole, and what that cost.
fn opened(db: &Path) -> Cost {
    let (base, _) = sqlite_memory(true);
    let mut head = None;
    let brokkr = peak_of(|| {
        let store = Store::open_read_only(db).unwrap();
        head = Some(store.open_run(grown::RUN).unwrap().seq);
    });
    let (_, high) = sqlite_memory(false);
    assert_eq!(head, Some(grown::EVENTS as u64), "the whole run is opened");
    Cost {
        brokkr,
        sqlite: u64::try_from(high - base).unwrap(),
    }
}

/// The median of eleven timed reads, in microseconds.
fn median_micros(mut read: impl FnMut()) -> u128 {
    let mut times: Vec<u128> = (0..11)
        .map(|_| {
            let started = std::time::Instant::now();
            read();
            started.elapsed().as_micros()
        })
        .collect();
    times.sort_unstable();
    times[times.len() / 2]
}

/// Ruling 9 on opening a run: Brokkr's peak to open the grown run of
/// 7,510 events is the same to the byte in a journal of 10,000 events and
/// one of 1,000,000, and within its budget, which holds no more than a
/// tenth of the run's envelopes: the opening holds one envelope at a time,
/// never the run. With one envelope stretched to a mebibyte of plain text,
/// Brokkr's peak grows by that envelope and no more, since the row is
/// parsed from the text SQLite holds and its hash streams from the parsed
/// envelope, and SQLite's, which holds the row it steps to, by that
/// envelope and [`SQLITE_SLACK`]; each stays within its budget and that
/// growth. Stretched
/// with escaped text instead, Brokkr's peak grows by at most three
/// envelopes, as the 2026-10-24 amendment to ruling 9 bounds it (see
/// [`the_escaped_stretch_holds`]). Ruling 9's cap for Brokkr's share is
/// 256 KiB.
#[test]
fn opening_a_run_holds_one_envelope_at_a_time() {
    let dir = tempfile::tempdir().unwrap();
    let small_db = journal(dir.path(), "small.db", 10_000, Stretch::Unstretched);
    let large_db = journal(dir.path(), "large.db", 1_000_000, Stretch::Unstretched);
    let wide_db = journal(dir.path(), "wide.db", 10_000, Stretch::Plain);
    let escaped_small_db = journal(dir.path(), "escaped-small.db", 10_000, Stretch::Escaped);
    let escaped_large_db = journal(dir.path(), "escaped-large.db", 1_000_000, Stretch::Escaped);
    // Read once unmeasured, so what a process allocates once and keeps
    // is allocated before any opening is measured.
    opened(&small_db);
    let (small, large, wide) = (opened(&small_db), opened(&large_db), opened(&wide_db));
    let escaped = [opened(&escaped_small_db), opened(&escaped_large_db)];
    eprintln!("small: {small:?}; large: {large:?}; wide: {wide:?}; escaped: {escaped:?}");
    let run_bytes: u64 = opened_run(Stretch::Unstretched)
        .iter()
        .map(|event| serde_json::to_string(event).unwrap().len() as u64)
        .sum();
    eprintln!("the run's envelopes: {run_bytes} bytes");
    // The records scripts/measure-budgets.sh reads, each a budget.
    println!("heap\topen-run\t{}", small.brokkr.max(large.brokkr));
    println!("heap\topen-run-sqlite\t{}", small.sqlite.max(large.sqlite));
    println!(
        "heap\topen-run-escaped\t{}",
        escaped[0].brokkr.max(escaped[1].brokkr)
    );
    let store = Store::open_read_only(&small_db).unwrap();
    let open = median_micros(|| drop(store.open_run(grown::RUN).unwrap()));
    let load = median_micros(|| drop(store.load(grown::RUN).unwrap()));
    println!("wall\topen-run-7510\t{open}");
    println!("wall\tload-run-7510\t{load}");
    assert_eq!(
        large.brokkr, small.brokkr,
        "1,000,000 events against 10,000"
    );
    let (brokkr, sqlite) = (
        budgets::budget_named("open-run"),
        budgets::budget_named("open-run-sqlite"),
    );
    assert!(
        brokkr <= 256 * 1024,
        "the budget {brokkr} is past ruling 9's 256 KiB"
    );
    assert!(
        brokkr <= run_bytes / 10,
        "the budget {brokkr} is past a tenth of the run's {run_bytes} bytes"
    );
    for (name, cost) in [("10,000", small), ("1,000,000", large)] {
        assert!(
            cost.brokkr <= brokkr,
            "{name}: {} bytes; the budget is {brokkr}",
            cost.brokkr
        );
        assert!(
            cost.sqlite <= sqlite,
            "{name}: SQLite's {} bytes; the budget is {sqlite}",
            cost.sqlite
        );
    }
    let stretch = STRETCH as u64;
    let growth = wide.brokkr - small.brokkr;
    assert!(
        growth <= stretch,
        "a mebibyte envelope grew the opening's peak by {growth} bytes"
    );
    assert!(
        wide.brokkr <= brokkr + stretch,
        "{} bytes with one envelope a mebibyte; the budget is {brokkr} and the envelope",
        wide.brokkr
    );
    let held = wide.sqlite - small.sqlite;
    assert!(
        held <= stretch + SQLITE_SLACK,
        "a mebibyte envelope grew SQLite's high-water by {held} bytes"
    );
    assert!(
        wide.sqlite <= sqlite + stretch + SQLITE_SLACK,
        "SQLite's {} bytes with one envelope a mebibyte; the budget is {sqlite}, the envelope and the slack",
        wide.sqlite
    );
    the_escaped_stretch_holds(small, wide, escaped);
}

/// The 2026-10-24 amendment to ruling 9: an envelope whose text carries
/// escapes is copied through the parser's scratch buffer before the string
/// it keeps, so with one envelope stretched to a mebibyte of escaped text
/// Brokkr's peak grows over the unstretched opening's by at most three
/// envelopes, the same to the byte in a journal of 10,000 events and one
/// of 1,000,000, within its budget, and past the plain stretch's by more
/// than the envelope, so the scratch copy is the one measured: the journals'
/// paths, which the store holds, differ by a few bytes too.
fn the_escaped_stretch_holds(unstretched: Cost, plain: Cost, escaped: [Cost; 2]) {
    let [small, large] = escaped;
    assert_eq!(
        large.brokkr, small.brokkr,
        "escaped, 1,000,000 events against 10,000"
    );
    let budget = budgets::budget_named("open-run-escaped");
    let stretch = STRETCH as u64;
    let envelopes = 3 * stretch;
    for (name, cost) in [("10,000", small), ("1,000,000", large)] {
        let growth = cost.brokkr - unstretched.brokkr;
        assert!(
            growth <= envelopes,
            "{name}: an escaped mebibyte envelope grew the opening's peak by {growth} bytes"
        );
        assert!(
            cost.brokkr <= budget,
            "{name}: {} bytes with one escaped envelope a mebibyte; the budget is {budget}",
            cost.brokkr
        );
    }
    assert!(
        small.brokkr > plain.brokkr + stretch,
        "the escaped stretch's {} bytes are not an envelope past the plain stretch's {}",
        small.brokkr,
        plain.brokkr
    );
}
