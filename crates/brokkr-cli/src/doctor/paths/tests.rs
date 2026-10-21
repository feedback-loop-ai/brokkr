//! `brokkr doctor`'s summary check (decision 0089 ruling 4): a run whose
//! summary row stands at its head and is not its fold's is a failure,
//! named with its first differing column, as is a run that does not load
//! wherever its row stands; a journal whose rows agree adds no line.

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
            "MISSING  summary: run 'corrupt' does not load: chain: event 1: event_hash does not \
             match canonical content",
            "MISSING  summary: run 'edited': its summary row stands at its head, and its \
             phase is not its fold's",
            "MISSING  summary: run 'gone' has no summary row",
        ]
    );
}

/// The frozen broken-chain journal, planted in `db` as a writer from
/// before the summary leaves it: its run and events, nothing folded.
fn plant_broken_chain(db: &Path) {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/journals/conclude-broken-chain-hand-built.ndjson");
    let conn = rusqlite::Connection::open(db).unwrap();
    brokkr_store::test_support::plant_run(&conn, "conclude-broken-chain-hand-built", "{}").unwrap();
    for line in std::fs::read_to_string(fixture).unwrap().lines() {
        let event = serde_json::from_str(line).unwrap();
        brokkr_store::test_support::plant_envelope(&conn, &event).unwrap();
    }
}

/// A run that does not load is a failure whether its row stands behind
/// its head, which no open's catch-up can fold, or at it, as a row folded
/// before the chain broke beneath it would stand (decision 0089 ruling 7:
/// the doctor's sweep reads every run whole).
#[test]
fn a_run_that_does_not_load_is_a_failure_behind_its_head_and_at_it() {
    let dir = tempfile::tempdir().unwrap();
    let db = journal(dir.path(), &["healthy"]);
    plant_broken_chain(&db);
    let unread = "MISSING  summary: run 'conclude-broken-chain-hand-built' does not load: \
                  chain: event 11: event_hash does not match canonical content";
    let to_its_head = "UPDATE run_summary SET folded_seq = head_seq, folded_hash = head_hash";
    for (place, statements) in [("behind its head", ""), ("at its head", to_its_head)] {
        edit(&db, statements);
        let mut report = report();
        report_database(&mut report, Ok(&db));
        assert!(!report.healthy, "{place}");
        assert_eq!(report.lines[1..], [unread], "{place}");
    }
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

/// An event index that is not what its run's events project to is a
/// failure, named by the first seq where they part and how (decision
/// 0089 ruling 4): an index row is the trigger's, so it never lags.
#[test]
fn an_event_index_that_is_not_its_events_is_a_failure_named_by_its_seq() {
    let dir = tempfile::tempdir().unwrap();
    let db = journal(dir.path(), &["edited", "gone", "stray"]);
    edit(
        &db,
        "UPDATE event_index SET member = 'm' WHERE run_id = 'edited' AND seq = 1;
         DELETE FROM event_index WHERE run_id = 'gone';
         INSERT INTO event_index (run_id, seq) VALUES ('stray', 9), ('absent', 4);",
    );
    let mut report = report();
    report_database(&mut report, Ok(&db));
    assert!(!report.healthy);
    assert_eq!(
        report.lines[1..],
        [
            "MISSING  summary: run 'edited': the event index row of its event 1 is not the \
             event's in its member",
            "MISSING  summary: run 'gone': its event 1 has no event index row",
            "MISSING  summary: run 'stray': the event index holds a row at 9, where the run \
             holds no event",
            "MISSING  summary: run 'absent': the event index holds a row at 4, where the run \
             holds no event",
        ]
    );
}
