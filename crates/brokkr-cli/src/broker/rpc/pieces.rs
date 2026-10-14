//! Where what the broker writes is cut, and how each piece is written
//! (decision 0065 slice two, U6f1b; MB3, MB4; operator ruling 2026-10-13):
//! every frame the harness is told and every line of the broker's own
//! standard error is cut only at safe points, never inside a number or an
//! escape as the final guard reads them, raw or decoded, into pieces of at
//! most [`ATOMIC`] bytes, each written whole once poll finds its sink
//! ready, so a pipe is never left holding part of a number.
//!
//! A point is safe where the byte after it is inert, no digit, point, sign,
//! exponent letter, backslash or UTF-8 continuation byte, and no escape
//! spans it, escapes read from the first byte as the guard decodes them. A
//! number the guard reads, raw or decoded, is written only in bytes that
//! are not inert and in escapes, and the inert byte after a safe point
//! begins no escape and decodes to itself: so no number goes on past a safe
//! point, a reading of the bytes before it stops there as it stops at their
//! end, and those bytes hold only numbers, needles and escapes the whole
//! holds. What passed the guard whole passes it cut there.
//!
//! Two limits are the operator's (ruling 2026-10-13). A write of at most
//! [`ATOMIC`] bytes, made once poll finds the sink ready, can still wait
//! where another process writes the same pipe at that moment; the deadline
//! and cancellation are read again around every write. And [`ATOMIC`],
//! POSIX's least `PIPE_BUF`, is used on Linux and macOS alike instead of
//! each platform's own.

use std::io::{ErrorKind, Write};
use std::os::fd::AsFd;
use std::time::{Duration, Instant};

use rustix::event::PollFlags;

use super::{ready, ATOMIC};
use crate::broker::session::{timely, Startup};

#[cfg(test)]
pub(in crate::broker) mod tests;

/// Whether a piece may end just before `byte`: no number the guard reads
/// is written with it, nor does an escape begin with it, nor is it inside
/// a character.
fn inert(byte: u8) -> bool {
    !matches!(
        byte,
        b'0'..=b'9' | b'.' | b'+' | b'-' | b'e' | b'E' | b'\\' | 0x80..=0xbf
    )
}

/// How many bytes the escape `escape`, from its backslash, spans as the
/// guard decodes it: a `\u` and four hex digits; else the backslash and the
/// byte after it; else a last backslash alone.
fn spanned(escape: &[u8]) -> usize {
    let hex = escape.get(2..6);
    match (
        escape.get(1),
        hex.is_some_and(|hex| hex.iter().all(u8::is_ascii_hexdigit)),
    ) {
        (Some(b'u'), true) => 6,
        (Some(_), _) => 2,
        (None, _) => 1,
    }
}

/// Every safe point of `bytes`, in order, past its first byte: each before
/// an inert byte that no escape spans.
pub(in crate::broker) fn safe(bytes: &[u8]) -> impl Iterator<Item = usize> + '_ {
    let mut at = 0;
    std::iter::from_fn(move || {
        while at < bytes.len() {
            let here = at;
            at += match bytes[here] {
                b'\\' => spanned(&bytes[here..]),
                _ => 1,
            };
            if (here > 0) & inert(bytes[here]) {
                return Some(here);
            }
        }
        None
    })
}

/// Where `bytes` is cut into pieces of at most `most` bytes, each ending at
/// a safe point ([`safe`]) or at their end, each as long as one can be;
/// none where `most` bytes in a row hold no safe point, which can be cut
/// nowhere safely.
pub(in crate::broker) fn cut(bytes: &[u8], most: usize) -> Option<Vec<usize>> {
    let (mut ends, mut start, mut last) = (Vec::new(), 0, 0);
    for point in safe(bytes).chain([bytes.len()]) {
        if point - start > most {
            (last > start).then_some(())?;
            ends.push(last);
            start = last;
            (point - start <= most).then_some(())?;
        }
        last = point;
    }
    ends.extend((last > start).then_some(last));
    Some(ends)
}

/// A frame the harness is told, and the ends of the pieces it is cut into
/// ([`cut`]), each at most [`ATOMIC`] bytes.
#[derive(Debug, PartialEq)]
pub(in crate::broker) struct Pieces {
    bytes: Vec<u8>,
    ends: Vec<usize>,
}

