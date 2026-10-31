//! The accepted stop's additive columns (#909): the seq an operator stop
//! was accepted at while no ending closed it, and the stamp that proves
//! the columns were written at the folded head, as the aggregate columns
//! carry theirs. A writer from before them leaves the stamp off, so its
//! row is sent behind and the open's backfill refolds it.

use rusqlite::Connection;

use crate::StoreError;

/// The trigger's name, as the summary's intact check lists it.
pub(super) const TRIGGER: &str = "run_summary_stopping";

/// The columns, in the row's order, and their types.
const COLUMNS: [(&str, &str); 2] = [
    ("stop_accepted_seq", "INTEGER"),
    ("stop_folded_seq", "INTEGER"),
];

/// The columns as the table's definition lists them.
pub(super) fn defined() -> String {
    super::columns::typed(&COLUMNS)
}

/// The row's own test that its accepted stop is not the one its folded
/// head states: `stop_folded_seq` is written in the same statement as
/// `folded_seq`, so they differ only for a row a writer from before the
/// columns folded. `prefix` names the row in the statement that asks.
pub(super) fn stale(prefix: &str) -> String {
    format!("{prefix}stop_folded_seq IS NOT {prefix}folded_seq")
}

/// The trigger that sends behind a row a writer from before the columns
/// folded, `IF NOT EXISTS` as the table's own are; its own update does not
/// fire it again, as SQLite fires no trigger recursively unless asked.
pub(super) fn migration() -> String {
    format!(
        "CREATE TRIGGER IF NOT EXISTS {TRIGGER} AFTER UPDATE OF folded_hash ON run_summary
            WHEN {stale} BEGIN
            UPDATE run_summary SET folded_seq = NULL, folded_hash = NULL
                WHERE run_id = NEW.run_id;
        END;",
        stale = stale("NEW.")
    )
}

/// Does the table carry both columns, which a journal from before them
/// does not? A pure read.
pub(super) fn shaped(conn: &Connection) -> Result<bool, StoreError> {
    super::columns::has(conn, &COLUMNS)
}
