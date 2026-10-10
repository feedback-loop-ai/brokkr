//! What the transport holds of what a driver sends it (#433): one stdout
//! line at a time, the stderr it keeps as evidence, and the checkpoints
//! its report carries. A runaway or buggy driver, a megabyte line with no
//! newline, a stderr flood or a checkpoint loop, must not exhaust the
//! engine's memory, and its attempt must end in a truthful outcome.
//!
//! A line is refused once it runs past [`FRAME_BYTES`], before it is held
//! whole, decoded as UTF-8 or parsed as JSON. A result is one line, so the
//! frame limit is its limit too. Checkpoints past [`CHECKPOINTS`], or past
//! [`CHECKPOINT_BYTES`] of their lines, are refused before they are
//! journaled or kept. Either refusal ends the attempt indeterminate: the
//! driver may already have had effects, so it is neither a success nor a
//! failure a retry may safely repeat, and #403's teardown ends its tree.
//! Stderr past [`STDERR_BYTES`] is no failure: it is still drained, so the
//! driver never blocks on a full pipe, and what is kept is its first and
//! last bytes around a marker that counts what was dropped.
//!
//! Every limit is a count, compared without overflow, so no value of one
//! means unlimited, and zero admits nothing.

use std::collections::VecDeque;
use std::io::{BufRead, ErrorKind, Read};

use serde_json::Value;
use thiserror::Error;

use crate::AttemptOutcome;

/// The most bytes of one stdout protocol line, its newline excluded:
/// 16 MiB.
pub(super) const FRAME_BYTES: usize = 16 * 1024 * 1024;

/// The most bytes of the driver's stderr its report keeps: 1 MiB, half
/// from the start and half from the end.
pub(super) const STDERR_BYTES: usize = 1024 * 1024;

/// The most checkpoints one attempt journals and its report keeps.
pub(super) const CHECKPOINTS: usize = 100_000;

/// The most bytes of checkpoint lines, newlines included, one attempt
/// journals and its report keeps: 64 MiB.
pub(super) const CHECKPOINT_BYTES: usize = 64 * 1024 * 1024;

/// The most bytes of a refused line its refusal quotes.
const EVIDENCE_BYTES: usize = 64;

/// The limits of one driver's transport. Production takes
/// [`Limits::DEFAULT`]; the tests inject small ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Limits {
    /// Bytes of one stdout line: [`FRAME_BYTES`].
    pub(super) frame_bytes: usize,
    /// Bytes of stderr kept: [`STDERR_BYTES`].
    pub(super) stderr_bytes: usize,
    /// Checkpoints kept: [`CHECKPOINTS`].
    pub(super) checkpoints: usize,
    /// Bytes of checkpoint lines kept: [`CHECKPOINT_BYTES`].
    pub(super) checkpoint_bytes: usize,
}

impl Limits {
    pub(super) const DEFAULT: Limits = Limits {
        frame_bytes: FRAME_BYTES,
        stderr_bytes: STDERR_BYTES,
        checkpoints: CHECKPOINTS,
        checkpoint_bytes: CHECKPOINT_BYTES,
    };
}

/// A limit the driver exceeded, in the operator's words: the limit, its
/// unit, and evidence bounded by [`EVIDENCE_BYTES`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum Exceeded {
    #[error("driver stdout line exceeded the frame limit of {limit} bytes; it began \"{head}\"")]
    Frame { limit: usize, head: String },
    #[error("driver checkpoints exceeded the limit of {limit} checkpoints")]
    Checkpoints { limit: usize },
    #[error(
        "driver checkpoints exceeded the limit of {limit} bytes: {count} checkpoints \
         held {held} bytes when one of {offered} bytes arrived"
    )]
    CheckpointBytes {
        limit: usize,
        count: usize,
        held: usize,
        offered: usize,
    },
}

impl Exceeded {
    /// The attempt's outcome: indeterminate, which parks (decision 0006).
    pub(super) fn outcome(&self) -> AttemptOutcome {
        AttemptOutcome::Indeterminate {
            reason: format!("{self}; the driver was ended, and what it did is unknown"),
        }
    }
}

