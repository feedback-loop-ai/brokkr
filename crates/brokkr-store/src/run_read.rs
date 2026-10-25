//! The run view's read (decision 0089 ruling 7, slice 2b): after a run's
//! opening ([`Store::open_run`]), its trail through `event_index`'s partial
//! index and the events by key, its seat rows, its journal a page at a
//! time by seq and what caused the page's rows, its summary row's refusal
//! and its head's stamp, all read in one snapshot at the head the opening
//! verified. A journal whose projections are not read, or a run that moved
//! past that head, is said so ([`RunRead::Kept`]), and the reader reads the
//! run the kept way (ruling 8).
//!
//! Where each field of `brokkr_view::RunView` reads from, after the
//! opening, composed by `brokkr_view::composed_view`. "Trail" is every
//! event but a checkpoint; "facts" are the seat rows grouped by
//! participant (`brokkr_core::seat::row::facts`):
//!
//! | Field | Reads today | Its source here |
//! |---|---|---|
//! | `entry`, `worktree`, `recipe`, `notices`, `boundary` | `run/started` and every `effect/started` | the trail |
//! | `summary`, `commands` | the fold | the trail folded, its seq and hash the opened head's (a checkpoint moves only the fold's seq and hash); none where the summary row records the fold's refusal at that head, a checkpoint's included |
//! | `ruling`, `unavailable`, `clarify` | the fold, the rulings, the supersedes and the last success | the trail and the fold |
//! | `participants`: which, in order, key, label, member, phase, visit, attempts, provenance, effort pin | requests, starts and checkpoints | the facts, each under the request its first row names, and the trail's requests and starts |
//! | `participants`: model, effort, boundary, usage, costs, turns, items, transcript, status, carried | requests, starts, terminals and checkpoints | the facts |
//! | `participants`: `activity` | a working seat's last `seat-turn` tool and target; a member's first checkpoint, its finish or last checkpoint; a seat's start and terminal | the facts' `tool`, `target`, `started_at`, `finish` and `last_at`; the trail |
//! | `participants`: `terminal_line`, `session_id` | a member's last `panel-member-finished`; the session record's `session_id` | the facts' `finish` and `session_id` |
//! | `participants`: `checkpoints` | every checkpoint | none: the seat view's checkpoint pages |
//! | `live` | the open attempt's checkpoints: each member's first, its last `seat-turn`'s turn and tool | `seat::row::live` |
//! | `phases`: name, visits, current, strategy, returns | phase entries, rulings, the fold | the trail |
//! | `phases`: `columns`, `plain` | the newest effect's checkpoints: its finished sequence steps' names and its members in first order | `seat::row::inner`; each node's state and cells the facts' |
//! | `journal` | every event | the journal page; each row's model and boundary from its own payload, the trail's starts and the facts |
//! | `journal`: `causation_seq` | every event's id | the causes, by `event_index`'s ids |
//! | `event_count` | every event, counted | the opened head's seq, a position from 1 |
//! | `dashboard`: path, decision, last seat, last review | phases, rulings, terminals | the trail, the last visit ending at the head's stamp |
//! | `dashboard`: a visit's `model` | every model its checkpoints and terminals reported | `seat::row::models` |
//! | `scopes`, `view_version` | nothing of the run | nothing |

use std::collections::BTreeMap;

use brokkr_core::envelope::EventEnvelope;
use brokkr_core::fold::{fold, RunState};
use brokkr_core::seat::row::SeatRow;
use rusqlite::{params, OptionalExtension};

use crate::seat_summary::rows_at;
use crate::{head_of, patiently, Opened, SeatRows, Store, StoreError};

/// The journal rows one page holds, as many as a seat's checkpoint page
/// and the fleet's page of runs hold (ruling 7).
const JOURNAL_PAGE: u64 = 50;

/// Whether a run's summary row records the fold's refusal, read only where
/// it stands at the head it is asked at.
const REFUSED: &str = "SELECT refusal_seq IS NOT NULL FROM run_summary
     WHERE run_id = ?1 AND folded_seq = ?2 AND folded_hash = ?3";

/// The stamp of a run's event, by its key.
const STAMP: &str = "SELECT json_extract(envelope, '$.recorded_at') FROM events
     WHERE run_id = ?1 AND seq = ?2";

/// What the run view reads of one run at the head its opening verified.
#[derive(Debug, Clone)]
pub enum RunRead {
    /// Read at that head: the trail's envelopes in seq order, the seat
    /// rows, and a page of the journal's envelopes, checkpoints included,
    /// in seq order. A page holds every row after the cursor it was asked
    /// past up to 50 or the head; the next page is asked past its last
    /// row's seq, and the page whose last row is the head is the last.
    /// `causes` holds the seq each of the page's causation ids names, an
    /// id no event of the run carries holding none; `state` is the fold
    /// at the head, none where it refuses there; `last_at` is the head's
    /// stamp.
    Current {
        trail: Vec<EventEnvelope>,
        seats: Vec<SeatRow>,
        page: Vec<EventEnvelope>,
        causes: BTreeMap<String, u64>,
        state: Option<Box<RunState>>,
        last_at: String,
    },
    /// Not read at that head: the journal's event index or summary is not
    /// read, the run's last event is no longer the head it was opened at,
    /// it has none, its seat rows or its summary row do not stand at its
    /// head, or a row of its index names no cause. The reader reads the
    /// run the kept way (ruling 8).
    Kept,
}

