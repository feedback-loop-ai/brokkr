//! Doctor's unrecorded-tree line (decision 0104 ruling 6): a directory
//! under a recorded root that no seat-tree row names is listed with its
//! size, a hostile name is stripped, a count that refuses, finds the tree
//! gone or reads only a lower bound warns, and nothing is removed.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use brokkr_core::envelope::EventType;
use brokkr_store::Store;
use serde_json::json;

use super::*;
use crate::render::Safe;

const RUN: &str = "run-doctor";

/// A temporary directory under the canonical temporary root: macOS's
/// `/var` is a link to `/private/var`, and R5 keeps every tree under a
/// root that is not its own canonical path.
fn tempdir() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}

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

/// Record `attempt` as a succeeded attempt of `RUN`, so the sweep would
/// remove a tree its row names rather than keep it.
fn succeeded(store: &mut Store, attempt: &str) {
    let phase = json!({"phase": "work"});
    store
        .append_next(RUN, EventType::PhaseEntered, phase, None, None)
        .unwrap();
    let requested = json!({"effect_id": "effect", "seat": "worker"});
    store
        .append_next(RUN, EventType::EffectRequested, requested, None, None)
        .unwrap();
    let began = json!({"effect_id": "effect", "attempt_id": attempt});
    store
        .append_next(
            RUN,
            EventType::EffectStarted,
            began,
            None,
            Some(attempt.to_string()),
        )
        .unwrap();
    let result = json!({"effect_id": "effect", "result": {"result": "ok"}});
    store
        .append_next(
            RUN,
            EventType::EffectSucceeded,
            result,
            None,
            Some(attempt.to_string()),
        )
        .unwrap();
}

#[test]
fn a_tree_no_row_names_is_listed_with_its_size_and_never_removed() {
    let dir = tempdir();
    let mut store = journal(dir.path());
    succeeded(&mut store, "11111111-1111-4111-8111-111111111111");
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
fn a_journal_with_no_seat_rows_adds_no_line_and_a_gone_root_is_reported() {
    let empty = tempdir();
    let store = journal(empty.path());
    assert_eq!(no_line(&store), Vec::<String>::new());

    let dir = tempdir();
    let mut store = journal(dir.path());
    succeeded(&mut store, "11111111-1111-4111-8111-111111111111");
    let root = dir.path().join("missing").join("scratch");
    store
        .record_seat_tree(RUN, "effect", "11111111-1111-4111-8111-111111111111", &root)
        .unwrap();
    let tree = root.join("11111111-1111-4111-8111-111111111111");
    assert_eq!(
        no_line(&store),
        [format!(
            "warn     tree: {}: gone before the count, kept: its root is gone",
            tree.display()
        )]
    );
}

#[test]
fn a_count_that_finds_the_tree_gone_or_refuses_is_a_warning() {
    let dir = tempdir();
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

/// A kept tree whose count finds it gone or refuses still names the tree,
/// its reason and the count's words: doctor never drops a kept tree.
#[test]
fn a_kept_count_that_finds_the_tree_gone_or_refuses_still_names_its_reason() {
    let dir = tempdir();
    let mut report = empty_report();
    let gone = dir.path().join("gone");
    kept_line(&mut report, &gone, "its attempt ended failed");
    let file = dir.path().join("file");
    std::fs::write(&file, b"x").unwrap();
    kept_line(&mut report, &file, "its attempt ended failed");
    assert!(report.healthy);
    assert_eq!(
        report.lines,
        [
            format!(
                "warn     tree: {}: gone before the count, kept: its attempt ended failed",
                gone.display()
            ),
            format!(
                "warn     tree: {}: the count failed: removing {}: Not a directory (os error \
                 20), kept: its attempt ended failed",
                file.display(),
                file.display()
            ),
        ]
    );
    assert!(file.is_file(), "nothing removes a tree whose count refused");
}

/// A kept tree's path and reason pass through `safe`: a hostile scratch
/// name and a hostile reason leave no control byte in doctor's line.
#[test]
fn a_hostile_kept_name_and_reason_are_stripped_from_the_line() {
    let dir = tempdir();
    let tree = dir.path().join("kept\x1b[2Jname");
    std::fs::create_dir_all(&tree).unwrap();
    let reason = "its attempt ended \x1b[2Jfailed";
    let mut report = empty_report();
    kept_line(&mut report, &tree, reason);

    let path = Safe::new(&tree.display().to_string()).as_str().to_string();
    let reason = Safe::new(reason).as_str().to_string();
    let bytes = brokkr_protocol::tree::count_tree(&tree)
        .unwrap()
        .unwrap()
        .bytes;
    assert_eq!(
        report.lines,
        [format!(
            "warn     tree: {path}: {bytes} bytes, kept: {reason}"
        )]
    );
    assert!(!report.lines[0].contains('\x1b'));
}

/// A tree with one sealed directory, and the count that reads only its
/// lower bound. The caller restores the directory's mode.
fn sealed_tree(dir: &Path) -> (PathBuf, brokkr_protocol::tree::Counted) {
    let tree = dir.join("tree");
    let sealed = tree.join("sealed");
    std::fs::create_dir_all(&sealed).unwrap();
    std::fs::write(sealed.join("data"), b"x").unwrap();
    std::fs::set_permissions(&sealed, std::fs::Permissions::from_mode(0o000)).unwrap();
    let counted = brokkr_protocol::tree::count_tree(&tree).unwrap().unwrap();
    (tree, counted)
}

/// Assert the report holds exactly `line` and stays healthy.
fn assert_line(report: &Report, line: String) {
    assert!(report.healthy);
    assert_eq!(report.lines, [line]);
}

/// Report `store` with `statement` applied to its journal first, and hand
/// back the one warning line it adds.
fn one_warning_after(store: &Store, statement: &str) -> String {
    let db = store.path().to_path_buf();
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute_batch(statement)
        .unwrap();
    let mut report = empty_report();
    self::report(&mut report, store);
    assert!(report.healthy);
    assert_eq!(report.lines.len(), 1);
    report.lines[0].clone()
}

/// A kept-tree read that refuses is a warning, never a silent drop: the
/// sweep's derivation is surfaced the way the row read is. Read directly,
/// because the row read also needs the sweep table and would refuse first.
#[test]
fn a_kept_trees_read_that_refuses_is_a_warning() {
    let dir = tempdir();
    let store = journal(dir.path());
    let db = store.path().to_path_buf();
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute_batch("DROP TABLE tree_sweeps")
        .unwrap();
    let mut report = empty_report();
    report_kept(&mut report, &store);

    assert!(report.healthy);
    assert_eq!(report.lines.len(), 1);
    let line = &report.lines[0];
    assert!(
        line.starts_with("warn     tree: kept trees not read: "),
        "{line}"
    );
    assert!(line.contains("tree_sweeps"), "{line}");
}

#[test]
fn a_lower_bound_count_says_at_least_and_removes_nothing() {
    if rustix::process::geteuid().is_root() {
        return;
    }
    let dir = tempdir();
    let (tree, counted) = sealed_tree(dir.path());
    assert!(counted.unreadable.is_some());
    let mut unrecorded = empty_report();
    line(&mut unrecorded, &tree);
    let mut kept = empty_report();
    kept_line(&mut kept, &tree, "its attempt ended indeterminate");
    std::fs::set_permissions(tree.join("sealed"), std::fs::Permissions::from_mode(0o700)).unwrap();

    assert_line(
        &unrecorded,
        format!(
            "warn     tree: {}: at least {} bytes, named by no seat tree",
            tree.display(),
            counted.bytes
        ),
    );
    assert_line(
        &kept,
        format!(
            "warn     tree: {}: at least {} bytes, kept: its attempt ended indeterminate",
            tree.display(),
            counted.bytes
        ),
    );
    assert!(tree.is_dir(), "nothing removes an unreadable tree");
}

#[test]
fn a_root_that_is_not_its_own_canonical_path_is_named() {
    let dir = tempdir();
    let mut store = journal(dir.path());
    let real = dir.path().join("real");
    let root = real.join("scratch");
    std::fs::create_dir_all(root.join("orphan")).unwrap();
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let named_root = link.join("scratch");
    store
        .record_seat_tree(
            RUN,
            "effect",
            "11111111-1111-4111-8111-111111111111",
            &named_root,
        )
        .unwrap();

    let mut report = empty_report();
    self::report(&mut report, &store);

    assert!(report.healthy);
    assert_eq!(
        report.lines,
        [
            format!(
                "warn     tree: {}: not its own canonical path, so the sweep keeps its trees",
                named_root.display()
            ),
            format!(
                "warn     tree: {}: gone before the count, kept: its root is not its own \
                 canonical path",
                named_root
                    .join("11111111-1111-4111-8111-111111111111")
                    .display()
            ),
        ]
    );
}

#[test]
fn an_entry_that_vanishes_before_its_stat_adds_no_line() {
    let dir = tempdir();
    let root = dir.path().join("scratch");
    std::fs::create_dir_all(root.join("orphan")).unwrap();
    let mut report = empty_report();

    list_with(&mut report, &root, &Named::default(), |_| {
        Err(std::io::Error::from(std::io::ErrorKind::NotFound))
    });

    assert!(report.healthy);
    assert!(report.lines.is_empty());
}

#[test]
fn an_unreadable_root_adds_no_line_and_is_left_alone() {
    let dir = tempdir();
    let root = dir.path().join("scratch");
    let orphan = root.join("orphan");
    std::fs::create_dir_all(&orphan).unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o300)).unwrap();
    let mut report = empty_report();

    list(&mut report, &root, &Named::default());

    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(report.healthy);
    assert!(report.lines.is_empty());
    assert!(
        orphan.is_dir(),
        "nothing removes a tree under an unreadable root"
    );
}

