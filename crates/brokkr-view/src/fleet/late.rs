//! A repaired run's late tail, as a surface paints it (decision 0108
//! ruling 4): kept and shown, never applied.

use super::RunState;
use serde::Serialize;

/// A run's late tail, as an operator's repair record named it (decision
/// 0108 ruling 4): kept and shown, never applied.
#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct LateTail {
    /// The seq of the repair annotation that named it.
    pub repaired_at: u64,
    /// The run's terminal event, which the tail follows.
    pub terminal_seq: u64,
    /// The operator who repaired the run.
    pub operator: String,
    /// The tail's events, in order.
    pub events: Vec<LateEvent>,
}

/// One event of a run's late tail: its seq and its wire type.
#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct LateEvent {
    pub seq: u64,
    #[serde(rename = "type")]
    pub event_type: String,
}

impl LateTail {
    /// The sentence every surface prints for it: derived here once, as
    /// the fleet's other facts are (decision 0071 ruling 7).
    pub fn line(&self) -> String {
        let count = self.events.len();
        let events = if count == 1 { "event" } else { "events" };
        format!(
            "a late tail of {count} {events}, repaired by {}",
            self.operator
        )
    }
}

/// The late tail `state` carries, as a surface paints it.
pub(crate) fn late_tail(state: &RunState) -> Option<LateTail> {
    let tail = state.late_tail.as_ref()?;
    Some(LateTail {
        repaired_at: tail.repaired_at,
        terminal_seq: tail.terminal_seq,
        operator: tail.operator.clone(),
        events: tail
            .events()
            .map(|(seq, event_type)| LateEvent {
                seq,
                event_type: event_type.as_str().to_string(),
            })
            .collect(),
    })
}
