//! The broker's own standard error (decision 0065 slice two, U6f1a, U6f1b;
//! MB3, MB4): the one path every byte written there passes, and a served
//! server's standard error, drained beside its output.
//!
//! Every byte written there is a line a session's [`Voice`] passed
//! ([`Said`], which nothing else makes): its display text sanitized line by
//! line as every rendered line is (`render::Safe`), masked by the shared
//! masker on its raw bytes, decoded lossily and masked again as the text it
//! sanitizes to, so that a bound value a control byte split is masked once
//! it is joined, and kept only where the final guard, read as text, finds
//! the very bytes kept safe (`secret::Guard`), and every piece it is cut
//! into at its safe points (`rpc::pieces`), a line one write takes whole,
//! safe on its own. A line it does not, or that cannot be cut safely, is
//! kept as a fixed line in its place, where that is safe, and else not at
//! all. [`told`] is the one writer of the broker's standard error, and
//! takes nothing but those lines, each while the bindings that guarded the
//! session's ledger are still held: the session's lifetime calls it with
//! a served session's server's tail before that ledger is closed, which
//! closes Clean only where every piece was taken whole, or the reader had
//! closed its end, a write meeting a broken pipe (operator rulings
//! 2026-10-14), and with a failed session's tail and the broker's own last
//! line ([`last`]), its `error: ` included, after. It writes on the
//! session's own thread, a piece to a write, each once poll finds the
//! descriptor writable, within a fixed bound and never past it, and only
//! while the attempt is not cancelled, so a standard error nobody reads
//! never holds the broker's end, no write outlives the bindings, and its
//! descriptor's status flags, which every holder of it shares, are never
//! changed.
//!
//! The server's standard error is kept only as a bounded tail of such
//! lines. The raw bytes are split at newlines before anything is masked,
//! and the bytes since the last one are the raw overlap carried across
//! reads: a bound value is one line of the operator's store, so neither it
//! nor any of its shared encodings holds a newline, and a value split
//! across reads is masked once its line is whole. The production guard
//! that no encoding holds one is left out as unreachable;
//! `a_bound_value_never_holds_the_newline_the_drain_splits_at` is its
//! removal control. Each line is whole before it is decoded, so a character
//! split across reads is decoded whole. A line past its bound is passed
//! over to its end, and only its marker is kept. One line is kept each
//! turn, from the server's startup on (U6f1b); when the session ends, every
//! whole line the pipe still holds is, within a bound. A last line without
//! its newline is kept once the pipe has ended, when nothing more can
//! complete it, and never before.
//!
//! Every piece of every line kept of the server's standard error, a marker
//! in a line's place included, begins with [`SERVER`], so that no line the
//! server writes, `error: ` and all, reads as the broker's own: the label
//! is fixed bytes, counted inside each piece's bound and guarded with it,
//! and the broker's exit status, not its standard error, says how the
//! session ended (U6f2; operator ruling 2026-10-14).

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::os::fd::{AsFd, OwnedFd};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use brokkr_protocol::secret::{self, BoundSecret, Guard};

use super::{Ended, Startup};
use crate::broker::rpc::pieces::{self, put, Put};
use crate::broker::rpc::{Line, Lines, ATOMIC};
use crate::render::Safe;

#[cfg(test)]
mod tests;

/// The most raw bytes one line holds, and the most masked bytes the tail
/// keeps.
const LINE_MAX: usize = 64 << 10;
const TAIL_MAX: usize = 64 << 10;

/// How long the broker's standard error may take what is written to it
/// once the session ends: a fixed bound that nothing extends, and after
/// which nothing waits.
const TOLD: Duration = Duration::from_secs(1);

/// The most bytes the pipe is read for once the session ends.
const CLOSING_MAX: usize = 16 * TAIL_MAX;

/// What is kept of a line past [`LINE_MAX`].
const PASSED_OVER: &[u8] = b"[MCP server stderr line exceeds the broker limit]";

/// What is kept of a line the final guard refuses.
const UNSAFE: &str = "[MCP server stderr line cannot be safely masked]";

/// What the broker says in place of its last line where the final guard
/// refuses that line.
const UNREPORTED: &str = "error: MCP broker session failed and its cause cannot be safely reported";

/// What begins every piece of a line kept of the server's standard error.
pub(in crate::broker) const SERVER: &str = "MCP server stderr: ";

