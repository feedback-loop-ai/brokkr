//! A damaged journal is one refusal (#1000): a page SQLite reads as
//! corrupt, a file it reads as no database, a journal recording no
//! schema and an envelope that is not JSON each refuse as
//! [`StoreError::Damaged`], naming the file, and what is not damage keeps
//! its own words.

use std::path::{Path, PathBuf};

use crate::test_support::damaged::{damage_table, overwrite, replace_envelope, written, NOT_JSON};
use crate::{Damage, Store, StoreError};

/// What a damaged read refuses with: the journal, the damage and the
/// words, as the refusal of `journal` damaged by `damage` prints them,
/// `shown` being the path as the words escape it.
fn refusal(journal: &Path, shown: &str, damage: Damage) -> (PathBuf, Damage, String) {
    let words = format!(
        "the journal '{shown}' is damaged ({damage}); nothing was read from it; restore it \
         from a backup with `brokkr restore` and see the guide's \"A damaged journal\" (#1000)"
    );
    (journal.to_path_buf(), damage, words)
}

/// The damage `error` names and the journal it names, or what it was.
fn damaged(error: StoreError) -> (PathBuf, Damage, String) {
    let words = error.to_string();
    match error {
        StoreError::Damaged { journal, damage } => (journal, damage, words),
        other => panic!("not damage: {other:?}: {words}"),
    }
}

/// A journal at `forge.db` in a new directory, holding run `r1` as the
/// store's writer left it.
fn journal() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    written(&db, &["r1"]);
    (dir, db)
}

/// A page of the `events` table SQLite reads as corrupt refuses every read
/// of the run's events as the journal's damage, naming the file: `load`,
/// the opening and the opening that keeps what it reads.
#[test]
fn a_corrupt_page_is_the_journals_damage_by_name() {
    let (_dir, db) = journal();
    damage_table(&db, "events");
    let store = Store::open_read_only(&db).unwrap();
    let expected = refusal(&db, &db.display().to_string(), Damage::Malformed);
    assert!(expected
        .2
        .contains("(SQLite reads its disk image as malformed)"));
    assert_eq!(damaged(store.load("r1").unwrap_err()), expected);
    assert_eq!(damaged(store.open_run("r1").unwrap_err()), expected);
    assert_eq!(damaged(store.open_kept("r1").unwrap_err()), expected);
    // A page of the listing reads inside one snapshot, not under patience.
    let stands = store.stands_at("r1", (2, "x")).unwrap_err();
    assert_eq!(damaged(stands), expected);
}

/// A file whose header is no SQLite header refuses both opens as no
/// database, its path escaped as an operator's path is.
#[test]
fn a_file_that_is_no_database_is_refused_by_both_opens() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("for\nge.db");
    written(&db, &["r1"]);
    overwrite(&db, 0, b"not a journal at all");
    let shown = format!("{}/for\\nge.db", dir.path().display());
    let expected = refusal(&db, &shown, Damage::NotADatabase);
    assert!(expected.2.contains("(SQLite reads it as no database)"));
    let read_only = Store::open_read_only(&db).map(drop).unwrap_err();
    assert_eq!(damaged(read_only), expected);
    assert_eq!(damaged(Store::open(&db).map(drop).unwrap_err()), expected);
}

/// A journal whose `meta` table records no schema refuses the read-only
/// open as damage: what a damaged page read as before SQLite 3.51.3, and
/// what 0.12.0 printed as "Query returned no rows".
#[test]
fn a_journal_recording_no_schema_is_damage() {
    let (_dir, db) = journal();
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute("DELETE FROM meta WHERE key = 'database_schema'", [])
        .unwrap();
    let refused = Store::open_read_only(&db).map(drop).unwrap_err();
    let expected = refusal(&db, &db.display().to_string(), Damage::NoSchema);
    assert!(expected.2.contains("(it records no database schema)"));
    assert_eq!(damaged(refused), expected);
}

/// A stored envelope that is not JSON refuses `load`, the opening, the
/// opening that keeps what it reads, the suffix read and the one-event
/// read, naming the run and the row's seq.
#[test]
fn an_envelope_that_is_not_json_is_named_by_run_and_seq() {
    let (_dir, db) = journal();
    let first = Store::open_read_only(&db).unwrap().load("r1").unwrap()[0].clone();
    replace_envelope(&db, "r1", 2, NOT_JSON);
    let store = Store::open_read_only(&db).unwrap();
    let damage = Damage::Envelope {
        run_id: "r1".into(),
        seq: 2,
    };
    let expected = refusal(&db, &db.display().to_string(), damage);
    assert!(expected
        .2
        .contains("(run 'r1' holds an envelope at seq 2 that is not JSON)"));
    assert_eq!(damaged(store.load("r1").unwrap_err()), expected);
    assert_eq!(damaged(store.open_run("r1").unwrap_err()), expected);
    assert_eq!(damaged(store.open_kept("r1").unwrap_err()), expected);
    let after = store.load_after("r1", 1, &first.event_hash).unwrap_err();
    assert_eq!(damaged(after), expected);
    assert_eq!(damaged(store.last_event("r1", 2).unwrap_err()), expected);
}

