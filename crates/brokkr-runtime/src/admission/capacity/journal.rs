//! The journal a pass measured, which the rooms it found claim in alone
//! (decision 0095 ruling 7): the entry a room holds and the census it
//! counted are that journal's, and an entry of the same id in another
//! journal is not the one measured.
//!
//! A journal is its file, as the filesystem names it: the device and the
//! inode its store's path resolves to, read when the pass measures and
//! again before the claim reads or writes anything. A new store on the
//! same path, through a symlink to the file or a hard link of it, is the
//! same journal and claims. A copy, or a file renamed into its place, is
//! another, since it is written apart from the file measured, and is
//! refused, as a path that cannot be read is.

use std::os::unix::fs::MetadataExt;

use brokkr_store::Store;

use crate::admission::AdmissionError;

/// The file a journal is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Journal {
    dev: u64,
    ino: u64,
}

impl Journal {
    /// The journal `store` opened, or `None` when its path cannot be read.
    pub(super) fn of(store: &Store) -> Option<Journal> {
        let found = std::fs::metadata(store.path()).ok()?;
        Some(Journal {
            dev: found.dev(),
            ino: found.ino(),
        })
    }
}

/// Refuse `store` unless it is the journal `measured`, which the pass
/// read; a pass that could not read its own refuses every store.
pub(super) fn measured_in(measured: Option<Journal>, store: &Store) -> Result<(), AdmissionError> {
    match measured.is_some() && Journal::of(store) == measured {
        true => Ok(()),
        false => Err(AdmissionError::OtherJournal(store.path().to_path_buf())),
    }
}
