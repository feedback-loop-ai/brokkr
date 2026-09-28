//! The checkpoints a seat streams while it works, journaled so that a
//! peer's lock on the shared journal delays a row and never ends the
//! attempt (#394).
//!
//! A checkpoint is telemetry until the attempt's terminal event. So when
//! an append meets [`StoreError::Contended`], which wrote nothing, the
//! checkpoint is held, in order, and the held rows are tried again, oldest
//! first, each time the seat hands over another one. No append made while
//! the seat works waits, not even the first: the sink runs on the thread
//! that reads the seat's pipe, and a reader stalled for a patience would
//! fill that pipe and hold the seat past its deadline, where the watchdog
//! kills it. A peer that has the lock costs the reader nothing, so the
//! seat is never told and never stopped: the lock delays the row, not the
//! work.
//!
//! Once the seat stops, what is still held gets its settlement, the
//! terminal event's treatment: [`SETTLING_PATIENCES`] of the store's
//! patiences. A checkpoint the lock outlasts even then is stranded, and
//! the attempt it belonged to is never reported as a success:
//! [`Settled::outcome`] makes it indeterminate, which parks for the
//! operator, and says how many rows were not journaled.
//!
//! The hold is bounded by [`HELD_BYTES`]. A checkpoint that meets a full
//! hold is not journaled either, and is counted the same way.

use std::collections::{BTreeMap, VecDeque};

use brokkr_core::envelope::EventType;
use brokkr_core::EventEnvelope;
use brokkr_protocol::{AttemptOutcome, AttemptReport};
use brokkr_store::{SeatRecordError, Store, StoreError};
use serde_json::{json, Value};

use super::{refused_outcome, EngineError};

/// The most serialized checkpoint bytes one attempt holds while a peer
/// keeps the journal's write lock. A checkpoint that finds the hold empty
/// is held whatever its size, so the hold never exceeds this by more than
/// that one row. Spilling past it to a file is #433's.
pub(super) const HELD_BYTES: usize = 16 * 1024 * 1024;

/// How many of the store's patiences an attempt's settlement gets — its
/// terminal event, the evidence journaled with it, and the checkpoints
/// still held when its seat stops — before a peer's lock is handed up.
/// The wait #394 measured ran 42 s against one 30 s patience.
pub(super) const SETTLING_PATIENCES: u32 = 3;

/// `append` tried again while a peer's lock outlasts each of `patiences`
/// patiences; anything else it returns, it returns at once.
pub(super) fn within_patiences<T>(
    patiences: u32,
    mut append: impl FnMut() -> Result<T, StoreError>,
) -> Result<T, StoreError> {
    let mut tries = 1;
    loop {
        match append() {
            Err(contended) if contended.is_contention() && tries < patiences => tries += 1,
            settled => return settled,
        }
    }
}

/// One of the store's two ways to append the next event: patiently, or
/// without waiting at all.
type Append = fn(
    &mut Store,
    &str,
    EventType,
    Value,
    Option<String>,
    Option<String>,
) -> Result<EventEnvelope, StoreError>;

/// An attempt's terminal event that a peer's lock kept out of the journal
/// through all [`SETTLING_PATIENCES`]. The engine carries it to its lawful
/// end, whose window is spent on this real outcome rather than on an
/// indeterminate naming the lock.
pub(super) struct HeldOutcome {
    pub(super) event_type: EventType,
    pub(super) payload: Value,
    pub(super) attempt_id: Option<String>,
}

/// The attempt a sink journals for.
pub(super) struct Attempt<'a> {
    pub(super) run_id: &'a str,
    pub(super) effect_id: &'a str,
    pub(super) attempt_id: &'a str,
}

/// One checkpoint waiting for the lock, with the site it came from: a
/// panel member's tag, or empty for a single driver.
struct Held {
    owner: String,
    checkpoint: Value,
    bytes: usize,
}

/// An attempt's checkpoint sink: the one writer of its live checkpoints.
///
/// `between` runs before every append the sink tries, which is the
/// instant a peer's lock matters; production passes a no-op, and the
/// contention tests take and release a peer's lock there (the same seam
/// the fenced operator appends expose).
pub(super) struct Checkpoints<'a, B: FnMut(&mut Store)> {
    store: &'a mut Store,
    cause: &'a mut Option<String>,
    attempt: Attempt<'a>,
    held: VecDeque<Held>,
    held_bytes: usize,
    limit: usize,
    between: B,
    /// Checkpoints that met a full hold, by site.
    lost: BTreeMap<String, usize>,
    /// Checkpoints still held when settlement ran out, by site.
    stranded: BTreeMap<String, usize>,
    /// A checkpoint the seat-record fence refused (decision 0034, ruling
    /// 6), and the site it rode under. No later checkpoint is journaled.
    refusal: Option<(String, SeatRecordError)>,
    /// A storage failure that is neither contention nor a refusal. No
    /// later checkpoint is journaled, and it ends the attempt as today.
    failure: Option<StoreError>,
}

/// What an attempt's checkpoints came to once its seat stopped.
pub(super) struct Settled {
    refusal: Option<(String, SeatRecordError)>,
    lost: BTreeMap<String, usize>,
    stranded: BTreeMap<String, usize>,
    limit: usize,
}

impl<'a> Checkpoints<'a, fn(&mut Store)> {
    pub(super) fn new(
        store: &'a mut Store,
        cause: &'a mut Option<String>,
        attempt: Attempt<'a>,
    ) -> Self {
        Checkpoints::holding(store, cause, attempt, HELD_BYTES, |_| {})
    }
}

