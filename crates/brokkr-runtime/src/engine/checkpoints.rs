//! The live checkpoints of an attempt in flight, and what a peer's lock
//! on the shared journal may do to them (#394).
//!
//! A checkpoint is telemetry until the attempt's terminal event. Before
//! this buffer, a checkpoint append that outlasted the store's patience
//! ended the whole engine, the seat's process tree died with it, and the
//! next resume found an attempt it could only call indeterminate: the
//! seat's work so far was thrown away for a row that would have landed a
//! moment later. Now a contended row stays here, in the driver session's
//! buffer, and is retried — in order, with backoff — each time a seat
//! hands over another, so a lock wait delays the row and never the
//! attempt. Only once the seats have ended must every row land, because
//! the terminal event may not precede them; a lock that still holds then
//! goes up as the typed contention, and the engine settles the attempt
//! (see `Engine::lawful_end_under_contention`).

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use brokkr_core::envelope::EventType;
use brokkr_store::{SeatRecordError, Store, StoreError};
use serde_json::{json, Value};

/// The first wait after a contended try before a new row tries the
/// journal again; it doubles on every contention in a row and returns
/// here as soon as a row lands.
const BACKOFF_FLOOR: Duration = Duration::from_secs(1);

/// The longest a buffered row waits between tries while its seat works.
const BACKOFF_CEILING: Duration = Duration::from_secs(60);

/// The fenced-race `between` seam at the checkpoint append: called with
/// the engine's store immediately before every row tries the journal.
/// Production installs a no-op; a test installs a peer that takes and
/// releases the journal's write lock, so a lock held while a seat works
/// is proved on every run rather than whenever two threads interleave.
pub(crate) type Between = Box<dyn FnMut(&mut Store) + Send>;

/// The production seam: nothing happens between.
pub(super) fn unraced() -> Between {
    Box::new(|_| {})
}

/// A row the seat-record fence refused (decision 0034, ruling 6), with
/// the member tag it rode under.
pub(super) type Refusal = (String, SeatRecordError);

/// Why a buffered row did not land.
#[derive(Debug)]
enum Unlanded {
    /// The fence refused this row. It is dropped and, with it, the
    /// attempt.
    Refused(Refusal),
    /// Any other store error. The row and every row behind it stay
    /// buffered.
    Store(StoreError),
}

/// One attempt's checkpoints on their way to the journal, in the order
/// the seats handed them over.
pub(super) struct CheckpointJournal<'a> {
    run_id: &'a str,
    effect_id: &'a str,
    attempt_id: &'a str,
    /// Rows not yet journaled, each with the member tag it rode under
    /// (empty for a single site).
    pending: VecDeque<(String, Value)>,
    /// No row tries the journal before this instant while a seat works.
    retry_at: Option<Instant>,
    backoff: Duration,
    /// The first row that failed for any reason but contention. Once it
    /// is set no later row is journaled: the attempt is already lost,
    /// and its seats are left to run out so nothing of theirs is lost
    /// with it.
    stopped: Option<Unlanded>,
}

impl<'a> CheckpointJournal<'a> {
    pub(super) fn new(run_id: &'a str, effect_id: &'a str, attempt_id: &'a str) -> Self {
        CheckpointJournal {
            run_id,
            effect_id,
            attempt_id,
            pending: VecDeque::new(),
            retry_at: None,
            backoff: BACKOFF_FLOOR,
            stopped: None,
        }
    }

    /// Take one row while its seat is working. The row joins the buffer
    /// behind any already waiting; the buffer then tries the journal
    /// unless it is still backing off at `now`. Contention is not an
    /// ending here: the rows wait for the next try.
    pub(super) fn offer(
        &mut self,
        store: &mut Store,
        cause: &mut Option<String>,
        between: &mut Between,
        row: (String, Value),
        now: Instant,
    ) {
        if self.stopped.is_some() {
            return;
        }
        self.pending.push_back(row);
        if self.retry_at.is_some_and(|due| now < due) {
            return;
        }
        match self.flush(store, cause, between) {
            Ok(()) => {}
            Err(Unlanded::Store(error)) if error.is_contention() => {
                self.retry_at = Some(Instant::now() + self.backoff);
                self.backoff = (self.backoff * 2).min(BACKOFF_CEILING);
            }
            Err(stop) => self.stopped = Some(stop),
        }
    }

    /// The seats have ended: every buffered row lands now, because the
    /// attempt's conclusion cannot land ahead of them. A fence refusal
    /// comes back for the caller to make the attempt's outcome; any
    /// other store error — a lock that still outlasts the patience
    /// included — is returned as it came, with the rows still buffered.
    pub(super) fn finish(
        &mut self,
        store: &mut Store,
        cause: &mut Option<String>,
        between: &mut Between,
    ) -> Result<Option<Refusal>, StoreError> {
        let stop = match self.stopped.take() {
            Some(stop) => stop,
            None => match self.flush(store, cause, between) {
                Ok(()) => return Ok(None),
                Err(stop) => stop,
            },
        };
        match stop {
            Unlanded::Refused(refusal) => Ok(Some(refusal)),
            Unlanded::Store(error) => Err(error),
        }
    }

