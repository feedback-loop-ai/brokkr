//! The pure step facts a sequence's settling reads: its driver's outcome
//! with its stderr tail, the step's own result vocabulary, the change a
//! dialect step expands, and a dialect step's outcome. Moved out of
//! `sequence.rs` mechanically so that file stays within ruling 4's 800-line
//! ceiling (decision 0103 slice 4c2b); no behaviour changed.

use brokkr_protocol::AttemptOutcome;
use serde_json::Value;

use super::super::{stderr_tail, DriverRun, Unproven};
use crate::bundle::{DialectExecution, SequenceStep};

/// A step driver's outcome, its failure or indeterminate reason with the
/// driver's stderr tail beside it: the tail rides on the attempt's
/// terminal event, as a single seat's does. It reads the settled outcome
/// (#403): a step whose tree is not proven over is indeterminate.
pub(super) fn tailed_outcome(report: &brokkr_protocol::AttemptReport) -> AttemptOutcome {
    let tail = stderr_tail(&report.stderr);
    match report.settled_outcome() {
        AttemptOutcome::Succeeded { result } => AttemptOutcome::Succeeded { result },
        AttemptOutcome::Failed { error } => AttemptOutcome::Failed {
            error: format!("{error}; stderr tail: {tail}"),
        },
        AttemptOutcome::Indeterminate { reason } => AttemptOutcome::Indeterminate {
            reason: format!("{reason}; stderr tail: {tail}"),
        },
    }
}

/// Whether a dialect step's argv or state names `{change}`.
pub(crate) fn needs_change(execution: &DialectExecution) -> bool {
    execution
        .argv
        .iter()
        .chain(execution.state.iter().flatten())
        .any(|token| token.contains("{change}"))
}

/// What is wrong with a step's result against its own vocabulary, if
/// anything.
pub(super) fn vocabulary_problem(step: &SequenceStep, result: &Value) -> Option<String> {
    match result.get("result").and_then(Value::as_str) {
        None => Some("has no 'result' string".to_string()),
        Some(word) if !step.results.iter().any(|allowed| allowed == word) => Some(format!(
            "reported '{word}', outside its declared results {:?}",
            step.results
        )),
        Some(_) => None,
    }
}

pub(crate) fn nearest_change(context: &Value) -> Option<String> {
    crate::phase_name::CHANGE_SOURCES.iter().find_map(|phase| {
        context
            .pointer(&format!("/results/{phase}/inputs/change"))
            .and_then(Value::as_str)
            .map(str::to_string)
    })
}

pub(crate) fn expand_dialect_argv(argv: &[String], change: &str) -> Vec<String> {
    argv.iter()
        .map(|token| token.replace("{change}", change))
        .collect()
}

/// A dialect step's outcome, and into `evidence` what its terminal event
/// carries when its tree is not proven over (#403).
pub(crate) fn dialect_attempt_outcome(run: DriverRun, evidence: &mut Unproven) -> AttemptOutcome {
    match run {
        DriverRun::SpawnFailed(error) => AttemptOutcome::Failed { error },
        DriverRun::Ran(report) => {
            *evidence = Unproven::seat(&report);
            report.settled_outcome()
        }
    }
}
