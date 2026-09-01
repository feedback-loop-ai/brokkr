use super::*;
use serde_json::json;

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    (dir, store)
}

/// One turn of a run that folds: enter a phase, request an effect, run
/// it, succeed, and rule the way back to the same phase. Five real
/// events, so a concurrency test contends with the vocabulary the engine
/// actually writes rather than with a repeated filler event.
fn one_cycle(store: &mut Store, run_id: &str, turn: usize) {
    let effect_id = format!("effect-{turn}");
    for (event_type, payload) in [
        (EventType::PhaseEntered, json!({"phase": "implement"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": effect_id, "seat": "implementer"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": effect_id, "attempt_id": format!("attempt-{turn}")}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": effect_id, "result": "complete"}),
        ),
        (
            EventType::TransitionDecided,
            json!({
                "from": "implement",
                "result": "complete",
                "rule_id": "WORK",
                "next": "implement",
                "severity": null,
                "inputs": {},
                "problem": null,
            }),
        ),
    ] {
        store
            .append_next(run_id, event_type, payload, None, None)
            .expect("an append to this writer's own run never conflicts");
    }
}

/// Many fires, one journal: writers on DIFFERENT runs in the SAME file,
/// each with its own connection, racing from a shared barrier — the
/// open, the `create_run`, and every append all contending.
///
/// Per-run hash chains are independent by construction, since no two
/// writers ever compete for one `(run_id, seq)` slot. What this proves
/// is the store layer underneath them: that SQLite's database-wide write
/// lock, WAL, and the busy timeout carry real concurrent writers without
/// dropping a write, forking a chain, or failing an open. Every
/// assertion here is on the finished journals, so the test is
/// deterministic even though the interleaving never is.
#[test]
fn parallel_burns_on_different_runs_share_one_journal() {
    use std::sync::{Arc, Barrier};

    const WRITERS: usize = 4;
    const TURNS: usize = 12;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("realm.db");
    // Nothing pre-creates the journal: the writers race to bring it into
    // existence too, which is where the WAL conversion and the migration
    // contend.
    let gate = Arc::new(Barrier::new(WRITERS));
    let writers: Vec<_> = (0..WRITERS)
        .map(|writer| {
            let path = path.clone();
            let gate = Arc::clone(&gate);
            std::thread::spawn(move || {
                let run_id = format!("run-{writer}");
                gate.wait();
                let mut store = Store::open(&path).expect("a racing open still opens");
                store
                    .create_run(&run_id, "feat", "self", &json!({"files": {}}))
                    .expect("a racing create_run still creates");
                store
                    .append_next(
                        &run_id,
                        EventType::RunStarted,
                        json!({"feature": "feat", "manifest": {}}),
                        None,
                        None,
                    )
                    .expect("a racing first append still appends");
                for turn in 0..TURNS {
                    one_cycle(&mut store, &run_id, turn);
                }
            })
        })
        .collect();
    for writer in writers {
        writer.join().expect("no writer panicked");
    }

    // Every chain, read back through one connection afterward.
    let reader = Store::open(&path).unwrap();
    assert_eq!(reader.list_runs().unwrap().len(), WRITERS);
    let mut every_event_id = std::collections::HashSet::new();
    for writer in 0..WRITERS {
        let run_id = format!("run-{writer}");
        // `load` verifies the chain and refuses to return a partial one.
        let events = reader.load(&run_id).unwrap();
        assert_eq!(
            events.len(),
            1 + TURNS * 5,
            "{run_id} lost or gained an event"
        );
        // Contiguous from 1, every link intact, and nothing from another
        // writer's run anywhere in it.
        let mut previous = ZERO_HASH.to_string();
        for (index, event) in events.iter().enumerate() {
            assert_eq!(event.seq, index as u64 + 1, "{run_id} seq {index}");
            assert_eq!(event.run_id, run_id, "{run_id} carries a foreign run_id");
            assert_eq!(event.correlation_id, run_id);
            assert_eq!(event.previous_hash, previous, "{run_id} chain broke");
            previous = event.event_hash.clone();
            assert!(
                every_event_id.insert(event.event_id.clone()),
                "an event id was reused across runs"
            );
        }
        // And the whole thing still folds: interleaved writing did not
        // scramble anyone's run state.
        let state = brokkr_core::fold(&events).unwrap();
        assert_eq!(state.phase.as_deref(), Some("implement"));
        assert_eq!(state.seq, events.len() as u64);
    }
}

/// Opening a shared journal takes no write lock. A burn starting while a
/// sibling holds the write lock must not have to wait for it, let alone
/// fail: the schema is *read*, and only a journal that has never been
/// migrated is written to. Regression fence for the measured defect —
/// the old prologue wrote `INSERT OR IGNORE INTO meta` on every open and
/// starved past its whole busy timeout against a busy peer.
#[test]
fn opening_a_journal_a_peer_is_writing_takes_no_write_lock() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("realm.db");
    let mut peer = Store::open(&db).unwrap();
    peer.create_run("held", "feat", "self", &json!({})).unwrap();

    // A peer's write transaction, left open and uncommitted: for as long
    // as this lives, nobody else can write this file.
    let holder = Connection::open(&db).unwrap();
    holder.execute_batch("BEGIN IMMEDIATE").unwrap();
    holder
        .execute(
            "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at)
             VALUES ('holder', 'feat', 'self', '{}', '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();

    // Opening is a read, so it does not queue behind that lock. Asked on
    // another thread with a deadline far below the busy timeout, because
    // the regression this fences is a WAIT: opening inline would sit on
    // the lock for the full thirty seconds and then fail, which reads as
    // a hung test rather than as the defect.
    let (done, opened) = std::sync::mpsc::channel();
    let probe = db.clone();
    let opener = std::thread::spawn(move || {
        let read = Store::open(&probe).map(|store| {
            (
                store.list_runs().unwrap().len(),
                store.head_hash("held").unwrap(),
            )
        });
        let _ = done.send(read);
    });
    let read = opened
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("opening queued behind the peer's write lock instead of reading")
        .expect("a racing open still opens");
    opener.join().unwrap();
    assert_eq!(read, (1, (0, ZERO_HASH.to_string())));

    holder.execute_batch("ROLLBACK").unwrap();
}

/// Compare-and-append: the fence a caller needs when what it may legally
/// write depends on the state it just read. The check is inside the
/// append's own transaction, so a peer landing in between takes the head
/// away rather than slipping underneath the decision.
#[test]
fn a_fenced_append_lands_only_on_the_head_it_was_decided_against() {
    let (_dir, mut store) = store();
    store
        .create_run("r1", "feat", "self", &json!({"files": {}}))
        .unwrap();
    // An empty run has a head too, and it can be fenced on.
    store
        .append_next_if_head(
            "r1",
            0,
            ZERO_HASH,
            EventType::RunStarted,
            json!({"feature": "feat", "manifest": {}}),
            None,
            None,
        )
        .unwrap();

    let (seq, hash) = store.head_hash("r1").unwrap();
    // The peer this fence exists for: another writer, appending between
    // the caller's read and its write.
    store
        .append_next(
            "r1",
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            None,
            None,
        )
        .unwrap();

    let moved = store.append_next_if_head(
        "r1",
        seq,
        &hash,
        EventType::PhaseEntered,
        json!({"phase": "review"}),
        None,
        None,
    );
    assert!(
        matches!(
            moved,
            Err(StoreError::HeadMoved {
                expected_seq: 1,
                found_seq: 2
            })
        ),
        "{moved:?}",
    );
    // Nothing was written: a fence that fails leaves no trace, which is
    // the whole difference between refusing and repenting.
    assert_eq!(store.load("r1").unwrap().len(), 2);

    // Re-read, re-decide, and it lands.
    let (seq, hash) = store.head_hash("r1").unwrap();
    store
        .append_next_if_head(
            "r1",
            seq,
            &hash,
            EventType::PhaseEntered,
            json!({"phase": "review"}),
            None,
            None,
        )
        .unwrap();
    let events = store.load("r1").unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[2].previous_hash, events[1].event_hash);
}

/// The append-only guards are repaired on open. `MIGRATION_V1` used to
/// run unconditionally, so a journal that had lost a trigger got it back
/// for free; reading the schema instead would have left one recorded as
/// migrated and unguarded forever.
#[test]
fn a_journal_that_lost_its_guards_gets_them_back_on_open() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    {
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
        store
            .conn
            .execute_batch("DROP TRIGGER events_append_only_update")
            .unwrap();
        assert!(
            store
                .conn
                .execute("UPDATE events SET envelope = 'x'", [])
                .is_ok(),
            "the trigger really was gone",
        );
    }

    let store = Store::open(&db).unwrap();
    assert!(
        store
            .conn
            .execute("UPDATE events SET envelope = 'y'", [])
            .is_err(),
        "reopening restored the guard",
    );
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
    let state = brokkr_core::fold(&events).unwrap();
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
    assert_eq!(state.status, brokkr_core::fold::Status::Running);
    assert_eq!(state.phase.as_deref(), Some("verify"));
    // The accepted stop rode the in-flight attempt to its boundary:
    // instead of the succeeded effect's normal `Decide`, the run
    // concludes per the operator's command.
    assert_eq!(state.cursor, brokkr_core::fold::Cursor::Stop);
    assert!(!state.riding_stop, "the riding command is spent");
    assert_eq!(state.pending_command, None, "the stop was disposed of");
}

