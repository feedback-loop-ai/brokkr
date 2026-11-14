//! `brokkr sweep` (decision 0104 ruling 9, as R3 amends it): a dry run
//! opens read-only, writes no row, holder or migration, and lists what a
//! pass would remove; a pass takes the dispatch lock, removes the tree and
//! paints the same lines through `Safe`; and a second sweeper is refused
//! by the lock, by name, recording nothing.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use brokkr_core::envelope::EventType;
use brokkr_runtime::DispatcherError;
use brokkr_store::{Store, Swept as Closed};
use clap::Parser;
use serde_json::json;

use super::*;
use crate::tests::env_guard::EnvGuard;
use crate::{Cli, Cmd};

const RUN: &str = "run-sweep";
const EFFECT: &str = "effect-sweep";
const ATTEMPT: &str = "11111111-1111-4111-8111-111111111111";

/// A machine of the test's own: `TMPDIR` and `XDG_STATE_HOME` at a
/// directory the test owns, so the hands half and the verb's places read
/// nothing of the host's. The directory is kept, not deleted: another
/// test's `tempfile::tempdir()` can stand inside it while the guard holds
/// `TMPDIR`, and deleting it would take that test's journal with it.
fn own_host() -> (EnvGuard, PathBuf) {
    let mut env = EnvGuard::lock();
    let places = tempfile::Builder::new()
        .prefix("brokkr-sweep-")
        .tempdir()
        .unwrap()
        .keep();
    let places = std::fs::canonicalize(&places).unwrap();
    env.set("TMPDIR", &places);
    env.set("XDG_STATE_HOME", places.join("state"));
    (env, places)
}

fn db(dir: &Path) -> PathBuf {
    dir.join("forge.db")
}

fn append(
    store: &mut Store,
    event_type: EventType,
    payload: serde_json::Value,
    attempt: Option<&str>,
) {
    store
        .append_next(RUN, event_type, payload, None, attempt.map(str::to_string))
        .unwrap();
}

/// A run whose effect succeeded on `ATTEMPT`.
fn succeeded_run(store: &mut Store) {
    store
        .create_run(RUN, "a feature", "self", &json!({}))
        .unwrap();
    append(
        store,
        EventType::RunStarted,
        json!({"feature": "a feature", "manifest": {}}),
        None,
    );
    append(
        store,
        EventType::PhaseEntered,
        json!({"phase": "work"}),
        None,
    );
    append(
        store,
        EventType::EffectRequested,
        json!({"effect_id": EFFECT, "seat": "worker"}),
        None,
    );
    append(
        store,
        EventType::EffectStarted,
        json!({"effect_id": EFFECT, "attempt_id": ATTEMPT}),
        Some(ATTEMPT),
    );
    append(
        store,
        EventType::EffectSucceeded,
        json!({"effect_id": EFFECT, "result": {"result": "ok"}}),
        Some(ATTEMPT),
    );
}

fn scratch(root: &Path) -> PathBuf {
    std::fs::create_dir_all(root).unwrap();
    root.to_path_buf()
}

fn plant(root: &Path) -> PathBuf {
    let tree = root.join(ATTEMPT);
    std::fs::create_dir_all(tree.join("tmp")).unwrap();
    std::fs::write(tree.join("tmp").join("scratch"), b"xy").unwrap();
    tree
}

fn seat(store: &mut Store, root: &Path) {
    store.record_seat_tree(RUN, EFFECT, ATTEMPT, root).unwrap();
}

fn parsed(database: &Path, dry_run: bool) -> SweepArgs {
    let mut argv = vec!["brokkr", "sweep"];
    if dry_run {
        argv.push("--dry-run");
    }
    argv.extend(["--db", database.to_str().unwrap()]);
    let Cmd::Sweep(args) = Cli::try_parse_from(argv).unwrap().command else {
        panic!("parsed as a sweep command")
    };
    args
}

fn completed() -> ExitCode {
    ExitCode::from(Exit::Completed)
}

fn count(db: &Path, sql: &str) -> i64 {
    rusqlite::Connection::open(db)
        .unwrap()
        .query_row(sql, [], |row| row.get(0))
        .unwrap()
}

