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
//! the reader, even one that holds every kept head — is read whole again,
//! exactly as [`super::read_hearth`] reads it, and never shown as it was.
//! A quarantined run has no fold to carry, so it is read whole again
//! whenever its journal's head moves, and at every refresh when what
//! refused it was timing rather than the journal's bytes ([`lasting`]).
//! What a kept run stands on is the store's own premise for the suffix
//! read: events are append-only by trigger, and the head is compared on
//! sequence and hash both. The suffix read re-reads no row below the
//! head, so the premise holds only of the file the runs were read from:
//! the kept fleet is bound to that file ([`Journal`]), stated before its
//! store is opened and again after the read, and a journal at the same
//! path that is not it drops every kept run, whatever heads its runs
//! still hold. The same file may still have been written beneath the
//! reader — a copy over it in place, which keeps the inode and may keep
//! the length, or a change committed into its write-ahead log, which
//! leaves the file itself untouched until a checkpoint — so every refresh
//! counts the rows below each kept head ([`recount`]), and a run short of
//! its head's seq is read whole.

use std::collections::{HashMap, HashSet};
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use brokkr_core::fold::{fold_onto, RunState};
use brokkr_core::{EventEnvelope, EventType};
use brokkr_store::{Store, StoreError};

use super::{acknowledged, did_not_fold, entry, judged, Listing, Quarantine};

/// What the journal's `runs` table lists a run as: its id, feature and
/// creation stamp, in [`Store::list_runs`]'s order.
type Listed = (String, String, String);

/// One hearth's runs as the fleet keeps them between refreshes, in the
/// journal's listing order.
#[derive(Default)]
pub(crate) struct KeptFleet {
    /// The journal file the runs were read from, as it stood before the
    /// last refresh that bound them opened its store; `None` before the
    /// first, and after a refresh whose file was another by the time its
    /// read was done.
    journal: Option<Journal>,
    runs: Vec<(Listed, KeptRun)>,
    /// The runs the last refresh read whole rather than carried.
    whole: HashSet<String>,
}

impl KeptFleet {
    /// Bring every listed run up to the journal, and say how many events
    /// that read: the work this refresh paid for, which grows with what
    /// landed since the last one and not with the journal. `before` is the
    /// journal file stated before `store` was opened onto it. A refresh
    /// the store refuses is this refresh's refusal: a listing refused
    /// leaves the runs kept as they were before it — none, when the
    /// journal was not the file they were kept from — and a count or a
    /// whole read refused leaves nothing kept, so the next refresh reads
    /// every run whole. A journal that is not the file the runs were kept
    /// from ([`Self::replaced`]) has every run read whole now; one that is
    /// has its kept runs counted ([`recount`]) at every refresh, whatever
    /// its file's clocks say, because a write the store commits lands in
    /// its write-ahead log and leaves the journal file as it was. The runs are bound to `before` only when the file is still it once
    /// the read is done: otherwise every run is read whole at the next
    /// refresh.
    pub(crate) fn refresh(
        &mut self,
        store: &Store,
        before: Option<Journal>,
    ) -> Result<usize, StoreError> {
        if !self.stands_on(before) {
            self.runs.clear();
        }
        let listed = store.list_runs()?;
        let mut kept: HashMap<String, KeptRun> = self
            .runs
            .drain(..)
            .map(|((run_id, _, _), run)| (run_id, run))
            .collect();
        recount(store, &mut kept)?;
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
        self.whole = due.into_iter().map(str::to_string).collect();
        let mut read = 0;
        for (listing, carried) in listed.into_iter().zip(carried) {
            let (run, events) = carried
                .or_else(|| whole.next())
                .expect("every run not carried was read whole, in listing order");
            read += events;
            self.runs.push((listing, run));
        }
        let after = Journal::at(store.path());
        self.journal = before.filter(|before| after.is_some_and(|after| before.holds(after)));
        Ok(read)
    }

    /// Whether runs are kept from a journal file that `now`, stated at the
    /// journal's path, is not: another file, by device and inode, or one
    /// shorter than it was, or one that cannot be stated, or runs whose
    /// file was another by the time their read was done. A fleet that has
    /// kept nothing was read from no file, and none replaced it.
    pub(crate) fn replaced(&self, now: Option<Journal>) -> bool {
        !self.runs.is_empty() && !self.stands_on(now)
    }

