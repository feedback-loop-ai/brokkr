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
//! the terminal event may not precede them — the seats' rows and the
//! engine's own markers alike. That landing waits the lock out too, for
//! [`LANDING_TRIES`] tries on the same backoff, and only a lock that
//! outlasts all of them goes up as the typed contention, for the engine
//! to settle the attempt (see `Engine::lawful_end_under_contention`).

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use brokkr_core::envelope::EventType;
use brokkr_store::{SeatRecordError, Store, StoreError};
use serde_json::{json, Value};

/// The first wait after a contended try before the journal is tried
/// again; it doubles on every contention in a row and returns here as
/// soon as a row lands.
const BACKOFF_FLOOR: Duration = Duration::from_secs(1);

/// The longest wait between two tries.
const BACKOFF_CEILING: Duration = Duration::from_secs(60);

/// How many tries a row gets once its attempt's seats have ended, each a
/// whole store patience, with the backoff waited out between them:
/// about six minutes at the default patience, against the seat's work
/// that a give-up would throw away. The engine's settle of the attempt
/// spends the same.
pub(super) const LANDING_TRIES: usize = 8;

/// The wait after `pause`, doubling toward the ceiling.
fn doubled(pause: Duration) -> Duration {
    (pause * 2).min(BACKOFF_CEILING)
}

/// The waits between the [`LANDING_TRIES`] tries of a landing, in order.
pub(super) fn landing_pauses() -> impl Iterator<Item = Duration> {
    std::iter::successors(Some(BACKOFF_FLOOR), |pause| Some(doubled(*pause)))
        .take(LANDING_TRIES - 1)
}

/// The engine's two seams at the appends of an attempt in flight
/// (#394). Production races nothing and waits in real time; a test
/// takes a peer's write lock at the exact try it means and lets it go
/// while the engine waits, so a lock held past the patience is proved on
/// every run rather than whenever two threads interleave.
pub(super) struct Seams {
    /// The fenced-race `between` seam: called with the engine's store
    /// immediately before every buffered row tries the journal.
    pub(super) between: Box<dyn FnMut(&mut Store) + Send>,
    /// How a landing waits out a backoff pause.
    pub(super) wait: Box<dyn FnMut(Duration) + Send>,
}

impl Seams {
    /// Production: nothing happens between, and a pause is slept.
    pub(super) fn unraced() -> Self {
        Seams {
            between: Box::new(|_| {}),
            wait: Box::new(std::thread::sleep),
        }
    }
}

/// A row on its way to the journal, with the panel member tag it rode
/// under (`None` for a single site or an engine marker).
pub(super) type Row = (Option<String>, Value);