/// A run whose effect closed, its tree planted, and the bytes it holds.
fn ready(name: &str) -> (EnvGuard, PathBuf, PathBuf, Store, PathBuf, u64) {
    let (env, places) = own_host();
    let database = db(&places);
    let root = scratch(&places.join(name).join("scratch"));
    let mut store = Store::open(&database).unwrap();
    succeeded_run(&mut store);
    seat(&mut store, &root);
    let tree = plant(&root);
    let freed = brokkr_protocol::tree::count_tree(&tree)
        .unwrap()
        .unwrap()
        .bytes;
    (env, places, database, store, tree, freed)
}

#[test]
fn a_dry_run_writes_nothing_and_lists_what_a_pass_would_remove() {
    let (_env, places, database, store, tree, freed) = ready("run");
    drop(store);
    // A version 5 queue holds `seat_trees` without the sweep table: a read
    // must list the rows and leave the version and the table as it found.
    rusqlite::Connection::open(&database)
        .unwrap()
        .execute_batch(
            "DROP TABLE tree_sweeps;
             UPDATE meta SET value = '5' WHERE key = 'queue_schema';",
        )
        .unwrap();

    let mut out = Vec::new();
    let exit = sweep(places.as_path(), parsed(&database, true), &mut out).unwrap();

    assert_eq!(exit, completed());
    let said = String::from_utf8(out).unwrap();
    assert_eq!(
        said,
        format!(
            "sweep: would remove {}: {freed} bytes\nsweep: would free {freed} bytes\n",
            tree.display()
        )
    );
    assert!(tree.exists(), "a dry run removes nothing");
    assert_eq!(
        count(
            &database,
            "SELECT COUNT(*) FROM meta WHERE key = 'queue_schema' AND value = '5'"
        ),
        1,
        "a dry run does not migrate"
    );
    assert_eq!(
        count(
            &database,
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'tree_sweeps'"
        ),
        0,
        "a dry run writes no table"
    );
    assert_eq!(
        count(&database, "SELECT COUNT(*) FROM dispatch_holders"),
        0,
        "a dry run takes no hold"
    );
}

#[test]
fn a_pass_removes_records_and_prints_its_lines() {
    // A seat can write a control byte into a tree name: the line must not
    // carry it to the operator's terminal (#468).
    let (_env, places, database, store, tree, freed) = ready("run\x1b[2J");

    let mut out = Vec::new();
    let exit = sweep(places.as_path(), parsed(&database, false), &mut out).unwrap();

    assert_eq!(exit, completed());
    let said = String::from_utf8(out).unwrap();
    let named = Safe::new(&tree.display().to_string()).as_str().to_string();
    assert_eq!(
        said,
        format!("sweep: removed {named}: {freed} bytes\nsweep: freed {freed} bytes\n")
    );
    assert!(!said.contains('\x1b'), "a control byte reached the output");
    assert!(said.contains("run[2J"), "{said}");
    assert!(!tree.exists());
    let seq = store.seat_trees(RUN).unwrap()[0].seq;
    let rows = store.tree_sweeps().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, Closed::Scratch(seq));
    assert_eq!(rows[0].path, tree);
    assert_eq!(rows[0].bytes, Some(freed as i64));
    assert_eq!(rows[0].generation, 1);
}

#[test]
fn a_second_sweeper_is_refused_by_the_lock_by_name_recording_nothing() {
    let (_env, places, database, mut store, tree, _) = ready("run");
    let held = brokkr_runtime::take_dispatch(&mut store).unwrap().unwrap();

    let refused = sweep(places.as_path(), parsed(&database, false), &mut Vec::new()).unwrap_err();

    assert!(
        matches!(
            refused.downcast_ref::<DispatcherError>(),
            Some(DispatcherError::Dispatched(at, Some(_))) if at == &database
        ),
        "{refused:?}"
    );
    assert!(tree.exists(), "the refused sweeper removed nothing");
    assert_eq!(
        count(&database, "SELECT COUNT(*) FROM dispatch_holders"),
        1,
        "the refused sweeper recorded no holder"
    );
    assert_eq!(
        count(&database, "SELECT COUNT(*) FROM tree_sweeps"),
        0,
        "the refused sweeper recorded no sweep"
    );
    drop(held);
}
