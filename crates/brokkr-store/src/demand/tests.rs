//! The reads admission sizes an entry by (decision 0125): a recipe's
//! latest runs and the events of theirs that size it, the runs no terminal
//! event ends, and an open of another journal that leaves its directory as
//! it found it.

use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::params;
use serde_json::{json, Value};

use super::{UsageEvent, UsageRow};
use crate::{Store, StoreError};

/// Plant a run of `recipe` created at `at`, past every fence: the reads
/// here are of rows, so no chain is needed.
fn run(store: &Store, run: &str, recipe: &str, at: &str) {
    store
        .conn
        .execute(
            "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at)
             VALUES (?1, 'feat', ?2, '{}', ?3)",
            params![run, recipe, at],
        )
        .unwrap();
}

/// Plant `events` on `run`, each an envelope of its type and payload.
fn events(store: &Store, run: &str, events: &[(&str, Value)]) {
    for (seq, (kind, payload)) in events.iter().enumerate() {
        let envelope = json!({"type": kind, "payload": payload}).to_string();
        store
            .conn
            .execute(
                "INSERT INTO events (run_id, seq, event_hash, envelope) VALUES (?1, ?2, 'h', ?3)",
                params![run, seq as i64 + 1, envelope],
            )
            .unwrap();
    }
}

fn usage(peak: u64) -> Value {
    json!({"peak_memory_bytes": peak, "mean_memory_bytes": 1, "peak_load_millicores": 2,
           "mean_load_millicores": 1, "cpu_ms": 1, "wall_ms": 1})
}

fn ended(peak: u64) -> UsageEvent {
    UsageEvent::Ended(usage(peak).to_string())
}

fn row(run: &str, recipe: &str, at: &str, event: UsageEvent) -> UsageRow {
    UsageRow {
        run: run.into(),
        recipe: recipe.into(),
        created_at: at.into(),
        event,
    }
}

/// Each recipe's latest runs alone, newest first, and of their events only
/// a phase entered and an ending that carries usage: an ending without
/// it, another event, and an envelope that only names `usage` in its text
/// are not read.
#[test]
fn the_history_reads_each_recipes_latest_runs_and_only_the_events_that_size_it() {
    let (_dir, store) = crate::tests::store();
    for (name, recipe, at) in [
        ("a1", "self", "2026-11-01T00:00:00Z"),
        ("a2", "self", "2026-11-02T00:00:00Z"),
        ("a3", "self", "2026-11-03T00:00:00Z"),
        ("b1", "verify", "2026-11-01T00:00:00Z"),
    ] {
        run(&store, name, recipe, at);
    }
    events(&store, "a1", &[("phase/entered", json!({"phase": "old"}))]);
    events(
        &store,
        "a2",
        &[
            ("phase/entered", json!({"phase": "implement"})),
            ("effect/succeeded", json!({"usage": usage(10)})),
        ],
    );
    events(
        &store,
        "a3",
        &[
            ("phase/entered", json!({"phase": "verify"})),
            ("effect/failed", json!({"result": "no usage"})),
            ("effect/requested", json!({"note": "\"usage\""})),
            ("effect/indeterminate", json!({"usage": usage(30)})),
        ],
    );
    events(
        &store,
        "b1",
        &[("effect/succeeded", json!({"usage": usage(5)}))],
    );
    let read = store.usage_history(2).unwrap();
    let at = |day: &str| format!("2026-11-0{day}T00:00:00Z");
    assert_eq!(
        read,
        vec![
            row("a3", "self", &at("3"), UsageEvent::Entered("verify".into())),
            row("a3", "self", &at("3"), ended(30)),
            row(
                "a2",
                "self",
                &at("2"),
                UsageEvent::Entered("implement".into())
            ),
            row("a2", "self", &at("2"), ended(10)),
            row("b1", "verify", &at("1"), ended(5)),
        ]
    );
}

/// A run a terminal event ends is not unended; one without, parked or
/// running alike, is, with its recipe.
#[test]
fn the_unended_runs_are_those_no_terminal_event_ends() {
    let (_dir, store) = crate::tests::store();
    run(&store, "done", "self", "2026-11-01T00:00:00Z");
    run(&store, "going", "verify", "2026-11-02T00:00:00Z");
    events(&store, "done", &[("run/completed", json!({}))]);
    events(
        &store,
        "going",
        &[("phase/entered", json!({"phase": "implement"}))],
    );
    let unended = store.unended_runs().unwrap();
    assert_eq!(
        unended,
        vec![super::Unended {
            run: "going".into(),
            recipe: "verify".into()
        }]
    );
}

/// Every file of `dir`, by name, with its bytes.
fn files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().into_owned();
            (name, std::fs::read(entry.path()).unwrap())
        })
        .collect()
}

/// A closed journal is read through the untouched open, and its directory
/// is byte for byte as it was: no `-wal`, `-shm` or lock. A path with
/// nothing at it is refused and left with nothing.
#[test]
fn the_untouched_open_reads_a_journal_and_leaves_its_directory_byte_identical() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("weird %?#name.db");
    {
        let store = Store::open(&journal).unwrap();
        run(&store, "a", "self", "2026-11-01T00:00:00Z");
        events(
            &store,
            "a",
            &[("effect/succeeded", json!({"usage": usage(7)}))],
        );
    }
    let before = files(dir.path());
    assert_eq!(before.len(), 1, "{:?}", before.keys());
    let read = Store::open_untouched(&journal)
        .unwrap()
        .usage_history(20)
        .unwrap();
    assert_eq!(read.len(), 1);
    assert_eq!(files(dir.path()), before);
    let absent = dir.path().join("absent.db");
    assert!(matches!(
        Store::open_untouched(&absent),
        Err(StoreError::Sqlite(_))
    ));
    assert_eq!(files(dir.path()), before);
}
