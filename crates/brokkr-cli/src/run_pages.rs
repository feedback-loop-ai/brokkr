//! The run view of one run as its pages read it (#753; decision 0089 slice
//! 3): the one home of `inspect`, `seats`, `watch` and `/api/view/<run>`'s
//! read, and of the seat readers' participants ([`participants`]). The run
//! is opened first, its chain read whole and verified, and
//! refused there in the store's words as `Store::load` refuses it (ruling
//! 5); then it is read at the head its opening verified and composed from
//! its trail and its seat rows (ruling 7). A read the store says is kept,
//! a run whose summary row records the fold's refusal, and a view the
//! rows do not compose read the run whole, as today (ruling 8).
//!
//! What each reader shows decides what it reads ([`Shows`]): a readout that
//! prints every row and every checkpoint keeps every event its opening
//! verifies, bounded by the run, and composes the view once from them, one
//! read's trail and its seat rows, each participant's checkpoints made
//! from the events kept; a terminal frame reads one read's trail, the rows
//! it shows, and no journal page or checkpoint, and so does a seat reader,
//! which shows a participant's row and none of its checkpoints.

use brokkr_core::fold::{fold, FoldError, RunState};
use brokkr_core::EventEnvelope;
use brokkr_store::{Opened, RunRead, Store, StoreError};
use brokkr_view::{composed_view, composed_whole, Participant, Parts, RunView};

/// What a reader shows of a run, and so what it reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shows {
    /// Every journal row and every participant's checkpoints: `inspect
    /// --json`, `inspect --capability-call`, `seats --json` and
    /// `/api/view/<run>`, which print the view whole.
    Everything,
    /// A terminal frame: `inspect`'s, `seats`' and `watch`'s, which paint
    /// the trail's journal rows (`in_trail`, every one an event of the
    /// trail) and no participant's checkpoints. Its journal holds the
    /// trail's rows, none of them naming its cause, and its participants
    /// no checkpoint. The seat readers' read ([`participants`]).
    Frame,
}

impl Shows {
    /// What a readout shows: everything where it prints the view whole, a
    /// terminal frame otherwise.
    pub(crate) fn of(whole: bool) -> Self {
        match whole {
            true => Shows::Everything,
            false => Shows::Frame,
        }
    }
}

/// A run's view as its reader shows it, and the run's fold, or the fold's
/// refusal, which `inspect` and `seats` refuse the run with.
pub(crate) struct Viewed {
    pub(crate) view: RunView,
    pub(crate) folded: Result<RunState, FoldError>,
}

/// `run`'s view in `store` as a reader that `shows` it reads it: opened,
/// keeping what the opening verifies where it shows everything, then read
/// at the head its opening verified, or read whole where that read is
/// kept.
pub(crate) fn viewed(store: &Store, run: &str, shows: Shows) -> Result<Viewed, StoreError> {
    let paged = match shows {
        Shows::Everything => {
            let (opened, journal) = store.open_kept(run)?;
            composed(store, run, &opened, Some(&journal))?
        }
        Shows::Frame => composed(store, run, &store.open_run(run)?, None)?,
    };
    match paged {
        Some(viewed) => Ok(viewed),
        None => whole(store, run),
    }
}

/// `run`'s participants as a seat reader reads them: `brokkr transcript`,
/// `/api/presentation/<run>/<key>`, `/api/transcript/<run>/<key>` and
/// `/sse/transcript/<run>/<key>`, which each show one participant's row,
/// its key, label, transcript reference, provenance, legacy id and whether
/// it works, and none of its checkpoints. Each is its row as the frame's
/// read composes it (ruling 7), with no checkpoint, which none of them
/// prints; read whole where that read is kept (ruling 8). The store's
/// refusal of the run, or the fold's where it opens and does not fold, as
/// each refused it reading the run whole.
pub(crate) fn participants(
    store: &Store,
    run: &str,
) -> Result<Result<Vec<Participant>, FoldError>, StoreError> {
    let Viewed { view, folded } = viewed(store, run, Shows::Frame)?;
    Ok(folded.map(|_| view.participants))
}

/// `run` read whole and folded, as every reader read it before its pages.
fn whole(store: &Store, run: &str) -> Result<Viewed, StoreError> {
    let events = store.load(run)?;
    let folded = fold(&events);
    let view = brokkr_view::run_view(&events, folded.as_ref().ok());
    Ok(Viewed { view, folded })
}

/// `run`'s view composed once from one read at `opened`, its trail, its
/// seat rows and its summary row, asked past the head so the read holds
/// no journal row beside the trail, a checkpoint of the run's first seqs
/// least of all (ruling 9). With `journal`, every event the opening kept,
/// it is the view whole, each participant's checkpoints made from it;
/// without, a frame's, its journal the trail's rows, none naming its
/// cause: the page asked past the head is empty, and so are the causes
/// read with it. `None` where the read is kept, its summary row records
/// the fold's refusal or the rows do not compose.
fn composed(
    store: &Store,
    run: &str,
    opened: &Opened,
    journal: Option<&[EventEnvelope]>,
) -> Result<Option<Viewed>, StoreError> {
    let RunRead::Current {
        trail,
        seats,
        causes,
        state: Some(state),
        last_at,
        ..
    } = store.run_read(run, opened, opened.seq)?
    else {
        return Ok(None);
    };
    let view = match journal {
        Some(journal) => composed_whole(&trail, &seats, journal, &last_at, Some(&state)),
        None => {
            let parts = Parts {
                trail: &trail,
                seats: &seats,
                page: &trail,
                causes: &causes,
                head_seq: opened.seq,
                last_at: &last_at,
            };
            composed_view(&parts, Some(&state))
        }
    };
    let folded = Ok(*state);
    Ok(view.map(|view| Viewed { view, folded }))
}

#[cfg(test)]
mod tests;
