//! What waits for the harness to take it (decision 0065 slice two, U6e,
//! U6f1b; MB3, MB4): whole frames, each cut at its safe points, written a
//! piece at a time as far as the harness takes them, never waiting on it,
//! moved whole out of the call path to keep that within its ceiling
//! (U6f1a).

use std::collections::VecDeque;
use std::io::Write;
use std::os::fd::AsFd;
use std::time::{Duration, Instant};

use super::{UNTAKEN_FRAMES, UNTAKEN_MAX};
use crate::broker::rpc::pieces::{put, Pieces, Put};

/// What became of the harness's output: open; closed by the harness, a
/// write meeting a broken pipe, which alone ends the session as its input's
/// end does; a frame not taken by its deadline, or the attempt cancelled,
/// which ends it with a timeout; or undelivered, where a write took part of
/// a piece, as a sink that is not a pipe may, took nothing, or erred
/// otherwise, or its poll erred, which ends it as a delivery failure
/// (operator rulings 2026-10-13, 2026-10-14). Nothing is written after any
/// of them, so no frame is ever left unfinished and another begun.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::broker::rpc) enum Delivery {
    Open,
    Gone,
    Stalled,
    Undelivered,
}

/// What waits for the harness to take it: whole frames in order, each with
/// the deadline by which it must be taken, and how many bytes of the first
/// it has taken already, which end one of its pieces.
#[derive(Default)]
pub(in crate::broker::rpc) struct Outbox {
    pub(in crate::broker::rpc) frames: VecDeque<(Pieces, Instant)>,
    pub(super) taken: usize,
}

impl Outbox {
    /// Whether the harness's next frame may be read: fewer than
    /// [`UNTAKEN_FRAMES`] frames, and fewer than [`UNTAKEN_MAX`] bytes,
    /// wait for it.
    pub(super) fn room(&self) -> bool {
        let bytes: usize = self.frames.iter().map(|(line, _)| line.bytes().len()).sum();
        (self.frames.len() < UNTAKEN_FRAMES) & (bytes - self.taken < UNTAKEN_MAX)
    }

    /// Write as much of what waits to `output` as it takes now, a piece to
    /// a write, each only once poll finds it ready, while its frame's
    /// deadline, the one that applies to it, is ahead and the attempt is
    /// not `cancelled`, both read again after the poll and before the write
    /// ([`put`]); never waiting for it and never changing its status flags,
    /// and no frame begun before the one ahead of it is whole: stalled,
    /// with nothing written, where a frame waiting is past its deadline,
    /// and stalled where that deadline passes or the attempt is cancelled
    /// between pieces; gone where the harness has closed its end;
    /// undelivered where a piece is taken short or not at all, or the
    /// output fails otherwise; open otherwise.
    pub(in crate::broker::rpc) fn flushed(
        &mut self,
        output: &mut (impl Write + AsFd),
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Delivery {
        let now = Instant::now();
        if self.frames.iter().any(|(_, deadline)| now >= *deadline) {
            return Delivery::Stalled;
        }
        while let Some((line, deadline)) = self.frames.front() {
            let piece = line.after(self.taken);
            match put(output, piece, (*deadline, cancelled), Duration::ZERO) {
                Put::Taken => self.taken += piece.len(),
                Put::Unready => break,
                Put::Late => return Delivery::Stalled,
                Put::Closed => return Delivery::Gone,
                Put::Failed | Put::Short => return Delivery::Undelivered,
            }
            if self.taken == line.bytes().len() {
                self.frames.pop_front();
                self.taken = 0;
            }
        }
        Delivery::Open
    }
}
