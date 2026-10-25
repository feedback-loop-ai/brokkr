//! The runs a pass counts against the machine's ceilings: every run of
//! the journal that reads `Running`, direct `brokkr run` runs among them
//! (decision 0095 ruling 6). A run an older brokkr left with a row and no
//! events, from a crash between its two writes before a run's start was
//! one transaction, is an unstarted run (ruling 4): it spawned nothing, so
//! it is counted against no ceiling, and the pass judges every entry
//! beside it rather than failing as on a run that does not fold. The pass
//! names each one ([`unstarted`]), whether or not an entry waits on it.

use brokkr_core::fold::{fold, Status};
use brokkr_store::Store;

use super::{Live, Seats};
use crate::admission::{AdmissionError, Reason};

/// The runs the journal holds running now.
pub(super) fn census(store: &Store) -> Result<Vec<Live>, AdmissionError> {
    let mut live = Vec::new();
    for (run, ..) in store.list_runs()? {
        let events = store.load(&run)?;
        if events.is_empty() {
            continue;
        }
        let status = fold(&events)
            .map_err(|source| AdmissionError::Fold {
                run: run.clone(),
                source,
            })?
            .run
            .status();
        if status == Status::Running {
            let seats = Seats::of(store.manifest(&run)?);
            live.push(Live { run, seats });
        }
    }
    Ok(live)
}

/// Each run of the journal with a row and no events, in the order the
/// journal lists them, named as the unstarted run no entry waits on: an
/// event's sequence starts at one, so a head at zero holds none.
pub(super) fn unstarted(store: &Store) -> Result<Vec<Reason>, AdmissionError> {
    let mut unstarted = Vec::new();
    for (run, ..) in store.list_runs()? {
        if store.head_hash(&run)?.0 == 0 {
            unstarted.push(Reason::UnstartedRun { wait: None, run });
        }
    }
    Ok(unstarted)
}
