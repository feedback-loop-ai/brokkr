//! The dispatcher (decision 0068, as decisions 0095 and 0110 amend it): the
//! one process a journal admits to start its queue's entries. Built here:
//! how it takes the journal (0095 ruling 2), by its lock and its holder row,
//! whose generation every step it takes is fenced by in the store; its
//! reconciliation of what a dispatcher left mid-start (0095 ruling 3); its
//! one pass under that lock (0110 ruling 1), which reconciles, sweeps, reads
//! the bound, judges and starts each admitted entry through its claim,
//! start, link and `spawn` row; one entry's start with the start's disposal
//! by variant (0110 ruling 2); in safe code, what the driver each run
//! is handed to needs (0110 rulings 3 and 4): the binary's identity, the
//! driver's log and the driver's own checks; and the standing loop that
//! repeats the pass while it stands, reaping the drivers it spawned
//! (0110 ruling 5), over seams for its clock, its stop and its drivers.
//! `brokkr dispatch`, its `drive`, `set-bound` and `status` are built in
//! brokkr-cli, which starts the driver's process and holds the signals.
//! Nothing here prints.

use std::path::PathBuf;

use brokkr_store::Generation;
use thiserror::Error;
use uuid::Uuid;

mod lock;
mod pass;
mod reconcile;
mod spawn;
mod standing;
mod start;

pub use lock::{dispatched, take_dispatch, Dispatcher};
pub use pass::{
    pass, BoundOutcome, Contended, Ended, Ending, Held, Left, Passed, Reclaimed, Seams, Spawning,
    Stage, Started, Subject, SweepOutcome, SweepRefused,
};
pub use reconcile::{reconcile, Settled};
pub use spawn::{driven, driver_log, driver_log_path, workspace, Binary, DriverError};
pub use standing::{stand, Exited, Reaped, Stand, Stopped, Tick};
pub use start::{ClaimRefused, Spawn, SpawnFailed, Spawned};

/// Why this process could not take a journal as its dispatcher, or, as
/// one, act in a store. Nothing was recorded, and a lock refused is not
/// held.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum DispatcherError {
    /// Another dispatcher holds the journal's lock: the one its latest
    /// holder row names, by its generation, its pid and when it took the
    /// journal, or none where it has not recorded itself yet.
    #[error(
        "dispatch: the journal {} is dispatched by {}; one dispatcher per journal (decision 0095)",
        .0.display(),
        named(.1.as_ref())
    )]
    Dispatched(PathBuf, Option<(Generation, u32, String)>),
    /// The journal's lock file is a link, of more than one link, or not
    /// this user's alone.
    #[error(
        "dispatch: the lock {} is not this user's own single file, so the journal was not \
         dispatched (decision 0095)",
        .0.display()
    )]
    Lock(PathBuf),
    /// A dispatcher handed a store whose journal is not the one it took,
    /// by the id each names itself by: the dispatcher's journal, then the
    /// store's, `None` for one with no id. Nothing of the queue was read,
    /// and nothing was written.
    #[error(
        "dispatch: this dispatcher took journal {}, and the store it was handed holds journal \
         {}; a dispatcher acts in the journal it took alone, so nothing was written (decision \
         0095)",
        id(.0.as_ref()),
        id(.1.as_ref())
    )]
    OtherJournal(Option<Uuid>, Option<Uuid>),
}

/// A journal as the id it names itself by.
fn id(journal: Option<&Uuid>) -> String {
    match journal {
        Some(id) => id.to_string(),
        None => "(no id)".into(),
    }
}

/// A dispatcher holding a lock, as its holder row names it. The lock
/// excludes only on this host, so the holder is on it.
fn named(holder: Option<&(Generation, u32, String)>) -> String {
    match holder {
        Some((generation, pid, since)) => format!(
            "generation {} (pid {pid} on this host since {since})",
            generation.0
        ),
        None => "a dispatcher that has not recorded itself yet".into(),
    }
}

#[cfg(test)]
mod tests;