/// The one way a line reaches the broker's standard error: the bindings a
/// session looked up, which it is masked by, their final guard, and the
/// label each piece it passes begins with, none for the broker's own.
pub(in crate::broker) struct Voice<'b> {
    bindings: &'b [BoundSecret],
    guard: Guard<'b>,
    label: &'static str,
}

/// One line a [`Voice`] passed, its newline included: the only bytes
/// [`told`] writes, made nowhere else.
pub(in crate::broker) struct Said(String);

#[cfg(test)]
impl Said {
    /// The line's text, which tests read.
    pub(in crate::broker) fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'b> Voice<'b> {
    /// The broker's own voice of `bindings`, none where none were looked
    /// up: its pieces unlabelled.
    pub(in crate::broker) fn new(bindings: &'b [BoundSecret]) -> Voice<'b> {
        Voice {
            bindings,
            guard: Guard::new(bindings),
            label: "",
        }
    }

    /// `raw` as the broker shows it ([`Voice::shown`]) where the final guard
    /// finds those very bytes safe, and each piece it is cut into, a line
    /// of at most [`ATOMIC`] bytes, its label and newline included, safe on
    /// its own, cut before it is guarded, so that wherever its writes stop,
    /// what a reader holds is whole pieces the guard passed; else `fixed`
    /// so shown, where it does that; else nothing.
    pub(in crate::broker) fn said(&self, raw: &[u8], fixed: &str) -> Option<Said> {
        let safe = |text: &str| self.guard.text(text.as_bytes());
        [raw, fixed.as_bytes()].into_iter().find_map(|text| {
            let shown = self.shown(text);
            let pieces = cut(&shown, self.label)?;
            let passed = safe(&shown) && pieces.split_inclusive('\n').all(safe);
            passed.then_some(Said(pieces))
        })
    }

    /// `raw` masked as it is and decoded lossily, each of its lines
    /// sanitized for display ([`Safe`]), which may join a bound value a
    /// control byte split, masked again as that text, and its newline
    /// after.
    fn shown(&self, raw: &[u8]) -> String {
        let decoded = secret::mask_text(raw, self.bindings);
        let lines: Vec<String> = decoded
            .split('\n')
            .map(|line| Safe::new(line).as_str().to_owned())
            .collect();
        secret::mask_text(lines.join("\n").as_bytes(), self.bindings) + "\n"
    }
}

/// The lines of `shown`, each ending in its newline, cut at their safe
/// points into lines of at most [`ATOMIC`] bytes, each beginning with
/// `label`, that label and newline included ([`pieces::cut`]), each on a
/// character's boundary; none where a line cannot be cut safely.
fn cut(shown: &str, label: &str) -> Option<String> {
    let mut pieces = String::with_capacity(shown.len());
    for line in shown.split_inclusive('\n') {
        let line = line.strip_suffix('\n').unwrap_or(line);
        let ends = pieces::cut(line.as_bytes(), ATOMIC - 1 - label.len())?;
        let mut start = 0;
        for end in ends.iter().copied().chain(ends.is_empty().then_some(0)) {
            pieces.push_str(label);
            pieces.push_str(&line[start..end]);
            pieces.push('\n');
            start = end;
        }
    }
    Some(pieces)
}

/// The server's standard error read as lines without blocking, the voice
/// that passes what is kept of it, the lines kept and their bytes, and
/// whether the pipe has ended.
pub(in crate::broker) struct Drain<'b, R> {
    lines: Lines<R>,
    voice: Voice<'b>,
    tail: VecDeque<Said>,
    kept: usize,
    ended: bool,
}

impl<'b, R: Read + AsFd> Drain<'b, R> {
    /// `pipe` drained, read only once it is readable, masked by `bindings`
    /// and each piece kept labelled the server's ([`SERVER`]).
    pub(in crate::broker) fn new(pipe: R, bindings: &'b [BoundSecret]) -> Drain<'b, R> {
        Drain {
            lines: Lines::new(pipe),
            voice: Voice {
                label: SERVER,
                ..Voice::new(bindings)
            },
            tail: VecDeque::new(),
            kept: 0,
            ended: false,
        }
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
    pub(in crate::broker) fn tail(self) -> Vec<Said> {
        self.tail_within(CLOSING_MAX)
    }

    /// [`Drain::tail`], the pipe read for at most `most` bytes, which only
    /// tests make fewer than [`CLOSING_MAX`].
    pub(in crate::broker) fn tail_within(mut self, most: usize) -> Vec<Said> {
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
        self.tail.into()
    }

    /// Keep the line `raw` as the voice passes it, or its marker in its
    /// place ([`Voice::said`]); and drop the oldest lines kept past
    /// [`TAIL_MAX`].
    fn keep(&mut self, raw: &[u8]) {
        let Some(line) = self.voice.said(raw, UNSAFE) else {
            return;
        };
        self.kept += line.0.len();
        self.tail.push_back(line);
        while self.kept > TAIL_MAX {
            self.kept -= self.tail.pop_front().map_or(0, |line| line.0.len());
        }
    }
}

/// The exit code `serve` leaves with once its session has `ended`, and its
/// last line: none where it was served; else the line any failed command
/// prints, rendered whole, its `error: ` included ([`crate::report_to`]),
/// and passed by the voice of the session's `bindings` ([`Voice::said`]),
/// or [`UNREPORTED`] in its place, or none where neither passes; and none
/// where the bindings are unknown, which nothing can be guarded by
/// (operator ruling 2026-10-13). The exit code says the failure either way.
pub(in crate::broker) fn last(
    ended: Result<(), Ended>,
    bindings: Option<&[BoundSecret]>,
) -> (ExitCode, Option<Said>) {
    let Err(ended) = ended else {
        return (ExitCode::SUCCESS, None);
    };
    let mut rendered = Vec::new();
    let code = crate::report_to(&ended.into(), &mut rendered);
    let line = rendered.strip_suffix(b"\n").unwrap_or(&rendered);
    let said = bindings.and_then(|bindings| Voice::new(bindings).said(line, UNREPORTED));
    (code, said)
}

/// Write `said` to the broker's own standard error, through a descriptor
/// of its own that shares it and whose status flags are never changed,
/// within [`TOLD`] ([`told_to`]), and only until the attempt is
/// `cancelled` while it is written: a cancellation that ended the session
/// still has its cause told within that bound, as every refusal is. What
/// became of the writing: taken where there was nothing to write.
pub(in crate::broker) fn told(said: &[Said], cancelled: &(dyn Fn() -> bool + Sync)) -> Put {
    told_through(
        std::io::stderr().as_fd().try_clone_to_owned(),
        said,
        cancelled,
    )
}

/// [`told`], through `sink`, the descriptor of its own it set up: failed,
/// a delivery failure, where that could not be set up and there was
/// something to write (operator ruling 2026-10-14).
pub(super) fn told_through(
    sink: std::io::Result<OwnedFd>,
    said: &[Said],
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Put {
    match (said.is_empty(), sink) {
        (true, _) => Put::Taken,
        (false, Ok(sink)) => {
            let ended = cancelled();
            let since = move || !ended && cancelled();
            let sink = std::fs::File::from(sink);
            told_to(said, sink, (Instant::now() + TOLD, &since))
        }
        (false, Err(_)) => Put::Failed,
    }
}

/// Write `said` to `sink` on this thread, one piece of a line to a write
/// ([`Voice::said`]), each whole once poll finds the sink writable, by
/// `attempt`'s deadline and no later and only while it is not cancelled,
/// both read again after every poll and before every write ([`put`]), so a
/// sink that takes nothing holds no one's end, and every write has returned
/// when this does: none outlives the bindings the lines were passed by. A
/// pipe takes each piece whole or not at all; a write that takes part of
/// one, as a terminal's or a file's may, ends the writing, and the session
/// as a delivery failure, and what it took is no emission of the broker's
/// (operator ruling 2026-10-13). What became of the writing: taken where
/// all of it was written whole; else what became of the piece it ended at.
pub(in crate::broker) fn told_to(
    said: &[Said],
    mut sink: impl Write + AsFd,
    attempt: Startup<'_>,
) -> Put {
    let pieces = said.iter().flat_map(|line| line.0.split_inclusive('\n'));
    for piece in pieces.map(str::as_bytes) {
        loop {
            match put(&mut sink, piece, attempt, Duration::MAX) {
                Put::Taken => break,
                Put::Unready => {}
                ended @ (Put::Late | Put::Closed | Put::Failed | Put::Short) => return ended,
            }
        }
    }
    Put::Taken
}