#[test]
fn a_tree_recreated_at_a_closed_rows_path_is_named_with_its_size() {
    let dir = tempdir();
    let mut store = journal(dir.path());
    let root = dir.path().join("scratch");
    std::fs::create_dir_all(&root).unwrap();
    let attempt = "11111111-1111-4111-8111-111111111111";
    let tree = root.join(attempt);
    store
        .record_seat_tree(RUN, "effect", attempt, &root)
        .unwrap();
    let row = store.seat_trees(RUN).unwrap()[0].clone();
    let held = brokkr_runtime::take_dispatch(&mut store).unwrap().unwrap();
    store
        .record_tree_sweep(
            brokkr_store::Swept::Scratch(row.seq),
            &tree,
            None,
            held.generation.0,
        )
        .unwrap();
    std::fs::create_dir_all(tree.join("home")).unwrap();
    std::fs::write(tree.join("home").join("cache"), b"x").unwrap();
    let bytes = brokkr_protocol::tree::count_tree(&tree)
        .unwrap()
        .unwrap()
        .bytes;

    let path = Safe::new(&tree.display().to_string()).as_str().to_string();
    assert_eq!(
        no_line(&store),
        [format!(
            "warn     tree: {path}: {bytes} bytes, named only by a closed seat tree"
        )]
    );
    assert!(tree.is_dir(), "doctor removes nothing");
}

#[test]
fn a_seat_trees_read_that_refuses_is_a_warning() {
    let dir = tempdir();
    let store = journal(dir.path());
    assert_eq!(
        one_warning_after(&store, "DROP TABLE seat_trees"),
        "warn     tree: not read: sqlite: no such table: seat_trees"
    );
}
