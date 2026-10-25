//! The journal a pass measured, which the rooms it found claim in alone
//! (decision 0095 ruling 7): the entry a room holds and the census it
//! counted are that journal's, and an entry of the same id in another
//! journal is not the one measured.
//!
//! A journal is the id it names itself by (#430, 2026-10-24), stored
//! inside it and read through a store's own connection: by the pass, and
//! again inside the claim's transaction, which refuses unless the two
//! match. No path, symlink or file renamed or replaced since bears on it.
//! A new store on the same file, by its path or through a symlink to it,
//! is the same journal and claims. A byte copy carries the id and is the
//! same journal too: two copies each dispatched from are outside the
//! threat model. A journal created apart has its own id and is refused,
//! as one with no id is: one opened only to read before any open that
//! may write named it.

use std::path::Path;

use brokkr_core::envelope::EventEnvelope;
use brokkr_store::Store;
use uuid::Uuid;

use super::{Awaiting, Recheck, Room};
use crate::admission::AdmissionError;

/// The id a journal names itself by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Journal(Uuid);

impl Journal {
    /// The journal `store`'s connection holds, or `None` when it has no
    /// id.
    pub(super) fn of(store: &Store) -> Result<Option<Journal>, AdmissionError> {
        Ok(store.journal()?.map(Journal))
    }
}

/// Refuse the journal `named`, as the claim's transaction read it through
/// the store at `path`, unless it is the journal `measured`, which the
/// pass read; a pass or a claim over a journal with no id refuses.
fn measured_in(
    measured: Option<Journal>,
    named: Option<Uuid>,
    path: &Path,
) -> Result<(), AdmissionError> {
    match measured.is_some() && named.map(Journal) == measured {
        true => Ok(()),
        false => Err(AdmissionError::OtherJournal(path.to_path_buf())),
    }
}

/// A run with an event past a pass's high-water, as the claim's
/// transaction hands it: its id, its events and its manifest.
type Moved = (String, Vec<EventEnvelope>, String);

impl Room {
    /// The claim's judge, for a claim through the store at `path`: the
    /// journal its transaction reads, refused first unless the one this
    /// room's pass measured, then [`Room::recheck`].
    pub(super) fn judge(
        &self,
        path: &Path,
    ) -> impl FnMut(Option<Uuid>, &[Awaiting], &[Moved]) -> Result<(), Recheck> + '_ {
        let path = path.to_path_buf();
        move |named, waits, moved| {
            let other = |error| Recheck::Failed(Box::new(error));
            measured_in(self.journal, named, &path).map_err(other)?;
            self.recheck(waits, moved)
        }
    }
}
