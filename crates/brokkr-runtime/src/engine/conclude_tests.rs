//! The other door: closing a run from its journal alone.
//!
//! Every store here is rebuilt from a committed fixture pair
//! (`fixtures/journals/conclude-*.ndjson` + `.manifest.json`) by
//! inserting the sealed envelopes VERBATIM — never re-appending them
//! through `append_next`, which would re-seal each one and quietly heal
//! the tampered fixture into a valid chain. What the fixture says is
//! what the store holds.

use super::*;
use brokkr_store::StoreError;
use std::path::{Path, PathBuf};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/journals")
}

/// Replay a fixture pair into a fresh store under a caller-chosen
/// manifest — the pin `resume` would read and `conclude` never does. The
/// fixture files are opened read-only and never edited.
fn replay(db: &Path, name: &str, manifest: &Value) -> Store {
    let ndjson = std::fs::read_to_string(fixtures().join(format!("{name}.ndjson"))).unwrap();
    let mut store = Store::open(db).unwrap();
    store.create_run(name, "fixture", "self", manifest).unwrap();
    let connection = rusqlite::Connection::open(db).unwrap();
    for line in ndjson.lines().filter(|line| !line.trim().is_empty()) {
        let envelope: EventEnvelope = serde_json::from_str(line).unwrap();
        connection
            .execute(
                "INSERT INTO events (run_id, seq, event_hash, envelope) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![name, envelope.seq as i64, envelope.event_hash, line],
            )
            .unwrap();
    }
    store
}

/// The manifest the fixture pair was exported with — pinned at engine
/// 0.3.6, an engine this tree has moved past.
fn pinned(name: &str) -> Value {
    let raw = std::fs::read_to_string(fixtures().join(format!("{name}.manifest.json"))).unwrap();
    serde_json::from_str(&raw).unwrap()
}

fn store_for(dir: &tempfile::TempDir, name: &str) -> Store {
    replay(&dir.path().join(format!("{name}.db")), name, &pinned(name))
}

fn events(store: &Store, run_id: &str) -> Vec<EventEnvelope> {
    store.load(run_id).unwrap()
}

