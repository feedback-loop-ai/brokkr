//! The fleet read a page at a time (decision 0089 rulings 5, 6 and 8).
//! A journal the summary pages is read by the summary's index, a page of
//! rows at a time, and never a run's events but where ruling 5 asks: each
//! row is checked against its run's last event, and a run whose row
//! disagrees, or whose row no writer wrote, is read whole and listed as
//! its fold says. The runs behind their summary are read whole before a
//! page is, at most [`AHEAD_ROWS`] a refresh on the console, kept between
//! refreshes while each head stands, and placed among the page's rows by
//! their own facts. A journal the summary does not page yet, read by a
//! reader that cannot migrate it, is read the kept way (#744) and paged
//! in memory by the same sections.

use std::collections::{HashMap, HashSet};

use brokkr_core::page::{newest_first, Place};
use brokkr_store::{Paged, RowRefusal, Store, StoreError, Unread};
use brokkr_view::{Cursor, Page, RunRow, Section};

use super::{read_run, Journal, KeptFleet, ListedRun};

/// What a fleet reader keeps of one hearth between refreshes.
pub(crate) enum Pages {
    /// A journal the summary pages: its runs behind their summary.
    Summary(Behind),
    /// A journal no summary pages yet, read the kept way, and its rows
    /// listed whole by last activity.
    Kept { fleet: KeptFleet, rows: Vec<RunRow> },
}

impl Default for Pages {
    fn default() -> Pages {
        Pages::Summary(Behind::default())
    }
}

/// The runs behind their summary, each read whole and kept at the head
/// it was read at, by last activity.
#[derive(Default)]
pub(crate) struct Behind {
    journal: Option<Journal>,
    runs: HashMap<String, (u64, String)>,
    rows: Vec<RunRow>,
    /// The runs the last refresh read whole rather than kept.
    whole: HashSet<String>,
    /// How many runs past those read are behind, counted by the index.
    unread: u64,
}

impl Pages {
    /// Bring what is kept up to the journal, and say how many events that
    /// read: on a journal the summary pages, those of the runs behind it,
    /// at most `behind` of them; on one it does not, those that landed
    /// since the last refresh. `before` is the journal file stated before
    /// `store` was opened onto it.
    pub(crate) fn refresh(
        &mut self,
        store: &Store,
        before: Option<Journal>,
        behind: usize,
    ) -> Result<usize, StoreError> {
        let paged = store.pages()?;
        if paged != matches!(self, Pages::Summary(_)) {
            *self = match paged {
                true => Pages::Summary(Behind::default()),
                false => Pages::Kept {
                    fleet: KeptFleet::default(),
                    rows: Vec::new(),
                },
            };
        }
        match self {
            Pages::Summary(kept) => kept.refresh(store, before, behind),
            Pages::Kept { fleet, rows } => {
                let read = fleet.refresh(store, before)?;
                *rows = brokkr_view::run_rows(&fleet.entries()).runs;
                Ok(read)
            }
        }
    }

    /// The page after `after`, or the first, of up to `limit` rows at
    /// `now`: each row within its bounds, or with its values whole for
    /// each run `whole` names, read by its run's key.
    pub(crate) fn page(
        &self,
        store: &Store,
        now: &str,
        after: Option<&Cursor>,
        limit: usize,
        whole: &dyn Fn(&str) -> bool,
    ) -> Result<Page, StoreError> {
        match self {
            Pages::Summary(kept) => brokkr_view::page(after, limit, |section, place, wanted| {
                kept.section(store, Ask { now, whole }, section, place, wanted)
            }),
            Pages::Kept { rows, .. } => {
                brokkr_view::page(after, limit, |section, place, wanted| {
                    Ok(brokkr_view::section_of_rows(
                        rows, section, now, place, wanted,
                    ))
                })
            }
        }
    }

    /// How many runs list under "older" at `now`, counted by the index
    /// alone on a journal the summary pages.
    pub(crate) fn older(&self, store: &Store, now: &str) -> Result<usize, StoreError> {
        let (rows, counted) = match self {
            Pages::Summary(kept) => {
                let mut counted = 0;
                for range in brokkr_view::ranges(Section::Older, now) {
                    counted += store.counted(&range)?;
                }
                (&kept.rows, counted as usize)
            }
            Pages::Kept { rows, .. } => (rows, 0),
        };
        let older = brokkr_view::section_of_rows(rows, Section::Older, now, None, usize::MAX);
        Ok(counted + older.len())
    }

    /// How many runs behind their summary the last refresh left unread.
    pub(crate) fn unread(&self) -> u64 {
        match self {
            Pages::Summary(kept) => kept.unread,
            Pages::Kept { .. } => 0,
        }
    }

    /// Whether the last refresh read run `run_id` whole.
    pub(crate) fn was_read_whole(&self, run_id: &str) -> bool {
        match self {
            Pages::Summary(kept) => kept.whole.contains(run_id),
            Pages::Kept { fleet, .. } => fleet.was_read_whole(run_id),
        }
    }

    /// Whether what is kept was read from a journal file that `now` is not.
    pub(crate) fn replaced(&self, now: Option<Journal>) -> bool {
        match self {
            Pages::Summary(kept) => kept.journal.is_some() && !kept.stands_on(now),
            Pages::Kept { fleet, .. } => fleet.replaced(now),
        }
    }
}

