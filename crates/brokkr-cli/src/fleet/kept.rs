//! The fleet read KEPT between refreshes (#744). [`super::read_hearth`]
//! loads, verifies and folds every event of every run each time it is
//! asked, and holds them all while it does: on a journal of three
//! quarters of a million events that is seconds of work and gigabytes
//! of memory, for a console that asks every two seconds.
//!
//! A [`KeptFleet`] keeps each run's folded row instead, and the next
//! refresh asks the store only for what landed after the head that fold
//! stands at ([`Store::load_after`], the suffix read #354 built for the
//! engine). Nothing new is no events read; N new events are N read,
//! verified as the continuation of exactly that head, and carried onto
//! the state with [`fold_onto`], which is the whole-journal fold by
//! construction. Event bodies are not kept: only the rulings and
//! commands the residual findings are read from, so a supersede landing
//! later still marks a finding read earlier.
//!
//! **Fail closed to a whole read.** A run the suffix read cannot carry —
//! a gap, a head the journal no longer holds, a journal replaced beneath
//! the reader — is read whole again, exactly as [`super::read_hearth`]
//! reads it, and never shown as it was. A quarantined run has no fold to
//! carry, so it is read whole again whenever its journal's head moves.
//! What a kept run stands on is the store's own premise for the suffix
//! read: events are append-only by trigger, and the head is compared on
//! sequence and hash both.

use std::collections::HashMap;
use std::path::Path;

use brokkr_core::fold::{fold_onto, RunState};
use brokkr_core::{EventEnvelope, EventType};
use brokkr_store::{Store, StoreError};

use super::{acknowledged, did_not_fold, entry, read_run, Listing, Quarantine};

/// What the journal's `runs` table lists a run as: its id, feature and
/// creation stamp, in [`Store::list_runs`]'s order.
type Listed = (String, String, String);

/// One hearth's runs as the fleet keeps them between refreshes, in the
/// journal's listing order.
#[derive(Default)]
pub(crate) struct KeptFleet {
    runs: Vec<(Listed, KeptRun)>,
}

impl KeptFleet {
    /// Bring every listed run up to the journal, and say how many events
    /// that read: the work this refresh paid for, which grows with what
    /// landed since the last one and not with the journal. A refresh the
    /// store refuses is this refresh's refusal: a listing refused leaves
    /// the runs kept as they were, and a whole read refused leaves nothing
    /// kept, so the next refresh reads every run whole.
    pub(crate) fn refresh(&mut self, store: &Store) -> Result<usize, StoreError> {
        let listed = store.list_runs()?;
        let mut kept: HashMap<String, KeptRun> = self
            .runs
            .drain(..)
            .map(|((run_id, _, _), run)| (run_id, run))
            .collect();
        let carried: Vec<Option<(KeptRun, usize)>> = listed
            .iter()
            .map(|(run_id, _, _)| {
                let run = kept.remove(run_id)?;
                run.carried(store, run_id)
            })
            .collect();
        let due: Vec<&str> = listed
            .iter()
            .zip(&carried)
            .filter(|(_, carried)| carried.is_none())
            .map(|((run_id, _, _), _)| run_id.as_str())
            .collect();
        let mut whole = read_whole(store, &due)?.into_iter();
        let mut read = 0;
        for (listing, carried) in listed.into_iter().zip(carried) {
            let (run, events) = carried
                .or_else(|| whole.next())
                .expect("every run not carried was read whole, in listing order");
            read += events;
            self.runs.push((listing, run));
        }
        Ok(read)
    }

    /// The rows every fleet surface builds this hearth's listing from.
    pub(crate) fn entries(&self) -> Vec<brokkr_view::RunEntry<'_>> {
        self.runs
            .iter()
            .map(|((run_id, feature, created_at), run)| {
                let state = match &run.read {
                    Read::Folded(state) => Ok(&**state),
                    Read::Refused { quarantine, .. } => Err(quarantine),
                };
                entry(
                    Listing {
                        run_id,
                        feature,
                        created_at,
                    },
                    run.last_recorded_at.as_deref(),
                    state,
                    &run.residuals,
                )
            })
            .collect()
    }
}

/// One hearth of a many-hearth world read the kept way, once: its runs,
/// and the words of the refusal when its journal will not open or list.
/// Opened READ-ONLY, as [`super::read_journal`] opens it (decision 0026
/// ruling 5).
pub(crate) fn read_kept(journal: &Path) -> (KeptFleet, Option<String>) {
    let mut fleet = KeptFleet::default();
    let refused = Store::open_read_only(journal)
        .and_then(|store| fleet.refresh(&store))
        .err();
    (fleet, refused.map(|error| error.to_string()))
}

/// The most connections that read whole runs at once. A whole read is
/// parsing and verifying every event, bound by the processor and not the
/// disk, so the first refresh of a large journal spreads its runs across
/// readers of their own.
const READERS: usize = 8;

