//! One stream turn read to its end: the rows a fed harness prints, each
//! handed to its driver's fold, the harness stopped at the first row the
//! fold refuses, and its exit and stderr taken. The one home Qoder's
//! (#1178) and Antigravity's (#1200) drivers read a turn through, moved out
//! of the qoder driver unchanged.

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

/// The turn of the harness `fed`, each JSON row of its stdout handed whole
/// to `row`, a line that is not JSON passed over. A row `row` refuses
/// stops the harness there, and the turn reads [`STOPPED_AT_REFUSAL`].
pub(super) fn streamed(
    fed: (Spawned, StderrCapture),
    mut row: impl FnMut(Value) -> Result<(), PreSessionRefusal>,
) -> Result<Turn, String> {
    let (mut child, stderr) = fed;
    let stdout = child.stdout.take().expect("piped");
    let mut stopped = None;
    for line in std::io::BufReader::new(stdout).lines() {
        let Ok(line) = line else { break };
        let Ok(event) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if let Err(refused) = row(event) {
            stopped = Some(refused);
            break;
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
