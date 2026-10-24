//! The journal a pass measured, which the rooms it found claim in alone
//! (decision 0095 ruling 7): the entry a room holds and the census it
//! counted are that journal's, and an entry of the same id in another
//! journal is not the one measured.
//!
//! A journal is the file its store's connection holds, by the device and
//! inode [`Store::journal`] read when the store opened it, read when the
//! pass measures and again before the claim reads or writes anything. It
//! is never read through the path as given, which a symlink retargeted
//! under an open store points at another file. A new store on the same
//! path, or through a symlink to the file, is the same journal and claims.
//! A copy is another, since it is written apart from the file measured,
//! and is refused, as a file renamed into a store's place, an unlinked
//! one, and one that cannot be read are. A hard link of the file, opened
//! while a store holds it, is refused by SQLite at [`Store::open`] as a
//! disk I/O error, before any pass or claim.

use brokkr_store::Store;

use crate::admission::AdmissionError;

/// The file a journal is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Journal {
    dev: u64,
    ino: u64,
}

impl Journal {
    /// The journal `store`'s connection holds, or `None` when its file
    /// cannot be read.
    pub(super) fn of(store: &Store) -> Option<Journal> {
        let (dev, ino) = store.journal()?;
        Some(Journal { dev, ino })
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
