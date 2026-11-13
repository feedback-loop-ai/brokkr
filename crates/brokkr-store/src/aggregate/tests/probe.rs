//! The bounded readiness probe (decision 0089 slice 4, U5a): the probe
//! answers exactly what `Store::list_runs` answers over every shape a
//! journal can plant, its one unseeable shape named, and its cost does not
//! grow with the runs. The hard-stop statement's bytes are pinned too, so
//! reading `brokkr_core::policy::audit::STOP` there changes no SQL.

use brokkr_core::canonical::sha256_bytes;
use rusqlite::Connection;

use crate::aggregate::{migrate_unreadable, unreadable_intact, RUNS_UNREADABLE, UNREADABLE_INDEX};
use crate::test_support::plant_run;
use crate::tests::store as fresh;
use crate::Store;

/// The probe's answer and the listing's, as the oracle states it.
fn answers(store: &Store) -> (bool, bool) {
    (store.runs_readable().unwrap(), store.list_runs().is_ok())
}

fn assert_agree(store: &Store, what: &str) {
    let (probe, oracle) = answers(store);
    assert_eq!(probe, oracle, "{what}: probe {probe}, listing {oracle}");
}

/// The `runs` table without its columns' text affinity or `NOT NULL`
/// constraints, the shape a journal planted behind the store's back can
/// carry: the store's own writers never write one, and a written integer,
/// real or NULL would take the column's affinity otherwise.
fn relaxed_runs(conn: &Connection) {
    conn.execute_batch(
        "DROP TABLE runs;
         CREATE TABLE runs (
             run_id PRIMARY KEY, feature, bundle_name, manifest, created_at
         );",
    )
    .unwrap();
    crate::run_select::migrate(conn).unwrap();
    migrate_unreadable(conn).unwrap();
}

#[test]
fn the_probe_answers_what_the_listing_answers() {
    let (_dir, store) = fresh();
    plant_run(&store.conn, "run", "{}").unwrap();
    assert_eq!(answers(&store), (true, true), "a current journal");

    for column in ["run_id", "feature", "created_at"] {
        for value in ["NULL", "5", "1.5", "x'00'"] {
            let (_dir, store) = fresh();
            relaxed_runs(&store.conn);
            plant_run(&store.conn, "run", "{}").unwrap();
            store
                .conn
                .execute(&format!("UPDATE runs SET {column} = {value}"), [])
                .unwrap();
            assert!(!answers(&store).1, "{column} = {value} refuses the listing");
            assert_agree(&store, &format!("{column} = {value}"));
        }
    }
}

#[test]
fn the_probe_does_not_grow_with_the_runs() {
    let (_dir, store) = fresh();
    plant_run(&store.conn, "run-0", "{}").unwrap();
    let plan = super::explain(&store, RUNS_UNREADABLE.as_str(), []);
    assert!(
        plan.iter().any(|step| step.contains(UNREADABLE_INDEX)),
        "the probe reads the partial index: {plan:?}"
    );
    let step = store.runs_readable_steps().unwrap();
    assert!(step > 0, "the probe runs a statement");
    for index in 1..1_000 {
        plant_run(&store.conn, &format!("run-{index}"), "{}").unwrap();
    }
    assert_eq!(
        store.runs_readable_steps().unwrap(),
        step,
        "the probe visits its index's entries alone"
    );
    assert!(store.list_runs().is_ok());
    assert_eq!(store.list_runs().unwrap().len(), 1_000);
}

#[test]
fn a_journal_from_before_the_index_is_repaired_on_open() {
    let (dir, store) = fresh();
    plant_run(&store.conn, "run", "{}").unwrap();
    store
        .conn
        .execute_batch("DROP INDEX runs_unreadable")
        .unwrap();
    assert!(!unreadable_intact(&store.conn).unwrap());
    drop(store);

    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    assert!(
        unreadable_intact(&store.conn).unwrap(),
        "the open added the index"
    );
    assert!(store.runs_readable().unwrap());
    assert!(store.list_runs().is_ok());
}

#[test]
fn an_older_writers_schema_batch_keeps_the_index() {
    let (_dir, store) = fresh();
    store
        .conn
        .execute_batch(crate::schema::MIGRATION_V1)
        .unwrap();
    assert!(
        unreadable_intact(&store.conn).unwrap(),
        "no older repair drops an index on runs"
    );
}

#[test]
fn a_text_value_that_is_not_utf8_is_the_probes_one_blind_spot() {
    let (_dir, store) = fresh();
    plant_run(&store.conn, "run", "{}").unwrap();
    store
        .conn
        .execute("UPDATE runs SET feature = CAST(x'80' AS TEXT)", [])
        .unwrap();
    let stored: String = store
        .conn
        .query_row("SELECT typeof(feature) FROM runs", [], |row| row.get(0))
        .unwrap();
    assert_eq!(stored, "text", "SQLite stores the bytes as text");
    assert!(
        store.runs_readable().unwrap(),
        "the probe sees text and says the listing reads"
    );
    let refused = store
        .list_runs()
        .expect_err("the listing cannot read the bytes");
    assert_eq!(
        refused.to_string(),
        "sqlite: invalid utf-8 sequence of 1 bytes from index 0 at index 1"
    );
}

#[test]
fn the_hard_stop_statement_is_byte_identical() {
    let sql = super::super::runs_statement("");
    assert_eq!(
        sha256_bytes(sql.as_bytes()),
        "f78baabfd200baa1cc6692ce1bf1c6a04072cca67f697df152aafce354c2347e",
        "the statement is the one before this unit, byte for byte"
    );
}

/// L4: the probe's index is checked by its definition, not its name. An
/// index a build of another text made is not the one the probe reads
/// through: its answer is still exact, but it scans every run (508 steps
/// at 50, against 8). The open replaces it, so the bound survives a
/// changed definition.
#[test]
fn a_rebuilt_probe_index_of_another_text_is_replaced_on_open() {
    let (dir, store) = fresh();
    for index in 0..50 {
        plant_run(&store.conn, &format!("run-{index}"), "{}").unwrap();
    }
    store
        .conn
        .execute_batch(
            "DROP INDEX runs_unreadable;
             CREATE INDEX runs_unreadable ON runs (run_id) WHERE feature IS NULL;",
        )
        .unwrap();
    assert!(
        !unreadable_intact(&store.conn).unwrap(),
        "the text is not this build's"
    );
    assert_eq!(
        store.runs_readable_steps().unwrap(),
        508,
        "that predicate cannot answer the probe, so it scans every run"
    );
    assert!(store.runs_readable().unwrap(), "the answer is still exact");
    drop(store);

    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    assert!(
        unreadable_intact(&store.conn).unwrap(),
        "the open restored this build's index"
    );
    assert_eq!(
        store.runs_readable_steps().unwrap(),
        8,
        "the probe reads this build's index alone"
    );
    assert!(store.runs_readable().unwrap());
}
