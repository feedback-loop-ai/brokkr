//! A repaired run's late tail, as a surface paints it (decision 0108
//! ruling 4): kept and shown, never applied.

use brokkr_core::fold::RunState;
pub use brokkr_core::page::{LateEvent, LateRange, LateTail};

/// What the one sentence needs of a tail: its event count and its
/// repairer. The whole tail and the summary row's compact range both
/// state them, so both paint the same words.
pub trait LateLine {
    fn late_count(&self) -> usize;
    fn late_operator(&self) -> &str;
}

impl LateLine for LateTail {
    fn late_count(&self) -> usize {
        self.count()
    }

    fn late_operator(&self) -> &str {
        &self.operator
    }
}

impl LateLine for LateRange {
    fn late_count(&self) -> usize {
        usize::try_from(self.count).unwrap_or(usize::MAX)
    }

    fn late_operator(&self) -> &str {
        &self.operator
    }
}

/// The sentence every surface prints for a tail: derived here once, as
/// the fleet's other facts are (decision 0071 ruling 7).
pub fn late_line(tail: &impl LateLine) -> String {
    let count = tail.late_count();
    let events = if count == 1 { "event" } else { "events" };
    format!(
        "a late tail of {count} {events}, repaired by {}",
        tail.late_operator()
    )
}

/// The compact tail `state` carries, as a listing row paints it.
pub(crate) fn late_range(state: &RunState) -> Option<LateRange> {
    state.late_tail.as_ref().map(LateRange::of)
}

/// The whole tail `state` carries, as `inspect` lists it.
pub(crate) fn late_tail(state: &RunState) -> Option<LateTail> {
    state.late_tail.as_ref().map(LateTail::of)
}
