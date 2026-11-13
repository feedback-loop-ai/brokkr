//! The run summary's late-tail columns (decision 0108 ruling 4): the
//! tail a repair named, kept as the wire shape every surface paints, and
//! the stamp that proves the column was folded at the row's head, as the
//! ending's carry theirs (#909). A writer from before the column leaves
//! its stamp off, so a trigger sends the row behind and the open's
//! backfill refolds it.

use brokkr_core::page::LateTail;
use brokkr_core::summary::Summary;
use rusqlite::types::Value;
use rusqlite::Row;

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

/// The columns `summary`'s late tail, folded at its seq, is written as.
pub(super) fn written(summary: &Summary) -> Vec<(&'static str, Value)> {
    let tail = summary.late_tail.as_ref().map_or(Value::Null, |tail| {
        Value::Text(serde_json::to_string(tail).expect("a late tail serializes"))
    });
    vec![
        ("late_tail", tail),
        ("late_folded_seq", Value::Integer(summary.folded_seq as i64)),
    ]
}

/// The late tail the row's column states, parsed once at the page's edge.
pub(crate) fn read(row: &Row<'_>, at: usize) -> Result<Option<LateTail>, RowRefusal> {
    let Some(text) = row.get::<_, Option<String>>(at)? else {
        return Ok(None);
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|error| RowRefusal::Column(format!("late_tail: {error}")))
}
