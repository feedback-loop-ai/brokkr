//! An end after `start` read into the outcome it earned (decisions 0006
//! and 0079): a foreign message, a success with no payload, and every end
//! that read no result. They live here so `process.rs` stays inside its
//! measured ceiling.

use serde_json::Value;

use super::{evidence, Ended, Standing};
use crate::{AttemptOutcome, ResultStatus};

/// The end of a message after `start` that is not this invocation's:
/// `said` of a foreign effect, or, the effect being this one, of a
/// foreign attempt (decision 0096 rulings 1 and 3). `current` is the
/// pair `start` carried, local to the invocation: a session a driver
/// resumed grants its earlier attempt nothing. The effect is judged
/// first, so a message foreign in both reads as it always did, and the
/// foreign bytes are described, never quoted. `None` admits a message
/// that is the current attempt's in both.
pub(super) fn refuse_foreign(
    said: &str,
    current: (&str, &str),
    received: (&str, &str),
) -> Option<Ended> {
    let (identity, held, want) = match (received.0 == current.0, received.1 == current.1) {
        (false, false) | (false, true) => ("effect", received.0, current.0),
        (true, false) => ("attempt", received.1, current.1),
        (true, true) => return None,
    };
    let said = format!("{said} {identity}");
    let field = format!("{identity}_id");
    Some(Ended::Unread(evidence::mismatch(&said, &field, held, want)))
}

/// An end after `start` that read no result: whatever the driver said
/// instead, what the attempt did is unknown, so it parks rather than
/// retries (decision 0006's 2026-10-11 addendum, ruling 1).
pub(super) fn unread(error: &str) -> AttemptOutcome {
    AttemptOutcome::Indeterminate {
        reason: format!(
            "{error}; no result was read after start, and what the attempt did is unknown"
        ),
    }
}

/// The outcome a driver's result reached. A success with no payload
/// carries no result of the executor's, so it is an end with none read,
/// and a line the reader refused outranks it (decision 0079 ruling 3).
pub(super) fn reached(
    status: ResultStatus,
    result: Option<Value>,
    error: Option<String>,
) -> Standing {
    match status {
        ResultStatus::Succeeded => match result {
            Some(result) => Standing::Final(AttemptOutcome::Succeeded { result }),
            None => Standing::Yielding(unread("succeeded result carried no payload")),
        },
        ResultStatus::Failed => Standing::Final(AttemptOutcome::Failed {
            error: error.unwrap_or_else(|| "driver reported failure".into()),
        }),
    }
}
