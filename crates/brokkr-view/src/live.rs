//! The events a derived scan reads: every event, less a repaired run's
//! late tail, which is kept and shown but never applied (decision 0108
//! ruling 4).

use brokkr_core::fold::RunState;
use brokkr_core::EventEnvelope;

/// The journal a derived scan reads.
pub(crate) enum Applied<'a> {
    /// No repair names a tail: the scan reads the journal as it stands.
    Whole(&'a [EventEnvelope]),
    /// A repaired tail is kept and shown, never applied.
    Without(Vec<EventEnvelope>),
}

impl Applied<'_> {
    /// The events to scan.
    pub(crate) fn as_slice(&self) -> &[EventEnvelope] {
        match self {
            Applied::Whole(events) => events,
            Applied::Without(events) => events,
        }
    }
}

/// The events `state`'s repaired tail leaves to a derived scan: the
/// journal whole where no repair names a tail, and the journal without
/// that tail where one does.
pub(crate) fn derived<'a>(events: &'a [EventEnvelope], state: Option<&RunState>) -> Applied<'a> {
    let late = state
        .and_then(|state| state.late_tail.as_ref())
        .map(|tail| (tail.first_seq, tail.last_seq));
    if late.is_none() {
        return Applied::Whole(events);
    }
    Applied::Without(
        events
            .iter()
            .filter(|event| !brokkr_core::summary::in_late(late, event.seq))
            .cloned()
            .collect(),
    )
}