/// One read of a stdout line.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Frame {
    /// The line, its newline included when it had one.
    Line(Vec<u8>),
    Eof,
    /// The line ran past the limit; the evidence of how it began.
    Over {
        head: String,
    },
}

/// The next line of `reader`, holding no more than `limit` bytes of it
/// before its newline. A line is read to its newline or to EOF, as
/// `read_line` reads one, and refused at the first byte past the limit.
pub(super) fn read_frame(reader: &mut impl BufRead, limit: usize) -> std::io::Result<Frame> {
    let mut line = Vec::new();
    loop {
        let ready = match reader.fill_buf() {
            Ok(ready) => ready,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if ready.is_empty() {
            return Ok(if line.is_empty() {
                Frame::Eof
            } else {
                Frame::Line(line)
            });
        }
        let newline = ready.iter().position(|&byte| byte == b'\n');
        let body = newline.unwrap_or(ready.len());
        // `line` never holds more than `limit`, so this cannot underflow.
        if body > limit - line.len() {
            line.extend_from_slice(&ready[..body.min(EVIDENCE_BYTES)]);
            return Ok(Frame::Over {
                head: evidence(&line),
            });
        }
        let taken = newline.map_or(body, |at| at + 1);
        line.extend_from_slice(&ready[..taken]);
        reader.consume(taken);
        if newline.is_some() {
            return Ok(Frame::Line(line));
        }
    }
}

/// The first [`EVIDENCE_BYTES`] of `bytes`, ASCII-escaped, so neither a
/// control byte nor invalid UTF-8 reaches the operator raw.
fn evidence(bytes: &[u8]) -> String {
    bytes[..bytes.len().min(EVIDENCE_BYTES)]
        .escape_ascii()
        .to_string()
}

/// The driver's stderr drained to EOF, of which no more than `limit`
/// bytes are kept: the whole of it when it fits, as UTF-8 or nothing, as
/// it was always kept. Otherwise its first half and its last half of
/// `limit`, around a marker counting the bytes dropped between them, each
/// half decoded lossily since a cut may split a character.
pub(super) fn retained_stderr(mut stderr: impl Read, limit: usize) -> String {
    let head_limit = limit / 2;
    let tail_limit = limit - head_limit;
    let (mut head, mut tail) = (Vec::new(), VecDeque::new());
    let mut dropped = 0usize;
    let mut chunk = [0u8; 8192];
    loop {
        let read = match stderr.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(_) => break,
        };
        let (first, rest) = chunk[..read].split_at((head_limit - head.len()).min(read));
        head.extend_from_slice(first);
        tail.extend(rest);
        let excess = tail.len().saturating_sub(tail_limit);
        tail.drain(..excess);
        dropped = dropped.saturating_add(excess);
    }
    if dropped == 0 {
        head.extend(tail);
        return String::from_utf8(head).unwrap_or_default();
    }
    format!(
        "{}\n[{dropped} bytes of driver stderr dropped]\n{}",
        String::from_utf8_lossy(&head),
        String::from_utf8_lossy(tail.make_contiguous())
    )
}

/// The checkpoints an attempt's report keeps, and the bytes of their
/// lines.
#[derive(Debug, Default)]
pub(super) struct Retained {
    pub(super) checkpoints: Vec<Value>,
    bytes: usize,
}

impl Retained {
    /// Keep `data`, a checkpoint whose line was `bytes` long, unless it
    /// is one past either checkpoint limit: the kept checkpoint, to be
    /// journaled, or the limit it exceeded.
    pub(super) fn keep(
        &mut self,
        data: Value,
        bytes: usize,
        limits: &Limits,
    ) -> Result<&Value, Exceeded> {
        let count = self.checkpoints.len();
        if count >= limits.checkpoints {
            return Err(Exceeded::Checkpoints {
                limit: limits.checkpoints,
            });
        }
        let held = self.bytes.saturating_add(bytes);
        if held > limits.checkpoint_bytes {
            return Err(Exceeded::CheckpointBytes {
                limit: limits.checkpoint_bytes,
                count,
                held: self.bytes,
                offered: bytes,
            });
        }
        self.bytes = held;
        self.checkpoints.push(data);
        Ok(&self.checkpoints[count])
    }
}

#[cfg(test)]
mod tests;
