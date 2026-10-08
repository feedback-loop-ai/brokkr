//! The checkpoints a seat streams while it works, journaled so that a
//! peer's lock on the shared journal delays a row and never ends the
//! attempt (#394).
//!
//! A checkpoint is telemetry until the attempt's terminal event. So when
//! an append meets [`StoreError::Contended`], which wrote nothing, the
//! checkpoint is held, in order, and the held rows are tried again, in one
//! transaction behind which the next checkpoint rides, each time the seat
//! hands over another one, and every [`RETRY_INTERVAL`] the seat stays
//! quiet (#464). No append made while the seat works waits, not even the
//! first. The sink runs beside the driver that drains the seat's pipe,
//! which hands it each checkpoint over a handoff of [`ARRIVALS`]: a sink
//! that falls behind backs the driver up, and so the pipe and the seat,
//! as a slow reader always has. A sink stalled for a patience would hold
//! the seat past its deadline that way, where the watchdog kills it. A
//! peer that has the lock costs the sink nothing, so the seat is never
//! told and never stopped: the lock delays the row, not the work.
//!
//! Once the seat stops, what is still held gets its settlement, the
//! terminal event's treatment: [`SETTLING_PATIENCES`] of the store's
//! patiences. A checkpoint the lock outlasts even then is stranded, and
//! the attempt it belonged to is never reported as a success:
//! [`Settled::outcome`] makes it indeterminate, which parks for the
//! operator, and says how many rows were not journaled.
//!
//! The hold is bounded by [`HELD_BYTES`]. A checkpoint that meets a full
//! hold is not journaled either, and is counted the same way. So is one
//! held behind a refused checkpoint, or offered after it, when it belongs
//! to another site than the refused one.

use std::collections::BTreeMap;
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender};
use std::time::Duration;

use brokkr_core::envelope::EventType;
use brokkr_protocol::{AttemptOutcome, AttemptReport};
use brokkr_store::{Chain, Rows, SeatRecordError, Store, StoreError};
use serde_json::{json, Value};

use super::capability_calls::{self, Stamp};
use super::EngineError;

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

/// How long a working seat may stay quiet before what is held is tried
/// again, without waiting, so a lock that lets go while the seat works
/// lands the hold then and not at the seat's next checkpoint or its stop.
const RETRY_INTERVAL: Duration = Duration::from_secs(1);

/// How many checkpoints a driver hands its sink before it waits for the
/// sink to take one. Only the hold, which [`HELD_BYTES`] bounds, keeps
/// what the sink has taken.
pub(super) const ARRIVALS: usize = 64;

/// The handoff from a site's driver, or a panel's members, to the one
/// sink that journals their checkpoints: bounded by [`ARRIVALS`].
pub(super) fn arrivals<T>() -> (SyncSender<T>, Receiver<T>) {
    std::sync::mpsc::sync_channel(ARRIVALS)
}

/// The capability-call attribution group (CC1; SC4's five fields) that
/// only the engine writes. `tool` is legacy telemetry and is not in it.
const ATTRIBUTION: [&str; 5] = [
    "capability",
    "dialect",
    "call_id",
    "call_state",
    "response_sha256",
];

/// `checkpoint` without any attribution field a driver supplied, so no
/// driver's value, forged or not, reaches the journal whatever a
/// seat-record version would admit. Everything else passes unchanged.
fn without_driver_attribution(checkpoint: Value) -> Value {
    match checkpoint {
        Value::Object(mut object) => {
            for field in ATTRIBUTION {
                object.remove(field);
            }
            Value::Object(object)
        }
        other => other,
    }
}

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

/// One of the store's two ways to append a chain of events in one
/// transaction: patiently, or without waiting at all.
type Append = fn(
    &mut Store,
    &str,
    EventType,
    Rows<'_, Held>,
    Option<String>,
    Option<String>,
) -> Result<Chain, StoreError>;

/// A checkpoint's size as the hold counts it: its serialized bytes.
fn serialized_len(checkpoint: &Value) -> usize {
    checkpoint.to_string().len()
}