/// The reforging fixture is a real journal, not a picture of one: it
/// chains, verifies and folds like any export the engine wrote. It is
/// HAND-BUILT — its name says so — because when the return arc needed a
/// backward transition to draw, no run in this repository had taken one
/// (decision 0022 landed the same day). It is built to the documented
/// shape: `REVIEW-REFORGE` twice, then `REVIEW-REFORGE-EXHAUSTED-DEBT`
/// to `ship`, ending `done`. Trim a real reforged export over it the
/// day one exists.
#[test]
fn the_reforging_fixture_chains_verifies_and_folds_to_a_completed_run() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/journals/reforging-the-road-back-hand-built.ndjson");
    let ndjson = std::fs::read_to_string(&fixture).unwrap();
    let state = verify_export(&ndjson).unwrap();

    assert_eq!(state.run_id, "reforging-the-road-back-hand-built");
    assert_eq!(state.seq, 80, "the export's last event");
    assert_eq!(state.status, brokkr_core::fold::Status::Completed);
    // Three entries into implement: the first visit and the two the
    // reforgings sent back, which is decision 0022's own bound.
    assert_eq!(state.visits.get("implement"), Some(&3));
    assert_eq!(state.visits.get("review"), Some(&3));
    assert!(
        !ndjson.contains("/home/"),
        "a hand-built fixture carries no operator's path"
    );
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

