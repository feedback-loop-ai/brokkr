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
//!
//! The disarmed window is why the store, not the fence, holds the last
//! rule (#440): inside every append's transaction a non-annotation event
//! is refused once the run has reached a terminal event. A driver that was
//! still working when its run was concluded therefore meets
//! [`StoreError::AfterTerminal`] at its next write and ends in
//! [`EngineError::ConcludedBeneath`], writing nothing more and folding no
//! further, instead of leaving an event after `run/stopped`.

use brokkr_core::envelope::EventType;
use brokkr_core::park::ParkCause;
use brokkr_core::EventEnvelope;
use brokkr_store::{CooldownAppend, QueueRefusal, StoreError};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{checkpoints, cooldown, Engine, EngineError};

#[cfg(test)]
pub(super) mod tests;

/// The one field a settlement reads of the terminal it replaces, parsed
/// once into its type rather than read by key (fix round 0106b1r9, E5).
#[derive(Debug, serde::Deserialize)]
struct Settled {
    effect_id: String,
}

impl Settled {
    /// The effect `terminal` closes; one that names none is the engine's
    /// own broken invariant, for every terminal it builds names its effect.
    fn of(terminal: &Value) -> Result<Settled, EngineError> {
        serde::Deserialize::deserialize(terminal).map_err(|error| {
            EngineError::Invariant("append", format!("a terminal names no effect: {error}"))
        })
    }
}

/// The settlement of an attempt whose terminal the store refused for a wait
/// row it did not write (fix round 0106b1r6, D3): `effect/indeterminate`,
/// naming the refusal, carrying no wait, and marked `wait_unwritten`
/// (`provider-wait/v2`), so a replay that finds no park after it, the
/// park's append refused or the engine stopped between the two, parks with
/// the cause the live path gives (fix round 0106b1r10, H4). One type writes
/// it and reads it back.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct UnwrittenWait {
    effect_id: String,
    attempt_id: Option<String>,
    reason: String,
    wait_unwritten: bool,
}

/// The cause a replay parks with where `event` settles an unwritten wait:
/// the engine's own refusal, its detail the event's reason, as
/// [`cooldown::park_of`] gives it live. `None` for any other settlement.
pub(super) fn unwritten_cause(event: &EventEnvelope) -> Option<ParkCause> {
    let settled = UnwrittenWait::deserialize(&event.payload).ok()?;
    settled
        .wait_unwritten
        .then(|| ParkCause::ruled(None, settled.reason))
}

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
        let landed = match appended {
            Ok(landed) => landed,
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
            // The run was concluded beneath this driver: its report cannot
            // be journaled, and the conclusion already closed the attempt
            // indeterminate. End here, without retrying or folding again.
            Err(error @ StoreError::AfterTerminal { .. }) => {
                return Err(EngineError::concluded_beneath(error, attempt_id.as_deref()))
            }
            // A held or cooling row the store did not write rolled the
            // terminal back with it (fix round 0106b1r6, D3): no wait is
            // journaled that the store does not hold. Only a terminal carries
            // rows, so only a terminal meets this.
            Err(StoreError::Queue(refusal @ QueueRefusal::CooldownRowUnwritten(_))) => {
                let effect = Settled::of(&payload)?;
                return self.unwritten_wait(&effect.effect_id, attempt_id, refusal);
            }
            Err(error) => return Err(error.into()),
        };
        // A held wait this drive journaled ends the drive at its next turn
        // (decision 0106 ruling 2): a later drive meeting it before its end
        // is a resume, and refuses. Only a held row the store says it wrote
        // marks it, never the rows asked for: one whose end passed before
        // the event's stamp is not written (fix round 0106b1r10, H4).
        let envelope = landed.envelope;
        if terminal && landed.held_rows > 0 {
            self.cooldown.held = cooldown::HeldDrive::Written;
        }
        self.current_cause = Some(envelope.event_id.clone());
        self.fence = fence
            .filter(|_| event_type != EventType::EffectStarted)
            .map(|_| (envelope.seq, envelope.event_hash.clone()));
        Ok(envelope)
    }

    /// Settle the attempt whose terminal the store refused for a wait row
    /// it did not write: `effect/indeterminate`, naming the refusal and
    /// carrying no wait, so it never runs on as if the wait were held (fix
    /// round 0106b1r6, D3). The run then parks through the one
    /// [`cooldown::park_of`], as a probe round the table did not write
    /// parks: ruled, the engine's own refusal (fix round 0106b1r9, E4).
    fn unwritten_wait(
        &mut self,
        effect_id: &str,
        attempt_id: Option<String>,
        refusal: QueueRefusal,
    ) -> Result<EventEnvelope, EngineError> {
        let error = cooldown::CooldownError::WaitUnwritten(Box::new(refusal));
        let settled = UnwrittenWait {
            effect_id: effect_id.to_string(),
            attempt_id: attempt_id.clone(),
            reason: error.to_string(),
            wait_unwritten: true,
        };
        let envelope =
            self.append_raw(EventType::EffectIndeterminate, json!(settled), attempt_id)?;
        self.park(&cooldown::park_of(&error))?;
        Ok(envelope)
    }
}
