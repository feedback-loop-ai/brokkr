//! A repaired run's late tail, as a surface paints it (decision 0108
//! ruling 4): kept and shown, never applied.

use brokkr_core::fold::RunState;
pub use brokkr_core::page::{LateEvent, LateTail};

/// The sentence every surface prints for a tail: derived here once, as
/// the fleet's other facts are (decision 0071 ruling 7).
pub fn late_line(tail: &LateTail) -> String {
    let count = tail.events.len();
    let events = if count == 1 { "event" } else { "events" };
    format!(
        "a late tail of {count} {events}, repaired by {}",
        tail.operator
    )
}

/// The late tail `state` carries, as a surface paints it.
pub(crate) fn late_tail(state: &RunState) -> Option<LateTail> {
    state.late_tail.as_ref().map(LateTail::of)
}