/// A journal and the manifest published beside it are one piece of
/// evidence in two files, so they are scrubbed through ONE redaction:
/// the same original path is the same placeholder in both, machine
/// detail that appears only in the manifest still moves, and a username
/// learned from either is scrubbed from both.
#[test]
fn one_redaction_scrubs_a_journal_and_the_manifest_beside_it_alike() {
    let ndjson = concat!(
        r#"{"payload":{"driver":"/home/alice/forge/target/debug/forge"},"seq":1}"#,
        "\n",
    );
    let manifest = json!({
        "bundle_name": "recipe",
        "realms": {
            "source": "/home/alice/clients/acme/realms.json",
            "map": {"realms": [{"name": "acme", "path": "/home/alice/forge"}]},
        },
        "owner": "alice",
    });
    let raw = serde_json::to_string(&manifest).unwrap();

    let mut redactor = Redactor::learn(&[ndjson, &raw]);
    let journal = redactor.journal(ndjson).unwrap();
    let scrubbed = redactor.document(&manifest);

    assert!(journal.contains(r#""driver":"[path-1]""#), "{journal}");
    // The realm path is the driver path's prefix — a distinct path,
    // numbered after it, and numbered once for the whole pair.
    assert_eq!(
        scrubbed["realms"]["map"]["realms"][0]["path"],
        json!("[path-2]")
    );
    assert!(!journal.contains("[path-2]"), "{journal}");
    // The map file appears nowhere in the journal, and still moves.
    let source = scrubbed["realms"]["source"].as_str().unwrap();
    assert!(source.starts_with("[path-"), "{source}");
    assert_eq!(scrubbed["owner"], json!("[user-1]"));
    assert_eq!(scrubbed["bundle_name"], json!("recipe"));
    assert!(!serde_json::to_string(&scrubbed).unwrap().contains("alice"));

    // Scrubbing the journal alone is still exactly `redact_export`.
    assert_eq!(redact_export(ndjson).unwrap(), journal);
}

// ── import: the verb paired with export (decision 0027) ──────────────

/// A native run in its own journal, exported the way `brokkr export`
/// exports it: the canonical NDJSON and the pinned manifest beside it.
fn exported_run(run_id: &str, phase: &str) -> (tempfile::TempDir, Store, String, Value) {
    let (dir, mut store) = store();
    let manifest = json!({"bundle_name": "self", "files": {"recipe": "sha"}});
    store
        .create_run(run_id, "the missing verb", "self", &manifest)
        .unwrap();
    store
        .append_next(
            run_id,
            EventType::RunStarted,
            json!({"feature": "the missing verb", "manifest": manifest}),
            None,
            None,
        )
        .unwrap();
    store
        .append_next(
            run_id,
            EventType::PhaseEntered,
            json!({"phase": phase}),
            None,
            None,
        )
        .unwrap();
    let ndjson = store.export_ndjson(run_id).unwrap();
    (dir, store, ndjson, manifest)
}

/// A destination journal, empty until something relocates into it.
fn destination() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("destination.db")).unwrap();
    (dir, store)
}

