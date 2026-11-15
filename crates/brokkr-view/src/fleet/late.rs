//! A repaired run's late tail, as a surface paints it (decision 0108
//! ruling 4): kept and shown, never applied.

use brokkr_core::fold::RunState;
pub use brokkr_core::page::{LateEvent, LateRange, LateTail};

/// The sentence every surface prints for a tail: derived here once, as
/// the fleet's other facts are (decision 0071 ruling 7). The count and
/// the repairer are the only facts it needs, so a whole tail and a row's
/// compact range paint the same words without a trait between them.
pub fn late_line(count: usize, operator: &str) -> String {
    let events = if count == 1 { "event" } else { "events" };
    format!("a late tail of {count} {events}, repaired by {operator}")
}

/// The compact tail `state` carries, as a listing row paints it.
pub(crate) fn late_range(state: &RunState) -> Option<LateRange> {
    state.late_tail.as_ref().map(LateRange::of)
}

/// The whole tail `state` carries, as `inspect` lists it.
pub(crate) fn late_tail(state: &RunState) -> Option<LateTail> {
    state.late_tail.as_ref().map(LateTail::of)
}
