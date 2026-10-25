//! The dispatcher (decision 0068, as decision 0095 amends it): the one
//! process a journal admits to start its queue's entries. Built so far:
//! how it takes the journal (ruling 2), by its lock and its holder row,
//! whose generation every step it takes is fenced by in the store. Its
//! reconciliation, its pass and the `brokkr dispatch` verb that runs them
//! are #430's later slices.

use std::path::PathBuf;

use brokkr_store::Generation;
use thiserror::Error;

mod lock;

pub use lock::{take_dispatch, Dispatcher};

/// Why this process could not take a journal as its dispatcher. Nothing
/// was recorded, and the lock is not held.
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