/// What a page asks of each row it reads: the clock it sections by, and
/// which runs' values are read whole.
#[derive(Clone, Copy)]
struct Ask<'a> {
    now: &'a str,
    whole: &'a dyn Fn(&str) -> bool,
}

impl Behind {
    /// Read each of the first `limit` runs behind their summary whole,
    /// but those kept at the head they still stand at, from the file they
    /// were kept from.
    fn refresh(
        &mut self,
        store: &Store,
        before: Option<Journal>,
        limit: usize,
    ) -> Result<usize, StoreError> {
        if !self.stands_on(before) {
            self.runs.clear();
        }
        let (listed, unread) = store.behind(limit)?;
        let mut kept: HashMap<String, RunRow> = self
            .rows
            .drain(..)
            .map(|row| (row.run_id.clone(), row))
            .collect();
        let mut heads = std::mem::take(&mut self.runs);
        self.whole.clear();
        let mut read = 0;
        for run_id in listed {
            let head = store.head_hash(&run_id)?;
            let row = match (heads.remove(&run_id), kept.remove(&run_id)) {
                (Some(at), Some(row)) if at == head => row,
                _ => {
                    let (row, events) = read_whole(store, &run_id)?;
                    read += events;
                    self.whole.insert(run_id.clone());
                    row
                }
            };
            self.runs.insert(run_id, head);
            self.rows.push(row);
        }
        self.rows
            .sort_by(|a, b| newest_first(a.place()).cmp(&newest_first(b.place())));
        self.unread = unread;
        self.journal = before;
        Ok(read)
    }

    /// Whether the runs kept were read from the file `now`.
    fn stands_on(&self, now: Option<Journal>) -> bool {
        self.journal
            .zip(now)
            .is_some_and(|(kept, now)| kept.holds(now))
    }

    /// Up to `wanted` rows of `section` below `after`: its ranges of the
    /// summary's index, each row checked against its run's head, merged
    /// with the runs behind their summary that list there.
    fn section(
        &self,
        store: &Store,
        ask: Ask<'_>,
        section: Section,
        after: Option<Place<'_>>,
        wanted: usize,
    ) -> Result<Vec<RunRow>, StoreError> {
        let ranges = brokkr_view::ranges(section, ask.now);
        let mut rows = Vec::new();
        for paged in store.section(&ranges, after, wanted)? {
            rows.push(listed(store, paged, ask.whole)?);
        }
        rows.extend(brokkr_view::section_of_rows(
            &self.rows, section, ask.now, after, wanted,
        ));
        rows.sort_by(|a, b| newest_first(a.place()).cmp(&newest_first(b.place())));
        rows.truncate(wanted);
        Ok(rows)
    }
}

/// One paged row as the fleet lists it: the summary's, while its run's
/// last event is the head it was folded to (ruling 5), its values whole
/// where `whole` names its run; the run read whole when it is not, or
/// when the row holds a value no writer writes; and a refusal for a row
/// whose run id no writer admits, which names no run to read.
fn listed(store: &Store, paged: Paged, whole: &dyn Fn(&str) -> bool) -> Result<RunRow, StoreError> {
    match paged {
        Ok(mut row) => {
            if !store.stands_at(&row.run_id, (row.folded.0, &row.folded.1))? {
                return Ok(read_whole(store, &row.run_id)?.0);
            }
            let kept = match whole(&row.run_id) {
                true => Some(store.whole(&mut row)?),
                false => None,
            };
            Ok(brokkr_view::summary_row(&row, kept.as_ref()))
        }
        Err(Unread {
            run_id,
            order_at,
            why: why @ RowRefusal::RunIdPastBound,
        }) => Ok(past_bound(&run_id, order_at, &why)),
        Err(Unread { run_id, .. }) => Ok(read_whole(store, &run_id)?.0),
    }
}

/// The row of a run read whole, as the whole read lists it, and how many
/// events that read.
fn read_whole(store: &Store, run_id: &str) -> Result<(RunRow, usize), StoreError> {
    let (feature, created_at) = store.listing(run_id)?;
    let (events, state) = read_run(store, run_id);
    let residuals = brokkr_view::residual_findings(run_id, &events);
    let run = ListedRun {
        run_id: run_id.to_string(),
        feature,
        created_at,
        events,
        state,
        residuals,
    };
    Ok((brokkr_view::run_row(&run.entry()), run.events.len()))
}

/// A row whose run id is past what any writer admits, listed refused at
/// its place by the first bytes of its id, marked cut, and never read.
fn past_bound(run_id: &str, order_at: i64, why: &RowRefusal) -> RunRow {
    let mut end = run_id.len().min(brokkr_store::RUN_ID_MAX);
    while !run_id.is_char_boundary(end) {
        end -= 1;
    }
    let shown = format!("{}…", &run_id[..end]);
    let words = why.to_string();
    let mut row = brokkr_view::run_row(&brokkr_view::RunEntry {
        run_id: &shown,
        feature: "",
        created_at: "",
        last_recorded_at: None,
        state: None,
        detail: Some(brokkr_view::Quarantine::DoesNotLoad.in_words(&words)),
        residuals: &[],
    });
    row.order_at = order_at;
    row
}

#[cfg(test)]
mod tests;
