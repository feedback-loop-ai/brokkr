//! Doctor's unrecorded-tree line (decision 0104 ruling 6): a directory
//! under a recorded root that no seat-tree row names is listed with its
//! size, a hostile name is stripped, a count that refuses or finds the
//! tree gone warns, and nothing is removed.

use std::path::Path;

use brokkr_core::envelope::EventType;
use brokkr_store::Store;
use serde_json::json;

use super::*;
use crate::render::Safe;

const RUN: &str = "run-doctor";

fn empty_report() -> Report {
    Report {
        healthy: true,
        lines: Vec::new(),
    }
}

fn journal(dir: &Path) -> Store {
    let store = Store::open(&dir.join("forge.db")).unwrap();
    let mut store = store;
    store
        .create_run(RUN, "a feature", "self", &json!({}))
        .unwrap();
    store
        .append_next(
            RUN,
            EventType::RunStarted,
            json!({"feature": "a feature", "manifest": {}}),
            None,
            None,
        )
        .unwrap();
    store
}

#[test]
fn a_tree_no_row_names_is_listed_with_its_size_and_never_removed() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = journal(dir.path());
    let root = dir.path().join("scratch");
    let named = root.join("11111111-1111-4111-8111-111111111111");
    let orphan = root.join("orphan\x1b[2J");
    std::fs::create_dir_all(&named).unwrap();
    std::fs::create_dir_all(&orphan).unwrap();
    std::fs::write(orphan.join("data"), b"x").unwrap();
    std::fs::write(root.join("loose-file"), b"x").unwrap();
    std::os::unix::fs::symlink(&named, root.join("link")).unwrap();
    store
        .record_seat_tree(RUN, "effect", "11111111-1111-4111-8111-111111111111", &root)
        .unwrap();
    let bytes = brokkr_protocol::tree::count_tree(&orphan)
        .unwrap()
        .unwrap()
        .bytes;

    let mut report = empty_report();
    self::report(&mut report, &store);

    assert!(
        report.healthy,
        "doctor never refuses for an unrecorded tree"
    );
    let path = Safe::new(&orphan.display().to_string())
        .as_str()
        .to_string();
    assert_eq!(
        report.lines,
        [format!(
            "warn     tree: {path}: {bytes} bytes, named by no seat tree"
        )]
    );
    assert!(!report.lines[0].contains('\x1b'));
    assert!(orphan.is_dir(), "nothing removes an unrecorded tree");
}

/// What a report of `store` says, held healthy: no line is a warning, not
/// a refusal.
fn no_line(store: &Store) -> Vec<String> {
    let mut report = empty_report();
    self::report(&mut report, store);
    assert!(report.healthy);
    report.lines
}

#[test]
fn a_journal_with_no_seat_rows_or_a_root_that_cannot_be_listed_adds_no_line() {
    let empty = tempfile::tempdir().unwrap();
    let store = journal(empty.path());
    assert_eq!(no_line(&store), Vec::<String>::new());

    let dir = tempfile::tempdir().unwrap();
    let mut store = journal(dir.path());
    store
        .record_seat_tree(
            RUN,
            "effect",
            "11111111-1111-4111-8111-111111111111",
            &dir.path().join("missing").join("scratch"),
        )
        .unwrap();
    assert_eq!(no_line(&store), Vec::<String>::new());
}

#[test]
fn a_count_that_finds_the_tree_gone_or_refuses_is_a_warning() {
    let dir = tempfile::tempdir().unwrap();
    let _store = journal(dir.path());
    let mut report = empty_report();
    let gone = dir.path().join("gone");
    line(&mut report, &gone);
    let file = dir.path().join("file");
    std::fs::write(&file, b"x").unwrap();
    line(&mut report, &file);
    assert!(report.healthy);
    assert_eq!(
        report.lines,
        [
            format!(
                "warn     tree: {}: gone before the count, named by no seat tree",
                gone.display()
            ),
            format!(
                "warn     tree: the count failed: removing {}: Not a directory (os error 20)",
                file.display()
            ),
        ]
    );
    assert!(file.is_file(), "nothing removes a tree whose count refused");
}

#[test]
fn a_seat_trees_read_that_refuses_is_a_warning() {
    let dir = tempfile::tempdir().unwrap();
    let store = journal(dir.path());
    let db = store.path().to_path_buf();
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute_batch("DROP TABLE seat_trees")
        .unwrap();
    let mut report = empty_report();
    self::report(&mut report, &store);
    assert!(report.healthy);
    assert_eq!(
        report.lines,
        ["warn     tree: not read: sqlite: no such table: seat_trees"]
    );
}
