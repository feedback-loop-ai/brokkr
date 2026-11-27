//! One stream turn read to its end: the lines a fed harness prints, each
//! handed to its driver, the harness stopped at the first line the driver
//! refuses, and its exit and stderr taken. The one home Antigravity's
//! driver (#1200) reads a turn through ([`streamed_lines`]); Qoder's driver
//! (#1178) reads its rows as its own and stops on this module's
//! [`STOPPED_AT_REFUSAL`] and [`UNCONCLUDED`]. Both read a pin by
//! [`plain`] and a turn's stderr through [`Observed`].

use std::io::BufRead;

use thiserror::Error;

use super::escapes::without_ansi;
use super::spawn::Spawned;
use super::stderr::{CapturedStderr, StderrCapture};
use super::{lost, stop_harness};
use crate::PreSessionRefusal;

/// The exit a refused seat reads when the driver stopped its harness at
/// the refusing row: the harness never answered, so the stop is the
/// driver's refusal and not a signal the harness died of. The refusal
/// path journals no exit code; `lost::refusal_to_start` reads this one as
/// a code of the command's own, which it is not, so it is named here.
pub(super) const STOPPED_AT_REFUSAL: i32 = 1;

/// How a stream turn ended: its exit, whether the harness exited 0 on its
/// own, read from its real status and never from the stop's reported code,
/// whether its stdout failed to read before its end, and its stderr, as
/// captured and not yet read.
pub(super) struct Turn {
    pub(super) exit_code: i32,
    /// A harness stopped at a refusing line after it had exited 0 on its
    /// own, its work done, exited clean, whatever [`STOPPED_AT_REFUSAL`]
    /// reports (#1178 H3).
    pub(super) clean: bool,
    pub(super) unread: bool,
    pub(super) captured: CapturedStderr,
}

/// How the reading of a stdout ended: the refusal it stopped at, and
/// whether a read failed before end of file.
#[derive(Debug, PartialEq)]
pub(super) struct Read {
    pub(super) stopped: Option<PreSessionRefusal>,
    pub(super) unsettled: bool,
    pub(super) unread: bool,
}

/// The shared driver's words for a harness whose wait failed: their one
/// home, read by every driver and by [`Unconcluded`] (decision 0071
/// ruling 5).
pub(super) const UNCONCLUDED: &str = "agent CLI did not conclude";

/// Why a turn did not conclude: the wait on its harness failed, its cause
/// carried. It prints in the shared driver's words, rendered once where
/// the seat's driver renders its end (decision 0071 ruling 8).
#[derive(Debug, Error)]
#[error("{UNCONCLUDED}: {0}")]
pub(super) struct Unconcluded(#[from] pub(super) std::io::Error);

/// What a driver makes of one line of its harness's stdout.
pub(super) enum Line {
    /// Read on.
    Read,
    /// Stop the harness here, refused.
    Stopped(PreSessionRefusal),
    /// Stop the harness here: a rejoin named a different or unconfirmable session.
    Unsettled,
}

/// The turn of the harness `fed`, each line of its stdout handed to `line`
/// as its bytes, without its line ending, to the end of the stream or a
/// stdout that can no longer be read, which the turn reports. A line
/// `line` stops at stops the harness there, and the turn reads
/// [`STOPPED_AT_REFUSAL`].
pub(super) fn streamed_lines(
    fed: (Spawned, StderrCapture),
    line: impl FnMut(&[u8]) -> Line,
) -> Result<Turn, Unconcluded> {
    let (mut child, stderr) = fed;
    let stdout = child.stdout.take().expect("piped");
    turn_read((child, stderr), std::io::BufReader::new(stdout), line)
}

/// The turn of the harness `fed`, its stdout read from `reader` by
/// [`read_lines`]: the one place a [`Turn`] is built, whatever the reader.
pub(super) fn turn_read(
    fed: (Spawned, StderrCapture),
    reader: impl BufRead,
    line: impl FnMut(&[u8]) -> Line,
) -> Result<Turn, Unconcluded> {
    let (mut child, stderr) = fed;
    // The reader is dropped before the wait, so a harness still writing
    // meets a closed pipe rather than a full one.
    let Read {
        stopped,
        unsettled,
        unread,
    } = read_lines(reader, line);
    let halted = stopped.is_some() || unsettled;
    let status = match halted {
        true => stop_harness(&mut child)?,
        false => child.wait()?,
    };
    let captured = stderr.take(halted);
    let exit_code = match halted {
        true => STOPPED_AT_REFUSAL,
        false => lost::exit_code(status),
    };
    Ok(Turn {
        exit_code,
        clean: status.success(),
        unread,
        captured,
    })
}

/// Each line of `reader` handed to `line`, as [`streamed_lines`] reads a
/// stdout: to its end, a line `line` stops at, or a read that
/// fails, which ends the reading and is reported (L3).
pub(super) fn read_lines(mut reader: impl BufRead, mut line: impl FnMut(&[u8]) -> Line) -> Read {
    let mut bytes = Vec::new();
    loop {
        match reader.read_until(b'\n', &mut bytes) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => {
                return Read {
                    stopped: None,
                    unsettled: false,
                    unread: true,
                }
            }
        }
        match line(ended(&bytes)) {
            Line::Read => bytes.clear(),
            Line::Stopped(refused) => {
                return Read {
                    stopped: Some(refused),
                    unsettled: false,
                    unread: false,
                }
            }
            Line::Unsettled => {
                return Read {
                    stopped: None,
                    unsettled: true,
                    unread: false,
                }
            }
        }
    }
    Read {
        stopped: None,
        unsettled: false,
        unread: false,
    }
}

/// `bytes` without the `\n` that ends it and a `\r` before that, as
/// [`BufRead::lines`] ends a line.
fn ended(bytes: &[u8]) -> &[u8] {
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    bytes.strip_suffix(b"\r").unwrap_or(bytes)
}

/// The most characters a plain model id holds.
pub(super) const PLAIN_ID_LIMIT: usize = 80;

/// Whether `text` is a plain model id: 1 to [`PLAIN_ID_LIMIT`] characters,
/// the first an ASCII letter or a digit, so no plain id reads as a flag
/// (H6), and each other an ASCII letter, a digit, `.`, `_` or `-`. No such
/// character is one JSON or Rust escapes, and none is one of the mask's
/// replacement text (`[secret:NAME]`), so a plain id has no escaped form
/// and no mask turns another value into one. Qoder's and Antigravity's
/// drivers each refuse a pin that is not one.
pub(super) fn plain(text: &str) -> bool {
    let character = |byte: u8| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-');
    let first = text
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_alphanumeric());
    first && text.len() <= PLAIN_ID_LIMIT && text.bytes().all(character)
}

/// A harness's stderr as its driver's checks read it: the raw text, lossy
/// UTF-8, and the same text [`without_ansi`], neither masked. A marker
/// found in either is said (V3): a false refusal costs a refused seat, a
/// missed one accepts a model nobody pinned. Both stay here and are only
/// matched (U2): neither is returned, emitted, logged or put in any error,
/// so it has no `Debug`.
pub(super) struct Observed {
    raw: String,
    stripped: String,
}

impl Observed {
    pub(super) fn of(bytes: &[u8]) -> Observed {
        let raw = String::from_utf8_lossy(bytes).into_owned();
        let stripped = without_ansi(&raw);
        Observed { raw, stripped }
    }

    pub(super) fn said(&self, marker: &str) -> bool {
        self.raw.contains(marker) || self.stripped.contains(marker)
    }
}
