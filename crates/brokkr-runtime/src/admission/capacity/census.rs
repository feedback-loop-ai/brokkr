//! The runs a pass counts against the machine's ceilings: every run of
//! the journal that reads `Running`, direct `brokkr run` runs among them
//! (decision 0095 ruling 6). A run an older brokkr left with a row and no
//! events, from a crash between its two writes before a run's start was
//! one transaction, is an unstarted run (ruling 4): it spawned nothing, so
//! it is counted against no ceiling, and the pass judges every entry
//! beside it rather than failing as on a run that does not fold. The pass
//! names each one, whether or not an entry waits on it.

use brokkr_core::envelope::EventEnvelope;
use brokkr_core::fold::{fold, Status};
use brokkr_store::Store;

use super::{Live, Seats};
use crate::admission::awaited::never_started;
use crate::admission::{AdmissionError, Reason};

/// What one reading of the journal's runs found: the runs running now,
/// when it counted them, and each unstarted run, named as the one no
/// entry waits on, in the order the journal lists them.
pub(super) struct Census {
    pub(super) live: Vec<Live>,
    pub(super) unstarted: Vec<Reason>,
}

/// Read each run of the journal once, naming the unstarted ones, and
/// when `counted`, folding every other to count the ones running now.
pub(super) fn census(store: &Store, counted: bool) -> Result<Census, AdmissionError> {
    let mut census = Census {
        live: Vec::new(),
        unstarted: Vec::new(),
    };
    for (run, ..) in store.list_runs()? {
        let events = store.load(&run)?;
        if never_started(&events) {
            census
                .unstarted
                .push(Reason::UnstartedRun { wait: None, run });
        } else if counted {
            census.live.extend(running(store, run, &events)?);
        }
    }
    Ok(census)
}

/// `run`, holding `events`, as the census counts it when it reads
/// `Running`.
fn running(
    store: &Store,
    run: String,
    events: &[EventEnvelope],
) -> Result<Option<Live>, AdmissionError> {
    let status = match fold(events) {
        Ok(state) => state.run.status(),
        Err(source) => return Err(AdmissionError::Fold { run, source }),
    };
    if status != Status::Running {
        return Ok(None);
    }
    let seats = Seats::of(store.manifest(&run)?);
    Ok(Some(Live { run, seats }))
}
