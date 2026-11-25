//! The evaluation inputs the engine computes from the fold, moved here
//! whole from `fold.rs` so the fold stays within the house's file bound.

use serde_json::{Map, Value};

use super::{counts_as_failure, RunState};
use crate::policy::{CONSECUTIVE_FAILURES, STRATEGY};

/// The evaluation inputs the ENGINE computes from the journal (never
/// accepted from a seat): the consecutive-failure counter including the
/// failure being decided, matching the referee's counting, when the
/// consecutive-failure rule (`counts_as_failure`) counts `result`.
/// `counter_read` says whether the phase's rules read the counter.
pub fn computed_inputs(
    state: &RunState,
    phase: &str,
    result: &str,
    counter_read: bool,
) -> Map<String, Value> {
    let mut inputs = Map::new();
    if counts_as_failure(result, counter_read) {
        let prior = state.consecutive_failures.get(phase).copied().unwrap_or(0);
        inputs.insert(CONSECUTIVE_FAILURES.to_string(), Value::from(prior + 1));
    }
    if let Some(strategy) = &state.strategy {
        inputs.insert(STRATEGY.to_string(), Value::from(strategy.clone()));
    }
    inputs
}
