//! The fleet's page key (decision 0089 ruling 6), named once for the two
//! crates that read it: the store reads a [`Range`] of the summary's
//! index, and `brokkr-view` says which ranges each of the fleet's sections
//! holds. A page is read in one order, [`newest_first`], whether its rows
//! come from the index or from a run read whole.

use std::cmp::Reverse;

use crate::fold::Status;
use crate::policy::Severity;
use crate::summary::{Bounded, Hired, Refused, Ruled, Stated};

/// The lane a run's summary row stands in, as its generated `lane` column
/// words it. A lane is not a displayed fact: it only says which range a
/// row is read from, and `brokkr-view` still places every row it is handed
/// by its own facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lane {
    /// The row's fold is not at its run's head, or no writer has judged
    /// the run: it is read whole before it is placed.
    Behind,
    /// The fold refused the run, and no acknowledgement stands at its head.
    Refused,
    /// The fold refused the run, and the operator acknowledged it there.
    Acknowledged,
    /// Waiting on the operator.
    Parked,
    /// Running with an operator stop accepted and no ending written: it
    /// is stopping, not running.
    Stopping,
    Running,
    /// Completed or stopped.
    Finished,
}

impl Lane {
    /// The word the `lane` column holds for it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Lane::Behind => "behind",
            Lane::Refused => "refused",
            Lane::Acknowledged => "acknowledged",
            Lane::Parked => "parked",
            Lane::Stopping => "stopping",
            Lane::Running => "running",
            Lane::Finished => "finished",
        }
    }
}

/// The one bound a range puts on `order_at`, the milliseconds the writer
/// parsed from a run's last stamp, beside the cursor's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    /// `order_at < at`.
    Below(i64),
    /// `order_at >= at`.
    AtOrAbove(i64),
    /// `order_at > at`.
    Above(i64),
    /// `order_at <= at`.
    AtOrBelow(i64),
}

/// One range of the summary's index: a lane, and at most one bound on
/// `order_at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub lane: Lane,
    pub bound: Option<Bound>,
}

/// A row's place in a section's order: its `order_at` and its run id.
pub type Place<'a> = (i64, &'a str);

/// The key every page lists a section's rows by, least first: last
/// activity, newest first, an unreadable time (`i64::MAX`) first of all,
/// then run id, highest first. The index's own order, `(order_at DESC,
/// run_id DESC)`, as SQLite compares text: byte by byte, as Rust compares
/// a `str`.
pub fn newest_first(place: Place<'_>) -> Reverse<Place<'_>> {
    Reverse(place)
}

/// A run's summary row as a page reads it back (decision 0089 ruling 2):
/// the columns a page selects, each within its bound. A [`Bounded`] value
/// holds its whole value only once [`Whole`] has been read for the row by
/// its key; a page reads none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub run_id: String,
    pub feature_line: String,
    pub created_at: Bounded,
    pub last_recorded_at: Option<Bounded>,
    /// The writer's `last_at_ms`, or `i64::MAX` where it read no time.
    pub order_at: i64,
    /// `None` exactly when the fold refused the run (`refused`).
    pub status: Option<Status>,
    /// An accepted operator stop with no ending written, at its seq, while
    /// the run still folds to running.
    pub stop_accepted_seq: Option<u64>,
    pub phase: Option<Bounded>,
    pub park_reason: Option<Bounded>,
    pub ruled: Option<Ruled>,
    pub hired: Option<Hired>,
    pub residual_open: Option<Severity>,
    pub refused: Option<Refused>,
    /// The head the writer judged: the run's last event's seq and hash.
    pub folded: (u64, String),
    /// The acknowledgement standing at that head, as the row's copy cuts
    /// it, while the fold refuses the run.
    pub acknowledged: Option<Copied>,
}

/// An acknowledgement's words as its trigger copies them onto the row,
/// each cut to its bound in characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Copied {
    pub operator: String,
    pub reason: String,
    pub at: String,
}

/// What a row keeps whole beyond its bounds, read by its run's key: the
/// feature, the residual findings, read back into their type, and the
/// acknowledgement standing at the run's head, its words whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Whole {
    pub feature: String,
    pub residuals: Vec<Stated>,
    pub acknowledged: Option<Copied>,
}

#[cfg(test)]
mod tests;