/// Text whose start fits no envelope is still not JSON when the rest does
/// not parse: serde's shape error comes first, and the refusal stays the
/// envelope's damage.
#[test]
fn text_that_fails_the_shape_before_the_syntax_is_still_not_json() {
    let (_dir, db) = journal();
    let damage = Damage::Envelope {
        run_id: "r1".into(),
        seq: 2,
    };
    let expected = refusal(&db, &db.display().to_string(), damage);
    for text in ["{}trailing", r#"{"run_id": 0"#] {
        replace_envelope(&db, "r1", 2, text);
        let store = Store::open_read_only(&db).unwrap();
        assert_eq!(damaged(store.load("r1").unwrap_err()), expected, "{text}");
        assert_eq!(
            damaged(store.open_run("r1").unwrap_err()),
            expected,
            "{text}"
        );
    }
}

/// JSON the parser refuses for its own limits, nesting past its depth or a
/// number past `f64`, is intact bytes: it keeps serde's words, not damage.
#[test]
fn json_past_the_parsers_limits_is_not_damage() {
    let (_dir, db) = journal();
    let deep = format!(r#"{{"payload": {}{}}}"#, "[".repeat(200), "]".repeat(200));
    let cases = [
        (
            deep.as_str(),
            "json: recursion limit exceeded at line 1 column 139",
        ),
        (
            r#"{"payload": 1e400}"#,
            "json: number out of range at line 1 column 17",
        ),
    ];
    for (text, words) in cases {
        replace_envelope(&db, "r1", 2, text);
        let store = Store::open_read_only(&db).unwrap();
        let json = store.load("r1").unwrap_err();
        assert!(matches!(json, StoreError::Json(_)), "{json:?}");
        assert_eq!(json.to_string(), words);
    }
}

/// What is not damage keeps its own words: JSON that is no envelope, a
/// chain that does not verify, a peer's lock, a schema this build does not
/// read and a missing file.
#[test]
fn what_is_not_damage_keeps_its_own_words() {
    let (dir, db) = journal();
    let events = Store::open_read_only(&db).unwrap().load("r1").unwrap();
    replace_envelope(&db, "r1", 2, "{}");
    let store = Store::open_read_only(&db).unwrap();
    let json = store.load("r1").unwrap_err();
    assert!(matches!(json, StoreError::Json(_)), "{json:?}");
    assert_eq!(
        json.to_string(),
        "json: missing field `run_id` at line 1 column 2"
    );

    replace_envelope(&db, "r1", 2, &serde_json::to_string(&events[0]).unwrap());
    let chain = store.load("r1").unwrap_err();
    assert!(matches!(chain, StoreError::Chain(_)), "{chain:?}");
    assert_eq!(
        chain.to_string(),
        "chain: event 1: seq is not contiguous (expected 2)"
    );

    // In rollback mode a peer's exclusive lock shuts readers out.
    drop(store);
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.pragma_update(None, "journal_mode", "DELETE").unwrap();
    let mut held = Store::open_read_only(&db).unwrap();
    held.set_patience(std::time::Duration::ZERO).unwrap();
    conn.execute_batch("BEGIN EXCLUSIVE").unwrap();
    let contended = held.load("r1").unwrap_err();
    assert!(contended.is_contention(), "{contended:?}");
    conn.execute_batch("ROLLBACK").unwrap();

    conn.execute(
        "UPDATE meta SET value = '2' WHERE key = 'database_schema'",
        [],
    )
    .unwrap();
    let schema = Store::open_read_only(&db).map(drop).unwrap_err();
    assert!(
        matches!(schema, StoreError::SchemaMismatch { found: 2 }),
        "{schema:?}"
    );
    assert_eq!(schema.to_string(), "database schema 2 unsupported (want 1)");

    let absent = dir.path().join("absent.db");
    let missing = Store::open_read_only(&absent).map(drop).unwrap_err();
    assert!(matches!(missing, StoreError::Sqlite(_)), "{missing:?}");
    assert_eq!(
        missing.to_string(),
        format!("sqlite: unable to open database file: {}", absent.display())
    );
}
