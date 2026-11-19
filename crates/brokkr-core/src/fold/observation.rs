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
//!
//! A record is whole or it is no observation decide can stand on (fix
//! round 0109a1r6, W1): one that journals `covered` without
//! `checked_head`, or `checked_head` without `covered`, is
//! [`StepObservation::Partial`], never dropped. Each record names its
//! step, and the fold keeps the step the success names as its producer
//! (`RunState::ending_step`), so decide holds a terminal a gate step
//! produced to that step's own record exactly.

use serde::Deserialize;
use serde_json::Value;

use crate::findings::{CheckedHead, Covered, EngineMarker, EngineWritten};

/// What the open attempt's latest engine-authored gate step checkpoint
/// journaled as its observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepObservation {
    /// A commit id, with the `covered` record the same span stamped
    /// beside it, which projects it, and the step that journaled it, so
    /// decide can tell whether the terminal is that record's result (W1).
    Checked {
        head: CheckedHead,
        covered: Covered,
        step: Option<String>,
    },
    /// A record that journals one of `checked_head` and `covered` without
    /// the other. The protected phase's gate step records both, so decide
    /// refuses it there (W1); outside that phase a gate step stamps
    /// `covered` alone, and no decide reads the fold's observation.
    Partial,
    /// A record that is not one observation: a `checked_head` that is not
    /// a commit id, or a `covered` that does not read or does not project
    /// the record's own `checked_head`. It agrees with no head.
    Unreadable,
}

/// The observation `checkpoint` journals, if it is the engine's own step
/// marker and carries a checked head or a covered record: a driver's
/// record never is, and a work step's record carries neither. The
/// record's `covered` is kept, and one that disagrees with its own
/// `checked_head` makes the record no observation (fix round 0109a1r5,
/// V1); a record carrying one and not the other is partial (W1).
pub(super) fn of_checkpoint(checkpoint: &Value) -> Option<StepObservation> {
    (EngineMarker::authored(checkpoint) == Some(EngineMarker::StepFinished)).then_some(())?;
    let result = checkpoint.get("result")?;
    let step = checkpoint.get("step_name").and_then(Value::as_str);
    match (
        result.get(EngineWritten::CheckedHead.field()),
        result.get("covered"),
    ) {
        (None, None) => None,
        (Some(head), Some(covered)) => {
            Some(read(head, covered, step).unwrap_or(StepObservation::Unreadable))
        }
        (Some(_), None) | (None, Some(_)) => Some(StepObservation::Partial),
    }
}

/// `head` and `covered` as `step`'s one observation, if they read and
/// agree.
fn read(head: &Value, covered: &Value, step: Option<&str>) -> Option<StepObservation> {
    let head = CheckedHead::deserialize(head).ok()?;
    let covered = Covered::deserialize(covered).ok()?;
    covered
        .projects(head.as_str())
        .then(|| StepObservation::Checked {
            head,
            covered,
            step: step.map(str::to_string),
        })
}

#[cfg(test)]
mod tests;
