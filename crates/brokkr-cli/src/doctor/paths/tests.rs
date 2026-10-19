//! `brokkr doctor`'s summary check (decision 0089 ruling 4): a run whose
//! summary row stands at its head and is not its fold's is a failure,
//! named with its first differing column; a journal whose rows agree adds
//! no line.

use std::path::{Path, PathBuf};

use brokkr_core::envelope::EventType;
use serde_json::json;

use super::*;

/// A journal of `runs`, each created and started, written and closed.
fn journal(dir: &Path, runs: &[&str]) -> PathBuf {
    let db = dir.join("forge.db");
    let mut store = Store::open(&db).unwrap();
    for run in runs {
        store
            .create_run(run, "a feature", "self", &json!({}))
            .unwrap();
        let opened = json!({"feature": "a feature", "manifest": {}});
        store
            .append_next(run, EventType::RunStarted, opened, None, None)
            .unwrap();
    }
    db
}

fn edit(db: &Path, statements: &str) {
    rusqlite::Connection::open(db)
        .unwrap()
        .execute_batch(statements)
        .unwrap();
}

fn report() -> Report {
    Report {
        healthy: true,
        lines: Vec::new(),
    }
}

#[test]
fn a_journal_whose_rows_agree_with_their_folds_adds_no_line() {
    let dir = tempfile::tempdir().unwrap();
    let db = journal(dir.path(), &["r1", "r2"]);
    let mut report = report();
    report_database(&mut report, Ok(&db));
    assert!(report.healthy);
    assert_eq!(
        report.lines,
        [format!(
            "ok       database: {} opens (WAL, append-only triggers)",
            db.display()
        )]
    );
}

#[test]
fn a_row_at_its_head_that_is_not_its_fold_s_is_a_failure_named_by_its_column() {
    let dir = tempfile::tempdir().unwrap();
    let db = journal(dir.path(), &["edited", "gone", "corrupt"]);
    edit(
        &db,
        "UPDATE run_summary SET phase = 'review' WHERE run_id = 'edited';
         DELETE FROM run_summary WHERE run_id = 'gone';
         DROP TRIGGER events_append_only_update;
         UPDATE events SET envelope = replace(envelope, 'a feature', 'spoiled')
             WHERE run_id = 'corrupt';",
    );
    let mut report = report();
    report_database(&mut report, Ok(&db));
    assert!(!report.healthy);
    assert_eq!(
        report.lines[1..],
        [
            "MISSING  summary: run 'edited': its summary row stands at its head, and its \
             phase is not its fold's",
            "MISSING  summary: run 'gone' has no summary row",
            "MISSING  summary: run 'corrupt': its summary row stands at its head, and the run \
             does not load: chain: event 1: event_hash does not match canonical content",
        ]
    );
}

#[test]
fn a_summary_the_journal_cannot_be_read_for_is_a_failure() {
    let dir = tempfile::tempdir().unwrap();
    let db = journal(dir.path(), &["r1"]);
    let store = Store::open(&db).unwrap();
    edit(&db, "DROP TABLE run_summary");
    let mut report = report();
    report_summaries(&mut report, &store);
    assert!(!report.healthy);
    assert_eq!(
        report.lines,
        ["MISSING  summary: not read: sqlite: no such table: run_summary"]
    );
}
