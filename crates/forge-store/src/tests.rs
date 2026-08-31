use super::*;
use serde_json::json;

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    (dir, store)
}

#[test]
fn append_load_roundtrip_and_chain() {
    let (_dir, mut store) = store();
    store
        .create_run("r1", "feat", "self", &json!({"files": {}}))
        .unwrap();
    store
        .append_next(
            "r1",
            EventType::RunStarted,
            json!({"feature": "feat", "manifest": {}}),
            None,
            None,
        )
        .unwrap();
    store
        .append_next(
            "r1",
            EventType::PhaseEntered,
            json!({"phase": "intake"}),
            None,
            None,
        )
        .unwrap();
    let events = store.load("r1").unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].previous_hash, events[0].event_hash);
    let state = forge_core::fold(&events).unwrap();
    assert_eq!(state.phase.as_deref(), Some("intake"));
}

#[test]
fn events_are_append_only() {
    let (_dir, mut store) = store();
    store
        .create_run("r1", "feat", "self", &json!({"files": {}}))
        .unwrap();
    store
        .append_next(
            "r1",
            EventType::RunStarted,
            json!({"feature": "feat", "manifest": {}}),
            None,
            None,
        )
        .unwrap();
    let update = store.conn.execute("UPDATE events SET envelope = 'x'", []);
    assert!(update.is_err(), "update must be rejected by trigger");
    let delete = store.conn.execute("DELETE FROM events", []);
    assert!(delete.is_err(), "delete must be rejected by trigger");
}

#[test]
fn duplicate_run_and_missing_run_fail() {
    let (_dir, mut store) = store();
    store
        .create_run("r1", "feat", "self", &json!({"files": {}}))
        .unwrap();
    assert!(matches!(
        store.create_run("r1", "feat", "self", &json!({"files": {}})),
        Err(StoreError::RunExists(_))
    ));
    assert!(matches!(
        store.load("nope"),
        Err(StoreError::RunNotFound(_))
    ));
}

#[test]
fn export_verifies_offline() {
    let (_dir, mut store) = store();
    store
        .create_run("r1", "feat", "self", &json!({"files": {}}))
        .unwrap();
    store
        .append_next(
            "r1",
            EventType::RunStarted,
            json!({"feature": "feat", "manifest": {}}),
            None,
            None,
        )
        .unwrap();
    store
        .append_next(
            "r1",
            EventType::PhaseEntered,
            json!({"phase": "intake"}),
            None,
            None,
        )
        .unwrap();
    let ndjson = store.export_ndjson("r1").unwrap();
    let state = verify_export(&ndjson).unwrap();
    assert_eq!(state.seq, 2);

    // A tampered export fails closed.
    let tampered = ndjson.replace("intake", "ship");
    assert!(matches!(
        verify_export(&tampered),
        Err(VerifyError::Chain(_))
    ));
}

#[test]
fn schema_head_listing_and_append_conflict_fail_closed() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let mut store = Store::open(&db).unwrap();
    store
        .create_run("r1", "feat", "self", &json!({"files": {}}))
        .unwrap();

    assert_eq!(store.list_runs().unwrap().len(), 1);
    assert_eq!(store.head_hash("r1").unwrap(), (0, ZERO_HASH.to_string()));

    store
        .conn
        .execute_batch(
            "CREATE TEMP TRIGGER refuse_test_append BEFORE INSERT ON events
             BEGIN SELECT RAISE(IGNORE); END;",
        )
        .unwrap();
    assert!(matches!(
        store.append_next("r1", EventType::RunStarted, json!({}), None, None),
        Err(StoreError::AppendConflict { seq: 1 })
    ));

    store
        .conn
        .execute(
            "UPDATE meta SET value = 'not-a-schema' WHERE key = 'database_schema'",
            [],
        )
        .unwrap();
    drop(store);
    assert!(matches!(
        Store::open(&db),
        Err(StoreError::SchemaMismatch { found: 0 })
    ));
}

/// The journal the fold used to refuse, folded end to end. This is the
/// verbatim export of a live run (105 events, read-only evidence): the
/// operator's `stop` was accepted at seq 93 while the verify seat's
/// effect was in flight, the attempt kept checkpointing to seq 104, and
/// the effect succeeded at seq 105. The engine accepted and recorded
/// all of it — the journal is the authority (decision 0001) — so the
/// whole export must verify, chain, and fold without a refusal.
#[test]
fn the_operator_stop_that_came_mid_flight_folds_end_to_end() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/journals/tui-graph-the-selection-box-gets-80f98deb.ndjson");
    let ndjson = std::fs::read_to_string(&fixture).unwrap();
    let state = verify_export(&ndjson).unwrap();

    assert_eq!(state.run_id, "tui-graph-the-selection-box-gets-80f98deb");
    assert_eq!(state.seq, 105, "the export's last event");
    // No terminal event was captured in this export: the run is still
    // running, in the phase whose seat was working when it was stopped.
    assert_eq!(state.status, forge_core::fold::Status::Running);
    assert_eq!(state.phase.as_deref(), Some("verify"));
    // The accepted stop rode the in-flight attempt to its boundary:
    // instead of the succeeded effect's normal `Decide`, the run
    // concludes per the operator's command.
    assert_eq!(state.cursor, forge_core::fold::Cursor::Stop);
    assert!(!state.riding_stop, "the riding command is spent");
    assert_eq!(state.pending_command, None, "the stop was disposed of");
}

