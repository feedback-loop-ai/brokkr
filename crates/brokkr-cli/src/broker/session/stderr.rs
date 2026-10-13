//! A served server's standard error (decision 0065 slice two, U6f1a; MB4):
//! drained beside its output, so the server never waits on it, and kept
//! only as a bounded tail of masked lines. The raw bytes are split at
//! newlines before anything is masked, and the bytes since the last one
//! are the raw overlap carried across reads: a bound value is one line of
//! the operator's store, so neither it nor any of its shared encodings
//! holds a newline, and a value split across reads is masked once its line
//! is whole. The production guard that no encoding holds one is left out
//! as unreachable; `a_bound_value_never_holds_the_newline_the_drain_splits_at`
//! is its removal control. Each line is whole before it is decoded, so a
//! character split across reads is decoded whole: it is masked by the
//! shared masker on its raw bytes, decoded lossily, masked again as the
//! text it decodes to, and kept only where the final guard, read as text,
//! finds the very bytes kept safe (`secret::Guard`). A line it does not
//! is kept as its marker alone, or, where even that is not safe, not at
//! all. A line past its bound is passed over to its end, and only its
//! marker is kept. One line is kept each serving turn; when the session
//! ends, every whole line the pipe still holds is, within a bound. A last
//! line without its newline is kept once the pipe has ended, when nothing
//! more can complete it, and never before. The tail is written line by
//! line, each as the bytes the guard passed.

use std::collections::VecDeque;
use std::io::Read;
use std::os::fd::AsFd;

use brokkr_protocol::secret::{self, BoundSecret, Guard};

use crate::broker::rpc::{Line, Lines};

/// The most raw bytes one line holds, and the most masked bytes the tail
/// keeps.
const LINE_MAX: usize = 64 << 10;
const TAIL_MAX: usize = 64 << 10;

/// The most bytes the pipe is read for once the session ends.
const CLOSING_MAX: usize = 16 * TAIL_MAX;

/// What is kept of a line past [`LINE_MAX`].
const PASSED_OVER: &[u8] = b"[MCP server stderr line exceeds the broker limit]";

/// What is kept of a line the final guard refuses.
const UNSAFE: &str = "[MCP server stderr line cannot be safely masked]\n";

/// The server's standard error read as lines without blocking, the secrets
/// it is masked by and their final guard, the masked lines kept and their
/// bytes, and whether the pipe has ended.
pub(in crate::broker) struct Drain<'b, R> {
    lines: Lines<R>,
    bindings: &'b [BoundSecret],
    guard: Guard<'b>,
    tail: VecDeque<String>,
    kept: usize,
    ended: bool,
}

impl<'b, R: Read + AsFd> Drain<'b, R> {
    /// `pipe` drained and masked by `bindings`, or none where it cannot be
    /// read without blocking.
    pub(in crate::broker) fn new(pipe: R, bindings: &'b [BoundSecret]) -> Option<Drain<'b, R>> {
        Some(Drain {
            lines: Lines::new(pipe)?,
            bindings,
            guard: Guard::new(bindings),
            tail: VecDeque::new(),
            kept: 0,
            ended: false,
        })
    }

    /// One turn's read of the pipe, its next whole line kept: whether
    /// anything moved.
    pub(in crate::broker) fn drained(&mut self) -> bool {
        self.turn().is_some()
    }

    /// The pipe's next whole line kept, or the marker of one passed over:
    /// at most how many of its bytes that took; none where nothing moved.
    fn turn(&mut self) -> Option<usize> {
        match self.lines.next(LINE_MAX) {
            Line::Frame(line) => {
                self.keep(&line);
                Some(line.len() + 1)
            }
            Line::Over => {
                self.keep(PASSED_OVER);
                Some(LINE_MAX)
            }
            Line::Partial => Some(LINE_MAX),
            Line::Quiet => None,
            Line::Ended => {
                self.ended = true;
                None
            }
        }
    }

    /// The masked tail, oldest line first, each ending in its newline: every
    /// whole line the pipe still holds kept first, within [`CLOSING_MAX`]
    /// of its bytes, so a server still writing cannot hold the session's
    /// end; the last line without one kept only where the pipe has ended.
    pub(in crate::broker) fn tail(self) -> VecDeque<String> {
        self.tail_within(CLOSING_MAX)
    }

    /// [`Drain::tail`], the pipe read for at most `most` bytes, which only
    /// tests make fewer than [`CLOSING_MAX`].
    pub(in crate::broker) fn tail_within(mut self, most: usize) -> VecDeque<String> {
        let mut spent = 0;
        while spent <= most {
            let Some(bytes) = self.turn() else {
                break;
            };
            spent += bytes;
        }
        let rest = self.lines.rest();
        if self.ended & !rest.is_empty() {
            self.keep(&rest);
        }
        self.tail
    }

    /// Keep the line `raw`, masked, decoded and masked again, where the
    /// final guard finds those bytes safe; else its marker, where that is;
    /// and drop the oldest lines kept past [`TAIL_MAX`].
    fn keep(&mut self, raw: &[u8]) {
        let decoded = secret::mask_text(raw, self.bindings);
        let line = secret::mask_text(decoded.as_bytes(), self.bindings) + "\n";
        let safe = [line, UNSAFE.to_string()]
            .into_iter()
            .find(|line| self.guard.text(line.as_bytes()));
        let Some(line) = safe else {
            return;
        };
        self.kept += line.len();
        self.tail.push_back(line);
        while self.kept > TAIL_MAX {
            self.kept -= self.tail.pop_front().map_or(0, |line| line.len());
        }
    }
}
