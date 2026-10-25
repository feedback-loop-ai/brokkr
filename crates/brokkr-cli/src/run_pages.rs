//! The run view of one run as its pages read it (#753; decision 0089 slice
//! 3): the one home of `inspect`, `seats`, `watch` and `/api/view/<run>`'s
//! read. The run is opened first, its chain read whole and verified, and
//! refused there in the store's words as `Store::load` refuses it (ruling
//! 5); then it is read at the head its opening verified and composed from
//! its trail, its seat rows and its journal pages (ruling 7). A read the
//! store says is kept, a run whose summary row records the fold's refusal,
//! and a view the rows do not compose read the run whole, as today
//! (ruling 8).
//!
//! What each reader shows decides what it reads ([`Shows`]): a readout that
//! prints every row and every checkpoint reads every journal page and
//! every participant's checkpoint pages, older page by older page, bounded
//! by the run as its opening is; a terminal frame reads one read's trail,
//! the rows it shows, and no journal page or checkpoint.

use brokkr_core::fold::{fold, FoldError, RunState};
use brokkr_store::{Opened, RunRead, SeatRead, Store, StoreError};
use brokkr_view::{composed_view, page_rows, CheckpointRow, Participant, Parts, RunView};

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
    /// no checkpoint.
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
/// then read at the head its opening verified, or read whole where that
/// read is kept.
pub(crate) fn viewed(store: &Store, run: &str, shows: Shows) -> Result<Viewed, StoreError> {
    let opened = store.open_run(run)?;
    // A frame asks for its page past the head, so the read holds no
    // journal row beside the trail, a checkpoint of the run's first seqs
    // least of all (ruling 9).
    let paged = match shows {
        Shows::Everything => everything(store, run, &opened)?,
        Shows::Frame => {
            composed(store, run, &opened, opened.seq, Shows::Frame)?.map(|(viewed, _)| viewed)
        }
    };
    match paged {
        Some(viewed) => Ok(viewed),
        None => whole(store, run),
    }
}

/// `run` read whole and folded, as every reader read it before its pages.
fn whole(store: &Store, run: &str) -> Result<Viewed, StoreError> {
    let events = store.load(run)?;
    let folded = fold(&events);
    let view = brokkr_view::run_view(&events, folded.as_ref().ok());
    Ok(Viewed { view, folded })
}

/// `run`'s view composed from one read at `opened`, its journal page past
/// seq `after`, and the seq that page ends at; for a frame, its journal is
/// the trail's rows instead, none naming its cause: its page, asked past
/// the head, is empty, and so are the causes read with it. `None` where
/// the read is kept, its summary row records the fold's refusal or the
/// rows do not compose.
fn composed(
    store: &Store,
    run: &str,
    opened: &Opened,
    after: u64,
    shows: Shows,
) -> Result<Option<(Viewed, u64)>, StoreError> {
    let RunRead::Current {
        trail,
        seats,
        page,
        causes,
        state: Some(state),
        last_at,
    } = store.run_read(run, opened, after)?
    else {
        return Ok(None);
    };
    let shown = match shows {
        Shows::Everything => &page,
        Shows::Frame => &trail,
    };
    let parts = Parts {
        trail: &trail,
        seats: &seats,
        page: shown,
        causes: &causes,
        head_seq: opened.seq,
        last_at: &last_at,
    };
    let ends = page.last().map_or(opened.seq, |event| event.seq);
    let view = composed_view(&parts, Some(&state));
    let folded = Ok(*state);
    Ok(view.map(|view| (Viewed { view, folded }, ends)))
}

/// `run`'s view with every journal page and every participant's
/// checkpoints, read at `opened`: `None` where any read is kept.
fn everything(store: &Store, run: &str, opened: &Opened) -> Result<Option<Viewed>, StoreError> {
    let Some(mut viewed) = journal(store, run, opened)? else {
        return Ok(None);
    };
    let participants = viewed.view.participants.iter();
    let read = participants.map(|participant| checkpoints(store, run, opened, participant));
    let read: Vec<Option<Vec<CheckpointRow>>> = read.collect::<Result<_, _>>()?;
    let read: Option<Vec<Vec<CheckpointRow>>> = read.into_iter().collect();
    Ok(read.map(|read| {
        let participants = viewed.view.participants.iter_mut();
        for (participant, rows) in participants.zip(read) {
            participant.checkpoints = rows;
        }
        viewed
    }))
}

/// `run`'s view composed at `opened` from its first journal page, its
/// journal every page's rows in seq order, each page asked past the last:
/// `None` where a page's read is kept.
fn journal(store: &Store, run: &str, opened: &Opened) -> Result<Option<Viewed>, StoreError> {
    let (mut viewed, mut after) = (None::<Viewed>, 0);
    loop {
        let Some((page, ends)) = composed(store, run, opened, after, Shows::Everything)? else {
            return Ok(None);
        };
        let joined = match viewed.take() {
            None => page,
            Some(mut first) => {
                first.view.journal.extend(page.view.journal);
                first
            }
        };
        if ends == opened.seq {
            return Ok(Some(joined));
        }
        (viewed, after) = (Some(joined), ends);
    }
}

/// `participant`'s checkpoints, oldest first as the view shows them, read
/// at `opened` a page of 50 at a time from the newest, each page asked
/// before the last: `None` where its key names no seat, a page is kept or
/// a page names a checkpoint no row can be made of.
fn checkpoints(
    store: &Store,
    run: &str,
    opened: &Opened,
    participant: &Participant,
) -> Result<Option<Vec<CheckpointRow>>, StoreError> {
    let Some(seat) = participant.seat() else {
        return Ok(None);
    };
    let (mut rows, mut before) = (Vec::new(), None);
    loop {
        let read = match store.seat_read(run, opened, seat, before, page_rows)? {
            SeatRead::Current(page, next) => {
                let page: Option<Vec<CheckpointRow>> = page.into_iter().collect();
                page.map(|page| (page, next))
            }
            SeatRead::Kept => None,
        };
        let Some((page, next)) = read else {
            return Ok(None);
        };
        rows.extend(page);
        let Some(next) = next else {
            rows.reverse();
            return Ok(Some(rows));
        };
        before = Some(next);
    }
}

#[cfg(test)]
mod tests;