/// The placeholder contract: the same original path always becomes the
/// same placeholder, distinct paths stay distinct, numbering follows
/// first appearance journal-wide. Only absolute paths are rewritten —
/// relative paths, ratios, and bare or trailing slashes survive
/// verbatim, while a `//`-rooted network share is machine detail and
/// moves — and nothing outside the payload moves.
#[test]
fn redacted_export_rewrites_absolute_paths_to_stable_placeholders() {
    let ndjson = concat!(
        r#"{"payload":{"driver":"/home/alice/forge/target/debug/forge","args":["/home/alice/forge/README.md","docs/a.md"],"operator":"alice"},"seq":1}"#,
        "\n",
        "   \n",
        r#"{"payload":{"checkpoint":{"note":null,"target":"/home/alice/forge/README.md","turn":4},"text":"ran /home/alice/forge/target/debug/forge for alice (not xalice, alicex, or alice.txt) on a/b, 3 / 4, //share/x, dir/, the /home/ prefix, /Users/bob/Library, end /"}}"#,
        "\n",
        r#"{"seq":3}"#,
        "\n",
    );
    let redacted = redact_export(ndjson).unwrap();
    let lines: Vec<&str> = redacted.lines().collect();
    assert_eq!(lines.len(), 3, "the blank line drops, nothing else");
    // Payload fields walk in sorted key order, so `args` numbers first.
    assert_eq!(
        lines[0],
        r#"{"payload":{"args":["[path-1]","docs/a.md"],"driver":"[path-2]","operator":"[user-1]"},"seq":1}"#
    );
    assert_eq!(
        lines[1],
        r#"{"payload":{"checkpoint":{"note":null,"target":"[path-1]","turn":4},"text":"ran [path-2] for [user-1] (not xalice, alicex, or alice.txt) on a/b, 3 / 4, [path-3], dir/, the [path-4] prefix, [path-5], end /"}}"#
    );
    assert_eq!(
        lines[2], r#"{"seq":3}"#,
        "a line without a payload passes through"
    );
    // Same input, same output: redaction is deterministic.
    assert_eq!(redacted, redact_export(ndjson).unwrap());

    // A journal that leaks nothing comes back verbatim.
    let clean = "{\"payload\":{\"note\":\"clean\"}}\n";
    assert_eq!(redact_export(clean).unwrap(), clean);
}

/// The recognizer is cross-platform and quote-aware: drive-letter and
/// UNC paths move like POSIX ones, a quoted path carries its spaces to
/// the closing quote, scheme URLs survive as the declared bound, and a
/// non-ASCII username bounds like an ASCII one.
#[test]
fn redaction_covers_windows_shapes_quotes_and_non_ascii_usernames() {
    let ndjson = concat!(
        r#"{"payload":{"a":"C:/Users/dana/forge/x.rs","b":"C:\\Users\\dana\\forge","c":"\\\\srv\\share\\y","d":"'/home/ana maria/My Files/z.txt' stays whole","e":"see file:///home/dana/leak and https://example.com/ok","f":"/home/józef/tool ran for józef","g":"PATH=/usr/bin:/home/józef/bin"}}"#,
        "\n",
    );
    let redacted = redact_export(ndjson).unwrap();
    let line: serde_json::Value = serde_json::from_str(redacted.lines().next().unwrap()).unwrap();
    let field = |k: &str| line["payload"][k].as_str().unwrap().to_string();
    assert_eq!(field("a"), "[path-1]");
    assert_eq!(field("b"), "[path-2]");
    assert_eq!(field("c"), "[path-3]");
    assert_eq!(field("d"), "'[path-4]' stays whole");
    assert_eq!(
        field("e"),
        "see file:///home/[user-1]/leak and https://example.com/ok",
        "scheme URLs survive as the declared bound, but a username never does"
    );
    assert_eq!(field("f"), "[path-5] ran for [user-3]");
    assert_eq!(
        field("g"),
        "PATH=[path-6]:[path-7]",
        "colon-separated path lists split at the colon"
    );
    assert!(!redacted.contains("dana") && !redacted.contains("józef"));
}

#[test]
fn redacting_garbage_is_a_parse_refusal() {
    assert!(redact_export("not-json\n").is_err());
}

