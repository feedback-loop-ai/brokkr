//! The open attempt's latest gate step observation, carried forward by
//! the fold (decision 0109 ruling 7, fix rounds 0109a1r4, K2, and
//! 0109a1r5, V1).
//!
//! Every gate step of a protected sequence, the one that ends it
//! included, journals the head its span accepted as `checked_head`, and
//! its `covered` where covering was armed, in the step's checkpoint,
//! which the engine authors. The attempt's terminal result journals the
//! effect's observation too. Both are durable, so the fold carries the
//! latest step record whole to decide, which refuses when the terminal
//! disagrees with it on any field, and no HEAD is read to settle it.

use serde::Deserialize;
use serde_json::Value;

use crate::findings::{CheckedHead, Covered, EngineMarker, EngineWritten};

/// What the open attempt's latest engine-authored gate step checkpoint
/// journaled as its observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepObservation {
    /// A commit id, with the `covered` record the same span stamped
    /// beside it, which projects it, or none where covering was not armed.
    Checked {
        head: CheckedHead,
        covered: Option<Covered>,
    },
    /// A record that is not one observation: a `checked_head` that is not
    /// a commit id, or a `covered` that does not read or does not project
    /// the record's own `checked_head`. It agrees with no head.
    Unreadable,
}

/// The observation `checkpoint` journals, if it is the engine's own step
/// marker and carries a checked head: a driver's record never is. The
/// record's `covered` is kept, and one that disagrees with its own
/// `checked_head` makes the record no observation (fix round 0109a1r5,
/// V1).
pub(super) fn of_checkpoint(checkpoint: &Value) -> Option<StepObservation> {
    (EngineMarker::authored(checkpoint) == Some(EngineMarker::StepFinished)).then_some(())?;
    let result = checkpoint.get("result")?;
    let head = result.get(EngineWritten::CheckedHead.field())?;
    Some(read(head, result.get("covered")).unwrap_or(StepObservation::Unreadable))
}

/// `head` and `covered` as one observation, if they read and agree.
fn read(head: &Value, covered: Option<&Value>) -> Option<StepObservation> {
    let head = CheckedHead::deserialize(head).ok()?;
    let covered = match covered {
        Some(covered) => Some(Covered::deserialize(covered).ok()?),
        None => None,
    };
    let agrees = (covered.as_ref()).is_none_or(|covered| covered.projects(head.as_str()));
    agrees.then_some(StepObservation::Checked { head, covered })
}

#[cfg(test)]
mod tests;
