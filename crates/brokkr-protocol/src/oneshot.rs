//! One bounded seat, outside every run (decision 0020 ruling 4).
//!
//! The engine's seats belong to a run: each attempt is journaled, each
//! failure is answered by the retry ladder, each seat gets the run's
//! working directory. A standing overseer is none of those things — it
//! reads, it proposes, and it must not be able to write a run journal at
//! all. So this module is deliberately the smaller half of the engine's
//! driver path: spawn, one attempt, a deadline, a typed outcome, done.
//!
//! Three properties are structural here, not conventions:
//!
//! - **No journal.** This crate does not depend on `brokkr-store`, so
//!   nothing reachable from here can append an event to any run.
//! - **No repository.** The scratch directory the driver runs in is
//!   created HERE and removed when the call returns. A caller cannot
//!   hand this seat a repository tree, because there is no parameter to
//!   hand it in.
//! - **No retry ladder.** One attempt. A refusal is returned as a
//!   refusal; whether to ask again is the operator's, not this
//!   function's.

use std::path::Path;
use std::time::Duration;

use serde_json::Value;

use crate::process::DriverProcess;
use crate::AttemptOutcome;

/// The engine version this transport announces in its handshake.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// How much of a refused attempt's stderr rides back with the reason.
const STDERR_TAIL: usize = 2000;

/// What one bounded seat produced, or why it produced nothing. There is
/// no third arm: a seat that did not conclude produced nothing, and a
/// caller cannot mistake a refusal for an empty result.
#[derive(Debug, Clone)]
pub enum OneShot {
    Produced {
        result: Value,
        /// The attempt's checkpoints in order — where a driver reports
        /// its own cost and usage.
        checkpoints: Vec<Value>,
    },
    Refused {
        reason: String,
    },
}

fn tail(stderr: &str) -> &str {
    let mut start = stderr.len().saturating_sub(STDERR_TAIL);
    while !stderr.is_char_boundary(start) {
        start -= 1;
    }
    &stderr[start..]
}

/// Run one seat to a terminal outcome. `input` is called with the
/// scratch directory this function created, so the seat's result file
/// lands somewhere that is neither a repository nor a run's workspace.
pub fn run_once(
    command: &[String],
    seat: &str,
    deadline: Duration,
    input: impl FnOnce(&Path) -> Value,
) -> OneShot {
    run_once_in(
        tempfile::Builder::new().prefix("forge-oneshot-").tempdir(),
        command,
        seat,
        deadline,
        input,
    )
}

/// The scratch directory arrives as a `Result` so the staging failure is
/// an ordinary argument rather than an untestable branch.
fn run_once_in(
    scratch: std::io::Result<tempfile::TempDir>,
    command: &[String],
    seat: &str,
    deadline: Duration,
    input: impl FnOnce(&Path) -> Value,
) -> OneShot {
    let scratch = match scratch {
        Ok(scratch) => scratch,
        Err(error) => {
            return OneShot::Refused {
                reason: format!("could not stage a scratch directory: {error}"),
            }
        }
    };
    let process = match DriverProcess::spawn(command, scratch.path(), Some(deadline)) {
        Ok(process) => process,
        Err(error) => {
            return OneShot::Refused {
                reason: format!("driver did not spawn: {error}"),
            }
        }
    };
    // Fresh ids: this attempt correlates to nothing durable, which is
    // the whole point — there is no effect and no run for it to join.
    let effect_id = uuid::Uuid::new_v4().to_string();
    let attempt_id = uuid::Uuid::new_v4().to_string();
    let report = process.run_attempt(
        VERSION,
        &effect_id,
        &attempt_id,
        seat,
        input(scratch.path()),
        // Checkpoints are collected by the transport and read from the
        // report; there is no journal here to stream them into.
        |_| {},
    );
    let stderr = tail(&report.stderr).to_string();
    match report.outcome {
        AttemptOutcome::Succeeded { result } => OneShot::Produced {
            result,
            checkpoints: report.checkpoints,
        },
        AttemptOutcome::Failed { error } => OneShot::Refused {
            reason: format!("{error}; stderr tail: {stderr}"),
        },
        AttemptOutcome::Indeterminate { reason } => OneShot::Refused {
            reason: format!("{reason}; stderr tail: {stderr}"),
        },
    }
}

#[cfg(test)]
mod tests;
