//! What waits for the harness to take it (decision 0065 slice two, U6e;
//! MB3): whole frames written as far as the harness takes them, never
//! waiting on it, moved whole out of the call path to keep that within its
//! ceiling (U6f1a).

use std::collections::VecDeque;
use std::io::{ErrorKind, Write};
use std::time::Instant;

use super::{UNTAKEN_FRAMES, UNTAKEN_MAX};

/// What became of the harness's output: open; closed by the harness,
/// which ends the session as its input's end does; or a frame not taken by
/// its deadline, which ends it with a timeout. Nothing is written after
/// either, so no frame is ever left unfinished and another begun.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::broker::rpc) enum Delivery {
    Open,
    Gone,
    Stalled,
}

/// What waits for the harness to take it: whole frames in order, each with
/// the deadline by which it must be taken, and how many bytes of the first
/// it has taken already.
#[derive(Default)]
pub(in crate::broker::rpc) struct Outbox {
    pub(in crate::broker::rpc) frames: VecDeque<(Vec<u8>, Instant)>,
    pub(super) taken: usize,
}

impl Outbox {
    /// Whether the harness's next frame may be read: fewer than
    /// [`UNTAKEN_FRAMES`] frames, and fewer than [`UNTAKEN_MAX`] bytes,
    /// wait for it.
    pub(super) fn room(&self) -> bool {
        let bytes: usize = self.frames.iter().map(|(line, _)| line.len()).sum();
        (self.frames.len() < UNTAKEN_FRAMES) & (bytes - self.taken < UNTAKEN_MAX)
    }

    /// Write as much of what waits to `output` as it takes now, never
    /// waiting for it, and no frame begun before the one ahead of it is
    /// whole: stalled, with nothing written, where a frame waiting is past
    /// its deadline; gone where the output has ended or fails; open
    /// otherwise.
    pub(in crate::broker::rpc) fn flushed(&mut self, output: &mut impl Write) -> Delivery {
        let now = Instant::now();
        if self.frames.iter().any(|(_, deadline)| now >= *deadline) {
            return Delivery::Stalled;
        }
        while let Some((line, _)) = self.frames.front() {
            let taken = match output.write(&line[self.taken..]) {
                Ok(taken) => taken,
                Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                Err(_) => return Delivery::Gone,
            };
            self.taken += taken;
            if self.taken < line.len() {
                break;
            }
            self.frames.pop_front();
            self.taken = 0;
        }
        Delivery::Open
    }
}