/// The export path an adoption records as the place the run arrived from.
fn origin(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from("/exports/wave-1").join(name)
}

/// A one-event export sealed by hand under any run id and any payload —
/// how a stranger's export reaches this journal. `sealed()` recomputes
/// the hash over whatever it is given, so every chain built here
/// verifies; what a chain cannot vouch for is the *name* it was sealed
/// under, which is exactly what the gates past verification are for.
fn sealed_export(run_id: &str, payload: Value) -> String {
    let envelope = EventEnvelope {
        run_id: run_id.into(),
        seq: 1,
        event_id: "e1".into(),
        event_schema_version: 1,
        event_type: EventType::RunStarted,
        payload,
        causation_id: None,
        correlation_id: run_id.into(),
        attempt_id: None,
        recorded_at: "2026-01-01T00:00:00Z".into(),
        previous_hash: ZERO_HASH.into(),
        event_hash: String::new(),
    }
    .sealed();
    format!(
        "{}\n",
        serde_json::to_string(&serde_json::to_value(&envelope).unwrap()).unwrap()
    )
}

/// A journal that arrives is the journal that left: the destination
/// re-exports the very bytes it was handed, and folds to the same state.
#[test]
fn import_adopts_an_exported_run_byte_identically() {
    let (_source_dir, source, ndjson, manifest) = exported_run("r1", "intake");
    let (_dest_dir, mut dest) = destination();

    let adoption = dest
        .import_run(&ndjson, &manifest, &origin("r1.ndjson"))
        .unwrap();
    assert_eq!(adoption.run_id, "r1");
    assert_eq!(adoption.events, 2);

    // Bytes, not semantics: a re-export of the adopted run reproduces
    // the source export exactly.
    assert_eq!(dest.export_ndjson("r1").unwrap(), ndjson);
    let here = source.load("r1").unwrap();
    let there = dest.load("r1").unwrap();
    assert_eq!(
        format!("{:?}", brokkr_core::fold(&there).unwrap()),
        format!("{:?}", brokkr_core::fold(&here).unwrap()),
    );
    // The head is the head it always was — the bytes it was computed
    // over never moved.
    assert_eq!(adoption.head_hash, there.last().unwrap().event_hash);
    assert_eq!(dest.head_hash("r1").unwrap().1, adoption.head_hash);
    // The runs row is derived from the verified chain, not the sidecar.
    assert_eq!(dest.manifest("r1").unwrap(), manifest);
    assert_eq!(
        dest.list_runs().unwrap(),
        vec![(
            "r1".to_string(),
            "the missing verb".to_string(),
            there[0].recorded_at.clone(),
        )]
    );
}

