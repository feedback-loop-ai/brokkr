//! A run's view read from the store as its first screen and its pages
//! read it (#753; decision 0089 slice 2b): the run opened, read at the head
//! its opening verified a journal page at a time, and composed from the
//! trail, the seat rows and each page, or said kept where the read says
//! the run is read whole. brokkr-cli's parity, corrupt-run and heap tests
//! read a run so.

use brokkr_store::{Opened, RunRead, Store, StoreError};
use brokkr_view::{composed_view, Parts, RunView};

/// `run`'s view composed from the store's read at `opened`, its journal
/// page past seq `after`, and the seq that page ends at; `None` where the
/// read says the run is kept.
pub(crate) fn page(
    store: &Store,
    run: &str,
    opened: &Opened,
    after: u64,
) -> Result<Option<(RunView, u64)>, StoreError> {
    let RunRead::Current {
        trail,
        seats,
        page,
        causes,
        state,
        last_at,
    } = store.run_read(run, opened, after)?
    else {
        return Ok(None);
    };
    let parts = Parts {
        trail: &trail,
        seats: &seats,
        page: &page,
        causes: &causes,
        head_seq: opened.seq,
        last_at: &last_at,
    };
    let view = composed_view(&parts, state.as_deref()).expect("rows a writer keeps compose");
    let ends = page.last().map_or(opened.seq, |event| event.seq);
    Ok(Some((view, ends)))
}

/// `run` opened, then its view composed page by page to its head, each
/// page asked past the last; `None` where the read says it is kept.
pub(crate) fn pages(store: &Store, run: &str) -> Result<Option<Vec<RunView>>, StoreError> {
    let opened = store.open_run(run)?;
    let (mut pages, mut after) = (Vec::new(), 0);
    loop {
        let Some((view, ends)) = page(store, run, &opened, after)? else {
            return Ok(None);
        };
        pages.push(view);
        if ends == opened.seq {
            return Ok(Some(pages));
        }
        after = ends;
    }
}
