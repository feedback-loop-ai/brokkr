//! Shape (B) of #440, through the shipped binary: a command that would
//! dispose of itself on a concluded run is refused before it writes, and
//! the run's read surfaces still read it.

use std::path::Path;
use std::process::{Command, Output};

use brokkr_core::envelope::EventType;

#[path = "../../../tests/support/late.rs"]
mod late;

#[path = "../../../tests/support/repair.rs"]
mod repair;
use brokkr_core::fold::Status;
use serde_json::json;

fn brokkr(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}

/// A run that has opened and done nothing else: `conclude` can close it
/// without a bundle, because a stop concludes wherever the run stands.
fn started(db: &Path) {
    let mut store = brokkr_store::Store::open(db).unwrap();
    store.create_run("late", "f", "test", &json!({})).unwrap();
    store
        .append_next(
            "late",
            EventType::RunStarted,
            json!({"feature": "f", "manifest": {}}),
            None,
            None,
        )
        .unwrap();
}

#[test]
fn an_operator_stop_after_a_conclusion_refuses_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let db = db.to_str().unwrap();
    started(Path::new(db));

    let concluded = brokkr(
        dir.path(),
        &[
            "conclude",
            "--run",
            "late",
            "--cause",
            "withdrawn",
            "--reason",
            "closing",
            "--db",
            db,
        ],
    );
    assert_eq!(
        concluded.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&concluded.stderr)
    );
    let before = brokkr_store::Store::open_read_only(Path::new(db)).unwrap();
    let before_events = before.load("late").unwrap();
    assert_eq!(
        before_events.last().unwrap().event_type,
        EventType::RunStopped
    );
    let before = (before_events.len(), before.head_hash("late").unwrap());

    let late = brokkr(
        dir.path(),
        &[
            "operator",
            "stop",
            "--run",
            "late",
            "--cause",
            "withdrawn",
            "--reason",
            "again",
            "--db",
            db,
        ],
    );
    assert_eq!(late.status.code(), Some(1), "{late:?}");
    assert_eq!(
        String::from_utf8_lossy(&late.stderr),
        "error: run 'late' is already concluded (stopped); stop appends nothing\n"
    );

    let after = brokkr_store::Store::open_read_only(Path::new(db)).unwrap();
    let after_events = after.load("late").unwrap();
    let after = (after_events.len(), after.head_hash("late").unwrap());
    assert_eq!(
        after, before,
        "the refused command appended nothing, not even operator/commanded"
    );

    let runs = brokkr(dir.path(), &["runs", "--json", "--db", db]);
    assert_eq!(runs.status.code(), Some(0), "{runs:?}");
    assert!(String::from_utf8_lossy(&runs.stdout).contains("late"));
    let inspect = brokkr(
        dir.path(),
        &["inspect", "--run", "late", "--json", "--db", db],
    );
    assert_eq!(inspect.status.code(), Some(0), "{inspect:?}");
    assert!(String::from_utf8_lossy(&inspect.stdout).contains("stopped"));
}

/// A run with an attempt in flight: the journal a live driver is working
/// under when `conclude` closes the run beneath it.
fn in_flight(db: &Path, run: &str) -> brokkr_store::Store {
    let mut store = brokkr_store::Store::open(db).unwrap();
    store.create_run(run, "f", "test", &json!({})).unwrap();
    for (event_type, payload, attempt) in [
        (
            EventType::RunStarted,
            json!({"feature": "f", "manifest": {}}),
            None,
        ),
        (EventType::PhaseEntered, json!({"phase": "work"}), None),
        (
            EventType::EffectRequested,
            json!({"effect_id": "e", "seat": "work"}),
            None,
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "e", "attempt_id": "a"}),
            Some("a".to_string()),
        ),
    ] {
        store
            .append_next(run, event_type, payload, None, attempt)
            .unwrap();
    }
    store
}

