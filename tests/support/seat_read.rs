//! A participant's checkpoints read from the store as the seat view's
//! pages read them (#753; decision 0089 slice 2b): the run opened, then a
//! page of 50 of the participant's checkpoints at a time, newest first, at
//! the head the opening verified, each page below the cursor the one
//! before handed over, and each checkpoint made into its row by
//! brokkr-view; or said kept where the read says the run is read whole.
//! brokkr-cli's parity, corrupt-run and heap tests read a seat so.

use brokkr_core::envelope::EventEnvelope;
use brokkr_store::{Opened, SeatRead, Store, StoreError};
use brokkr_view::{page_rows, CheckpointRow};

/// A participant: its effect, and its member, `None` for the seat's own.
pub(crate) type Seat<'a> = (&'a str, Option<&'a str>);

/// A page's rows, newest first, and the cursor of the next.
pub(crate) type Page = (Vec<CheckpointRow>, Option<u64>);

/// A page of `seat`'s checkpoints in `run` at `opened`, below `before`
/// where a page resumes; `None` where the read says the run is kept.
pub(crate) fn page(
    store: &Store,
    run: &str,
    opened: &Opened,
    seat: Seat<'_>,
    before: Option<u64>,
) -> Result<Option<Page>, StoreError> {
    let rows = |starts: &mut dyn Iterator<Item = EventEnvelope>| page_rows(seat.1, starts);
    let SeatRead::Current(rows, next) = store.seat_read(run, opened, seat, before, rows)? else {
        return Ok(None);
    };
    let rows: Option<Vec<CheckpointRow>> = rows.into_iter().collect();
    Ok(Some((
        rows.expect("a page's checkpoints name their attempts"),
        next,
    )))
}

/// `run` opened, then every page of `seat`'s checkpoints, the newest
/// first; `None` where the read says the run is kept.
pub(crate) fn pages(
    store: &Store,
    run: &str,
    seat: Seat<'_>,
) -> Result<Option<Vec<Vec<CheckpointRow>>>, StoreError> {
    let opened = store.open_run(run)?;
    let (mut pages, mut before) = (Vec::new(), None);
    loop {
        let Some((rows, next)) = page(store, run, &opened, seat, before)? else {
            return Ok(None);
        };
        pages.push(rows);
        if next.is_none() {
            return Ok(Some(pages));
        }
        before = next;
    }
}
