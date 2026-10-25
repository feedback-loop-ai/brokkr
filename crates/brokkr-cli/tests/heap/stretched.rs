//! The journals the readouts of one run are measured in (decision 0089
//! ruling 9): the grown run, one of its envelopes stretched to a mebibyte
//! or not, beside generated runs that bring the journal to a size, and the
//! median of timed reads. Shared by the heap binaries of opening a run and
//! of its run view.

use std::path::{Path, PathBuf};

use brokkr_core::envelope::EventEnvelope;
use brokkr_store::test_support::{self, Generated, Width};
use brokkr_store::Store;

/// The clock the generated runs beside the measured one are stamped by.
const NOW: &str = "2026-10-20T00:00:00Z";

/// How long a stretched envelope's text is: a mebibyte.
pub(crate) const STRETCH: usize = 1 << 20;

/// The generated runs that bring a journal holding the grown run to
/// `events` events: the fleet's 2,000, the remainder on the first.
fn beside(events: usize) -> Vec<Generated> {
    let others = events - super::grown::EVENTS;
    let mut runs = test_support::fleet(NOW, (others / 2_000) as u64, Width::Narrow);
    runs[0].events += (others % 2_000) as u64;
    runs
}

/// How an envelope is stretched, if it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stretch {
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

/// `events`, a run grown from the grown run, chained, its envelope at `seq`
/// stretched by its payload's `field` to exactly [`STRETCH`] bytes of text
/// unless it is unstretched.
pub(crate) fn run_stretched(
    mut events: Vec<EventEnvelope>,
    seq: usize,
    field: &str,
    stretch: Stretch,
) -> Vec<EventEnvelope> {
    let stretched = stretch != Stretch::Unstretched;
    if stretched {
        let event = &mut events[seq - 1];
        event.payload[field] = "".into();
        let text = serde_json::to_string(event).unwrap().len();
        event.payload[field] = stretch.text(STRETCH - text).into();
    }
    let events = super::grown::chained(events);
    let text = serde_json::to_string(&events[seq - 1]).unwrap().len();
    assert_eq!(text == STRETCH, stretched, "the stretched envelope's text");
    events
}

/// A journal under `dir` named `name` of `events` events: `run`, and the
/// generated runs beside it, written in seconds.
pub(crate) fn journal(dir: &Path, name: &str, events: usize, run: &[EventEnvelope]) -> PathBuf {
    let db = dir.join(name);
    let started = std::time::Instant::now();
    let store = Store::open(&db).unwrap();
    test_support::generate(&store, &beside(events)).unwrap();
    drop(store);
    super::grown::planted(&db, run);
    eprintln!("{name}: {events} events in {:?}", started.elapsed());
    db
}

/// The median of eleven timed reads, in microseconds.
pub(crate) fn median_micros(mut read: impl FnMut()) -> u128 {
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