/// Arrival is bookkeeping BESIDE the chain: queryable on the adopted
/// run, absent on a native one, and invisible to the events either way.
#[test]
fn an_adopted_run_carries_arrival_metadata_beside_an_untouched_chain() {
    let (_source_dir, source, ndjson, manifest) = exported_run("r1", "intake");
    let (_dest_dir, mut dest) = destination();
    let from = origin("r1.ndjson");
    dest.import_run(&ndjson, &manifest, &from).unwrap();

    let arrival = dest
        .arrival("r1")
        .unwrap()
        .expect("adopted run has arrival");
    assert_eq!(arrival.imported_from, from.display().to_string());
    // An RFC3339 instant, not a placeholder.
    assert!(
        OffsetDateTime::parse(&arrival.imported_at, &Rfc3339).is_ok(),
        "{arrival:?}"
    );

    // The native original's row is untouched by this feature: nothing
    // on `create_run`'s path populates arrival columns.
    assert_eq!(source.arrival("r1").unwrap(), None);
    assert!(matches!(
        dest.arrival("nope"),
        Err(StoreError::RunNotFound(_))
    ));

    // And the chain the arrival sits beside says nothing about it.
    let stored: Vec<String> = dest
        .conn
        .prepare("SELECT envelope FROM events WHERE run_id = 'r1' ORDER BY seq")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for envelope in &stored {
        assert!(!envelope.contains("import"), "{envelope}");
    }
}

/// A journal from before the arrival columns — or one that crashed
/// between the two ALTERs that add them — is finished at the next open,
/// inside the immediate transaction, and a journal already whole writes
/// nothing. The half-migrated shape is real: two ALTERs are two
/// statements, and a process can die between them.
#[test]
fn a_journal_that_predates_the_arrival_columns_is_finished_at_open() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.db");
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(MIGRATION_V1).unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO meta (key, value) VALUES ('database_schema', ?1)",
            [&DATABASE_SCHEMA.to_string()],
        )
        .unwrap();
        // Half the migration already applied: one column of two.
        conn.execute("ALTER TABLE runs ADD COLUMN imported_at TEXT", [])
            .unwrap();
    }
    let store = Store::open(&path).unwrap();
    let present: Vec<String> = store
        .conn
        .prepare("SELECT name FROM pragma_table_info('runs')")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for (column, _) in ARRIVAL_COLUMNS {
        assert!(present.iter().any(|name| name == column), "{column}");
    }
    // And the finished journal is a plain read at the next open.
    assert!(!arrival_columns_missing(&Store::open(&path).unwrap().conn).unwrap());
}

