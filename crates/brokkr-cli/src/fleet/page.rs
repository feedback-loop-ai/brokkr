//! The fleet read a page at a time (decision 0089 rulings 5, 6 and 8).
//! A journal the summary pages is read by the summary's index, a page of
//! rows at a time, and never a run's events but where ruling 5 asks: each
//! row is checked against its run's last event, and a run whose row
//! disagrees, or whose row no writer wrote, is read whole and listed as
//! its fold says. The runs behind their summary are read whole before a
//! page is, at most [`AHEAD_ROWS`] a refresh, kept between refreshes
//! while each head stands, and placed among the page's rows by their own
//! facts; a listing of every run keys the rest a batch of one section at
//! a time, each batch one pass that reads them whole, and reads each
//! whole again where its page reaches its key, so what it holds is a
//! batch however many runs are behind. A journal the summary does not
//! page yet, read by a reader that cannot migrate it, is read the kept
//! way (#744) and paged in memory by the same sections.

use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

use brokkr_core::fold::RunState;
use brokkr_core::page::{newest_first, Place};
use brokkr_core::EventEnvelope;
use brokkr_store::{keyed, place_of, Keyed, Paged, RowRefusal, Store, StoreError, Unread};
use brokkr_view::{Cursor, Page, RunRow, Section};

use super::{judged, read_run, Journal, KeptFleet, ListedRun, Quarantine};

/// A run's journal as a read left it: its events, and its fold or why it
/// is quarantined.
type Judged = (Vec<EventEnvelope>, Result<RunState, Quarantine>);

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
    /// Where in the behind lane the runs the last refresh read end, while
    /// the lane holds more past them.
    past: Option<(i64, String)>,
    /// The runs behind past those, for a listing of every run.
    tail: Option<Tail>,
}

/// How many keys of the runs behind past a refresh a listing of every
/// run holds at once, each a place of 32 bytes and its run id, so its
/// memory is a batch's however many runs are behind; a pass reads each of
/// them whole once for every batch of a section it lists, so a listing
/// of every run reads them whole about `behind / TAIL_KEYS + 1` times
/// over (decision 0089 ruling 6).
pub(crate) const TAIL_KEYS: usize = 1_000;

/// The runs behind past those a refresh read, as a listing of every run
/// keys them: a batch of one section at a time, never every key.
struct Tail {
    /// How many keys a batch holds at most.
    keys: usize,
    batch: RefCell<Option<Batch>>,
    /// The sections they list under, once a pass has read them all, so
    /// a section none lists under costs no pass.
    listed: RefCell<Option<Vec<Section>>>,
}

/// The newest places of `section`, below `below` or from its top, among
/// the runs behind past a refresh: each run's place there, newest first,
/// and whether they are every run of `section` below `below`.
struct Batch {
    section: Section,
    below: Option<(i64, String)>,
    keys: Vec<(i64, String)>,
    every: bool,
}

/// `place` among the `keys` newest of `newest`: whether one fell past
/// them, so `newest` no longer holds every place it was handed.
fn held_past(
    newest: &mut BinaryHeap<Reverse<(i64, String)>>,
    place: (i64, String),
    keys: usize,
) -> bool {
    newest.push(Reverse(place));
    newest.len() > keys && newest.pop().is_some()
}

/// Whether `place` lists below `from`, every place below the top.
fn under(place: &(i64, String), from: Option<&(i64, String)>) -> bool {
    from.is_none_or(|from| (place.0, place.1.as_str()) < (from.0, from.1.as_str()))
}

impl Batch {
    /// Whether this batch holds the newest places of `section` below
    /// `from`: it was read from at or above `from`, and either holds every
    /// place below it or holds one below `from`, past which every place
    /// down to its last is its own.
    fn covers(&self, section: Section, from: Option<&(i64, String)>) -> bool {
        let read_above = match (from, &self.below) {
            (_, None) => true,
            (None, Some(_)) => false,
            (Some(from), Some(below)) => !under(below, Some(from)),
        };
        let holds = self.every || self.keys.last().is_some_and(|last| under(last, from));
        self.section == section && read_above && holds
    }
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

    /// What a listing of `store` reads it by, its journal file `before` it
    /// was opened: at most [`AHEAD_ROWS`] runs behind their summary read
    /// whole and the rest counted (ruling 6), whether it lists one page or
    /// every run.
    pub(crate) fn listing(store: &Store, before: Option<Journal>) -> Result<Pages, StoreError> {
        let mut pages = Pages::default();
        pages.refresh(store, before, brokkr_view::AHEAD_ROWS)?;
        Ok(pages)
    }

