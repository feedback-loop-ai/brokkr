//! The engine's fenced append (decision 0029): what the engine decides on
//! a fold lands only on the head that fold was taken from.
//!
//! Every turn the engine folds the journal, decides, and appends. The
//! fold's tip is the fence [`tip_of`] arms, and each event the turn
//! appends moves it to that event, so a turn's events chain onto one
//! another and onto nothing a peer wrote in between. A peer's append —
//! a second engine on a fresh process, `conclude`, an operator's stop
//! between effects — takes the head away: nothing is written, and the
//! turn ends in [`EngineError::JournalMoved`] rather than folding again.
//!
//! An attempt's opening disarms it. From `effect/started` until the next
//! fold, what the engine appends is the open attempt's own record — its
//! checkpoints, its outcome, the markers of its members and steps — which
//! the process holding the attempt writes on the seat's report, not on a
//! fold, and which `fold` reads back after the one peer that may lawfully
//! write mid-attempt: an operator stop riding the attempt to its boundary.

use brokkr_core::envelope::EventType;
use brokkr_core::EventEnvelope;
use brokkr_store::{CooldownAppend, StoreError};
use serde_json::Value;

use super::{checkpoints, cooldown, Engine, EngineError};

#[cfg(test)]
pub(super) mod tests;

/// What a fold that read `events` lets the engine append on: the cause
/// its next event names, and the head that event must still find.
pub(super) fn tip_of(events: &[EventEnvelope]) -> (Option<String>, Option<(u64, String)>) {
    let tip = events.last();
    (
        tip.map(|event| event.event_id.clone()),
        tip.map(|event| (event.seq, event.event_hash.clone())),
    )
}

impl Engine {
    /// Append one event, onto the fenced head while one is armed. An
    /// attempt's settlement — its terminal event, and the checkpoints the
    /// engine journals with it once the seat has stopped — is given
    /// [`checkpoints::SETTLING_PATIENCES`] against a peer's lock, so the
    /// attempt's real outcome lands when the lock lets go in time; every
    /// other event gets one patience (#394). A terminal event the lock
    /// outlasts is held for the lawful end.
    ///
    /// A terminal event's cool-downs are derived from that event's own
    /// `waits` HERE, the one place that appends it, and the store writes
    /// them with it in one transaction (decision 0103 ruling 2(a), slice
    /// 4d1). The held outcome's payload is the same record, so its landing
    /// derives the same rows.
    pub(super) fn append_raw(
        &mut self,
        event_type: EventType,
        payload: Value,
        attempt_id: Option<String>,
    ) -> Result<EventEnvelope, EngineError> {
        let terminal = matches!(
            event_type,
            EventType::EffectSucceeded | EventType::EffectFailed | EventType::EffectIndeterminate
        );
        let patiences = if terminal || event_type == EventType::EffectCheckpointed {
            checkpoints::SETTLING_PATIENCES
        } else {
            1
        };
        let cooling = cooldown::cooldown_rows(event_type, &payload);
        let fence = self.fence.clone();
        #[cfg(test)]
        tests::peer_between();
        let appended = checkpoints::within_patiences(patiences, || {
            let (run_id, cause) = (&self.run_id, self.current_cause.clone());
            let event = CooldownAppend {
                run_id,
                fenced: fence.as_ref().map(|(seq, hash)| (*seq, hash.as_str())),
                event_type,
                payload: payload.clone(),
                causation_id: cause,
                attempt_id: attempt_id.clone(),
            };
            self.store.append_with_cooldowns(event, &cooling)
        });
        let envelope = match appended {
            Ok(envelope) => envelope,
            Err(contended) if terminal && contended.is_contention() => {
                self.held_outcome = Some(checkpoints::HeldOutcome {
                    event_type,
                    payload,
                    attempt_id,
                });
                return Err(contended.into());
            }
            Err(StoreError::HeadMoved {
                expected_seq,
                found_seq,
            }) => {
                return Err(EngineError::JournalMoved {
                    run_id: self.run_id.clone(),
                    expected_seq,
                    found_seq,
                })
            }
            Err(error) => return Err(error.into()),
        };
        self.current_cause = Some(envelope.event_id.clone());
        self.fence = fence
            .filter(|_| event_type != EventType::EffectStarted)
            .map(|_| (envelope.seq, envelope.event_hash.clone()));
        Ok(envelope)
    }
}