/// An attempt's terminal event that a peer's lock kept out of the journal
/// through all [`SETTLING_PATIENCES`]. The engine carries it to its lawful
/// end, whose window is spent on this real outcome rather than on an
/// indeterminate naming the lock.
pub(super) struct HeldOutcome {
    pub(super) event_type: EventType,
    pub(super) payload: Value,
    pub(super) attempt_id: Option<String>,
}

/// Why a site's checkpoints stop being journaled: the seat-record fence
/// refused one (decision 0034, ruling 6), or the engine refused the call
/// one observed (CC1). Either fails the attempt in its own words.
#[derive(Debug, thiserror::Error)]
enum Refused {
    #[error(transparent)]
    Fence(SeatRecordError),
    #[error(transparent)]
    Call(capability_calls::Refusal),
}

/// The outcome of an attempt whose checkpoint the journal refused
/// (decision 0034, ruling 6), or whose observed call the engine refused
/// (CC1). A driver that went on to succeed did not: its account is
/// nonconforming, and the attempt fails on the refusal. A driver that
/// failed on its own keeps its own error beside the refusal. A driver
/// that was lost stays lost — indeterminate always parks (decision
/// 0006), and a refused checkpoint is no reason to retry a process whose
/// end nobody saw.
pub(super) fn refused_outcome(
    outcome: AttemptOutcome,
    refusal: &impl std::fmt::Display,
) -> AttemptOutcome {
    match outcome {
        AttemptOutcome::Succeeded { .. } => AttemptOutcome::Failed {
            error: refusal.to_string(),
        },
        AttemptOutcome::Failed { error } => AttemptOutcome::Failed {
            error: format!("{refusal}; the driver then failed: {error}"),
        },
        AttemptOutcome::Indeterminate { reason } => AttemptOutcome::Indeterminate { reason },
    }
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
}

/// The first refusal a sink met, and the site it fell on. No checkpoint
/// is journaled after it, and every one held behind it or offered after
/// it that belongs to another site is counted against that site, which
/// then never settles as its driver's success (#464). The refused site's
/// own are not counted: the refusal already fails it.
struct Latched {
    owner: String,
    refused: Refused,
    dropped: BTreeMap<String, usize>,
}

impl Latched {
    fn new(owner: &str, refused: Refused) -> Self {
        Latched {
            owner: owner.to_string(),
            refused,
            dropped: BTreeMap::new(),
        }
    }

    /// One checkpoint of `owner` that this refusal kept out of the journal.
    fn drop_behind(&mut self, owner: &str) {
        if owner != self.owner {
            *self.dropped.entry(owner.to_string()).or_default() += 1;
        }
    }
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
    held: Vec<Held>,
    /// The bytes of what is held, measured only once a second checkpoint
    /// joins the first (#464): an uncontended checkpoint, which finds the
    /// hold empty, is never serialized to be measured.
    held_bytes: usize,
    limit: usize,
    between: B,
    measure: fn(&Value) -> usize,
    /// How long the seat may stay quiet before what is held is tried
    /// again: [`RETRY_INTERVAL`].
    retry: Duration,
    /// Checkpoints that met a full hold, by site.
    lost: BTreeMap<String, usize>,
    /// Checkpoints still held when settlement ran out, by site.
    stranded: BTreeMap<String, usize>,
    /// A checkpoint the seat-record fence refused (decision 0034, ruling
    /// 6), or whose observed call the engine refused, and the site it rode
    /// under. No later checkpoint is journaled.
    refusal: Option<Latched>,
    /// A storage failure that is neither contention nor a refusal. No
    /// later checkpoint is journaled, and it ends the attempt as today.
    failure: Option<StoreError>,
}

