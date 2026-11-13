//! A run's residual findings as every surface shows them, read once.

use std::collections::BTreeMap;

use brokkr_core::fold::RunState;
use brokkr_core::residual::{closings_of, residuals_of, Mark};
use brokkr_core::summary::{in_late, Stated};
use brokkr_core::EventEnvelope;

use crate::{stated_finding, ResidualFinding};

/// Every residual finding a run's journal carries, with its supersede mark.
pub fn residual_findings(run_id: &str, events: &[EventEnvelope]) -> Vec<ResidualFinding> {
    residual_findings_at(run_id, events, None)
}

/// [`residual_findings`] with `state`, the run's fold, so a repaired run's
/// late tail closes no finding and states none: the same rule the fold and
/// the summary keep (decision 0108 ruling 4).
pub fn residual_findings_at(
    run_id: &str,
    events: &[EventEnvelope],
    state: Option<&RunState>,
) -> Vec<ResidualFinding> {
    let late = state
        .and_then(|state| state.late_tail.as_ref())
        .map(|tail| (tail.first_seq, tail.last_seq));
    let kept = |event: &&EventEnvelope| !in_late(late, event.seq);
    let marks: BTreeMap<u64, Mark> = closings_of(events.iter().filter(kept))
        .into_iter()
        .collect();
    residuals_of(events.iter().filter(kept))
        .iter()
        .map(|residual| {
            let superseded = marks.get(&residual.ruling.seq).cloned();
            stated_finding(run_id, Stated::of(residual, superseded))
        })
        .collect()
}
