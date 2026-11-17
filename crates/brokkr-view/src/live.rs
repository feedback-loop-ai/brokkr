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

/// The state whose repair names a late tail: the one place this module
/// asks "does a repair withhold events?", so [`derived`], [`last_applied`]
/// and [`applied_events`] cannot differ on it. A caller with no fold, or a
/// fold no repair touches, has none.
fn repaired(state: Option<&RunState>) -> Option<&RunState> {
    state.filter(|state| state.late_tail.is_some())
}

/// The events `state`'s repaired tail leaves to a derived scan: the
/// journal whole where no repair names a tail, and the journal without
/// that tail where one does. The range test is `RunState::applies`, the
/// one home the fold and every readout share.
pub(crate) fn derived<'a>(events: &'a [EventEnvelope], state: Option<&RunState>) -> Applied<'a> {
    let Some(state) = repaired(state) else {
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

/// The last event the fold applied of `events`: [`derived`]'s last event,
/// found without copying the journal. This module is the one home of "the
/// applied trail" (the repair test is `repaired`, the range test
/// `RunState::applies`), and the wait readout reads it here instead of
/// restating it (decision 0071 ruling 5). A repaired tail is kept and
/// shown and never applied (decision 0108 ruling 4).
///
/// The one place it parts from [`derived`] is a caller with no fold at all.
/// `derived` hands the journal whole to the figures main already derives
/// from a refused fold; a wait is a claim about the applied trail, and a
/// fold that refused applied none, so `None` here paints no wait (decision
/// 0103 slice 4f).
pub(crate) fn last_applied<'a>(
    events: &'a [EventEnvelope],
    state: Option<&RunState>,
) -> Option<&'a EventEnvelope> {
    let state = state?;
    match repaired(Some(state)) {
        Some(state) => events.iter().rev().find(|event| state.applies(event.seq)),
        None => events.last(),
    }
}

/// The applied events from a journal the caller already owns, for a reader
/// outside this crate that derives a figure from them: the CLI's compare
/// and ledger scans. A journal no repair withholds is handed back without
/// a copy, and one a repair withholds is filtered in place, so a read
/// holds one journal and not two. The range test is `RunState::applies`,
/// the one home this scan shares with `derived`.
pub fn applied_events(
    mut events: Vec<EventEnvelope>,
    state: Option<&RunState>,
) -> Vec<EventEnvelope> {
    if let Some(state) = repaired(state) {
        events.retain(|event| state.applies(event.seq));
    }
    events
}
