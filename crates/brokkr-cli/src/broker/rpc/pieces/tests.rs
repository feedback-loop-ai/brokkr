//! Where what the broker writes is cut, and how each piece is written
//! (decision 0065 slice two, U6f1b; MB4; operator ruling 2026-10-13): only
//! at safe points, so every prefix a reader can hold of a frame the guard
//! passed passes it too; never where some 512 bytes hold none; and each
//! piece offered whole, its attempt read before and after the poll.

use std::io::{ErrorKind, PipeReader, PipeWriter, Read, Write};
use std::os::fd::{AsFd, BorrowedFd};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use brokkr_protocol::secret::{self, BoundSecret, Guard};
use rustix::event::PollFlags;
use rustix::io::Errno;

use super::{cut, put, readiness, safe, written, Pieces, Put};
use crate::broker::rpc::{offered, ATOMIC};
use crate::broker::session::Startup;

/// `value` bound as `name` from an owner-only store on a canonical
/// temporary root.
pub(in crate::broker) fn bound(name: &str, value: &str) -> Vec<BoundSecret> {
    let root = tempfile::tempdir().unwrap();
    let store = root.path().canonicalize().unwrap().join("store.env");
    secret::store_set(&store, name, value).unwrap();
    secret::resolve_bindings(&store, &[name.to_string()]).unwrap()
}

/// A sink over a pipe that refuses its first `unready` writes as not ready,
/// as a holder's status flags would, and, where `stutter` is not zero, the
/// one write after every `stutter` it took, as a harness that takes a
/// little at a time leaves its pipe; takes at most `most` bytes of each
/// write, after `slow`, and refuses every write past its first `writes`
/// with `refused`, the error the system would; when each write it took
/// returned.
pub(in crate::broker) struct Sink {
    pipe: PipeWriter,
    pub(in crate::broker) unready: usize,
    pub(in crate::broker) stutter: usize,
    most: usize,
    writes: usize,
    slow: Duration,
    refused: Errno,
    pub(in crate::broker) at: Arc<Mutex<Vec<Instant>>>,
}

impl Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.unready > 0 {
            self.unready -= 1;
            return Err(Errno::AGAIN.into());
        }
        self.writes = self.writes.checked_sub(1).ok_or(self.refused)?;
        std::thread::sleep(self.slow);
        let taken = self.pipe.write(&bytes[..bytes.len().min(self.most)]);
        let mut at = self.at.lock().unwrap();
        at.push(Instant::now());
        if self.stutter > 0 && at.len().is_multiple_of(self.stutter) {
            self.unready = 1;
        }
        taken
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl AsFd for Sink {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.pipe.as_fd()
    }
}

/// A [`Sink`] taking `most` bytes of each of `writes` writes after `slow`,
/// refusing the rest as a broken pipe, and the pipe's reader.
pub(in crate::broker) fn sink(most: usize, writes: usize, slow: Duration) -> (Sink, PipeReader) {
    let (reader, pipe) = std::io::pipe().unwrap();
    let sink = Sink {
        pipe,
        unready: 0,
        stutter: 0,
        most,
        writes,
        slow,
        refused: Errno::PIPE,
        at: Arc::default(),
    };
    (sink, reader)
}

/// A [`Sink`] refusing its every write with `refused`, and the pipe's
/// reader.
pub(in crate::broker) fn refusing(refused: Errno) -> (Sink, PipeReader) {
    let (mut sink, reader) = sink(ATOMIC, 0, Duration::ZERO);
    sink.refused = refused;
    (sink, reader)
}

/// Everything `reader` holds once every writer of its pipe is gone.
pub(in crate::broker) fn held(mut reader: PipeReader) -> Vec<u8> {
    let mut held = Vec::new();
    reader.read_to_end(&mut held).unwrap();
    held
}

/// An attempt that is never cancelled, with `left` to its deadline.
pub(in crate::broker) fn within(left: Duration) -> Startup<'static> {
    (Instant::now() + left, &|| false)
}

