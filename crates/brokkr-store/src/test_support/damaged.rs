//! Journals damaged past every reader (#1000): a b-tree page SQLite reads
//! as corrupt, a header that is no database's, and an envelope that is not
//! JSON. Each journal is built by the store's own writer first, so the
//! damage is the only thing about it no writer would leave.

use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

use brokkr_core::envelope::EventType;
use serde_json::json;

use super::append;
use crate::Store;

/// A journal at `db` holding each run of `runs`, started and entered into
/// its first phase by the store's own writer, so it folds, and closed so
/// its WAL is checkpointed into the file.
pub fn written(db: &Path, runs: &[&str]) {
    let mut store = Store::open(db).unwrap();
    for run_id in runs {
        store
            .create_run(run_id, "feat", "self", &json!({"files": {}}))
            .unwrap();
        let started = json!({"feature": "feat", "manifest": {}});
        append(&mut store, run_id, EventType::RunStarted, started);
        let entered = json!({"phase": "work"});
        append(&mut store, run_id, EventType::PhaseEntered, entered);
    }
}

/// Overwrite the bytes of `db` at `offset` with `bytes`: at 0, a header
/// SQLite reads as no database.
pub fn overwrite(db: &Path, offset: u64, bytes: &[u8]) {
    let mut file = std::fs::OpenOptions::new().write(true).open(db).unwrap();
    file.seek(SeekFrom::Start(offset)).unwrap();
    file.write_all(bytes).unwrap();
}

/// Overwrite the root page of `table` in `db`, a table or an index, with
/// bytes no b-tree page holds, so SQLite reads it as corrupt.
pub fn damage_table(db: &Path, table: &str) {
    let conn = rusqlite::Connection::open(db).unwrap();
    let page: u64 = conn
        .query_row(
            "SELECT rootpage FROM sqlite_master WHERE type IN ('table', 'index') AND name = ?1",
            [table],
            |row| row.get(0),
        )
        .unwrap();
    let size: u64 = conn
        .query_row("PRAGMA page_size", [], |row| row.get(0))
        .unwrap();
    drop(conn);
    overwrite(db, (page - 1) * size, &vec![0xFF; size as usize]);
}

/// Replace `run_id`'s envelope at `seq` with `text`, past the append-only
/// guard, as a copy recovered by SQLite's `.recover` can hold one.
pub fn replace_envelope(db: &Path, run_id: &str, seq: i64, text: &str) {
    let conn = rusqlite::Connection::open(db).unwrap();
    conn.execute_batch("DROP TRIGGER IF EXISTS events_append_only_update")
        .unwrap();
    conn.execute(
        "UPDATE events SET envelope = ?3 WHERE run_id = ?1 AND seq = ?2",
        rusqlite::params![run_id, seq, text],
    )
    .unwrap();
}

/// The 36 bytes that are not JSON the recovered copy in #1000 held.
pub const NOT_JSON: &str = "ce4f9c0e8b1d4a6f9e2c7b3a5d8f1e0c2b4a";