    /// Journal every buffered row, oldest first, advancing the causal
    /// chain through each: the closing effect event names the last
    /// checkpoint as its cause.
    fn flush(
        &mut self,
        store: &mut Store,
        cause: &mut Option<String>,
        between: &mut Between,
    ) -> Result<(), Unlanded> {
        while let Some((member, checkpoint)) = self.pending.front() {
            between(store);
            let appended = store.append_next(
                self.run_id,
                EventType::EffectCheckpointed,
                json!({
                    "effect_id": self.effect_id,
                    "attempt_id": self.attempt_id,
                    "checkpoint": checkpoint,
                }),
                cause.clone(),
                Some(self.attempt_id.to_string()),
            );
            let member = member.clone();
            match appended {
                Ok(envelope) => *cause = Some(envelope.event_id),
                Err(StoreError::SeatRecord(error)) => {
                    self.pending.pop_front();
                    return Err(Unlanded::Refused((member, error)));
                }
                Err(error) => return Err(Unlanded::Store(error)),
            }
            self.pending.pop_front();
        }
        self.retry_at = None;
        self.backoff = BACKOFF_FLOOR;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::tests::in_flight_store;
    use brokkr_core::fold::fold;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    const RUN: &str = "buffered";

    /// A run standing mid-attempt, so a checkpoint row folds, with a
    /// patience the tests can afford to spend.
    fn in_flight(path: &std::path::Path) -> Store {
        let mut store = in_flight_store(path, RUN);
        store.set_patience(Duration::from_millis(100)).unwrap();
        store
    }

    fn steps(store: &Store) -> Vec<Value> {
        store
            .load(RUN)
            .unwrap()
            .into_iter()
            .filter(|event| event.event_type == EventType::EffectCheckpointed)
            .map(|event| event.payload["checkpoint"]["step"].clone())
            .collect()
    }

    fn row(step: u32) -> (String, Value) {
        (String::new(), json!({"step": step.to_string()}))
    }

    /// A peer's lock taken on the journal and held until the connection
    /// rolls back.
    fn locked(path: &std::path::Path) -> rusqlite::Connection {
        let holder = rusqlite::Connection::open(path).unwrap();
        holder.execute_batch("BEGIN IMMEDIATE").unwrap();
        holder
            .execute(
                "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at)
                 VALUES ('peer', 'feat', 'self', '{}', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        holder
    }

    /// Rows offered under a held lock wait in order, a row offered while
    /// the buffer backs off does not even try the journal, and the rows
    /// land oldest first once the lock is gone — each chained to the one
    /// before it.
    #[test]
    fn contended_rows_wait_back_off_and_land_in_order_once_the_lock_is_gone() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.db");
        let mut store = in_flight(&path);
        let mut cause = None;
        let tries = Arc::new(AtomicUsize::new(0));
        let counted = tries.clone();
        let mut between: Between = Box::new(move |_| {
            counted.fetch_add(1, Ordering::SeqCst);
        });
        let mut journal = CheckpointJournal::new(RUN, "effect", "attempt");

        let holder = locked(&path);
        journal.offer(&mut store, &mut cause, &mut between, row(1), Instant::now());
        assert_eq!(journal.backoff, BACKOFF_FLOOR * 2);
        journal.offer(&mut store, &mut cause, &mut between, row(2), Instant::now());
        assert_eq!(tries.load(Ordering::SeqCst), 1, "a backing-off row tried");
        assert_eq!(journal.pending.len(), 2);

        // Due again, and still locked: the backoff doubles.
        let due = Instant::now() + BACKOFF_CEILING;
        journal.offer(&mut store, &mut cause, &mut between, row(3), due);
        assert_eq!(journal.backoff, BACKOFF_FLOOR * 4);
        assert_eq!(tries.load(Ordering::SeqCst), 2);
        assert!(journal.stopped.is_none(), "contention stopped the rows");
        assert_eq!(cause, None);
        assert!(steps(&store).is_empty());

        holder.execute_batch("ROLLBACK").unwrap();
        let due = Instant::now() + BACKOFF_CEILING;
        journal.offer(&mut store, &mut cause, &mut between, row(4), due);
        assert_eq!(steps(&store), vec!["1", "2", "3", "4"]);
        assert_eq!(journal.retry_at, None);
        assert_eq!(journal.backoff, BACKOFF_FLOOR);
        let events = store.load(RUN).unwrap();
        assert_eq!(cause.as_deref(), Some(events[7].event_id.as_str()));
        assert_eq!(
            events[7].causation_id.as_deref(),
            Some(events[6].event_id.as_str())
        );
        fold(&events).expect("the landed rows fold");
    }

    /// Once the seats have ended, a lock that outlasts the patience is
    /// the typed contention, and the rows are still buffered: nothing
    /// was written and nothing was lost.
    #[test]
    fn finishing_under_a_held_lock_returns_the_contention_and_keeps_the_rows() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.db");
        let mut store = in_flight(&path);
        let mut cause = None;
        let mut between: Between = Box::new(|_| {});
        let mut journal = CheckpointJournal::new(RUN, "effect", "attempt");

        let _holder = locked(&path);
        journal.offer(&mut store, &mut cause, &mut between, row(1), Instant::now());
        let contended = journal
            .finish(&mut store, &mut cause, &mut between)
            .expect_err("a held lock cannot be finished past");
        assert!(
            matches!(
                &contended,
                StoreError::Contended {
                    operation: "append",
                    ..
                }
            ),
            "{contended:?}"
        );
        assert_eq!(journal.pending.len(), 1);
    }

    /// The ceiling bounds the backoff however long the lock holds.
    #[test]
    fn the_backoff_never_passes_its_ceiling() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.db");
        let mut store = in_flight(&path);
        let mut cause = None;
        let mut between: Between = Box::new(|_| {});
        let mut journal = CheckpointJournal::new(RUN, "effect", "attempt");
        journal.backoff = BACKOFF_CEILING;

        let _holder = locked(&path);
        journal.offer(&mut store, &mut cause, &mut between, row(1), Instant::now());
        assert_eq!(journal.backoff, BACKOFF_CEILING);
    }
}
