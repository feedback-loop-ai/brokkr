//! One stream turn read to its end: the lines a fed harness prints, each
//! handed to its driver, the harness stopped at the first line the driver
//! refuses, and its exit and stderr taken. The one home Qoder's (#1178) and
//! Antigravity's (#1200) drivers read a turn through: Qoder's reading of
//! JSON rows ([`streamed`]) is built on the one byte seam
//! ([`streamed_lines`]) Antigravity's driver reads its lines through.

use std::io::BufRead;

use serde_json::Value;

use super::spawn::Spawned;
use super::stderr::{CapturedStderr, StderrCapture};
use super::{conclude_harness, io_context, lost};
use crate::PreSessionRefusal;

/// The exit a refused seat reads when the driver stopped its harness at
/// the refusing row: the harness never answered, so the stop is the
/// driver's refusal and not a signal the harness died of. The refusal
/// path journals no exit code; `lost::refusal_to_start` reads this one as
/// a code of the command's own, which it is not, so it is named here.
pub(super) const STOPPED_AT_REFUSAL: i32 = 1;

/// How a stream turn ended: its exit, the refusal the driver stopped it
/// at, and its stderr, as captured and not yet read.
pub(super) struct Turn {
    pub(super) exit_code: i32,
    pub(super) stopped: Option<PreSessionRefusal>,
    pub(super) captured: CapturedStderr,
}

/// What a driver makes of one line of its harness's stdout.
pub(super) enum Line {
    /// Read on.
    Read,
    /// Read no further, and wait for the harness to end.
    Ended,
    /// Stop the harness here, refused.
    Stopped(PreSessionRefusal),
}

/// The turn of the harness `fed`, each line of its stdout handed to `line`
/// as its bytes, without its line ending, to the end of the stream or a
/// stdout that can no longer be read. A line `line` stops at stops the
/// harness there, and the turn reads [`STOPPED_AT_REFUSAL`].
pub(super) fn streamed_lines(
    fed: (Spawned, StderrCapture),
    mut line: impl FnMut(&[u8]) -> Line,
) -> Result<Turn, String> {
    let (mut child, stderr) = fed;
    let stdout = child.stdout.take().expect("piped");
    let mut stopped = None;
    // The reader is dropped before the wait, so a harness still writing
    // meets a closed pipe rather than a full one.
    {
        let mut reader = std::io::BufReader::new(stdout);
        let mut bytes = Vec::new();
        while reader.read_until(b'\n', &mut bytes).unwrap_or(0) > 0 {
            match line(ended(&bytes)) {
                Line::Read => bytes.clear(),
                Line::Ended => break,
                Line::Stopped(refused) => {
                    stopped = Some(refused);
                    break;
                }
            }
        }
    }
    let status = match stopped {
        Some(_) => conclude_harness(&mut child)?,
        None => io_context(child.wait(), "agent CLI did not conclude")?,
    };
    let captured = stderr.take(stopped.is_some());
    let exit_code = match stopped {
        Some(_) => STOPPED_AT_REFUSAL,
        None => lost::exit_code(status),
    };
    Ok(Turn {
        exit_code,
        stopped,
        captured,
    })
}

/// `bytes` without the `\n` that ends it and a `\r` before that, as
/// [`BufRead::lines`] ends a line.
fn ended(bytes: &[u8]) -> &[u8] {
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    bytes.strip_suffix(b"\r").unwrap_or(bytes)
}

/// The turn of the harness `fed`, each JSON row of its stdout handed whole
/// to `row`, a line that is not JSON passed over, and the reading ended at
/// a line that is not UTF-8. A row `row` refuses stops the harness there,
/// and the turn reads [`STOPPED_AT_REFUSAL`].
pub(super) fn streamed(
    fed: (Spawned, StderrCapture),
    mut row: impl FnMut(Value) -> Result<(), PreSessionRefusal>,
) -> Result<Turn, String> {
    streamed_lines(fed, |bytes| {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return Line::Ended;
        };
        let Ok(event) = serde_json::from_str::<Value>(text) else {
            return Line::Read;
        };
        match row(event) {
            Ok(()) => Line::Read,
            Err(refused) => Line::Stopped(refused),
        }
    })
}