/// Runs `due` read whole, in their order, by up to [`READERS`] read-only
/// connections onto `store`'s journal. A connection that will not open is
/// the refresh's refusal.
fn read_whole(store: &Store, due: &[&str]) -> Result<Vec<(KeptRun, usize)>, StoreError> {
    let journal = store.path();
    let share = due.len().div_ceil(READERS).max(1);
    std::thread::scope(|scope| {
        let readers: Vec<_> = due
            .chunks(share)
            .map(|runs| {
                scope.spawn(move || -> Result<Vec<(KeptRun, usize)>, StoreError> {
                    let store = Store::open_read_only(journal)?;
                    Ok(runs
                        .iter()
                        .map(|run_id| KeptRun::whole(&store, run_id))
                        .collect())
                })
            })
            .collect();
        let mut read = Vec::with_capacity(due.len());
        for reader in readers {
            // A reader that panicked has said why already, on its own
            // thread: there is no fleet to show without its runs.
            let runs = reader.join().expect("a whole read panicked");
            read.extend(runs?);
        }
        Ok(read)
    })
}

/// One run as the fleet keeps it.
struct KeptRun {
    /// The last event's `recorded_at`: a checkpoint advances it like any
    /// other event, which is what the fleet dates a run by.
    last_recorded_at: Option<String>,
    read: Read,
    /// The events [`brokkr_view::residual_findings`] reads (see
    /// [`rules`]), and no others.
    rulings: Vec<EventEnvelope>,
    residuals: Vec<brokkr_view::ResidualFinding>,
}

/// How far a kept run has been read.
enum Read {
    /// Folded to the state's own head, `seq` and `last_hash`: the next
    /// refresh asks for what landed after exactly that.
    Folded(Box<RunState>),
    /// Refused, at the head the journal stood at when it was read — the
    /// head asked for before the load, so a journal that moved during it
    /// is read again. `None` when the store would not say: never matched,
    /// so the run is read whole at every refresh.
    Refused {
        quarantine: Quarantine,
        head: Option<(u64, String)>,
    },
}

impl KeptRun {
    /// Run `run_id` read whole, as [`super::read_hearth`] reads it, and
    /// how many events that read.
    fn whole(store: &Store, run_id: &str) -> (KeptRun, usize) {
        let head = store.head_hash(run_id).ok();
        let (events, state) = read_run(store, run_id);
        let run = KeptRun {
            last_recorded_at: events.last().map(|event| event.recorded_at.clone()),
            read: match state {
                Ok(state) => Read::Folded(Box::new(state)),
                Err(quarantine) => Read::Refused { quarantine, head },
            },
            rulings: Vec::new(),
            residuals: Vec::new(),
        };
        let read = events.len();
        (run.ruled_by(run_id, events), read)
    }

    /// The run carried up to the journal and how many events that read,
    /// or `None` when it must be read whole: a suffix the store will not
    /// chain onto this fold's head, or a refused run whose journal moved.
    fn carried(self, store: &Store, run_id: &str) -> Option<(KeptRun, usize)> {
        match self.read {
            Read::Folded(state) => {
                let suffix = store.load_after(run_id, state.seq, &state.last_hash).ok()?;
                let read = suffix.len();
                let last = suffix.last();
                let read_to = match fold_onto(*state, &suffix) {
                    Ok(state) => Read::Folded(Box::new(state)),
                    Err(error) => Read::Refused {
                        quarantine: did_not_fold(store, run_id, &error),
                        head: last.map(|event| (event.seq, event.event_hash.clone())),
                    },
                };
                let run = KeptRun {
                    read: read_to,
                    last_recorded_at: last
                        .map(|event| event.recorded_at.clone())
                        .or(self.last_recorded_at),
                    ..self
                };
                Some((run.ruled_by(run_id, suffix), read))
            }
            Read::Refused { quarantine, head } => match store.head_hash(run_id) {
                // Unmoved: still refused as it was, and the operator's
                // acknowledgement, which lives beside the chain, read
                // again.
                Ok(now) if head.as_ref() == Some(&now) => {
                    let quarantine = Quarantine {
                        acknowledgement: acknowledged(store, run_id),
                        ..quarantine
                    };
                    let read = Read::Refused { quarantine, head };
                    Some((KeptRun { read, ..self }, 0))
                }
                _ => None,
            },
        }
    }

    /// The run with what `events` adds to its rulings, its residual
    /// findings derived again when they added any.
    fn ruled_by(mut self, run_id: &str, events: Vec<EventEnvelope>) -> KeptRun {
        let before = self.rulings.len();
        self.rulings
            .extend(events.into_iter().filter(|event| rules(event.event_type)));
        if self.rulings.len() > before {
            self.residuals = brokkr_view::residual_findings(run_id, &self.rulings);
        }
        self
    }
}

/// Whether [`brokkr_view::residual_findings`] reads an event of this
/// type: the rulings its findings are read from, and the operator's
/// commands that supersede them. Matched exhaustively, so a new event
/// type is decided here rather than silently dropped; the kept tests hold
/// it to the derivation over a journal of every type.
pub(super) fn rules(event_type: EventType) -> bool {
    match event_type {
        EventType::TransitionDecided | EventType::OperatorCommanded => true,
        EventType::RunStarted
        | EventType::PhaseEntered
        | EventType::EffectRequested
        | EventType::EffectStarted
        | EventType::EffectCheckpointed
        | EventType::EffectSucceeded
        | EventType::EffectFailed
        | EventType::EffectIndeterminate
        | EventType::OperatorAccepted
        | EventType::OperatorRejected
        | EventType::RunParked
        | EventType::RunCompleted
        | EventType::RunStopped => false,
    }
}