/// One broken link refuses the WHOLE import. No prefix of good events
/// is adopted, and the destination is left exactly as it was.
#[test]
fn import_refuses_a_broken_chain_and_adopts_no_prefix_of_it() {
    let (_source_dir, _source, ndjson, manifest) = exported_run("r1", "intake");
    let (_dest_dir, mut dest) = destination();

    // One byte inside a payload field of the SECOND event: its recorded
    // hash no longer covers its content, while every seq and
    // previous_hash still lines up.
    let mut lines: Vec<String> = ndjson.lines().map(str::to_string).collect();
    let mut second: Value = serde_json::from_str(&lines[1]).unwrap();
    second["payload"]["phase"] = json!("intakf");
    lines[1] = serde_json::to_string(&second).unwrap();
    let corrupt = format!("{}\n", lines.join("\n"));

    let refusal = dest
        .import_run(&corrupt, &manifest, &origin("r1.ndjson"))
        .unwrap_err();
    assert!(
        matches!(
            refusal,
            ImportError::Verify(VerifyError::Chain(ChainError::BadHash { seq: 2 }))
        ),
        "{refusal}"
    );
    // Nothing landed: not the run, not its first, good event.
    assert!(dest.list_runs().unwrap().is_empty());
    assert!(matches!(dest.load("r1"), Err(StoreError::RunNotFound(_))));

    // A line that is not an envelope at all refuses the same way.
    let refusal = dest
        .import_run("{ not an envelope }\n", &manifest, &origin("r1.ndjson"))
        .unwrap_err();
    assert!(
        matches!(refusal, ImportError::Verify(VerifyError::Parse(_))),
        "{refusal}"
    );
    assert!(dest.list_runs().unwrap().is_empty());
}

/// A chain can verify and still not be a run. The fold's refusal is the
/// import's refusal, with the fold's own citation.
#[test]
fn import_refuses_a_journal_the_fold_refuses() {
    // Every hash, seq and previous_hash is correct — the journal simply
    // does not open with `run/started`, so no fold can read it.
    let (_source_dir, mut source) = destination();
    source
        .create_run("r1", "feat", "self", &json!({"bundle_name": "self"}))
        .unwrap();
    source
        .append_next(
            "r1",
            EventType::PhaseEntered,
            json!({"phase": "intake"}),
            None,
            None,
        )
        .unwrap();
    let ndjson = source.export_ndjson("r1").unwrap();

    let (_dest_dir, mut dest) = destination();
    let refusal = dest
        .import_run(
            &ndjson,
            &json!({"bundle_name": "self"}),
            &origin("r1.ndjson"),
        )
        .unwrap_err();
    assert!(
        matches!(
            refusal,
            ImportError::Verify(VerifyError::Fold(
                brokkr_core::FoldError::FirstEventNotRunStarted { seq: 1 }
            ))
        ),
        "{refusal}"
    );
    assert!(dest.list_runs().unwrap().is_empty());

    // An empty export is the same kind of refusal: there is no run in it.
    let refusal = dest
        .import_run("", &json!({}), &origin("empty.ndjson"))
        .unwrap_err();
    assert!(
        matches!(
            refusal,
            ImportError::Verify(VerifyError::Fold(brokkr_core::FoldError::Empty))
        ),
        "{refusal}"
    );
}

/// A run_id is hashed into every envelope of its chain, so an adoption
/// can neither rename nor overwrite a collision. It refuses, and what is
/// already here is untouched.
#[test]
fn import_refuses_a_run_id_collision_outright() {
    let (_source_dir, _source, ndjson, manifest) = exported_run("r1", "intake");
    let (_dest_dir, mut dest) = destination();
    dest.import_run(&ndjson, &manifest, &origin("r1.ndjson"))
        .unwrap();

    // A genuinely DIFFERENT run that happens to share the run_id: other
    // events, other hashes, same name.
    let (_other_dir, _other, other, other_manifest) = exported_run("r1", "verify");
    assert_ne!(other, ndjson);
    let refusal = dest
        .import_run(&other, &other_manifest, &origin("r1.ndjson"))
        .unwrap_err();
    assert!(
        matches!(refusal, ImportError::Collision(ref run) if run == "r1"),
        "{refusal}"
    );
    // The copy already here is the one still here — not overwritten, not
    // appended to, not duplicated.
    assert_eq!(dest.export_ndjson("r1").unwrap(), ndjson);
    assert_eq!(dest.list_runs().unwrap().len(), 1);
}