/// Shape (A) through the shipped binary: a run concluded while its
/// attempt was open. The driver's next checkpoint meets the store's
/// terminal gate as a typed refusal and writes nothing, and the read
/// surfaces still read the run as stopped.
#[test]
fn a_run_concluded_beneath_a_live_driver_reads_stopped() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let db = db.to_str().unwrap();
    drop(in_flight(Path::new(db), "beneath"));
    let concluded = brokkr(
        dir.path(),
        &[
            "conclude",
            "--run",
            "beneath",
            "--cause",
            "withdrawn",
            "--reason",
            "closing",
            "--db",
            db,
        ],
    );
    assert_eq!(
        concluded.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&concluded.stderr)
    );

    let mut store = brokkr_store::Store::open(Path::new(db)).unwrap();
    let before = store.head_hash("beneath").unwrap();
    let error = store
        .append_next(
            "beneath",
            EventType::EffectCheckpointed,
            json!({"effect_id": "e", "attempt_id": "a",
                   "checkpoint": {"step": "seat-turn", "turn": 1}}),
            None,
            Some("a".into()),
        )
        .unwrap_err();
    assert!(
        matches!(
            error,
            brokkr_store::StoreError::AfterTerminal {
                status: Status::Stopped,
                event_type: EventType::EffectCheckpointed,
                ..
            }
        ),
        "{error:?}"
    );
    assert_eq!(store.head_hash("beneath").unwrap(), before);
    drop(store);

    let runs = brokkr(dir.path(), &["runs", "--json", "--db", db]);
    assert_eq!(runs.status.code(), Some(0), "{runs:?}");
    assert!(String::from_utf8_lossy(&runs.stdout).contains("beneath"));
    let inspect = brokkr(
        dir.path(),
        &["inspect", "--run", "beneath", "--json", "--db", db],
    );
    assert_eq!(inspect.status.code(), Some(0), "{inspect:?}");
    assert!(String::from_utf8_lossy(&inspect.stdout).contains("stopped"));
}

/// A run stopped at seq 5: an attempt in flight concluded by the
/// operator.
fn concluded_at_five(db: &Path) {
    let mut store = in_flight(db, "late");
    store
        .append_next(
            "late",
            EventType::RunStopped,
            json!({"reason": "concluded"}),
            None,
            None,
        )
        .unwrap();
}

/// A fresh directory and its `forge.db`, as the path text `--db` takes.
fn staged() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db").to_str().unwrap().to_string();
    (dir, db)
}

/// Plant `events` after the run's head, behind the store, each chained
/// onto the one before: the shapes 0.12.0 and a late driver left.
fn planted(db: &Path, events: &[(EventType, serde_json::Value)]) {
    let store = brokkr_store::Store::open_read_only(db).unwrap();
    let head = store.load("late").unwrap().pop().unwrap();
    drop(store);
    let connection = rusqlite::Connection::open(db).unwrap();
    brokkr_store::test_support::plant_chained(&connection, "late", &head, events).unwrap();
}

/// `brokkr operator repair` on run `late`, as the operator types it, with
/// `USER` pinned so the annotation's author is the test's.
fn repair_as(dir: &Path, db: &str, user: &str, reason: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .current_dir(dir)
        .env("USER", user)
        .args([
            "operator", "repair", "--run", "late", "--reason", reason, "--db", db,
        ])
        .output()
        .unwrap()
}

/// The repair succeeded at `seq`, and the verb's words say so.
fn repaired_at(dir: &Path, db: &str, user: &str, reason: &str, seq: u64) {
    let repaired = repair_as(dir, db, user, reason);
    let stderr = String::from_utf8_lossy(&repaired.stderr);
    assert_eq!(repaired.status.code(), Some(0), "{stderr}");
    let said = format!("recorded repair at seq {seq} on late");
    assert!(stderr.contains(&said), "{stderr}");
}

/// `brokkr inspect --run late` printed every `needle`.
fn inspect_shows(dir: &Path, db: &str, needles: &[&str]) {
    let inspect = brokkr(dir, &["inspect", "--run", "late", "--db", db]);
    assert_eq!(inspect.status.code(), Some(0), "{inspect:?}");
    let shown = String::from_utf8_lossy(&inspect.stdout);
    for needle in needles {
        assert!(shown.contains(needle), "{shown}");
    }
}

/// A run stopped at five with a settlement planted after it, repaired by
/// the shipped binary: shape (A)'s setup.
fn repaired_settlement() -> (tempfile::TempDir, String) {
    let (dir, db) = staged();
    concluded_at_five(Path::new(&db));
    planted(Path::new(&db), &[late::settlement()]);
    repaired_at(dir.path(), &db, "repairer", "the tail is evidence", 7);
    (dir, db)
}

/// `brokkr runs` says the settlement's sentence and its JSON row carries
/// the tail: the one assertion both the whole read and a summary row make.
fn runs_shows_settlement_tail(dir: &Path, db: &str) {
    let runs = brokkr(dir, &["runs", "--db", db]);
    assert_eq!(runs.status.code(), Some(0), "{runs:?}");
    let listed = String::from_utf8_lossy(&runs.stdout);
    assert!(
        listed.contains("a late tail of 1 event, repaired by repairer"),
        "{listed}"
    );

    let json = brokkr(dir, &["runs", "--json", "--db", db]);
    assert_eq!(json.status.code(), Some(0), "{json:?}");
    let parsed: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    let tail = &parsed["runs"][0]["late_tail"];
    assert_eq!(tail["operator"], "repairer");
    assert_eq!(tail["count"], 1);
    assert_eq!(tail["first_seq"], 6);
    assert_eq!(tail["last_seq"], 6);
    assert!(
        tail.get("events").is_none(),
        "the row keeps the sentence, not the per-event text"
    );
}

