//! The operations #354 put under `patiently`: listing, arrival and
//! adoption wait out their patience against a peer's lock and answer
//! typed contention naming themselves, never SQLite's `database is
//! locked`.

use std::path::Path;
use std::time::Duration;

use brokkr_store::{ImportError, Store, StoreError};
use rusqlite::Connection;
use serde_json::json;

const RUN: &str = "conclude-parked-hand-built";

/// A peer holding the lock its `begin` takes, which `IMMEDIATE` and
/// `EXCLUSIVE` take at once, until the test drops it.
fn lock_on(db: &Path, begin: &str) -> Connection {
    let holder = Connection::open(db).unwrap();
    holder.execute_batch(begin).unwrap();
    holder
}

fn contended_operation(error: &StoreError) -> Option<&'static str> {
    match error {
        StoreError::Contended { operation, .. } => Some(operation),
        _ => None,
    }
}

#[test]
fn listing_arrival_and_adoption_answer_typed_contention() {
    let export = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/journals")
        .join(format!("{RUN}.ndjson"));
    let ndjson = std::fs::read_to_string(&export).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("realm.db");
    let mut store = Store::open(&db).unwrap();
    store.set_patience(Duration::from_millis(50)).unwrap();

    let holder = lock_on(&db, "BEGIN IMMEDIATE");
    let refused = store.import_run(&ndjson, &json!({}), &export).unwrap_err();
    let ImportError::Store(adopting) = refused else {
        panic!("{refused}");
    };
    assert_eq!(contended_operation(&adopting), Some("import_run"));
    drop(holder);
    // Nothing was written, and the same call lands once the peer lets go.
    assert_eq!(store.list_runs().unwrap(), []);
    let adoption = store.import_run(&ndjson, &json!({}), &export).unwrap();
    drop(store);

    // Readers share a WAL journal with any writer, so the reads meet a
    // lock on the journal as an older tool left it, in rollback mode,
    // where an exclusive lock shuts readers out.
    Connection::open(&db)
        .unwrap()
        .pragma_update(None, "journal_mode", "DELETE")
        .unwrap();
    let mut reader = Store::open_read_only(&db).unwrap();
    reader.set_patience(Duration::from_millis(50)).unwrap();
    let holder = lock_on(&db, "BEGIN EXCLUSIVE");
    let listing = reader.list_runs().unwrap_err();
    assert_eq!(contended_operation(&listing), Some("list_runs"));
    let arriving = reader.arrival(RUN).unwrap_err();
    assert_eq!(contended_operation(&arriving), Some("arrival"));
    drop(holder);
    assert_eq!(reader.list_runs().unwrap().len(), 1);
    assert_eq!(reader.arrival(RUN).unwrap(), Some(adoption.arrival));
}
