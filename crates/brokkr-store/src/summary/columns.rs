//! The run summary table's columns, read by name once: the definition a
//! part writes and the shape check that holds it share this read, so a
//! column's name exists in one place (decision 0071 ruling 5).

use rusqlite::Connection;

use crate::StoreError;

/// The table's columns, by name.
pub(super) fn read(conn: &Connection) -> Result<Vec<String>, StoreError> {
    let mut statement = conn.prepare("SELECT name FROM pragma_table_info('run_summary')")?;
    let found: Vec<String> = statement
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(found)
}

/// The columns as the table's definition lists them, each name and its
/// type, in the order the definition states them.
pub(super) fn typed(columns: &[(&str, &str)]) -> String {
    let typed: Vec<String> = columns
        .iter()
        .map(|(name, kind)| format!("{name} {kind}"))
        .collect();
    typed.join(", ")
}

/// A part of the row added after it (#909; decision 0064), with a stamp of
/// its own: its columns in the row's order, the one of them a writer sets
/// with `folded_seq`, and the trigger that sends behind a row a writer from
/// before the part folded, whose stamp it left off.
pub(super) struct Stamped {
    pub(super) columns: &'static [(&'static str, &'static str)],
    pub(super) stamp: &'static str,
    pub(super) trigger: &'static str,
}

impl Stamped {
    /// The columns as the table's definition lists them.
    pub(super) fn defined(&self) -> String {
        typed(self.columns)
    }

    /// The row's own test that the part is not the one its folded head
    /// states: the stamp is written in the same statement as `folded_seq`,
    /// so they differ only for a row a writer from before the part folded.
    /// `prefix` names the row in the statement that asks.
    pub(super) fn stale(&self, prefix: &str) -> String {
        format!(
            "{prefix}{stamp} IS NOT {prefix}folded_seq",
            stamp = self.stamp
        )
    }

    /// The trigger, `IF NOT EXISTS` as the table's own are; its own update
    /// does not fire it again, as SQLite fires no trigger recursively
    /// unless asked.
    pub(super) fn migration(&self) -> String {
        format!(
            "CREATE TRIGGER IF NOT EXISTS {trigger} AFTER UPDATE OF folded_hash ON run_summary
            WHEN {stale} BEGIN
            UPDATE run_summary SET folded_seq = NULL, folded_hash = NULL
                WHERE run_id = NEW.run_id;
        END;",
            trigger = self.trigger,
            stale = self.stale("NEW.")
        )
    }

    /// Does the table carry every column of the part, which a journal from
    /// before it does not? A pure read.
    pub(super) fn shaped(&self, conn: &Connection) -> Result<bool, StoreError> {
        has(conn, self.columns)
    }
}

/// Does the table carry every one of `columns`, by name? A pure read.
pub(super) fn has(conn: &Connection, columns: &[(&str, &str)]) -> Result<bool, StoreError> {
    let found = read(conn)?;
    Ok(columns
        .iter()
        .all(|(column, _)| found.iter().any(|name| name == column)))
}
