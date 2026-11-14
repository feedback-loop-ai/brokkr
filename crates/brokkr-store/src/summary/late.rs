//! The run summary's late-tail columns (decision 0108 ruling 4): the
//! compact tail a repair named — the sentence's count, range, repair and
//! repairer — and the stamp that proves the column was folded at the
//! row's head, as the ending's carry theirs (#909). A writer from before
//! the column leaves its stamp off, so a trigger sends the row behind and
//! the open's backfill refolds it.

use brokkr_core::page::LateRange;
use brokkr_core::summary::{Account, Summary};
use rusqlite::types::Value;
use rusqlite::{Connection, Row};

use super::columns::Stamped;
use crate::page::RowRefusal;

/// The columns in the row's order, their stamp and their trigger, which
/// the summary's intact check lists.
pub(super) const PART: Stamped = Stamped {
    columns: &[("late_tail", "TEXT"), ("late_folded_seq", "INTEGER")],
    stamp: "late_folded_seq",
    shared: None,
    trigger: "run_summary_late",
};

/// The columns `summary`'s late tail, folded at its seq, is written as:
/// only what the row's sentence says, so the column stays small and every
/// operator name is cut to its one bound.
pub(super) fn written(summary: &Summary) -> Vec<(&'static str, Value)> {
    let tail = summary.late_tail.as_ref().map_or(Value::Null, |range| {
        Value::Text(serde_json::to_string(range).expect("a late range serializes"))
    });
    vec![
        ("late_tail", tail),
        ("late_folded_seq", Value::Integer(summary.folded_seq as i64)),
    ]
}

/// The run's whole account, read inside the append's transaction: what a
/// suffix repair that named a carried event asks the writer to refold.
pub(super) fn refold(tx: &Connection, run_id: &str) -> Result<Account, crate::StoreError> {
    let events = crate::read::rows_in(tx, run_id, None)?;
    Ok(Account::whole(run_id, &events))
}

/// The compact late tail the row's column states, parsed once at the
/// page's edge: a row naming an unknown field, or a column that does not
/// read, refuses as unreadable.
pub(crate) fn read(row: &Row<'_>, at: usize) -> Result<Option<LateRange>, RowRefusal> {
    let Some(text) = row.get::<_, Option<String>>(at)? else {
        return Ok(None);
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|error| RowRefusal::Column(format!("late_tail: {error}")))
}
