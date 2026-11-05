//! The accepted stop's additive columns (#909): the seq an operator stop
//! was accepted at while no ending closed it, and the stamp that proves
//! the columns were written at the folded head, as the aggregate columns
//! carry theirs. A writer from before them leaves the stamp off, so its
//! row is sent behind and the open's backfill refolds it.

use super::columns::Stamped;

/// The columns, their stamp and their trigger, which the summary's intact
/// check lists.
pub(super) const PART: Stamped = Stamped {
    columns: &[
        ("stop_accepted_seq", "INTEGER"),
        ("stop_folded_seq", "INTEGER"),
    ],
    stamp: "stop_folded_seq",
    trigger: "run_summary_stopping",
};
