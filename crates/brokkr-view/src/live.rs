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
/// that tail where one does. The range test is `RunState::applies`, the
/// one home the fold and every readout share.
pub(crate) fn derived<'a>(events: &'a [EventEnvelope], state: Option<&RunState>) -> Applied<'a> {
    let Some(state) = state.filter(|state| state.late_tail.is_some()) else {
        return Applied::Whole(events);
    };
    Applied::Without(
        events
            .iter()
            .filter(|event| state.applies(event.seq))
            .cloned()
            .collect(),
    )
}

/// The applied events as owned values, for a reader outside this crate
/// that derives a figure from them: the CLI's compare and ledger scans.
pub fn applied_events(events: &[EventEnvelope], state: Option<&RunState>) -> Vec<EventEnvelope> {
    derived(events, state).as_slice().to_vec()
}