/// A row the seat-record fence refused (decision 0034, ruling 6), with
/// the member tag it rode under.
pub(super) type Refusal = (Option<String>, SeatRecordError);

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
    /// Rows not yet journaled, oldest first.
    pending: VecDeque<Row>,
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
        seams: &mut Seams,
        row: Row,
        now: Instant,
    ) {
        if self.stopped.is_some() {
            return;
        }
        self.pending.push_back(row);
        if self.retry_at.is_some_and(|due| now < due) {
            return;
        }
        match self.flush(store, cause, seams) {
            Ok(()) => {}
            Err(Unlanded::Store(error)) if error.is_contention() => {
                self.retry_at = Some(now + self.backoff);
                self.backoff = doubled(self.backoff);
            }
            Err(stop) => self.stopped = Some(stop),
        }
    }

    /// Take one row after the seats have ended, to land with the rest at
    /// [`CheckpointJournal::finish`]: the engine's own markers.
    pub(super) fn hold(&mut self, row: Row) {
        self.pending.push_back(row);
    }

    /// The seats have ended: every buffered row lands now, because the
    /// attempt's conclusion cannot land ahead of them. A peer's lock is
    /// waited out for [`LANDING_TRIES`] tries, [`landing_pauses`] apart.
    /// A fence refusal comes back for the caller to make the attempt's
    /// outcome; any other store error — a lock that outlasts every try
    /// included — is returned as it came, with the rows still buffered.
    pub(super) fn finish(
        &mut self,
        store: &mut Store,
        cause: &mut Option<String>,
        seams: &mut Seams,
    ) -> Result<Option<Refusal>, StoreError> {
        for pause in landing_pauses() {
            match self.land(store, cause, seams) {
                Err(Unlanded::Store(error)) if error.is_contention() => (seams.wait)(pause),
                landed => return finished(landed),
            }
        }
        finished(self.land(store, cause, seams))
    }

    /// One try at the whole buffer, unless an earlier row already
    /// stopped it.
    fn land(
        &mut self,
        store: &mut Store,
        cause: &mut Option<String>,
        seams: &mut Seams,
    ) -> Result<(), Unlanded> {
        match self.stopped.take() {
            Some(stop) => Err(stop),
            None => self.flush(store, cause, seams),
        }
    }

    /// Journal every buffered row, oldest first, advancing the causal
    /// chain through each: the closing effect event names the last
    /// checkpoint as its cause.
    fn flush(
        &mut self,
        store: &mut Store,
        cause: &mut Option<String>,
        seams: &mut Seams,
    ) -> Result<(), Unlanded> {
        while let Some((member, checkpoint)) = self.pending.front() {
            (seams.between)(store);
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

/// What a landing ended as, for the caller.
fn finished(landed: Result<(), Unlanded>) -> Result<Option<Refusal>, StoreError> {
    match landed {
        Ok(()) => Ok(None),
        Err(Unlanded::Refused(refusal)) => Ok(Some(refusal)),
        Err(Unlanded::Store(error)) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::tests::in_flight_store;
    use brokkr_core::fold::fold;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

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

    fn row(step: u32) -> Row {
        (None, json!({"step": step.to_string()}))
    }

    /// Seams that count the tries and record every pause, sleeping none
    /// of them; `on_pause` runs at each pause, after it is recorded.
    fn counting(
        on_pause: impl FnMut() + Send + 'static,
    ) -> (Seams, Arc<AtomicUsize>, Arc<Mutex<Vec<Duration>>>) {
        let tries = Arc::new(AtomicUsize::new(0));
        let pauses = Arc::new(Mutex::new(Vec::new()));
        let (counted, recorded) = (tries.clone(), pauses.clone());
        let mut on_pause = on_pause;
        let seams = Seams {
            between: Box::new(move |_| {
                counted.fetch_add(1, Ordering::SeqCst);
            }),
            wait: Box::new(move |pause| {
                recorded.lock().unwrap().push(pause);
                on_pause();
            }),
        };
        (seams, tries, pauses)
    }

    fn secs(pauses: &[u64]) -> Vec<Duration> {
        pauses.iter().copied().map(Duration::from_secs).collect()
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
        let (mut seams, tries, pauses) = counting(|| {});
        let mut journal = CheckpointJournal::new(RUN, "effect", "attempt");

        let holder = locked(&path);
        let first = Instant::now();
        journal.offer(&mut store, &mut cause, &mut seams, row(1), first);
        assert_eq!(journal.retry_at, Some(first + BACKOFF_FLOOR));
        assert_eq!(journal.backoff, BACKOFF_FLOOR * 2);
        journal.offer(&mut store, &mut cause, &mut seams, row(2), first);
        assert_eq!(tries.load(Ordering::SeqCst), 1, "a backing-off row tried");
        assert_eq!(journal.pending.len(), 2);

        // Due again, and still locked: the backoff doubles.
        let due = first + BACKOFF_CEILING;
        journal.offer(&mut store, &mut cause, &mut seams, row(3), due);
        assert_eq!(journal.retry_at, Some(due + BACKOFF_FLOOR * 2));
        assert_eq!(journal.backoff, BACKOFF_FLOOR * 4);
        assert_eq!(tries.load(Ordering::SeqCst), 2);
        assert!(journal.stopped.is_none(), "contention stopped the rows");
        assert_eq!(cause, None);
        assert!(steps(&store).is_empty());

        holder.execute_batch("ROLLBACK").unwrap();
        let due = due + BACKOFF_CEILING;
        journal.offer(&mut store, &mut cause, &mut seams, row(4), due);
        assert!(pauses.lock().unwrap().is_empty(), "a working seat slept");
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

    /// Once the seats have ended, a lock that lets go while the landing
    /// waits costs a pause and nothing else: the rows land, oldest
    /// first, and the finish reports no refusal.
    #[test]
    fn finishing_waits_a_held_lock_out_and_lands_every_row() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.db");
        let mut store = in_flight(&path);
        let mut cause = None;
        let mut holder = Some(locked(&path));
        let (mut seams, tries, pauses) = counting(move || {
            if let Some(peer) = holder.take() {
                peer.execute_batch("ROLLBACK").unwrap();
            }
        });
        let mut journal = CheckpointJournal::new(RUN, "effect", "attempt");

        journal.hold(row(1));
        journal.hold(row(2));
        let finished = journal.finish(&mut store, &mut cause, &mut seams);
        assert!(matches!(finished, Ok(None)), "{finished:?}");
        assert_eq!(*pauses.lock().unwrap(), secs(&[1]));
        assert_eq!(tries.load(Ordering::SeqCst), 3);
        assert_eq!(steps(&store), vec!["1", "2"]);
    }

    /// A lock that outlasts every landing try is the typed contention,
    /// after the whole backoff was waited out — doubling, and held at
    /// its ceiling — and the rows are still buffered: nothing was
    /// written and nothing was lost.
    #[test]
    fn finishing_under_a_lock_held_through_every_try_returns_the_contention_and_keeps_the_rows() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.db");
        let mut store = in_flight(&path);
        let mut cause = None;
        let (mut seams, tries, pauses) = counting(|| {});
        let mut journal = CheckpointJournal::new(RUN, "effect", "attempt");

        let _holder = locked(&path);
        journal.offer(&mut store, &mut cause, &mut seams, row(1), Instant::now());
        let contended = journal
            .finish(&mut store, &mut cause, &mut seams)
            .expect_err("a lock held through every try cannot be finished past");
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
        assert_eq!(*pauses.lock().unwrap(), secs(&[1, 2, 4, 8, 16, 32, 60]));
        assert_eq!(tries.load(Ordering::SeqCst), 1 + LANDING_TRIES);
        assert_eq!(journal.pending.len(), 1);
        assert!(steps(&store).is_empty());
    }
}
