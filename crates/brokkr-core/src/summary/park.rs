//! A park as the journal records it (decision 0089 ruling 7): the phase it
//! was entered from, its start, the operator's answer and the run's next
//! move. Read here once, from the run's events, so the writer keeps on the
//! park's `event_index` row what `brokkr analytics` derives from the run
//! read whole (decision 0071 ruling 5).
//!
//! A park waits on the operator until their next command, and stays parked
//! until the run moves again. An operator event is absorbed while it waits:
//! the first command answers it and none of them ends it. The run's next
//! event of any other kind ends it, at that event's time, and a later park
//! opens with the phase the run had then. Only events whose stamp parses
//! are read, as every reader of a run's times reads them.

use crate::envelope::{EventEnvelope, EventType};
use crate::stamp::parse_millis;

/// One park: the `run/parked` event that opened it and the two moments
/// later events give it. A moment is absent while it has not happened: an
/// operator has not answered, or the run has not moved again and the park
/// is still open, which a reader reads as its own `now`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Park {
    /// The seq of the `run/parked` event.
    pub seq: u64,
    /// The phase the run was in when it parked.
    pub phase: Option<String>,
    /// The park's own time.
    pub at: i64,
    /// The first operator command after the park, while it waited.
    pub answered: Option<i64>,
    /// The run's next move after the park.
    pub ended: Option<i64>,
}

impl Park {
    /// The bit length of the park's reach, `max(answered, ended) - at`, 0
    /// where that is not positive, or 64 while the park is open: the class
    /// a window reads the parks that reach it by (ruling 7).
    pub fn span_class(&self) -> u8 {
        let Some(ended) = self.ended else {
            return 64;
        };
        let reach = self
            .answered
            .unwrap_or(ended)
            .max(ended)
            .saturating_sub(self.at);
        if reach <= 0 {
            return 0;
        }
        (u64::BITS - (reach as u64).leading_zeros()) as u8
    }
}

/// Whether `event_type` is one of an operator's answers, which a park
/// absorbs rather than ends.
fn is_operator(event_type: EventType) -> bool {
    matches!(
        event_type,
        EventType::OperatorCommanded | EventType::OperatorAccepted | EventType::OperatorRejected
    )
}

/// The parks read so far, carried event by event: the run's phase, the
/// park it has open, and those it has ended.
#[derive(Debug, Clone, Default)]
pub(crate) struct Parking {
    phase: Option<String>,
    open: Option<Park>,
    done: Vec<Park>,
    /// The seq of every `run/parked` event read, an ignored one included:
    /// its stamp not parsing opens no park, but its row was still read and
    /// the writer records it processed.
    rows: Vec<u64>,
}

impl Parking {
    /// The reading carried over `event`, the run's next.
    pub(crate) fn note(&mut self, event: &EventEnvelope) {
        if event.event_type == EventType::RunParked {
            self.rows.push(event.seq);
        }
        let Some(at) = parse_millis(&event.recorded_at) else {
            return;
        };
        if let Some(park) = self.open.as_mut() {
            if is_operator(event.event_type) {
                if event.event_type == EventType::OperatorCommanded && park.answered.is_none() {
                    park.answered = Some(at);
                }
                return;
            }
            park.ended = Some(at);
            park.answered = Some(park.answered.unwrap_or(at));
            let ended = self.open.take().expect("a park is open");
            self.done.push(ended);
        }
        match event.event_type {
            EventType::PhaseEntered => {
                self.phase = crate::payload::PhaseEntered::read(event)
                    .ok()
                    .map(|entered| entered.phase.to_string());
            }
            EventType::RunParked => {
                self.open = Some(Park {
                    seq: event.seq,
                    phase: self.phase.clone(),
                    at,
                    answered: None,
                    ended: None,
                });
            }
            _ => {}
        }
    }

    /// Every park read so far, in the order they opened.
    pub(crate) fn parks(&self) -> Vec<Park> {
        let mut parks = self.done.clone();
        parks.extend(self.open.clone());
        parks
    }

    /// The seq of every `run/parked` event read so far, in order, an
    /// ignored one included.
    pub(crate) fn rows(&self) -> &[u64] {
        &self.rows
    }
}

/// Every park of the run `events` hold, in order from its first.
pub fn parks(events: &[EventEnvelope]) -> Vec<Park> {
    let mut parking = Parking::default();
    for event in events {
        parking.note(event);
    }
    parking.parks()
}

#[cfg(test)]
mod tests;