    /// [`Pages::listing`] for a listing of every run: the runs behind past
    /// those it read whole are keyed too, a batch of `keys` at a time
    /// ([`TAIL_KEYS`] but in a test), so each lists in its section at its
    /// place, read whole again when a page reaches it.
    pub(crate) fn every(
        store: &Store,
        before: Option<Journal>,
        keys: usize,
    ) -> Result<Pages, StoreError> {
        let mut pages = Pages::listing(store, before)?;
        if let Pages::Summary(kept) = &mut pages {
            kept.tail = kept.past.is_some().then(|| Tail {
                keys: keys.max(1),
                batch: RefCell::new(None),
                listed: RefCell::new(None),
            });
        }
        Ok(pages)
    }

    /// The page after `after`, or the first, of up to `limit` of the rows
    /// `ask` keeps, at its clock: each row within its bounds, or with its
    /// values whole for each run it names, read by its run's key.
    pub(crate) fn page(
        &self,
        store: &Store,
        ask: Ask<'_>,
        after: Option<&Cursor>,
        limit: usize,
    ) -> Result<Page, StoreError> {
        Ok(self.window(store, ask, after, (limit, limit))?.0)
    }

    /// [`Pages::page`] of `limit` rows, and the cursor past its first
    /// `first` where it runs on past them ([`brokkr_view::window`]).
    pub(crate) fn window(
        &self,
        store: &Store,
        ask: Ask<'_>,
        after: Option<&Cursor>,
        (limit, first): (usize, usize),
    ) -> Result<(Page, Option<Cursor>), StoreError> {
        let asked = (limit, ask.now);
        match self {
            Pages::Summary(kept) => {
                brokkr_view::window(after, asked, first, |section, place, wanted| {
                    kept.section(store, ask, section, place, wanted)
                })
            }
            Pages::Kept { rows, .. } => {
                brokkr_view::window(after, asked, first, |section, place, wanted| {
                    Ok(kept_of(rows, ask, section, place, wanted))
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
        let every: &dyn Fn(&RunRow) -> bool = &|_| true;
        let older =
            brokkr_view::section_of_rows(rows, Section::Older, now, None, (usize::MAX, every));
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

/// What a page asks of each row it reads: the clock it sections by, which
/// runs' values are read whole, and which rows it keeps, so a listing
/// that searches the fleet reads forward and holds only what it keeps.
#[derive(Clone, Copy)]
pub(crate) struct Ask<'a> {
    pub(crate) now: &'a str,
    pub(crate) whole: &'a dyn Fn(&str) -> bool,
    pub(crate) keep: &'a dyn Fn(&RunRow) -> bool,
}

/// Up to `wanted` of `rows`, listed whole, that `ask` keeps under
/// `section` below `after`.
fn kept_of(
    rows: &[RunRow],
    ask: Ask<'_>,
    section: Section,
    after: Option<Place<'_>>,
    wanted: usize,
) -> Vec<RunRow> {
    brokkr_view::section_of_rows(rows, section, ask.now, after, (wanted, ask.keep))
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
        let (listed, unread) = store.behind(None, limit)?;
        self.past = listed.last().filter(|_| unread > 0).map(resume);
        let mut kept: HashMap<String, RunRow> = self
            .rows
            .drain(..)
            .map(|row| (row.run_id.clone(), row))
            .collect();
        let mut heads = std::mem::take(&mut self.runs);
        self.whole.clear();
        let mut read = 0;
        for key in listed {
            let run_id = resume(&key).1;
            let head = store.head_hash(&run_id)?;
            let row = match (heads.remove(&run_id), kept.remove(&run_id)) {
                (Some(at), Some(row)) if at == head => row,
                _ => {
                    let (row, events) = read_keyed(store, &key)?;
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

    /// The newest `keys` places of `section` at `now` below `from`, or
    /// from its top, among the runs behind past those the refresh read:
    /// one pass of the behind lane under the listing's one read, a page of
    /// the index at a time, each run read whole for the section its fold
    /// lists it under and its place there, and no more than `keys` places
    /// held past their run's read (ruling 6); and every section the pass
    /// read one of them under.
    fn batch(
        &self,
        store: &Store,
        (section, now): (Section, &str),
        (from, keys): (Option<&(i64, String)>, usize),
    ) -> Result<(Batch, Vec<Section>), StoreError> {
        let mut newest = BinaryHeap::with_capacity(keys + 1);
        let mut every = true;
        let mut sections = Vec::new();
        let mut next = self.past.clone();
        while let Some((at, key)) = next.take() {
            let (listed, beyond) = store.behind(Some((at, &key)), brokkr_view::AHEAD_ROWS)?;
            for key in &listed {
                let row = read_keyed(store, key)?.0;
                let (order_at, run_id) = row.place();
                let place = (order_at, run_id.to_string());
                let under_section = brokkr_view::section_of(&row, now);
                if !sections.contains(&under_section) {
                    sections.push(under_section);
                }
                if under_section == section && under(&place, from) {
                    every &= !held_past(&mut newest, place, keys);
                }
            }
            next = listed.last().filter(|_| beyond > 0).map(resume);
        }
        let keys = newest.into_sorted_vec().into_iter();
        let batch = Batch {
            section,
            below: from.cloned(),
            keys: keys.map(|Reverse(place)| place).collect(),
            every,
        };
        Ok((batch, sections))
    }

    /// Up to `wanted` places of `section` at `now` below `from` among the
    /// runs behind past those the refresh read, newest first, from the
    /// batch the tail holds, which a pass reads anew where it holds none
    /// and `section` is one a pass has not found empty.
    fn tail_keys(
        &self,
        store: &Store,
        (section, now): (Section, &str),
        (from, wanted): (Option<&(i64, String)>, usize),
    ) -> Result<Vec<(i64, String)>, StoreError> {
        let Some(tail) = &self.tail else {
            return Ok(Vec::new());
        };
        let mut listed = tail.listed.borrow_mut();
        if listed
            .as_ref()
            .is_some_and(|listed| !listed.contains(&section))
        {
            return Ok(Vec::new());
        }
        let mut batch = tail.batch.borrow_mut();
        let held = batch.take().filter(|held| held.covers(section, from));
        let held = match held {
            Some(held) => held,
            None => {
                let (read, sections) = self.batch(store, (section, now), (from, tail.keys))?;
                *listed = Some(sections);
                read
            }
        };
        let keys = held.keys.iter().filter(|place| under(place, from));
        let keys = keys.take(wanted).cloned().collect();
        *batch = Some(held);
        Ok(keys)
    }

    /// Up to `wanted` of the runs behind past those the refresh read that
    /// list under `section` below `after` and that `ask` keeps, each read
    /// whole again at its key.
    fn tail_of(
        &self,
        store: &Store,
        ask: Ask<'_>,
        (section, after): (Section, Option<Place<'_>>),
        wanted: usize,
    ) -> Result<Vec<RunRow>, StoreError> {
        let mut rows = Vec::new();
        let mut from = after.map(|(at, key)| (at, key.to_string()));
        while rows.len() < wanted {
            let asked = (from.as_ref(), wanted - rows.len());
            let keys = self.tail_keys(store, (section, ask.now), asked)?;
            if keys.is_empty() {
                break;
            }
            for (order_at, run_id) in keys {
                let key = keyed(order_at, run_id);
                let row = read_keyed(store, &key)?.0;
                rows.extend(Some(row).filter(|row| (ask.keep)(row)));
                from = Some(resume(&key));
            }
        }
        Ok(rows)
    }

    /// Whether the runs kept were read from the file `now`.
    fn stands_on(&self, now: Option<Journal>) -> bool {
        self.journal
            .zip(now)
            .is_some_and(|(kept, now)| kept.holds(now))
    }

    /// Up to `wanted` rows of `section` below `after` that `ask` keeps:
    /// its ranges of the summary's index, each row checked against its
    /// run's head, merged with the runs behind their summary that list
    /// there, those read and those keyed past them. A run this refresh read as behind is listed by that read
    /// alone, wherever its row stands now: a writer that catches it up
    /// between the two reads moves its row into another range, and the
    /// run lists once (ruling 6).
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
        let mut from: Option<(i64, String)> = None;
        loop {
            // At most a page of the index at a time, so what a read holds
            // is a page whichever sections its rows fall in.
            let chunk = (wanted - rows.len()).min(brokkr_view::PAGE_ROWS);
            let below = from.as_ref().map(|(at, key)| (*at, key.as_str()));
            let read = store.section(&ranges, below.or(after), chunk)?;
            let ended = read.len() < chunk;
            from = read.last().map(|paged| {
                let (at, key) = place_of(paged);
                (at, key.to_string())
            });
            for paged in read {
                if self.runs.contains_key(place_of(&paged).1) {
                    continue;
                }
                let row = listed(store, paged, ask.whole)?;
                if (ask.keep)(&row) {
                    rows.push(row);
                }
            }
            if ended || rows.len() >= wanted {
                break;
            }
        }
        let held = (wanted, ask.keep);
        let behind = brokkr_view::section_of_rows(&self.rows, section, ask.now, after, held);
        rows.extend(behind);
        rows.extend(self.tail_of(store, ask, (section, after), wanted)?);
        rows.sort_by(|a, b| newest_first(a.place()).cmp(&newest_first(b.place())));
        rows.truncate(wanted);
        Ok(rows)
    }
}

/// One paged row as the fleet lists it: the summary's, while its run's
/// last event is the head it was folded to (ruling 5), its values whole
/// where `whole` names its run; the run read whole when it is not, or
/// when the row holds a value no writer writes; and a refusal for a row
/// whose run id no writer admits, which names no run to read. A row not
/// the summary's is paged where the summary's stood, and lists by its
/// own facts ([`brokkr_view::page`]).
fn listed(store: &Store, paged: Paged, whole: &dyn Fn(&str) -> bool) -> Result<RunRow, StoreError> {
    let at = place_of(&paged).0;
    let (key, mut row) = match paged {
        Ok(mut row) => {
            let read = if !store.stands_at(&row.run_id, (row.folded.0, &row.folded.1))? {
                read_whole(store, &row.run_id)?.0
            } else if !whole(&row.run_id) {
                return Ok(brokkr_view::summary_row(&row, None));
            } else {
                match store.whole(&mut row)? {
                    Ok(kept) => return Ok(brokkr_view::summary_row(&row, Some(&kept))),
                    Err(_) => read_whole(store, &row.run_id)?.0,
                }
            };
            (row.run_id, read)
        }
        Err(unread) => {
            let read = unread_row(store, &unread)?.0;
            (unread.run_id, read)
        }
    };
    row.paged_at = Some((at, key));
    Ok(row)
}

/// A key of the behind lane's place, which the lane resumes past.
fn resume(key: &Keyed) -> (i64, String) {
    match key {
        Ok(place) => place.clone(),
        Err(unread) => (unread.order_at, unread.run_id.clone()),
    }
}

/// The run at `key` of the behind lane, read whole, and how many events
/// that read; or, where the store refused the key, listed as a refused
/// row is, so every lane holds a run id to the one bound (decision 0089
/// rulings 2 and 5).
fn read_keyed(store: &Store, key: &Keyed) -> Result<(RunRow, usize), StoreError> {
    match key {
        Ok((_, run_id)) => read_whole(store, run_id),
        Err(unread) => unread_row(store, unread),
    }
}

/// A row the store refused, as the fleet lists it: refused, never read,
/// where its run id is past what a writer admits, which names no run to
/// read; its run read whole where the row holds a value no writer writes.
fn unread_row(store: &Store, unread: &Unread) -> Result<(RunRow, usize), StoreError> {
    match &unread.why {
        why @ RowRefusal::RunIdPastBound => {
            Ok((past_bound(&unread.run_id, unread.order_at, why), 0))
        }
        RowRefusal::Status(_)
        | RowRefusal::Severity(_)
        | RowRefusal::Column(_)
        | RowRefusal::Residuals(_) => read_whole(store, &unread.run_id),
    }
}

/// The row of a run read whole, as the whole read lists it, and how many
/// events that read.
fn read_whole(store: &Store, run_id: &str) -> Result<(RunRow, usize), StoreError> {
    let (events, state) = read_run(store, run_id);
    let read = events.len();
    Ok((row_of(store, run_id, (events, state))?, read))
}

/// The row of run `run_id`, which an opening refused with `refused`:
/// quarantined in the store's own words, as the fleet read whole lists a
/// run that does not load (decision 0089 ruling 5).
pub(crate) fn refused_row(
    store: &Store,
    run_id: &str,
    refused: StoreError,
) -> Result<RunRow, StoreError> {
    row_of(store, run_id, judged(store, run_id, Err(refused)))
}

/// The row of run `run_id` as `read` left it, by its `runs` row.
fn row_of(store: &Store, run_id: &str, read: Judged) -> Result<RunRow, StoreError> {
    let (feature, created_at) = store.listing(run_id)?;
    let (events, state) = read;
    let residuals = brokkr_view::residual_findings(run_id, &events);
    let run = ListedRun {
        run_id: run_id.to_string(),
        feature,
        created_at,
        events,
        state,
        residuals,
    };
    Ok(brokkr_view::run_row(&run.entry()))
}

/// A row whose run id is past what any writer admits, listed refused by
/// the first bytes of its id, marked cut, and never read; it is paged at
/// its whole id, which the page's order and every cursor compare.
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
    row.paged_at = Some((order_at, run_id.to_string()));
    row
}

#[cfg(test)]
mod tests;