#[test]
fn a_frame_is_cut_only_before_an_inert_byte_no_escape_spans() {
    // Before x, never before a digit, a point, a sign, an exponent letter
    // or a backslash, nor inside 1 or \n, nor inside a character.
    let frame = concat!("a1.e+-E,", "\\", "u0031x\\nyé");
    let points: Vec<usize> = safe(frame.as_bytes()).collect();
    assert_eq!(points, vec![7, 14, 17, 18]);
    // A \u escape is six bytes only where four hex digits follow; else
    // the backslash and the byte after it, or a last backslash alone.
    assert_eq!(safe(br"a\u12x").collect::<Vec<_>>(), vec![5]);
    assert_eq!(safe(br"a\").collect::<Vec<_>>(), Vec::<usize>::new());
    // Pieces as long as can be, each at most the bound, the last at the
    // end, never between two digits or before one; none where the bound's
    // bytes in a row hold no safe point.
    assert_eq!(cut(b"abcdgh", 4), Some(vec![4, 6]));
    assert_eq!(cut(b"abcdef", 4), Some(vec![3, 6]));
    assert_eq!(cut(b"xy12z", 3), Some(vec![1, 4, 5]));
    assert_eq!(cut(b"a123b", 4), Some(vec![4, 5]));
    assert_eq!(cut(b"ab1234c", 4), None);
    assert_eq!(cut(b"", 4), Some(vec![]));
}

#[test]
fn a_frame_holding_512_bytes_with_no_safe_point_cannot_be_cut() {
    let digits = [b"x".as_slice(), &[b'1'; ATOMIC]].concat();
    assert_eq!(Pieces::new(digits.clone()), None);
    let fits = [&digits[..ATOMIC], b"x"].concat();
    let pieces = Pieces::new(fits.clone()).unwrap();
    assert_eq!(
        (pieces.after(0), pieces.after(ATOMIC), pieces.bytes()),
        (&fits[..ATOMIC], &b"x"[..], &fits[..])
    );
}

/// The next of a fixed run of pseudo-random numbers.
fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// What generated frames are made of: digits, numbers, points, signs and
/// exponent letters, every escape the guard decodes, a \u escape of a
/// digit, a point, a sign and an exponent letter, and inert bytes.
const TOKENS: [&str; 34] = [
    "x",
    "1",
    "2",
    "3",
    "0",
    "12",
    "123",
    "0012",
    ".",
    "+",
    "-",
    "e",
    "E",
    ",",
    " ",
    "\"",
    ":",
    concat!("\\", "u0031"),
    concat!("\\", "u0032"),
    concat!("\\", "u0033"),
    concat!("\\", "u002b"),
    concat!("\\", "u002d"),
    concat!("\\", "u0065"),
    concat!("\\", "u002e"),
    r"\n",
    r"\t",
    r"\\",
    r"\1",
    r"\e",
    concat!("\\", "u00"),
    "é",
    "{",
    "}",
    "[",
];

#[test]
fn every_safe_prefix_of_a_frame_the_guard_passes_passes_it_too() {
    // Generated frames, read as JSON and as text: wherever one the guard
    // passes is cut safely, what a reader holds passes it too, though some
    // unsafe cut of one it passes is refused, so the property is not empty.
    let mut bindings = bound("PIN", "0012");
    bindings.extend(bound("ROUND", "1000"));
    let guard = Guard::new(&bindings);
    let (mut state, mut passed, mut unsafe_refused) = (0x2545_f491_4f6c_dd1d_u64, 0, 0);
    for _ in 0..4000 {
        let length = 1 + next(&mut state) % 24;
        let frame: String = (0..length)
            .map(|_| TOKENS[(next(&mut state) % TOKENS.len() as u64) as usize])
            .collect();
        let frame = frame.as_bytes();
        let points: Vec<usize> = safe(frame).collect();
        for json in [true, false] {
            let passes = |bytes: &[u8]| match json {
                true => guard.json(bytes),
                false => guard.text(bytes),
            };
            if !passes(frame) {
                continue;
            }
            passed += 1;
            for point in &points {
                let prefix = String::from_utf8_lossy(&frame[..*point]);
                assert!(passes(&frame[..*point]), "json {json}: {prefix:?}");
            }
            let refused = (1..frame.len()).any(|at| !passes(&frame[..at]));
            unsafe_refused += usize::from(refused);
        }
    }
    assert!(
        (passed > 1000) & (unsafe_refused > 100),
        "{passed} {unsafe_refused}"
    );
}

#[test]
fn a_piece_is_offered_whole_once_ready_and_never_once_its_attempt_has_ended() {
    // Taken whole by a pipe that has room.
    let (mut taking, reader) = sink(ATOMIC, 1, Duration::ZERO);
    let taken = put(
        &mut taking,
        b"one\n",
        within(Duration::from_secs(5)),
        Duration::ZERO,
    );
    drop(taking);
    assert_eq!((taken, held(reader)), (Put::Taken, b"one\n".to_vec()));
    // Part of it taken: taken short, a delivery failure, not an end.
    let (mut short, _reader) = sink(2, 1, Duration::ZERO);
    let taken = put(
        &mut short,
        b"one\n",
        within(Duration::from_secs(5)),
        Duration::ZERO,
    );
    assert_eq!(taken, Put::Short);
    // A write refused before it took any: by a signal or by status flags
    // someone else set, not taken; by a broken pipe, its reader gone,
    // closed; by a full device or an I/O error, failed, a delivery failure
    // (operator ruling 2026-10-14).
    for (refused, put_as) in [
        (Errno::INTR, Put::Unready),
        (Errno::AGAIN, Put::Unready),
        (Errno::PIPE, Put::Closed),
        (Errno::NOSPC, Put::Failed),
        (Errno::IO, Put::Failed),
    ] {
        let (mut refusing, _reader) = refusing(refused);
        let taken = put(
            &mut refusing,
            b"one\n",
            within(Duration::from_secs(5)),
            Duration::ZERO,
        );
        assert_eq!((refused, taken), (refused, put_as));
    }
    // A write that took nothing: failed, never closed.
    let (mut nothing, _reader) = sink(0, 1, Duration::ZERO);
    let taken = put(
        &mut nothing,
        b"one\n",
        within(Duration::from_secs(5)),
        Duration::ZERO,
    );
    assert_eq!(taken, Put::Failed);
    // The attempt past its deadline, or cancelled, before the poll: not
    // offered, and not waited for.
    let (mut late, reader) = sink(ATOMIC, 1, Duration::ZERO);
    assert_eq!(
        put(&mut late, b"one\n", within(Duration::ZERO), Duration::MAX),
        Put::Late
    );
    let cancelled: Startup<'_> = (Instant::now() + Duration::from_secs(5), &|| true);
    let begun = Instant::now();
    assert_eq!(
        put(&mut late, b"one\n", cancelled, Duration::MAX),
        Put::Late
    );
    assert!(begun.elapsed() < Duration::from_millis(500));
    drop(late);
    assert_eq!(held(reader), Vec::<u8>::new());
}

#[test]
fn a_cancellation_the_poll_outlived_writes_nothing() {
    // The sink is ready, and the attempt is cancelled while it is polled:
    // read again after the poll, the cancellation writes nothing.
    let reads = Arc::new(Mutex::new(0));
    let counted = Arc::clone(&reads);
    let cancelled = move || {
        let mut reads = counted.lock().unwrap();
        *reads += 1;
        *reads > 1
    };
    let (mut ready, reader) = sink(ATOMIC, 1, Duration::ZERO);
    let attempt: Startup<'_> = (Instant::now() + Duration::from_secs(5), &cancelled);
    let taken = put(&mut ready, b"one\n", attempt, Duration::ZERO);
    drop(ready);
    assert_eq!(
        (taken, *reads.lock().unwrap(), held(reader)),
        (Put::Late, 2, vec![])
    );
}

#[test]
fn only_a_broken_pipe_the_write_meets_is_a_closed_reader() {
    // What poll found is decided apart from the sink: a poll error is never
    // closure, a broken pipe among them, and no write is made after it;
    // only the write's own broken pipe is; every other error, and a write
    // that took nothing, is a delivery failure (operator ruling
    // 2026-10-14). Retried, as before: a signal and status flags.
    let wrote = &std::cell::Cell::new(0);
    let write = |result: std::io::Result<usize>| {
        move || {
            wrote.set(wrote.get() + 1);
            result
        }
    };
    let error = |errno: Errno| Err::<usize, _>(std::io::Error::from(errno));
    for (polled, put_as) in [
        (Errno::PIPE, Put::Failed),
        (Errno::NOMEM, Put::Failed),
        (Errno::INVAL, Put::Failed),
        (Errno::INTR, Put::Unready),
        (Errno::AGAIN, Put::Unready),
    ] {
        let decided = written(error(polled).map(|_| true), write(Ok(4)), 4);
        assert_eq!((polled, decided, wrote.get()), (polled, put_as, 0));
    }
    let unready = written(Ok(false), write(Ok(4)), 4);
    assert_eq!((unready, wrote.get()), (Put::Unready, 0));
    for (errno, result, put_as) in [
        (None, Ok(4), Put::Taken),
        (None, Ok(2), Put::Short),
        (None, Ok(0), Put::Failed),
        (Some(Errno::PIPE), error(Errno::PIPE), Put::Closed),
        (Some(Errno::NOSPC), error(Errno::NOSPC), Put::Failed),
        (Some(Errno::IO), error(Errno::IO), Put::Failed),
        (Some(Errno::BADF), error(Errno::BADF), Put::Failed),
        (Some(Errno::INTR), error(Errno::INTR), Put::Unready),
        (Some(Errno::AGAIN), error(Errno::AGAIN), Put::Unready),
    ] {
        let taken = result.as_ref().ok().copied();
        let decided = written(Ok(true), write(result), 4);
        assert_eq!((errno, taken, decided), (errno, taken, put_as));
    }
    assert_eq!(wrote.get(), 9);
}

#[test]
fn a_poll_a_signal_interrupts_is_unready_and_any_other_poll_error_is_its_callers() {
    // What poll found, decided apart from the poll: ready for what was
    // wanted, or at the descriptor's end, error or closing; not ready
    // where it found nothing or a signal interrupted its wait; and any
    // other error of its own, such as no memory, the caller's.
    let errno = |found: std::io::Result<bool>| found.map_err(|e| e.raw_os_error());
    let (out, nomem) = (PollFlags::OUT, Errno::NOMEM.raw_os_error());
    for (found, ready) in [
        (Ok(PollFlags::OUT), Ok(true)),
        (Ok(PollFlags::HUP), Ok(true)),
        (Ok(PollFlags::ERR), Ok(true)),
        (Ok(PollFlags::NVAL), Ok(true)),
        (Ok(PollFlags::IN), Ok(false)),
        (Ok(PollFlags::empty()), Ok(false)),
        (Err(Errno::INTR), Ok(false)),
        (Err(Errno::NOMEM), Err(Some(nomem))),
    ] {
        assert_eq!((found, errno(readiness(found, out))), (found, ready));
    }
}

#[test]
fn what_the_server_is_offered_and_partly_takes_before_a_write_errs_is_taken() {
    // Two writes' worth offered to the server's input, which takes the
    // first whole and then refuses: what it took is reported taken, not
    // the error, so it is never queued again; where it took nothing, the
    // error is its caller's.
    let bytes = vec![b'x'; 2 * ATOMIC];
    let (mut once, reader) = sink(ATOMIC, 1, Duration::ZERO);
    let taken = offered(&mut once, &bytes).map_err(|e| e.kind());
    drop(once);
    assert_eq!(
        (taken, held(reader)),
        (Ok(ATOMIC), bytes[..ATOMIC].to_vec())
    );
    let (mut never, _reader) = sink(ATOMIC, 0, Duration::ZERO);
    let taken = offered(&mut never, &bytes).map_err(|e| e.kind());
    assert_eq!(taken, Err(ErrorKind::BrokenPipe));
}