/// Adoption is once, not idempotent: the same export twice is the same
/// refusal a stranger sharing the run_id gets. Named on its own so
/// "import is idempotent" is never quietly assumed.
#[test]
fn a_second_import_of_the_same_export_is_not_a_quiet_no_op() {
    let (_source_dir, _source, ndjson, manifest) = exported_run("r1", "intake");
    let (_dest_dir, mut dest) = destination();
    let from = origin("r1.ndjson");
    let first = dest.import_run(&ndjson, &manifest, &from).unwrap();

    let refusal = dest.import_run(&ndjson, &manifest, &from).unwrap_err();
    assert!(
        matches!(refusal, ImportError::Collision(ref run) if run == "r1"),
        "{refusal}"
    );
    assert!(refusal.to_string().contains("already carries run 'r1'"));
    // Byte-identical to the first adoption, and its arrival is the first
    // arrival — the second call recorded nothing.
    assert_eq!(dest.export_ndjson("r1").unwrap(), ndjson);
    assert_eq!(dest.arrival("r1").unwrap(), Some(first.arrival));
    assert_eq!(dest.list_runs().unwrap().len(), 1);
}

/// A redacted export's recorded hashes never verify against its
/// rewritten bytes, so importing one could only ever adopt unverifiable
/// content. Refused by either mark, before content is read at all —
/// there is no `--force`.
#[test]
fn import_refuses_a_redacted_derivative_by_either_mark() {
    let (_source_dir, _source, ndjson, manifest) = exported_run("r1", "intake");
    let (_dest_dir, mut dest) = destination();

    // By filename — and with content that would fail every later gate
    // on its own, to prove the refusal happens before any of them.
    let refusal = dest
        .import_run(
            "this is not NDJSON at all",
            &manifest,
            &origin("r1.redacted.ndjson"),
        )
        .unwrap_err();
    assert!(
        matches!(refusal, ImportError::Redacted(ref named) if named == "r1.redacted.ndjson"),
        "{refusal}"
    );
    assert!(refusal.to_string().contains("redacted derivative"));

    // By the manifest's own marker, on a pair somebody renamed back —
    // here with a path that has no file name to sniff at all, and a
    // journal whose chain would otherwise verify.
    let marked = json!({"bundle_name": "self", "redacted": true});
    let refusal = dest
        .import_run(&ndjson, &marked, std::path::Path::new("/"))
        .unwrap_err();
    assert!(matches!(refusal, ImportError::Redacted(_)), "{refusal}");

    // The real redacted journal `export --redact` writes, under its own
    // name: refused, and nothing landed from any of the three attempts.
    let redacted = redact_export(&ndjson).unwrap();
    assert!(dest
        .import_run(&redacted, &marked, &origin("r1.redacted.ndjson"))
        .is_err());
    assert!(dest.list_runs().unwrap().is_empty());
}

/// The destination's `runs` row is derived from the verified chain, so an
/// export whose `run/started` cannot answer for it is refused rather than
/// filled in from the sidecar manifest no hash covers.
#[test]
fn import_refuses_an_export_whose_chain_cannot_answer_for_the_runs_row() {
    let sealed = |payload: Value| sealed_export("r1", payload);
    let (_dest_dir, mut dest) = destination();
    // A sidecar that says everything — and is believed for nothing.
    let sidecar = json!({"bundle_name": "self", "feature": "invented"});

    for (payload, field) in [
        (json!({"manifest": {"bundle_name": "self"}}), "feature"),
        (json!({"feature": "f"}), "manifest"),
    ] {
        let refusal = dest
            .import_run(&sealed(payload), &sidecar, &origin("r1.ndjson"))
            .unwrap_err();
        assert!(
            matches!(refusal, ImportError::Unattested { field: found } if found == field),
            "{refusal}"
        );
    }
    assert!(dest.list_runs().unwrap().is_empty());

    // A manifest without `bundle_name` is not refused: the column is a
    // denormalization nothing selects, and `runs.manifest` still carries
    // the chain's manifest exactly as the chain stated it.
    let bare = json!({"feature": "f", "manifest": {"files": {}}});
    dest.import_run(&sealed(bare), &sidecar, &origin("r1.ndjson"))
        .unwrap();
    assert_eq!(dest.manifest("r1").unwrap(), json!({"files": {}}));
    assert_eq!(
        dest.list_runs().unwrap(),
        vec![(
            "r1".to_string(),
            "f".to_string(),
            "2026-01-01T00:00:00Z".to_string()
        )]
    );
}