impl Store {
    /// `run_id`'s run view read at `opened`, the head its opening verified,
    /// its journal page past seq `after` (0 for the first), in one read
    /// transaction, so a peer's append between two of its reads cannot
    /// pass for that head. Each envelope is parsed and not verified: the
    /// opening verified the chain to that head, and an event the read
    /// finds there is refused as no row, never skipped.
    pub fn run_read(
        &self,
        run_id: &str,
        opened: &Opened,
        after: u64,
    ) -> Result<RunRead, StoreError> {
        self.run_read_with(run_id, opened, after, || {})
    }

    /// [`Store::run_read`], `between` run after the run's head is read and
    /// before its seat rows, trail and page are: the instant a peer's
    /// append lands, which the tests stage.
    pub(crate) fn run_read_with(
        &self,
        run_id: &str,
        opened: &Opened,
        after: u64,
        mut between: impl FnMut(),
    ) -> Result<RunRead, StoreError> {
        patiently("load", self.patience, || {
            let _snapshot = self.conn.unchecked_transaction()?;
            self.read_at(run_id, opened, after, &mut between)
        })
    }

    fn read_at(
        &self,
        run_id: &str,
        opened: &Opened,
        after: u64,
        between: &mut impl FnMut(),
    ) -> Result<RunRead, StoreError> {
        if !(self.reads_index()? && self.pages()?) {
            return Ok(RunRead::Kept);
        }
        let (seq, hash) = head_of(&self.conn, run_id)?;
        if (seq, hash.as_str()) != (opened.seq, opened.hash.as_str()) || seq == 0 {
            return Ok(RunRead::Kept);
        }
        between();
        let SeatRows::Current(seats) = rows_at(&self.conn, run_id, (seq, &hash))? else {
            return Ok(RunRead::Kept);
        };
        let Some(refused) = self.refused_at(run_id, opened)? else {
            return Ok(RunRead::Kept);
        };
        self.paged_at(run_id, opened, after, seats, refused)
    }

    /// The rest of the read at `opened`, whose `seats` stand there and
    /// whose summary row says whether the fold was `refused` there: the
    /// page past `after` and what caused its rows, the trail, the fold and
    /// the head's stamp.
    fn paged_at(
        &self,
        run_id: &str,
        opened: &Opened,
        after: u64,
        seats: Vec<SeatRow>,
        refused: bool,
    ) -> Result<RunRead, StoreError> {
        // The opening verified each seq the position it holds, from 1 to
        // the head, so a page's seqs are its keys.
        let last = after.saturating_add(JOURNAL_PAGE).min(opened.seq);
        let page: Vec<u64> = (after.saturating_add(1)..=last).collect();
        let page = self.envelopes_at(run_id, &page)?;
        let Some(causes) = self.index_causes(run_id, &caused(&page))? else {
            return Ok(RunRead::Kept);
        };
        let trail = self.envelopes_at(run_id, &self.index_trail(run_id)?)?;
        let head = params![run_id, opened.seq as i64];
        Ok(RunRead::Current {
            state: (!refused)
                .then(|| folded(&trail, opened))
                .flatten()
                .map(Box::new),
            last_at: self.conn.query_row(STAMP, head, |row| row.get(0))?,
            trail,
            seats,
            page,
            causes,
        })
    }

    /// Whether `run_id`'s summary row records the fold's refusal at
    /// `opened`, none where it does not stand there.
    fn refused_at(&self, run_id: &str, opened: &Opened) -> Result<Option<bool>, StoreError> {
        let head = params![run_id, opened.seq as i64, opened.hash];
        let refused = self.conn.query_row(REFUSED, head, |row| row.get(0));
        Ok(refused.optional()?)
    }
}

/// The causation ids `page`'s rows name.
fn caused(page: &[EventEnvelope]) -> Vec<&str> {
    let ids = page
        .iter()
        .filter_map(|event| event.causation_id.as_deref());
    ids.collect()
}

/// The fold at `opened` of the run whose trail is `trail`: a checkpoint
/// only checks the effect it names is open, so the trail folds to the
/// run's state but for the seq and hash the head moves it to.
fn folded(trail: &[EventEnvelope], opened: &Opened) -> Option<RunState> {
    fold(trail).ok().map(|state| RunState {
        seq: opened.seq,
        last_hash: opened.hash.clone(),
        ..state
    })
}

#[cfg(test)]
mod tests;