impl<'a, B: FnMut(&mut Store)> Checkpoints<'a, B> {
    /// A sink whose hold is `limit` bytes, calling `between` before each
    /// append; `causation_id` chains through `cause`, so the terminal
    /// event names the last checkpoint that landed.
    pub(super) fn holding(
        store: &'a mut Store,
        cause: &'a mut Option<String>,
        attempt: Attempt<'a>,
        limit: usize,
        between: B,
    ) -> Self {
        Checkpoints {
            store,
            cause,
            attempt,
            held: VecDeque::new(),
            held_bytes: 0,
            limit,
            between,
            lost: BTreeMap::new(),
            stranded: BTreeMap::new(),
            refusal: None,
            failure: None,
        }
    }

    /// Take one checkpoint from a working seat. What is already held goes
    /// first; when that still meets the lock, this one joins the hold, or
    /// is counted lost when the hold is full. Nothing here waits on the
    /// lock and nothing here ever fails the seat.
    pub(super) fn offer(&mut self, owner: &str, checkpoint: Value) {
        let contended = self.flush(Store::append_next_without_waiting).is_some();
        if self.refusal.is_some() || self.failure.is_some() {
            return;
        }
        let bytes = checkpoint.to_string().len();
        if contended && self.held_bytes + bytes > self.limit {
            *self.lost.entry(owner.to_string()).or_default() += 1;
            return;
        }
        self.held_bytes += bytes;
        self.held.push_back(Held {
            owner: owner.to_string(),
            checkpoint,
            bytes,
        });
        if !contended {
            self.flush(Store::append_next_without_waiting);
        }
    }

    /// The seat has stopped: journal what is held, within
    /// [`SETTLING_PATIENCES`]. What the lock outlasts is stranded and
    /// counted against its site; a storage failure is handed up.
    pub(super) fn settle(mut self) -> Result<Settled, EngineError> {
        let flushed = within_patiences(SETTLING_PATIENCES, || {
            self.flush(Store::append_next).map_or(Ok(()), Err)
        });
        if flushed.is_err() {
            for held in self.held.drain(..) {
                *self.stranded.entry(held.owner).or_default() += 1;
            }
        }
        if let Some(failure) = self.failure {
            return Err(failure.into());
        }
        Ok(Settled {
            refusal: self.refusal,
            lost: self.lost,
            stranded: self.stranded,
            limit: self.limit,
        })
    }

    /// Append what is held, oldest first, until the hold is empty or a
    /// peer's lock outlasts what `append` waits, which is returned and
    /// leaves the rest held. A refusal or a failure latches and empties
    /// the hold.
    fn flush(&mut self, append: Append) -> Option<StoreError> {
        while let Some(next) = self.held.front() {
            (self.between)(&mut *self.store);
            let appended = append(
                &mut *self.store,
                self.attempt.run_id,
                EventType::EffectCheckpointed,
                json!({
                    "effect_id": self.attempt.effect_id,
                    "attempt_id": self.attempt.attempt_id,
                    "checkpoint": next.checkpoint,
                }),
                self.cause.clone(),
                Some(self.attempt.attempt_id.to_string()),
            );
            match appended {
                // Causal chain advances through checkpoints too — the
                // closing effect event names the last checkpoint as its
                // cause.
                Ok(envelope) => {
                    *self.cause = Some(envelope.event_id);
                    self.held_bytes -= next.bytes;
                    self.held.pop_front();
                }
                Err(contended) if contended.is_contention() => return Some(contended),
                Err(StoreError::SeatRecord(error)) => {
                    self.refusal = Some((next.owner.clone(), error));
                    self.held.clear();
                }
                Err(failure) => {
                    self.failure = Some(failure);
                    self.held.clear();
                }
            }
        }
        None
    }
}

impl Settled {
    /// The outcome of the site `owner` names, as its checkpoints leave
    /// it: indeterminate when any were not journaled, with the fence's
    /// refusal named beside the count when there was one; failed when
    /// the fence refused one of them; and the driver's own otherwise.
    pub(super) fn outcome(&self, owner: &str, outcome: AttemptOutcome) -> AttemptOutcome {
        let refusal = self
            .refusal
            .as_ref()
            .filter(|(refused, _)| refused == owner)
            .map(|(_, refusal)| refusal);
        let mut reasons = self.unjournaled(owner);
        if reasons.is_empty() {
            return match refusal {
                Some(refusal) => refused_outcome(outcome, refusal),
                None => outcome,
            };
        }
        reasons.extend(refusal.map(ToString::to_string));
        AttemptOutcome::Indeterminate {
            reason: reasons.join("; "),
        }
    }

    /// Carry what the checkpoints of the site `owner` names leave of
    /// `report`'s outcome (#403): the received outcome stays where the
    /// driver put it, and the one the engine acts on, when the checkpoints
    /// changed it, rides beside it as the report's refusal.
    pub(super) fn carry(&self, owner: &str, report: &mut AttemptReport) {
        let acted = self.outcome(owner, report.outcome.clone());
        if acted != report.outcome {
            report.refused = Some(acted);
        }
    }

    /// Why any of `owner`'s checkpoints were not journaled, one reason per
    /// cause.
    fn unjournaled(&self, owner: &str) -> Vec<String> {
        let lost = self.lost.get(owner).map(|lost| {
            format!(
                "{lost} checkpoint(s) were not journaled: a peer held the journal's write \
                 lock past this attempt's {}-byte checkpoint hold",
                self.limit
            )
        });
        let stranded = self.stranded.get(owner).map(|stranded| {
            format!(
                "{stranded} checkpoint(s) were not journaled: a peer still held the \
                 journal's write lock {SETTLING_PATIENCES} patiences after the seat stopped"
            )
        });
        lost.into_iter().chain(stranded).collect()
    }
}
