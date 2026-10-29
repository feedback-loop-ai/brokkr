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

/// Does the table carry every one of `columns`, by name? A pure read.
pub(super) fn has(conn: &Connection, columns: &[(&str, &str)]) -> Result<bool, StoreError> {
    let found = read(conn)?;
    Ok(columns
        .iter()
        .all(|(column, _)| found.iter().any(|name| name == column)))
}
