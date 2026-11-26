//! Paint the current summary and share a page's overlapping wait reads.

use std::collections::HashMap;

use brokkr_core::page::{Row, Whole};
use brokkr_store::{place_of, Paged, Store, StoreError};
use brokkr_view::{Cooling, RunRow, Section, Standing, WaitFact, Waits, PAGE_ROWS};

use super::{listed, read_whole, Ask};
use crate::fleet::read_rounds;

/// The newest kept waits read by NeedsYou's stale scan, up to one page.
/// Both sections scan that part of the Running lane. Carry its rows to
/// the second scan so the same site of a listed run is read once.
#[derive(Default)]
pub(super) struct PageReads {
    overlap: HashMap<String, (RunRow, bool, bool)>,
}

impl PageReads {
    #[cfg(test)]
    pub(super) fn held(&self) -> usize {
        self.overlap.len()
    }

    pub(super) fn row(
        &mut self,
        store: &Store,
        paged: Paged,
        ask: Ask<'_>,
        (section, stale): (Section, Option<i64>),
        cooling: &[Cooling],
    ) -> Result<Option<RunRow>, StoreError> {
        let held = if section == Section::Running {
            self.overlap.remove(place_of(&paged).1)
        } else {
            None
        };
        if let Some((row, summary, running)) = held {
            return Ok(lists_here(&row, summary, running, section, ask.now, stale).then_some(row));
        }
        let (row, summary, running) = listed(store, paged, ask.whole, cooling)?;
        let here = lists_here(&row, summary, running, section, ask.now, stale);
        let remember = section == Section::NeedsYou
            && running
            && row.waits != Waits::default()
            && self.overlap.len() < PAGE_ROWS;
        if !(here || remember) || !(ask.keep)(&row) {
            return Ok(None);
        }
        if remember {
            self.overlap
                .insert(row.place().1.to_string(), (row.clone(), summary, running));
        }
        Ok(here.then_some(row))
    }
}

/// Whether `row`, read from `section`'s ranges, lists there at `now`: a
/// summary's Running row can stand in a wait its summary does not carry,
/// so its own facts decide it. A row read whole from the Running lane
/// below the stale bound is held by both reads: NeedsYou's stale range
/// and Running's whole lane. The stale read owns one whose fold places it
/// in NeedsYou, and the whole-lane read owns every other one, so the page
/// lists it in its own section's turn and every run lists once (rulings
/// 5 and 6, decision 0089 ruling 5). A row read whole from another lane
/// keeps its summary's place and lists by its fold in
/// [`brokkr_view::window`], so it is never dropped here. `stale` is
/// NeedsYou's bound, `None` where the clock does not read.
pub(super) fn lists_here(
    row: &RunRow,
    summary: bool,
    lane_running: bool,
    section: Section,
    now: &str,
    stale: Option<i64>,
) -> bool {
    if summary {
        return !matches!(row.verdict.standing, Standing::Running | Standing::Waiting)
            || brokkr_view::section_of(row, now) == section;
    }
    if !lane_running {
        return true;
    }
    let below = stale.is_some_and(|stale| row.place().0 < stale);
    let needs_you = brokkr_view::section_of(row, now) == Section::NeedsYou;
    match section {
        Section::NeedsYou => !below || needs_you,
        Section::Running => !(below && needs_you),
        Section::Recent | Section::Older => true,
    }
}

/// A standing row painted from its summary with its head's waits, or its
/// run read whole where that head's envelope is not the one it claims to
/// be, quarantined as the whole read quarantines it (0103 slice 4f, LOW 3).
pub(super) fn painted_or_whole(
    store: &Store,
    row: &Row,
    kept: Option<&Whole>,
    cooling: &[Cooling],
) -> Result<(RunRow, bool), StoreError> {
    match waits_at(store, row) {
        Ok(waits) => Ok((brokkr_view::summary_row(row, kept, &waits, cooling), true)),
        Err(_) => Ok((read_whole(store, &row.run_id, cooling)?.0, false)),
    }
}

/// The waits a summary row's fold head states, read from that one event:
/// a running row's head is where a wait lives (decision 0103 ruling
/// 2(a)). The reader reaches this only for a row that stands at its head
/// (`listed` reads every other row whole). For a repaired run that head is
/// the repair annotation, which the fold applies; the late tail it names
/// ends at the repair's seq less one (`LateTail::last_seq`), so the event
/// read here is never a late one. A repaired summary whose `folded_seq` is
/// a late event's seq is behind its head, does not stand, and is read
/// whole where `WaitFact::applied` decides. Any other standing carries
/// none, so its event is not read. A row the summary refuses paints none,
/// and `brokkr_view::fleet::page::summary_row` is where that is kept, once.
fn waits_at(store: &Store, row: &Row) -> Result<WaitFact, StoreError> {
    let Some(brokkr_core::fold::Status::Running) = row.status else {
        return Ok(WaitFact::NoWait);
    };
    let event = store.last_event(&row.run_id, row.folded.0)?;
    Ok(read_rounds(
        store,
        &row.run_id,
        WaitFact::of(event.as_ref()),
    ))
}
