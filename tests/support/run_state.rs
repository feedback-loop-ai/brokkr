//! The one place a test spells a `RunState` (#345), as `envelope.rs` is
//! for an envelope: a field the fold adds or reshapes is added here and
//! in the tests that assert it, and nowhere else. Each test binary
//! includes this file once through `#[path]`; it is test code by its
//! path, so it stays outside the coverage denominator.

use brokkr_core::fold::{Cursor, ParkReason, Run, RunState, Status};
use std::collections::BTreeMap;

/// Run `run_id` folded to `seq` and standing in `run`, in no phase and
/// with nothing else derived yet. A test sets only the fields its
/// assertion reads.
pub(crate) fn run_state(run_id: &str, seq: u64, run: Run) -> RunState {
    RunState {
        run_id: run_id.to_string(),
        seq,
        last_hash: "hash".to_string(),
        run,
        phase: None,
        consecutive_failures: BTreeMap::new(),
        visits: BTreeMap::new(),
        strategy: None,
        last_result: None,
        phase_results: BTreeMap::new(),
        reviewed_heads: None,
        last_decision: None,
        feature: None,
        pending_command: None,
        stop_accepted_seq: None,
    }
}

/// The run a fixture's `status` names: a running run between effects, at
/// `RequestEffect`, and a parked one waiting on `reason`.
pub(crate) fn standing(status: Status, reason: &str) -> Run {
    match status {
        Status::Running => Run::Running(Cursor::RequestEffect),
        Status::AwaitingOperator => Run::AwaitingOperator {
            reason: ParkReason::read(reason.to_string()),
        },
        Status::Completed => Run::Completed,
        Status::Stopped => Run::Stopped,
    }
}