/// A verified chain proves its bytes were not altered. It does not
/// prove whoever sealed them was entitled to the name they sealed them
/// under — the hashes are unkeyed, so a stranger's export can carry any
/// run id at all. The id does not stay in the database: `brokkr export`
/// composes `<out>/<run_id>.ndjson` from it and every readout prints it,
/// so an adoption refuses a name it would never have minted itself.
#[test]
fn import_refuses_a_run_id_this_journal_would_not_have_minted() {
    let attested =
        |id: &str| json!({"feature": "the missing verb", "manifest": {"files": {}}, "id": id});
    let (_dest_dir, mut dest) = destination();

    // Each of these verifies as a chain and folds; only the name is
    // wrong. The traversal one is the reason the gate exists: adopted,
    // it would make the next `export --out ./out` write outside `./out`.
    for name in [
        "../../../tmp/x",
        "run/../../etc",
        "wave-1/r1",
        "r1\\r2",
        "a.b",
        "",
        &"r".repeat(RUN_ID_MAX + 1),
    ] {
        let refusal = dest
            .import_run(
                &sealed_export(name, attested(name)),
                &json!({}),
                &origin("r1.ndjson"),
            )
            .unwrap_err();
        assert!(
            matches!(refusal, ImportError::UnadoptableRunId(_)),
            "{name:?} was not refused: {refusal}"
        );
    }

    // A name that reached here by being unprintable does not get to
    // reorder the sentence turning it away: the refusal is ASCII, and
    // the bidi override appears as its codepoint, not as itself.
    let hostile = "run\u{202e}drowssap\u{0007}";
    let refusal = dest
        .import_run(
            &sealed_export(hostile, attested(hostile)),
            &json!({}),
            &origin("r1.ndjson"),
        )
        .unwrap_err();
    let said = refusal.to_string();
    assert!(said.contains("run\\u{202e}drowssap\\u{0007}"), "{said}");
    assert!(
        !said.contains('\u{202e}') && !said.contains('\u{0007}'),
        "{said}"
    );

    // Nothing landed from any of them, and the gate is not so tight that
    // it refuses what this journal does mint: a feature slug and eight
    // hex characters, or an id merely carrying an underscore.
    assert!(dest.list_runs().unwrap().is_empty());
    for name in ["the-missing-verb-1608dfe7", "r_1", &"r".repeat(RUN_ID_MAX)] {
        dest.import_run(
            &sealed_export(name, attested(name)),
            &json!({}),
            &origin("r1.ndjson"),
        )
        .unwrap();
    }
    assert_eq!(dest.list_runs().unwrap().len(), 3);
}

/// The arrival columns are additive, and adding them is idempotent: a
/// journal an older binary left behind migrates on the open that needs
/// them, and an open that finds them adds nothing.
#[test]
fn arrival_columns_are_added_once_and_an_older_journal_migrates() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");

    // A journal as an older binary left it: MIGRATION_V1 only.
    let conn = Connection::open(&db).unwrap();
    conn.execute_batch(MIGRATION_V1).unwrap();
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('database_schema', '1')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at)
         VALUES ('old', 'before import existed', 'self', '{}', '2026-01-01T00:00:00Z')",
        [],
    )
    .unwrap();
    drop(conn);

    // Opening it migrates; the pre-existing run reads as native.
    let store = Store::open(&db).unwrap();
    assert_eq!(store.arrival("old").unwrap(), None);
    drop(store);

    // Opening it again finds the columns present and adds nothing —
    // `DATABASE_SCHEMA` never moved, so neither open refused.
    let store = Store::open(&db).unwrap();
    assert_eq!(store.list_runs().unwrap().len(), 1);
    assert_eq!(store.arrival("old").unwrap(), None);
}
