//! The run view's read (decision 0089 ruling 7, slice 2b): after a run's
//! opening ([`Store::open_run`]), its trail through `event_index`'s partial
//! index and the events by key, its seat rows, and its journal a page at a
//! time by seq, all read in one snapshot at the head the opening verified.
//! A journal whose projections are not read, or a run that moved past that
//! head, is said so ([`RunRead::Kept`]), and the reader reads the run the
//! kept way (ruling 8).
//!
//! Where each field of `brokkr_view::RunView` reads from, after the
//! opening. "Trail" is every event but a checkpoint; a field marked
//! **unkept** reads checkpoint facts no projection keeps, so no run view
//! is composed from this read until a projection keeps them:
//!
//! | Field | Reads today | Its source here |
//! |---|---|---|
//! | `entry`, `worktree`, `recipe`, `notices`, `boundary` | `run/started` and every `effect/started` | the trail |
//! | `summary`, `commands` | the fold | the trail folded, its head the opening's (a checkpoint moves only the fold's seq and hash); a checkpoint the fold refuses, the summary row's refusal |
//! | `ruling`, `unavailable`, `clarify` | the fold, the rulings, the supersedes and the last success | the trail and the fold |
//! | `participants`: key, label, phase, visit, attempts, provenance, effort pin | requests and starts | the trail |
//! | `participants`: model, effort, boundary, usage, costs, turns, items, transcript, status, carried | requests, starts, terminals and checkpoints | the seat rows' facts |
//! | `participants`: `activity` | a working seat's last `seat-turn` tool and target; a member's finish time or last checkpoint's | **unkept** |
//! | `participants`: `terminal_line` | a member's last `panel-member-finished` outcome as written, and its time | **unkept** |
//! | `participants`: `session_id` | the session record's `session_id` | **unkept** |
//! | `participants`: `checkpoints` | every checkpoint | the seat view's checkpoint pages |
//! | `live` | the open attempt's checkpoints: each member's first, its last `seat-turn`'s turn and tool | **unkept** |
//! | `phases`: name, visits, current, strategy, returns | phase entries, rulings, the fold | the trail |
//! | `phases`: `columns`, `plain` | the newest effect's checkpoints: its finished sequence steps' names and its members in first order | **unkept** |
//! | `journal` | every event | the journal page; each row's model and boundary from its own payload, the trail's starts and the seat rows |
//! | `journal`: `causation_seq` | every event's id | **unkept** where the cause is a checkpoint off the page: `event_index` keeps no event id |
//! | `event_count` | every event, counted | the opened head's seq, a position from 1 |
//! | `dashboard`: path, decision, last seat, last review | phases, rulings, terminals | the trail, the last visit ending at the head event's time |
//! | `dashboard`: a visit's `model` | every model its checkpoints reported | **unkept**: a seat row keeps its last |
//! | `scopes`, `view_version` | nothing of the run | nothing |

use brokkr_core::envelope::EventEnvelope;
use brokkr_core::seat::row::SeatRow;

use crate::seat_summary::rows_at;
use crate::{head_of, patiently, Opened, SeatRows, Store, StoreError};

/// The journal rows one page holds, as many as a seat's checkpoint page
/// and the fleet's page of runs hold (ruling 7).
const JOURNAL_PAGE: u64 = 50;

/// What the run view reads of one run at the head its opening verified.
#[derive(Debug, Clone)]
pub enum RunRead {
    /// Read at that head: the trail's envelopes in seq order, the seat
    /// rows, and a page of the journal's envelopes, checkpoints included,
    /// in seq order. A page holds every row after the cursor it was asked
    /// past up to 50 or the head; the next page is asked past its last
    /// row's seq, and the page whose last row is the head is the last.
    Current {
        trail: Vec<EventEnvelope>,
        seats: Vec<SeatRow>,
        page: Vec<EventEnvelope>,
    },
    /// Not read at that head: the journal's event index is not read, the
    /// run's last event is no longer the head it was opened at, or its
    /// seat rows do not stand at its head. The reader reads the run the
    /// kept way (ruling 8).
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
        if !self.reads_index()? {
            return Ok(RunRead::Kept);
        }
        let (seq, hash) = head_of(&self.conn, run_id)?;
        if (seq, hash.as_str()) != (opened.seq, opened.hash.as_str()) {
            return Ok(RunRead::Kept);
        }
        between();
        let SeatRows::Current(seats) = rows_at(&self.conn, run_id, (seq, &hash))? else {
            return Ok(RunRead::Kept);
        };
        let trail = self.envelopes_at(run_id, &self.index_trail(run_id)?)?;
        // The opening verified each seq the position it holds, from 1 to
        // the head, so a page's seqs are its keys.
        let last = after.saturating_add(JOURNAL_PAGE).min(seq);
        let page: Vec<u64> = (after.saturating_add(1)..=last).collect();
        let page = self.envelopes_at(run_id, &page)?;
        Ok(RunRead::Current { trail, seats, page })
    }
}

#[cfg(test)]
mod tests;
