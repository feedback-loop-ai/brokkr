//! Shared strict SQLite schema verification for standalone split databases
//! (`SqliteProjectionStore` and `SqliteCoordinationStore`).
//!
//! Derives the canonical user-object set (`table`, `index`, `trigger`, `view`
//! excluding SQLite's internal `sqlite_%` objects), exact `PRAGMA table_info`
//! column schemas, and `sqlite_master.sql` definitions once from the store's
//! own DDL so foreign tables, views, indexes, triggers, or colliding tables
//! with wrong columns are refused before switching journal mode to WAL.

use std::collections::BTreeMap;

use rusqlite::Connection;

use crate::StoreError;

const USER_OBJECTS_SQL: &str = "SELECT type, name, sql FROM sqlite_master
     WHERE type IN ('table', 'index', 'trigger', 'view')
       AND name NOT LIKE 'sqlite_%'
     ORDER BY type, name";

#[derive(Debug, PartialEq, Eq)]
struct ColumnSpec {
    name: String,
    col_type: String,
    notnull: i64,
    pk: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SchemaMatch {
    Empty,
    Exact,
    Foreign,
}

pub(super) struct ExpectedSchema {
    objects: BTreeMap<(String, String), String>,
    tables: BTreeMap<String, Vec<ColumnSpec>>,
}

impl ExpectedSchema {
    pub(super) fn from_initializer(init: impl FnOnce(&Connection)) -> Self {
        let conn = Connection::open_in_memory().expect("in-memory sqlite for canonical schema");
        init(&conn);
        let objects = read_user_objects(&conn).expect("read canonical sqlite_master");
        let mut tables = BTreeMap::new();
        for (kind, name) in objects.keys() {
            if kind == "table" {
                let cols = read_table_info(&conn, name).expect("read canonical table_info");
                tables.insert(name.clone(), cols);
            }
        }
        Self { objects, tables }
    }

    pub(super) fn matches(&self, conn: &Connection) -> Result<SchemaMatch, StoreError> {
        let actual = read_user_objects(conn)?;
        if actual.is_empty() {
            return Ok(SchemaMatch::Empty);
        }
        if actual.len() != self.objects.len() || !actual.keys().eq(self.objects.keys()) {
            return Ok(SchemaMatch::Foreign);
        }
        for (table, expected_cols) in &self.tables {
            if read_table_info(conn, table)? != *expected_cols {
                return Ok(SchemaMatch::Foreign);
            }
        }
        if actual != self.objects {
            return Ok(SchemaMatch::Foreign);
        }
        Ok(SchemaMatch::Exact)
    }
}

/// Decode `PRAGMA user_version` as a signed 64-bit integer so negative values
/// never fail with a SQLite integer-conversion error and are refused cleanly as
/// [`StoreError::ForeignDatabase`].
pub(super) fn read_user_version(conn: &Connection) -> Result<i64, StoreError> {
    Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

fn read_user_objects(conn: &Connection) -> Result<BTreeMap<(String, String), String>, StoreError> {
    let mut stmt = conn.prepare(USER_OBJECTS_SQL)?;
    let rows = stmt.query_map([], |row| {
        let kind: String = row.get(0)?;
        let name: String = row.get(1)?;
        let sql: Option<String> = row.get(2)?;
        Ok(((kind, name), sql.unwrap_or_default()))
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn read_table_info(conn: &Connection, table: &str) -> Result<Vec<ColumnSpec>, StoreError> {
    let mut stmt = conn
        .prepare(r#"SELECT name, type, "notnull", pk FROM pragma_table_info(?1) ORDER BY cid"#)?;
    let rows = stmt.query_map([table], |row| {
        Ok(ColumnSpec {
            name: row.get(0)?,
            col_type: row.get(1)?,
            notnull: row.get(2)?,
            pk: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}
