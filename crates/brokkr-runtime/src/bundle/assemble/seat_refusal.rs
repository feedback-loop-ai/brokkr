//! A seat's phase, its results and the dialect wrapper's verify body.
//!
//! The checks refuse in the one cause [`SeatRefusal`], one variant per
//! check, so a test can name the check that fired even where two checks
//! share their words. Its `Display` is the operator's text byte for byte;
//! [`refused`] folds it into the compile refusal, the one place it becomes
//! text (#353).

use brokkr_core::policy::Machine;
use serde_json::Value;

use super::super::{CompileError, ExecutableBody, Invocation, SeatBody};
use crate::phase_name;

/// The one sentence both wrapper checks print.
const WRAPPED: &str = "dialect verify currently requires a single or panel verify seat";

/// Why a seat, or the verify body the dialect wraps, was refused, each
/// field borrowed from the phase name or the seat's results.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum SeatRefusal<'a> {
    /// The seat names a phase the machine lacks.
    #[error("seat '{phase}' names a phase the policy does not have")]
    UnknownPhase { phase: &'a str },
    /// The seat's `results` are absent, not an array, or hold no string.
    #[error("seat '{phase}' needs non-empty 'results'")]
    NoResults { phase: &'a str },
    /// A result no rule from the seat's phase covers.
    #[error(
        "seat '{phase}' may emit '{result}' but no rule covers it; \
         result variants without an outer rule are rejected"
    )]
    Uncovered { phase: &'a str, result: &'a str },
    /// The verify body selects nothing by default, or selects a sequence.
    #[error("{WRAPPED}")]
    WrappedUnselected,
    /// The verify body is not itself an invocation.
    #[error("{WRAPPED}")]
    WrappedNotInvocation,
}

/// The compile refusal a seat refusal reads as: its own text.
pub(super) fn refused(refusal: SeatRefusal<'_>) -> CompileError {
    CompileError::Invalid(refusal.to_string())
}

/// The seat's phase is one the machine has.
pub(super) fn known_phase<'a>(machine: &Machine, phase: &'a str) -> Result<(), SeatRefusal<'a>> {
    if machine.phases.iter().any(|known| known == phase) {
        Ok(())
    } else {
        Err(SeatRefusal::UnknownPhase { phase })
    }
}

/// The seat's string `results`, its non-string elements dropped, refused
/// when none is left.
pub(super) fn declared_results<'a>(
    phase: &'a str,
    raw: &Value,
) -> Result<Vec<String>, SeatRefusal<'a>> {
    raw.get("results")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|r| !r.is_empty())
        .ok_or(SeatRefusal::NoResults { phase })
}

/// Every result covered by a rule from its phase, the first that is not
/// refused.
pub(super) fn covered_results<'a>(
    machine: &Machine,
    phase: &'a str,
    results: &'a [String],
) -> Result<(), SeatRefusal<'a>> {
    for result in results {
        let covered = machine
            .rules
            .iter()
            .any(|rule| rule.from == *phase && rule.result == *result);
        // Decision 0041 reserves the implementer's `oversized`
        // verdict before ruling 6 seats triage. Until that phase
        // exists there can be no edge to it; the unmatched verdict
        // parks fail-closed. Once triage exists, ordinary coverage
        // is mandatory and the roster test pins both bounded arms.
        let reserved_oversized = phase == phase_name::IMPLEMENT
            && result == "oversized"
            && !machine
                .phases
                .iter()
                .any(|known| known == phase_name::TRIAGE);
        if !covered && !reserved_oversized {
            return Err(SeatRefusal::Uncovered { phase, result });
        }
    }
    Ok(())
}

/// The verify body the dialect wraps: what it selects by default, which
/// must not be a sequence, and the invocation it must itself be.
pub(super) fn wrapped_body(
    body: &SeatBody,
) -> Result<(ExecutableBody<'_>, &Invocation), SeatRefusal<'static>> {
    let executable = match body.selected(None) {
        Some((executable, _)) if !matches!(executable, ExecutableBody::Sequence { .. }) => {
            executable
        }
        _ => return Err(SeatRefusal::WrappedUnselected),
    };
    match body {
        SeatBody::Invocation(invocation) => Ok((executable, invocation)),
        _ => Err(SeatRefusal::WrappedNotInvocation),
    }
}

#[cfg(test)]
mod tests;