/// `brokkr operator repair` over shape (A), through the shipped binary:
/// the settlement is named, the run folds as stopped, and `inspect` and
/// `runs` show the late tail it kept.
#[test]
fn an_operator_repair_of_a_settlement_reads_as_stopped_with_its_late_tail() {
    let (dir, db) = repaired_settlement();
    inspect_shows(
        dir.path(),
        &db,
        &[
            "stopped",
            "a late tail of 1 event, repaired by repairer",
            "seq 6 · effect/succeeded",
        ],
    );

    runs_shows_settlement_tail(dir.path(), &db);
}

/// Shape (B) through the shipped binary: the stop command and acceptance
/// 0.12.0 left after the conclusion, repaired as one range.
#[test]
fn an_operator_repair_of_a_late_stop_reads_as_stopped_with_its_late_tail() {
    let (dir, db) = staged();
    concluded_at_five(Path::new(&db));
    planted(Path::new(&db), &late::late_stop());
    repaired_at(dir.path(), &db, "operator", "0.12.0 left it", 8);
    inspect_shows(
        dir.path(),
        &db,
        &[
            "a late tail of 2 events, repaired by operator",
            "seq 6 · operator/commanded",
            "seq 7 · operator/accepted",
        ],
    );
    let inspect = brokkr(
        dir.path(),
        &["inspect", "--run", "late", "--json", "--db", &db],
    );
    assert_eq!(inspect.status.code(), Some(0), "{inspect:?}");
    let view: serde_json::Value = serde_json::from_slice(&inspect.stdout).unwrap();
    assert_eq!(view["summary"]["status"], "stopped");
    assert_eq!(
        view["summary"]["late_tail"]["events"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(view["summary"]["late_tail"]["terminal_seq"], 5);
}

/// F3: the late-tail sentence survives a writable open. The repair leaves
/// its summary row behind; a writable open refolds it, and `brokkr runs`
/// then reads the row rather than the run whole. The row carries the tail,
/// so the sentence and its events are still there.
#[test]
fn a_repaired_runs_late_tail_survives_a_writable_open() {
    let (dir, db) = repaired_settlement();

    // The writable open refolds the row the repair left behind it.
    drop(brokkr_store::Store::open(Path::new(&db)).unwrap());

    runs_shows_settlement_tail(dir.path(), &db);
}

/// A repair that succeeded at `seq` and names the lawful classify at
/// seq 6, whose words both L8 and L4 read.
fn repair_naming_the_classify(dir: &Path, db: &str, seq: u64) -> String {
    let repaired = repair_as(dir, db, "repairer", "kept");
    let stderr = String::from_utf8_lossy(&repaired.stderr).into_owned();
    assert_eq!(repaired.status.code(), Some(0), "{stderr}");
    assert!(
        stderr.contains(&format!("recorded repair at seq {seq} on late")),
        "{stderr}"
    );
    assert!(
        stderr.contains("seq 6 · operator/commanded (classify)"),
        "the skipped annotation is named by seq and type: {stderr}"
    );
    stderr
}

/// L8: a lawful classify between the terminal event and the break is in
/// the tail, so the repair makes it late. The verb names it by seq and
/// type, and says to record it again after the repair.
#[test]
fn a_repair_names_the_lawful_annotation_it_skips() {
    let (dir, db) = staged();
    concluded_at_five(Path::new(&db));
    planted(Path::new(&db), &[late::classified(), late::settlement()]);
    let stderr = repair_naming_the_classify(dir.path(), &db, 8);
    assert!(
        stderr.contains("record it again after the repair"),
        "the operator is told to record it again: {stderr}"
    );
    let store = brokkr_store::Store::open_read_only(Path::new(&db)).unwrap();
    let written = store.load("late").unwrap().pop().unwrap();
    assert_eq!(
        written.payload["args"],
        repair::repair_args(5, 6, 7, &["operator/commanded", "effect/succeeded"], "kept"),
        "the repair names exactly the tail it skips"
    );
    inspect_shows(dir.path(), &db, &["stopped", "a late tail of 2 events"]);
}

/// R4: the verb anchors on the FIRST terminal event, as the fold and
/// `operator_repair` do. A lawful classify after it is named even when a
/// second terminal-looking event is the break.
#[test]
fn a_repair_names_the_lawful_annotation_before_a_second_terminal_event() {
    let (dir, db) = staged();
    concluded_at_five(Path::new(&db));
    planted(
        Path::new(&db),
        &[
            late::classified(),
            (
                EventType::RunCompleted,
                json!({"reason": "the second terminal event is the break"}),
            ),
        ],
    );
    let stderr = repair_naming_the_classify(dir.path(), &db, 8);
    assert!(
        stderr.contains("record it again after the repair"),
        "the operator is told to record it again: {stderr}"
    );
}

/// R4: a second repair's tail holds the first repair record, which is no
/// lawful annotation to record again. The verb names only the annotations
/// the tail's read finds.
#[test]
fn a_repair_does_not_list_an_earlier_repair_as_an_annotation_to_record_again() {
    let (dir, db) = staged();
    concluded_at_five(Path::new(&db));
    planted(Path::new(&db), &[late::settlement(), late::settlement()]);
    repaired_at(dir.path(), &db, "repairer", "first", 8);
    planted(Path::new(&db), &[late::settlement()]);
    let repaired = repair_as(dir.path(), &db, "repairer", "second");
    let stderr = String::from_utf8_lossy(&repaired.stderr);
    assert_eq!(repaired.status.code(), Some(0), "{stderr}");
    assert!(
        stderr.contains("recorded repair at seq 10 on late"),
        "{stderr}"
    );
    assert!(
        !stderr.contains("(repair)"),
        "an earlier repair record is not an annotation to record again: {stderr}"
    );
    assert!(
        !stderr.contains("record it again after the repair"),
        "there is no lawful annotation to record again: {stderr}"
    );
}

/// `repair` takes `--run`, `--reason` and the journal: each flag that
/// belongs to another verb refuses before anything is read or written.
/// Removal control: drop any one operand of the check, and that flag's
/// case reaches the verb and says something else.
#[test]
fn an_operator_repair_refuses_every_flag_another_verb_takes() {
    let (dir, db) = staged();
    started(Path::new(&db));
    for flag in [
        ["--findings", "1"],
        ["--by-run", "other"],
        ["--cause", "superseded"],
        ["--by-ref", "ref"],
    ] {
        let mut args = vec![
            "operator", "repair", "--run", "late", "--reason", "why", "--db", &db,
        ];
        args.extend(flag);
        let refused = brokkr(dir.path(), &args);
        let stderr = String::from_utf8_lossy(&refused.stderr);
        assert_eq!(refused.status.code(), Some(1), "{flag:?}: {stderr}");
        assert!(
            stderr
                .contains("belong to other verbs; 'repair' takes --run, --reason and the journal"),
            "{flag:?}: {stderr}"
        );
    }
}

/// A live run refuses the verb, naming it, and writes nothing.
#[test]
fn an_operator_repair_on_a_live_run_refuses_and_writes_nothing() {
    let (dir, db) = staged();
    started(Path::new(&db));
    let before = brokkr_store::Store::open_read_only(Path::new(&db)).unwrap();
    let before = before.load("late").unwrap().len();
    let refused = brokkr(
        dir.path(),
        &[
            "operator", "repair", "--run", "late", "--reason", "why", "--db", &db,
        ],
    );
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    assert!(
        String::from_utf8_lossy(&refused.stderr)
            .contains("repair refused, nothing was written: run 'late' has no terminal event"),
        "{}",
        String::from_utf8_lossy(&refused.stderr)
    );
    let after = brokkr_store::Store::open_read_only(Path::new(&db)).unwrap();
    assert_eq!(after.load("late").unwrap().len(), before);
}

/// L4: a reserved word standing past the break was never applied — an
/// older binary journaled it and the fold refused it — so the verb does
/// not name it as a lawful annotation the repair makes late. Only the
/// classify that folded before the break is named.
#[test]
fn a_repair_names_only_the_annotation_that_folded_before_the_break() {
    let (dir, db) = staged();
    concluded_at_five(Path::new(&db));
    planted(
        Path::new(&db),
        &[
            late::classified(),
            late::settlement(),
            (
                EventType::OperatorCommanded,
                json!({"command_id": "c-settle", "command": "settle",
                       "operator": "operator", "args": {}}),
            ),
        ],
    );
    let stderr = repair_naming_the_classify(dir.path(), &db, 9);
    assert!(
        !stderr.contains("seq 8"),
        "a reserved word past the break was never applied: {stderr}"
    );
}