impl Pieces {
    /// `bytes` cut at their safe points; none where they cannot be.
    pub(in crate::broker) fn new(bytes: Vec<u8>) -> Option<Pieces> {
        let ends = cut(&bytes, ATOMIC)?;
        Some(Pieces { bytes, ends })
    }

    /// The frame's bytes.
    pub(in crate::broker) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The piece after the first `taken` bytes, which end a piece.
    pub(in crate::broker) fn after(&self, taken: usize) -> &[u8] {
        let next = self.ends.partition_point(|end| *end <= taken);
        &self.bytes[taken..self.ends.get(next).copied().unwrap_or(taken)]
    }
}

/// What became of a piece offered to a sink: taken whole; not taken, where
/// the sink was not ready, or a signal or its own status flags stopped the
/// write before it took any; not offered, where the attempt was no longer
/// timely; closed, where the write met a broken pipe, its reader gone, which
/// alone keeps the session's existing ending; failed, where the poll or the
/// write erred otherwise, a full device or an I/O error among them, or the
/// write took nothing, or the sink could not be set up; or taken short,
/// where it took part of it. Failed and short each end the session as a
/// delivery failure (operator rulings 2026-10-13, 2026-10-14).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::broker) enum Put {
    Taken,
    Unready,
    Late,
    Closed,
    Failed,
    Short,
}

/// Offer `piece`, at most [`ATOMIC`] bytes, to `sink` in one write, once
/// poll finds it ready within `wait` and never past `attempt`'s deadline,
/// `attempt` read before the poll and again after it, before the write, so
/// a deadline or cancellation the poll outlived writes nothing. A sink that
/// takes part of the piece, as one that is not a pipe may, took it short,
/// and what it took is no emission of the broker's (operator ruling
/// 2026-10-13); the status flags every holder of the sink shares are never
/// changed.
pub(in crate::broker) fn put(
    sink: &mut (impl Write + AsFd),
    piece: &[u8],
    attempt: Startup<'_>,
    wait: Duration,
) -> Put {
    if !timely(attempt) {
        return Put::Late;
    }
    let left = attempt.0.saturating_duration_since(Instant::now());
    let polled = ready(sink.as_fd(), PollFlags::OUT, wait.min(left));
    if !timely(attempt) {
        return Put::Late;
    }
    written(polled, || sink.write(piece), piece.len())
}

/// Whether a descriptor polled for `wanted` is ready, as poll `found` it
/// ([`ready`]), decided apart from the poll: its end, its error or its
/// closing count as ready, so the read or write it allows meets them; a
/// wait a signal interrupted, as not; any other error of the poll's is its
/// caller's.
pub(super) fn readiness(
    found: rustix::io::Result<PollFlags>,
    wanted: PollFlags,
) -> std::io::Result<bool> {
    let ended = PollFlags::HUP | PollFlags::ERR | PollFlags::NVAL;
    match found {
        Ok(revents) => Ok(revents.intersects(wanted | ended)),
        Err(rustix::io::Errno::INTR) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

/// What became of a piece of `length` bytes once poll `polled` its sink,
/// written by `write` only where poll found it ready: taken whole; taken
/// short; not taken, where poll found it unready or a signal or status
/// flags stopped either before the write took any; closed only where the
/// write itself met a broken pipe, never by poll alone; else failed, a
/// write that took nothing included (operator ruling 2026-10-14).
pub(super) fn written(
    polled: std::io::Result<bool>,
    write: impl FnOnce() -> std::io::Result<usize>,
    length: usize,
) -> Put {
    let retried = |error: &std::io::Error| {
        matches!(error.kind(), ErrorKind::Interrupted | ErrorKind::WouldBlock)
    };
    let wrote = match polled {
        Ok(true) => write(),
        Ok(false) => return Put::Unready,
        Err(error) if retried(&error) => return Put::Unready,
        Err(_) => return Put::Failed,
    };
    match wrote {
        Ok(taken) if taken == length => Put::Taken,
        Ok(1..) => Put::Short,
        Err(error) if retried(&error) => Put::Unready,
        Err(error) if error.kind() == ErrorKind::BrokenPipe => Put::Closed,
        Ok(0) | Err(_) => Put::Failed,
    }
}