    /// Whether the last refresh read run `run_id` whole: a reader that
    /// keeps the run's events beside the fleet reads them whole with it.
    pub(crate) fn was_read_whole(&self, run_id: &str) -> bool {
        self.whole.contains(run_id)
    }

    /// Whether what the kept runs carry stands on the file `now`.
    fn stands_on(&self, now: Option<Journal>) -> bool {
        self.journal
            .zip(now)
            .is_some_and(|(kept, now)| kept.holds(now))
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
    let before = Journal::at(journal);
    let refused = Store::open_read_only(journal)
        .and_then(|store| fleet.refresh(&store, before))
        .err();
    (fleet, refused.map(|error| error.to_string()))
}

/// The journal file a kept fleet was read from: the file at the
/// journal's path by device and inode, and its length. A journal moved,
/// copied or created into place is another inode; one copied over the old
/// file in place keeps the inode, and is told by a length shorter than
/// the one read, because a journal the store keeps only grows (no writer
/// of it vacuums), or else by the count of its rows ([`recount`]).
#[derive(Clone, Copy)]
pub(crate) struct Journal {
    device: u64,
    inode: u64,
    length: u64,
}

impl Journal {
    /// The file at `path` now, or `None` when it cannot be stated, which
    /// no kept fleet stands on.
    pub(crate) fn at(path: &Path) -> Option<Journal> {
        let file = std::fs::metadata(path).ok()?;
        Some(Journal {
            device: file.dev(),
            inode: file.ino(),
            length: file.len(),
        })
    }

    /// Whether `now` is still this file, grown or not.
    fn holds(self, now: Journal) -> bool {
        (self.device, self.inode) == (now.device, now.inode) && now.length >= self.length
    }
}

/// Drop from `kept` every folded run whose journal no longer holds each
/// row up to its head: the rows at or below the head, counted, short of
/// its seq (a chain verified from seq 1 holds exactly that many). The
/// suffix read confirms only the head's own row, so such a run would be
/// carried on rows that are gone; dropped, it is read whole. A refused
/// run carries no fold, and is read whole when its head moves. The count
/// reads the key's index entries and no event, so it adds nothing to the
/// events a refresh reads.
fn recount(store: &Store, kept: &mut HashMap<String, KeptRun>) -> Result<(), StoreError> {
    let heads: Vec<(&str, u64)> = kept
        .iter()
        .filter_map(|(run_id, run)| match &run.read {
            Read::Folded(state) => Some((run_id.as_str(), state.seq)),
            Read::Refused { .. } => None,
        })
        .collect();
    let counts = store.rows_through(&heads)?;
    let short: Vec<String> = heads
        .into_iter()
        .zip(counts)
        .filter(|((_, seq), count)| count != seq)
        .map(|((run_id, _), _)| run_id.to_string())
        .collect();
    for run_id in short {
        kept.remove(&run_id);
    }
    Ok(())
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
        let loaded = store.load(run_id);
        // A refusal kept at a head is a refusal of the journal's bytes,
        // which the same head refuses again. One of timing — a peer's
        // lock, an I/O error — is this refresh's only: kept at no head,
        // so the next refresh reads the run whole again.
        let head = head.filter(|_| loaded.as_ref().err().is_none_or(lasting));
        let (events, state) = judged(store, run_id, loaded);
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

/// Whether a load refused with `error` was refused by the journal's own
/// bytes, which refuse it again for as long as its head stands. Anything
/// else — a peer's lock, SQLite's own error, a run the store cannot find
/// — may read cleanly next time, so it is never kept. Matched
/// exhaustively, so a new refusal is decided here: the tests read a
/// broken chain kept and a refusal of timing read again.
fn lasting(error: &StoreError) -> bool {
    match error {
        StoreError::Json(_) | StoreError::Chain(_) | StoreError::RowBeforeChain { .. } => true,
        StoreError::Sqlite(_)
        | StoreError::RunNotFound(_)
        | StoreError::RunExists(_)
        | StoreError::SeatRecord(_)
        | StoreError::SchemaMismatch { .. }
        | StoreError::AppendConflict { .. }
        | StoreError::HeadMoved { .. }
        | StoreError::UnknownHead { .. }
        | StoreError::Queue(_)
        | StoreError::AlreadyAcknowledged { .. }
        | StoreError::Contended { .. } => false,
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
