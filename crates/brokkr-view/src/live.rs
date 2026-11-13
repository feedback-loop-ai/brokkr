//! The events a derived scan reads: every event, less a repaired run's
//! late tail, which is kept and shown but never applied (decision 0108
//! ruling 4).

use std::borrow::Cow;

use brokkr_core::fold::RunState;
use brokkr_core::EventEnvelope;

/// A run no repair names borrows its slice unchanged.
pub(crate) fn derived<'a>(
    events: &'a [EventEnvelope],
    state: Option<&RunState>,
) -> Cow<'a, [EventEnvelope]> {
    let late = state
        .and_then(|state| state.late_tail.as_ref())
        .map(|tail| (tail.first_seq, tail.last_seq));
    if late.is_none() {
        return Cow::Borrowed(events);
    }
    Cow::Owned(
        events
            .iter()
            .filter(|event| !brokkr_core::summary::in_late(late, event.seq))
            .cloned()
            .collect(),
    )
}
