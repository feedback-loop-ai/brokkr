//! The open attempt's latest gate step observation, carried forward by
//! the fold (decision 0109 ruling 7, fix round 0109a1r4, K2).
//!
//! A protected sequence's gate step journals the head its span accepted
//! as `checked_head` in the step's checkpoint, which the engine authors.
//! The attempt's terminal result journals the effect's observation too.
//! Both are durable, so the fold carries the step's forward to decide,
//! which refuses when they disagree, and no HEAD is read to settle it.

use serde::Deserialize;
use serde_json::Value;

use crate::findings::{CheckedHead, EngineMarker, EngineWritten};

/// What the open attempt's latest engine-authored gate step checkpoint
/// journaled as its `checked_head`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepObservation {
    /// A commit id.
    Checked(CheckedHead),
    /// A value that is not a commit id, which agrees with no head.
    Unreadable,
}

/// The observation `checkpoint` journals, if it is the engine's own step
/// marker and carries a checked head: a driver's record never is.
pub(super) fn of_checkpoint(checkpoint: &Value) -> Option<StepObservation> {
    (EngineMarker::authored(checkpoint) == Some(EngineMarker::StepFinished)).then_some(())?;
    let field = EngineWritten::CheckedHead.field();
    let head = checkpoint.get("result")?.get(field)?;
    Some(match CheckedHead::deserialize(head) {
        Ok(head) => StepObservation::Checked(head),
        Err(_) => StepObservation::Unreadable,
    })
}

#[cfg(test)]
mod tests;
