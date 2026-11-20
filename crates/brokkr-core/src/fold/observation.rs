//! The open attempt's engine-authored step records, carried forward by
//! the fold (decision 0109 ruling 7, fix rounds 0109a1r4, K2, 0109a1r5,
//! V1, 0109a1r6, W1, and 0109a1r7, X1).
//!
//! Every gate step of a protected sequence, the one that ends it
//! included, journals the head its span accepted as `checked_head`, and
//! its `covered` where covering was armed, in the step's checkpoint,
//! which the engine authors. The attempt's terminal result journals the
//! effect's observation too. Both are durable, so the fold carries every
//! step record of the attempt to decide, by the step it names, and no
//! HEAD is read to settle them.
//!
//! What the attempt owes is not read here: decide reads it from the
//! compiled body, one whole record for each gate step that ran (X1), so a
//! record removed or blanked in the journal removes no obligation. The
//! fold only says what each step's record journaled: a whole observation,
//! none ([`StepObservation::Unobserved`], a work step's record or a
//! blanked one), a partial one, or one that does not read. A partial or
//! unreadable record is latched for the attempt
//! ([`StepRecords::malformed`]), so no later record heals it.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

use crate::findings::{CheckedHead, Covered, EngineMarker, EngineWritten};

/// What one engine-authored step record journaled as its observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepObservation {
    /// A commit id, with the `covered` record the same span stamped
    /// beside it, which projects it.
    Checked { head: CheckedHead, covered: Covered },
    /// A record that journals neither `checked_head` nor `covered`: a work
    /// step's, which took no observation. A gate step the compiled body
    /// holds owes more, so decide refuses it there (X1).
    Unobserved,
    /// A record that journals one of `checked_head` and `covered` without
    /// the other. The protected phase's gate step records both, so decide
    /// refuses it there (W1); outside that phase a gate step stamps
    /// `covered` alone, and no decide reads the fold's records.
    Partial,
    /// A record that is not one observation: a `checked_head` that is not
    /// a commit id, a `covered` that does not read or does not project the
    /// record's own `checked_head`, or a record naming no step or carrying
    /// no result. It agrees with no head.
    Unreadable,
}

impl StepObservation {
    /// Whether this record is one no later record may heal (X1).
    fn is_malformed(&self) -> bool {
        matches!(self, StepObservation::Partial | StepObservation::Unreadable)
    }
}

/// The in-flight attempt's engine-authored step records (fix round
/// 0109a1r7, X1): each step's latest record by the step it names, and the
/// attempt's first malformed record, latched.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StepRecords {
    /// Each step's latest record, by the step's name.
    pub by_step: BTreeMap<String, StepObservation>,
    /// The attempt's first partial or unreadable record. Once set, it
    /// stays for the attempt, whatever record follows it.
    pub malformed: Option<StepObservation>,
}

impl StepRecords {
    /// Keep `checkpoint` if it is the engine's own step marker: a
    /// driver's record never is.
    pub(super) fn record(&mut self, checkpoint: &Value) {
        if EngineMarker::authored(checkpoint) != Some(EngineMarker::StepFinished) {
            return;
        }
        let step = checkpoint.get("step_name").and_then(Value::as_str);
        let seen = match (step, checkpoint.get("result")) {
            (Some(_), Some(result)) => of_result(result),
            (None, _) | (_, None) => StepObservation::Unreadable,
        };
        // A step runs once in an attempt (a retry is a new attempt, and a
        // restarted one is never re-run), so a second record of it is no
        // one observation.
        let again = step.is_some_and(|step| self.by_step.contains_key(step));
        let malformed = match again {
            true => Some(StepObservation::Unreadable),
            false => Some(seen.clone()).filter(StepObservation::is_malformed),
        };
        if self.malformed.is_none() {
            self.malformed = malformed;
        }
        if let Some(step) = step {
            self.by_step.insert(step.to_string(), seen);
        }
    }
}

/// The observation a step record's `result` journals. Its `covered` is
/// kept, and one that disagrees with its own `checked_head` makes the
/// record unreadable (fix round 0109a1r5, V1); a record carrying one and
/// not the other is partial (W1).
fn of_result(result: &Value) -> StepObservation {
    match (
        result.get(EngineWritten::CheckedHead.field()),
        result.get("covered"),
    ) {
        (None, None) => StepObservation::Unobserved,
        (Some(head), Some(covered)) => read(head, covered).unwrap_or(StepObservation::Unreadable),
        (Some(_), None) | (None, Some(_)) => StepObservation::Partial,
    }
}

/// `head` and `covered` as one observation, if they read and agree.
fn read(head: &Value, covered: &Value) -> Option<StepObservation> {
    let head = CheckedHead::deserialize(head).ok()?;
    let covered = Covered::deserialize(covered).ok()?;
    covered
        .projects(head.as_str())
        .then_some(StepObservation::Checked { head, covered })
}

#[cfg(test)]
mod tests;