/// What an attempt's checkpoints came to once its seat stopped.
pub(super) struct Settled {
    refusal: Option<Latched>,
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
            held: Vec::new(),
            held_bytes: 0,
            limit,
            between,
            measure: serialized_len,
            retry: RETRY_INTERVAL,
            lost: BTreeMap::new(),
            stranded: BTreeMap::new(),
            refusal: None,
            failure: None,
        }
    }

    /// This sink, measuring a checkpoint by `measure`: the tests count
    /// what the hold measures through it.
    #[cfg(test)]
    pub(super) fn measuring(self, measure: fn(&Value) -> usize) -> Self {
        Checkpoints { measure, ..self }
    }

    /// This sink, trying what is held again whenever the seat stays quiet
    /// for `retry`: the tests stage a quiet seat without waiting one out.
    #[cfg(test)]
    pub(super) fn retrying(self, retry: Duration) -> Self {
        Checkpoints { retry, ..self }
    }

    /// Run a single site's driver by `run` on a thread of its own, every
    /// checkpoint it forwards offered by `offer` here as it arrives, then
    /// settle: the report `run` returns, carrying what its checkpoints left
    /// of its outcome. The driver runs beside its sink so that what a
    /// peer's lock holds is tried again while the seat is quiet, and waits
    /// on the handoff while [`ARRIVALS`] checkpoints are still untaken.
    pub(super) fn drive(
        mut self,
        run: impl FnOnce(&mut dyn FnMut(&Value)) -> AttemptReport + Send,
        offer: impl FnMut(&mut Self, Value),
    ) -> Result<AttemptReport, EngineError> {
        let (sender, arrivals) = arrivals();
        let mut report = std::thread::scope(|scope| {
            // `arrivals` outlives this sender, so no send fails.
            let driver = scope.spawn(move || {
                run(&mut |data: &Value| {
                    let _ = sender.send(data.clone());
                })
            });
            self.take(&arrivals, offer);
            driver.join().expect("driver thread")
        });
        self.settle()?.carry("", &mut report);
        Ok(report)
    }

    /// Offer every checkpoint `arrivals` brings, by `offer`, until its
    /// last sender is gone. Whenever the seat stays quiet for the sink's
    /// retry interval, what is held is tried again without waiting.
    pub(super) fn take<T>(&mut self, arrivals: &Receiver<T>, mut offer: impl FnMut(&mut Self, T)) {
        loop {
            match arrivals.recv_timeout(self.retry) {
                Ok(arrival) => offer(self, arrival),
                Err(RecvTimeoutError::Timeout) => {
                    self.flush(Store::append_chain_without_waiting);
                }
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    /// Take one checkpoint from a working seat. It rides behind what is
    /// held, in one transaction; when that meets the lock, it joins the
    /// hold, or is counted lost when the hold is full. Nothing here waits
    /// on the lock and nothing here ever fails the seat. Both the
    /// single-site and the panel sink hand every driver checkpoint here,
    /// so this is where a driver's capability-call attribution is erased
    /// (CC1), and where `call`, the engine's own reading of the
    /// observation it took off the checkpoint, is written behind the
    /// erasure, or refuses it.
    pub(super) fn offer(
        &mut self,
        owner: &str,
        checkpoint: Value,
        call: Result<Option<Stamp>, capability_calls::Refusal>,
    ) {
        let mut checkpoint = without_driver_attribution(checkpoint);
        if self.failure.is_some() {
            return;
        }
        // No checkpoint is journaled after a refusal, and one offered after
        // it is counted against its site as one held behind it is (#464).
        // What was held before a refused call still lands, tried again on
        // every offer as on the quiet timer.
        if self.refusal.is_some() {
            self.flush(Store::append_chain_without_waiting);
        }
        if let Some(latched) = &mut self.refusal {
            latched.drop_behind(owner);
            return;
        }
        match call {
            Ok(Some(stamp)) => stamp.apply(&mut checkpoint),
            Ok(None) => {}
            // What is held came before the refused call, and still lands.
            Err(refusal) => {
                self.refusal = Some(Latched::new(owner, Refused::Call(refusal)));
                return;
            }
        }
        self.hold(owner, checkpoint);
    }

    /// Journal `checkpoint` behind what is held, in one transaction. A
    /// checkpoint that finds the hold empty is never measured; one that
    /// joins a hold measures it, and is lost when the lock keeps both and
    /// the hold would pass its limit.
    fn hold(&mut self, owner: &str, checkpoint: Value) {
        let bytes = if self.held.is_empty() {
            0
        } else {
            // The first held checkpoint is measured once another joins it.
            if self.held.len() == 1 {
                self.held_bytes = (self.measure)(&self.held[0].checkpoint);
            }
            (self.measure)(&checkpoint)
        };
        self.held.push(Held {
            owner: owner.to_string(),
            checkpoint,
        });
        self.held_bytes += bytes;
        let contended = self.flush(Store::append_chain_without_waiting).is_some();
        if contended && self.held_bytes > self.limit {
            self.held.pop();
            self.held_bytes -= bytes;
            *self.lost.entry(owner.to_string()).or_default() += 1;
        }
    }

    /// The seat has stopped: journal what is held, within
    /// [`SETTLING_PATIENCES`]. What the lock outlasts is stranded and
    /// counted against its site; a storage failure is handed up.
    pub(super) fn settle(mut self) -> Result<Settled, EngineError> {
        let flushed = within_patiences(SETTLING_PATIENCES, || {
            self.flush(Store::append_chain).map_or(Ok(()), Err)
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

    /// Append everything held, oldest first, in one transaction. A peer's
    /// lock that outlasts what `append` waits is returned, and leaves the
    /// hold as it was. Otherwise the hold empties: what the fence refused
    /// latches, with what it kept out counted behind it, and a failure
    /// latches as itself.
    fn flush(&mut self, append: Append) -> Option<StoreError> {
        if self.held.is_empty() {
            return None;
        }
        (self.between)(&mut *self.store);
        // Each row is built inside the transaction that holds the lock, so
        // an attempt the lock refuses copies nothing of the hold (#464).
        let (effect_id, attempt_id) = (self.attempt.effect_id, self.attempt.attempt_id);
        let row = |held: &Held| {
            json!({
                "effect_id": effect_id,
                "attempt_id": attempt_id,
                "checkpoint": held.checkpoint,
            })
        };
        let rows = Rows {
            rows: &self.held,
            payload: &row,
        };
        let appended = append(
            &mut *self.store,
            self.attempt.run_id,
            EventType::EffectCheckpointed,
            rows,
            self.cause.clone(),
            Some(self.attempt.attempt_id.to_string()),
        );
        match appended {
            Ok(chain) => self.landed(chain),
            Err(contended) if contended.is_contention() => return Some(contended),
            Err(failure) => self.failure = Some(failure),
        }
        self.held.clear();
        self.held_bytes = 0;
        None
    }

    /// What one chain of the hold came to. The causal chain advances
    /// through checkpoints too — the closing effect event names the last
    /// checkpoint as its cause. The row after the last that landed is the
    /// one the fence refused, and those behind it are dropped with it.
    fn landed(&mut self, chain: Chain) {
        let landed = chain.appended.len();
        if let Some(last) = chain.appended.last() {
            *self.cause = Some(last.event_id.clone());
        }
        let Some(refusal) = chain.refused else {
            return;
        };
        let owner = &self.held[landed].owner;
        let latched = self
            .refusal
            .get_or_insert_with(|| Latched::new(owner, Refused::Fence(refusal)));
        for held in &self.held[landed..] {
            latched.drop_behind(&held.owner);
        }
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
            .filter(|latched| latched.owner == owner)
            .map(|latched| &latched.refused);
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
        let dropped = self.refusal.as_ref().and_then(|latched| {
            let dropped = latched.dropped.get(owner)?;
            Some(format!(
                "{dropped} checkpoint(s) were not journaled: they came after the \
                 refused checkpoint of '{}'",
                latched.owner
            ))
        });
        lost.into_iter().chain(stranded).chain(dropped).collect()
    }
}

#[cfg(test)]
mod tests;
