//! The journal's dispatch lock (decision 0095 ruling 2): an exclusive,
//! non-blocking `flock` on `<journal>.dispatch.lock` beside the journal,
//! opened through the shared lock-file home, and, held, the holder row of
//! the next generation. Liveness is the lock, which the kernel lets go when
//! its process dies: there is no lease, no heartbeat and no clock read for
//! it.
//!
//! The lock is named beside the journal's path as its store was opened.
//! A lock file removed under a live dispatcher, or a journal reached by
//! another path, lets a second dispatcher take a lock of its own; its
//! holder row then fences the first out of the journal, which writes
//! nothing after, as the operator ruled a fence-out crash-equivalent
//! (#430, 2026-10-23).

use std::ffi::OsString;
use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};

use brokkr_store::{Generation, Store, StoreError};
use rustix::fs::{flock, FlockOperation, CWD};

use super::DispatcherError;
use crate::lock_file;

/// What a journal's dispatch lock is named, after the journal's own name.
const LOCK: &str = ".dispatch.lock";

/// This process as a journal's dispatcher: its lock, held until this is
/// dropped or the process ends, and the generation its holder row
/// recorded, the one each step it takes is journaled under.
#[derive(Debug)]
pub struct Dispatcher {
    pub generation: Generation,
    pub(super) _held: Held,
}

/// A dispatch lock held by its descriptor.
#[derive(Debug)]
pub(super) struct Held(pub(super) OwnedFd);

/// Let the lock go as it is dropped, not as its descriptor closes: a
/// process this one forked holds a copy of the descriptor, and the lock
/// with it, from its fork until its exec.
impl Drop for Held {
    fn drop(&mut self) {
        // Unlocking a lock this descriptor holds has no failure to act on.
        let _ = flock(&self.0, FlockOperation::Unlock);
    }
}

/// The dispatch lock beside `journal`.
fn beside(journal: &Path) -> PathBuf {
    let mut name = OsString::from(journal);
    name.push(LOCK);
    PathBuf::from(name)
}

/// Take the journal `store` holds as this process's dispatcher: its lock
/// at once or not at all, and, held, a holder row of the next generation.
/// Refused, recording nothing, where another dispatcher holds the lock,
/// naming the one the journal's latest holder row records, or where the
/// lock file is not this user's own single file.
pub fn take_dispatch(store: &mut Store) -> Result<Result<Dispatcher, DispatcherError>, StoreError> {
    let journal = store.path().to_path_buf();
    let path = beside(&journal);
    let Some(held) = lock_file::opened(CWD, &path) else {
        return Ok(Err(DispatcherError::Lock(path)));
    };
    // A lock that cannot be taken now, for whatever reason, is another's.
    if flock(&held, FlockOperation::NonBlockingLockExclusive).is_err() {
        let holder = store.dispatch_holder()?;
        return Ok(Err(DispatcherError::Dispatched(journal, holder)));
    }
    let generation = store.dispatch_hold(std::process::id())?;
    Ok(Ok(Dispatcher {
        generation,
        _held: Held(held),
    }))
}
