//! The stdout queue's byte bound (#433): a driver that writes past it
//! while the engine reads nothing is held at it, every line still
//! arrives in order once the engine reads, and a reader held at it stops
//! once the engine stops listening.

use std::collections::VecDeque;
use std::io::Read;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::limits::Limits;
use super::{queue, read_stdout, Stdout};

/// The bytes of each of `lines`' lines before its newline.
const LINE: usize = 7;

/// A driver's stdout that plays one line per read, counting its reads.
struct Played {
    lines: VecDeque<String>,
    reads: Arc<AtomicUsize>,
}

impl Read for Played {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let Some(line) = self.lines.pop_front() else {
            return Ok(0);
        };
        self.reads.fetch_add(1, Ordering::SeqCst);
        buf[..line.len()].copy_from_slice(line.as_bytes());
        Ok(line.len())
    }
}

/// `count` lines of [`LINE`] bytes and a newline each.
fn lines(count: usize) -> Vec<String> {
    (0..count).map(|n| format!("line-{n:02}\n")).collect()
}

/// A bound of five lines.
fn five_lines() -> Limits {
    Limits {
        stdout_bytes: 5 * LINE,
        ..Limits::DEFAULT
    }
}

/// `lines` played to a reader under `limits`: what it hands over, and a
/// count of its reads the reader holds as long as it runs.
fn played(lines: &[String], limits: Limits) -> (queue::Receiver<Stdout>, Arc<AtomicUsize>) {
    let reads = Arc::new(AtomicUsize::new(0));
    let stdout = Played {
        lines: lines.iter().cloned().collect(),
        reads: Arc::clone(&reads),
    };
    (read_stdout(stdout, limits).0, reads)
}

/// The reads once a fifth of a second passes without one, or after five
/// seconds of them.
fn held(reads: &AtomicUsize) -> usize {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let before = reads.load(Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(200));
        let after = reads.load(Ordering::SeqCst);
        if after == before || Instant::now() > until {
            return after;
        }
    }
}

/// The reader queues lines up to the byte bound, each weighed before its
/// newline, and has read one more, which waits: far below the line
/// bound. Once the engine reads, every line arrives in order, the last
/// one heavier than the bound included, since it waits only until
/// nothing is queued.
#[test]
fn a_driver_past_the_byte_bound_is_held_at_it_and_every_line_arrives_in_order() {
    let limits = five_lines();
    let mut played_lines = lines(20);
    played_lines.push(format!("{}\n", "x".repeat(limits.stdout_bytes + 1)));
    let (stdout, reads) = played(&played_lines, limits);

    assert_eq!(held(&reads), limits.stdout_bytes / LINE + 1);

    let mut arrived = Vec::new();
    while let Ok(Stdout::Line(line)) = stdout.recv_timeout(Duration::from_secs(5)) {
        arrived.push(line);
    }
    assert_eq!(arrived, played_lines);
}

/// A reader waiting on the byte bound when the engine stops listening
/// reads nothing more and lets the driver's stdout go, which closes the
/// pipe on the writer.
#[test]
fn a_reader_held_at_the_byte_bound_stops_once_the_engine_stops_listening() {
    let limits = five_lines();
    let (stdout, reads) = played(&lines(20), limits);
    assert_eq!(held(&reads), limits.stdout_bytes / LINE + 1);

    drop(stdout);
    let until = Instant::now() + Duration::from_secs(5);
    while Arc::strong_count(&reads) > 1 && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        Arc::strong_count(&reads),
        1,
        "the reader still holds stdout"
    );
    assert_eq!(reads.load(Ordering::SeqCst), limits.stdout_bytes / LINE + 1);
}