/// The committed fixture is the artifact the redaction exists for: the
/// public repo's one journal carrying `/home/<user>` paths. Redacting it
/// must be deterministic, must strip every absolute path and the
/// username inside, and must leave every envelope field except the
/// payload byte-identical — structure and seqs are untouched, hashes
/// stay as recorded (and so no longer verify; the manifest marking is
/// the CLI's job).
#[test]
fn the_fixture_journal_redacts_deterministically_and_keeps_no_paths() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/journals/tui-graph-the-selection-box-gets-80f98deb.ndjson");
    let ndjson = std::fs::read_to_string(&fixture).unwrap();
    let redacted = redact_export(&ndjson).unwrap();
    assert_eq!(redacted, redact_export(&ndjson).unwrap());

    assert!(ndjson.contains("/home/"), "the fixture is the residual");
    assert!(!redacted.contains("/home"));
    assert!(redacted.contains("[path-1]"));
    for path_start in ndjson.match_indices("/home/") {
        let username = ndjson[path_start.0 + "/home/".len()..]
            .split('/')
            .next()
            .unwrap();
        assert!(
            !redacted.contains(username),
            "a username survived: {username}"
        );
    }

    let originals: Vec<Value> = ndjson
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let sanitized: Vec<Value> = redacted
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(originals.len(), sanitized.len());
    for (original, sanitized) in originals.iter().zip(&sanitized) {
        for field in [
            "run_id",
            "seq",
            "type",
            "event_hash",
            "previous_hash",
            "recorded_at",
        ] {
            assert_eq!(original[field], sanitized[field]);
        }
    }
}

#[test]
fn exported_garbage_is_a_parse_refusal() {
    assert!(matches!(
        verify_export("not-json\n"),
        Err(VerifyError::Parse(_))
    ));
}

#[test]
fn open_and_create_surface_sqlite_failures_at_exact_boundaries() {
    // SQLite treats an empty filename as a private temporary database; this
    // also exercises the path-without-parent initialization branch.
    Store::open(Path::new("")).unwrap();

    let insert_dir = tempfile::tempdir().unwrap();
    let insert_db = insert_dir.path().join("insert.db");
    let conn = Connection::open(&insert_db).unwrap();
    conn.execute_batch(
        "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
         CREATE TRIGGER refuse_meta_insert BEFORE INSERT ON meta
         BEGIN SELECT RAISE(ABORT, 'refused'); END;",
    )
    .unwrap();
    drop(conn);
    assert!(matches!(
        Store::open(&insert_db),
        Err(StoreError::Sqlite(_))
    ));

    let query_dir = tempfile::tempdir().unwrap();
    let query_db = query_dir.path().join("query.db");
    let conn = Connection::open(&query_db).unwrap();
    conn.execute_batch(
        "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
         INSERT INTO meta (key, value) VALUES ('database_schema', x'ff');",
    )
    .unwrap();
    drop(conn);
    assert!(matches!(Store::open(&query_db), Err(StoreError::Sqlite(_))));

    let (_dir, mut store) = store();
    store
        .conn
        .execute_batch(
            "CREATE TEMP TRIGGER refuse_run_insert BEFORE INSERT ON runs
             BEGIN SELECT RAISE(ABORT, 'refused'); END;",
        )
        .unwrap();
    assert!(matches!(
        store.create_run("refused", "feature", "bundle", &json!({})),
        Err(StoreError::Sqlite(_))
    ));
}

#[test]
fn read_only_open_reads_the_journal_and_refuses_every_write() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let mut store = Store::open(&db).unwrap();
    store
        .create_run("r1", "feat", "self", &json!({"files": {}}))
        .unwrap();
    store
        .append_next(
            "r1",
            EventType::RunStarted,
            json!({"feature": "feat", "manifest": {}}),
            None,
            None,
        )
        .unwrap();
    drop(store);

    let mut reader = Store::open_read_only(&db).unwrap();
    assert_eq!(reader.list_runs().unwrap().len(), 1);
    assert_eq!(reader.load("r1").unwrap().len(), 1);
    // The refusal is SQLite's, not this module's discipline.
    assert!(matches!(
        reader.append_next("r1", EventType::PhaseEntered, json!({}), None, None),
        Err(StoreError::Sqlite(_))
    ));
    assert!(matches!(
        reader.create_run("r2", "feat", "self", &json!({})),
        Err(StoreError::Sqlite(_))
    ));
    drop(reader);

    // Nothing moved: the read-only session left the journal as it was.
    let after = Store::open(&db).unwrap();
    assert_eq!(after.load("r1").unwrap().len(), 1);
}

#[test]
fn read_only_open_refuses_a_missing_and_a_foreign_database() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        Store::open_read_only(&dir.path().join("absent.db")),
        Err(StoreError::Sqlite(_))
    ));

    // A SQLite file that is not a journal at all: it opens, and the
    // schema question is what refuses it. No migration runs to invent
    // the table it is missing.
    let foreign = dir.path().join("foreign.db");
    let conn = Connection::open(&foreign).unwrap();
    conn.execute_batch("CREATE TABLE something_else (key TEXT);")
        .unwrap();
    drop(conn);
    assert!(matches!(
        Store::open_read_only(&foreign),
        Err(StoreError::Sqlite(_))
    ));

    let db = dir.path().join("forge.db");
    drop(Store::open(&db).unwrap());
    let conn = Connection::open(&db).unwrap();
    conn.execute(
        "UPDATE meta SET value = 'not-a-schema' WHERE key = 'database_schema'",
        [],
    )
    .unwrap();
    drop(conn);
    assert!(matches!(
        Store::open_read_only(&db),
        Err(StoreError::SchemaMismatch { found: 0 })
    ));
}