/// The standing case: an operator stop accepted while an attempt was in
/// flight, the journal ending there. The stop is already in force, so
/// `conclude` issues no second command — it closes the attempt at the
/// boundary this process cannot establish completion for, and stops,
/// citing the operator who gave the command rather than the one who
/// typed `conclude`.
#[test]
fn a_stop_already_in_force_is_carried_to_run_stopped_without_a_second_command() {
    let dir = tempfile::tempdir().unwrap();
    let name = "conclude-stopped-mid-effect-hand-built";
    let mut store = store_for(&dir, name);

    let before = fold(&events(&store, name)).unwrap();
    assert_eq!(before.status, Status::Running);
    assert!(before.riding_stop, "the accepted stop is riding");
    assert!(matches!(before.cursor, Cursor::EffectInFlight { .. }));

    let state = conclude(&mut store, name, "someone-else", "closing the books").unwrap();
    assert_eq!(state.status, Status::Stopped);
    assert_eq!(state.cursor, Cursor::Idle);

    let after = events(&store, name);
    let appended: Vec<EventType> = after[before.seq as usize..]
        .iter()
        .map(|event| event.event_type)
        .collect();
    assert_eq!(
        appended,
        vec![EventType::EffectIndeterminate, EventType::RunStopped],
        "the attempt reached its boundary, and there the run stopped",
    );
    let indeterminate = &after[before.seq as usize];
    assert_eq!(
        indeterminate.payload["effect_id"].as_str(),
        Some("effect-verify")
    );
    assert_eq!(
        indeterminate.payload["attempt_id"].as_str(),
        Some("attempt-verify")
    );
    assert_eq!(
        indeterminate.attempt_id.as_deref(),
        Some("attempt-verify"),
        "the close is filed against the attempt it closes",
    );
    assert!(
        indeterminate.payload["reason"]
            .as_str()
            .unwrap()
            .contains("concluded from its journal"),
        "the close says it is a conclusion, not a crash recovery: {}",
        indeterminate.payload["reason"],
    );

    let cited = after.last().unwrap().payload["reason"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(cited.starts_with("OPERATOR-STOP: "), "{cited}");
    assert!(cited.contains("vyanakiev"), "{cited}");
    assert!(
        cited.contains("the verify seat is reading a tree that has moved on"),
        "{cited}",
    );
    assert!(
        !cited.contains("someone-else") && !cited.contains("closing the books"),
        "the operator who commanded the stop is the cause, not the one who closed it: {cited}",
    );
    assert_eq!(
        after
            .iter()
            .filter(|event| event.event_type == EventType::OperatorCommanded)
            .count(),
        1,
        "a stop in force is never re-commanded",
    );
}

/// A parked run has no command pending, so the conclusion is the
/// operator's own: `conclude` commands the stop under the invoking
/// operator's name, accepts it, and stops where the run stands. There is
/// nothing in flight, so nothing is closed as indeterminate.
#[test]
fn a_parked_run_is_stopped_under_the_name_of_the_operator_who_concluded_it() {
    let dir = tempfile::tempdir().unwrap();
    let name = "conclude-parked-hand-built";
    let mut store = store_for(&dir, name);

    let before = fold(&events(&store, name)).unwrap();
    assert_eq!(before.status, Status::AwaitingOperator);
    assert_eq!(before.cursor, Cursor::Idle);
    assert_eq!(before.pending_command, None);

    let state = conclude(
        &mut store,
        name,
        "vyanakiev",
        "the engine moved on without it",
    )
    .unwrap();
    assert_eq!(state.status, Status::Stopped);

    let after = events(&store, name);
    let appended: Vec<EventType> = after[before.seq as usize..]
        .iter()
        .map(|event| event.event_type)
        .collect();
    assert_eq!(
        appended,
        vec![
            EventType::OperatorCommanded,
            EventType::OperatorAccepted,
            EventType::RunStopped,
        ],
    );
    let commanded = &after[before.seq as usize];
    assert_eq!(commanded.payload["command"].as_str(), Some("stop"));
    assert_eq!(commanded.payload["operator"].as_str(), Some("vyanakiev"));

    let cited = after.last().unwrap().payload["reason"].as_str().unwrap();
    assert!(cited.starts_with("OPERATOR-STOP: "), "{cited}");
    assert!(cited.contains("vyanakiev"), "{cited}");
    assert!(cited.contains("the engine moved on without it"), "{cited}");
    assert!(
        !after
            .iter()
            .any(|event| event.event_type == EventType::EffectIndeterminate),
        "nothing was in flight, so nothing was closed",
    );
}

/// A command that never got its disposition — the crash between
/// `operator_command`'s two appends, or an incomplete replay — leaves
/// `pending_command` occupied, and step 4's guard reads the CURSOR, not
/// that field, so `conclude` commands its stop anyway and `fold` drops
/// the older command when it records the newer one.
///
/// That is the specified behaviour, and this pins what it actually costs
/// rather than reasoning about it: the orphan stays in the journal,
/// undisposed and readable, the conclusion is attributed to the operator
/// who typed `conclude` and not to a command nobody accepted, and the
/// finished journal folds clean. Noise in the trail, not a wrong state —
/// and a fact a later reader can check instead of infer.
#[test]
fn a_command_left_without_a_disposition_is_outlived_by_the_conclusion() {
    let dir = tempfile::tempdir().unwrap();
    let name = "conclude-parked-hand-built";
    let mut store = store_for(&dir, name);

    // The half-written command: `operator/commanded` with no
    // `operator/accepted` behind it, exactly as a crash between the two
    // appends would leave it.
    let head = events(&store, name).last().map(|e| e.event_id.clone());
    store
        .append_next(
            name,
            EventType::OperatorCommanded,
            json!({"command_id": "orphaned", "command": "retry", "args": {},
                   "operator": "someone-else"}),
            head,
            None,
        )
        .unwrap();
    let before = fold(&events(&store, name)).unwrap();
    assert_eq!(
        before.pending_command,
        Some(("orphaned".to_string(), "retry".to_string())),
    );

    let state = conclude(&mut store, name, "vyanakiev", "closing the books").unwrap();
    assert_eq!(state.status, Status::Stopped);

    let after = events(&store, name);
    let appended: Vec<EventType> = after[before.seq as usize..]
        .iter()
        .map(|event| event.event_type)
        .collect();
    assert_eq!(
        appended,
        vec![
            EventType::OperatorCommanded,
            EventType::OperatorAccepted,
            EventType::RunStopped,
        ],
    );

    // The orphan is still there and still undisposed: nothing accepted
    // or rejected it, and `conclude` neither pretended to nor erased it.
    assert!(
        after.iter().any(|event| {
            event.event_type == EventType::OperatorCommanded
                && event.payload["command_id"].as_str() == Some("orphaned")
        }),
        "the orphaned command was not erased",
    );
    assert!(
        !after.iter().any(|event| {
            matches!(
                event.event_type,
                EventType::OperatorAccepted | EventType::OperatorRejected
            ) && event.payload["command_id"].as_str() == Some("orphaned")
        }),
        "the orphaned command gained a disposition it was never given",
    );

    // Attributed to the operator who concluded, never to the command
    // nobody accepted — and the whole journal still folds.
    let cited = after.last().unwrap().payload["reason"].as_str().unwrap();
    assert!(cited.contains("vyanakiev"), "{cited}");
    assert!(cited.contains("commanded stop"), "{cited}");
    assert!(!cited.contains("someone-else"), "{cited}");
    assert_eq!(fold(&after).unwrap().status, Status::Stopped);
}

/// A run that already has its conclusion gets no second one. The refusal
/// comes before the first append: a concluded run's journal is exactly as
/// long afterwards as it was before.
#[test]
fn an_already_concluded_run_is_refused_and_nothing_is_appended() {
    let dir = tempfile::tempdir().unwrap();
    let name = "conclude-already-done-hand-built";
    let mut store = store_for(&dir, name);
    let before = events(&store, name);
    assert_eq!(fold(&before).unwrap().status, Status::Completed);

    let refusal = conclude(&mut store, name, "vyanakiev", "again").unwrap_err();
    assert!(
        matches!(&refusal, EngineError::AlreadyConcluded { run_id, status }
            if run_id == name && status == "completed"),
        "{refusal}",
    );
    assert!(
        refusal.to_string().contains("already concluded"),
        "{refusal}"
    );
    assert_eq!(
        events(&store, name).len(),
        before.len(),
        "a refusal appends nothing",
    );

    // And the same for a run stopped the ordinary way: conclude it once,
    // and the second attempt is refused as stopped.
    let parked = "conclude-parked-hand-built";
    let mut store = store_for(&dir, parked);
    conclude(&mut store, parked, "vyanakiev", "first").unwrap();
    let concluded = events(&store, parked);
    let refusal = conclude(&mut store, parked, "vyanakiev", "second").unwrap_err();
    assert!(
        matches!(&refusal, EngineError::AlreadyConcluded { status, .. } if status == "stopped"),
        "{refusal}",
    );
    assert_eq!(events(&store, parked).len(), concluded.len());
}

/// A tampered journal is refused whole, through the store's existing
/// chain verification — `conclude` adds no second check and swallows no
/// error. Nothing is appended to a journal that cannot be read.
#[test]
fn a_broken_chain_refuses_the_whole_conclusion() {
    let dir = tempfile::tempdir().unwrap();
    let name = "conclude-broken-chain-hand-built";
    let mut store = store_for(&dir, name);

    let refusal = conclude(&mut store, name, "vyanakiev", "close it").unwrap_err();
    assert!(
        matches!(
            &refusal,
            EngineError::Store(StoreError::Chain(
                brokkr_core::envelope::ChainError::BadHash { seq: 11 }
            ))
        ),
        "{refusal}",
    );

    // The store cannot be read at all, so "nothing was appended" is
    // checked against the rows themselves rather than through `load`.
    let connection = rusqlite::Connection::open(dir.path().join(format!("{name}.db"))).unwrap();
    let rows: i64 = connection
        .query_row(
            "SELECT count(*) FROM events WHERE run_id = ?1",
            rusqlite::params![name],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(rows, 11, "the fixture's eleven events and not one more");
}

/// The position reader refuses rather than guesses. Under an accepted
/// stop the fold admits exactly two cursors — `Stop`, and the in-flight
/// attempt the stop is riding — and a third would mean the fold or the
/// engine has a defect, not that `conclude` should pick a conclusion. It
/// is checked here directly, against cursors no journal can currently
/// reach through `conclude`, so the refusal exists before the defect
/// that would need it does.
#[test]
fn a_cursor_no_accepted_stop_can_produce_refuses_instead_of_guessing() {
    let riding = |cursor| {
        let mut state = super::tests::state(Some("verify"), cursor);
        state.riding_stop = true;
        state
    };

    assert_eq!(
        riding_attempt("run", &super::tests::state(Some("verify"), Cursor::Stop)).unwrap(),
        None,
        "Stop is where run/stopped belongs, and nothing is owed first",
    );

    let in_flight = Cursor::EffectInFlight {
        effect_id: "effect".into(),
        attempt_id: "attempt".into(),
        seat: "verify".into(),
        failed_attempts: 0,
    };
    assert_eq!(
        riding_attempt("run", &riding(in_flight.clone())).unwrap(),
        Some(("effect".to_string(), "attempt".to_string())),
    );

    // The same cursor with nothing riding it: an attempt in flight under
    // no accepted stop is not a conclusion to finish.
    let unridden = riding_attempt("run", &super::tests::state(Some("verify"), in_flight))
        .unwrap_err()
        .to_string();
    assert!(unridden.contains("EffectInFlight"), "{unridden}");
    assert!(unridden.contains("accepted stop"), "{unridden}");

    for cursor in [
        Cursor::Start,
        Cursor::RequestEffect,
        Cursor::Idle,
        Cursor::Park {
            reason: "parked".into(),
        },
    ] {
        let refusal = riding_attempt("run", &riding(cursor.clone()))
            .unwrap_err()
            .to_string();
        assert!(refusal.contains("run 'run'"), "{refusal}");
        assert!(refusal.contains(&format!("{cursor:?}")), "{refusal}");
    }
}

/// Past its refusals a conclusion still makes two writes, and the store
/// can refuse either one. Neither failure is swallowed — and what each
/// leaves behind is the point: a refused boundary close leaves the
/// journal exactly as long as it was, and a refused `run/stopped` leaves
/// it standing AT `Cursor::Stop`, the one position that event belongs
/// at, so a second `conclude` finishes what the first could not without
/// commanding a second stop. A conclusion interrupted is a conclusion
/// postponed, never a run stranded a second time.
#[test]
fn a_store_that_refuses_an_append_leaves_a_journal_that_can_still_be_concluded() {
    let dir = tempfile::tempdir().unwrap();

    // The boundary close refuses. The stop was already in force, so
    // `conclude` had commanded nothing of its own: nothing is appended.
    let mid = "conclude-stopped-mid-effect-hand-built";
    let mid_db = dir.path().join("indeterminate.db");
    let mut store = replay(&mid_db, mid, &pinned(mid));
    let before = events(&store, mid).len();
    super::tests::fail_event(&mid_db, "effect/indeterminate");
    let refusal = conclude(&mut store, mid, "vyanakiev", "close it").unwrap_err();
    assert!(
        refusal.to_string().contains("forced event failure"),
        "{refusal}",
    );
    assert_eq!(
        events(&store, mid).len(),
        before,
        "a refusal appends nothing"
    );

    // The terminal event refuses. The stop this conclusion commanded
    // stands, and the journal is left where the next one picks it up.
    let parked = "conclude-parked-hand-built";
    let parked_db = dir.path().join("stopped.db");
    let mut store = replay(&parked_db, parked, &pinned(parked));
    super::tests::fail_event(&parked_db, "run/stopped");
    let refusal = conclude(&mut store, parked, "vyanakiev", "close it").unwrap_err();
    assert!(
        refusal.to_string().contains("forced event failure"),
        "{refusal}",
    );
    let interrupted = fold(&events(&store, parked)).unwrap();
    assert_eq!(interrupted.status, Status::Running);
    assert_eq!(
        interrupted.cursor,
        Cursor::Stop,
        "the commanded stop stands, waiting for the event that records it",
    );

    rusqlite::Connection::open(&parked_db)
        .unwrap()
        .execute_batch("DROP TRIGGER forced_event_failure;")
        .unwrap();
    let state = conclude(&mut store, parked, "someone-else", "again").unwrap();
    assert_eq!(state.status, Status::Stopped);
    assert_eq!(
        events(&store, parked)
            .iter()
            .filter(|event| event.event_type == EventType::OperatorCommanded)
            .count(),
        1,
        "the stop the first attempt commanded was not commanded a second time",
    );
    let cited = events(&store, parked).last().unwrap().payload["reason"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(cited.contains("close it"), "{cited}");
    assert!(!cited.contains("again"), "{cited}");
}

/// The regression this door exists to route around, pinned as a
/// contrast rather than assumed: a run whose manifest pins an engine
/// this tree has moved past cannot be resumed — `resume` compiles the
/// bundle and refuses before it ever looks at the cursor — and can be
/// concluded regardless, because `conclude` never reads the manifest.
#[test]
fn an_engine_version_that_moved_refuses_resume_and_concludes_anyway() {
    let dir = tempfile::tempdir().unwrap();
    let name = "conclude-stopped-mid-effect-hand-built";
    let bundle = super::tests::bundle(dir.path(), super::tests::single_body(vec!["driver".into()]));

    // The pinned bundle, identical to the compiled one in every file
    // digest, differing ONLY in the engine that recorded it.
    let mut moved = bundle.manifest.clone();
    moved["engine"] = json!("0.3.6");
    assert_ne!(
        moved["engine"], bundle.manifest["engine"],
        "the fixture's engine must actually differ from this tree's",
    );

    let db = dir.path().join("moved.db");
    let mut store = replay(&db, name, &moved);

    match Engine::resume(Store::open(&db).unwrap(), bundle, name, None) {
        Err(EngineError::ManifestMismatch { run_id, detail }) => {
            assert_eq!(run_id, name);
            assert!(detail.contains("engine"), "{detail}");
        }
        Err(other) => panic!("expected a manifest mismatch, got {other}"),
        Ok(_) => panic!("resume opened a run pinned to an engine that has moved"),
    }

    let state = conclude(&mut store, name, "vyanakiev", "the engine moved; close it").unwrap();
    assert_eq!(
        state.status,
        Status::Stopped,
        "the same journal the pinned bundle locked out reaches its conclusion",
    );
}
