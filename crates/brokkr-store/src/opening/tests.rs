//! The opening's verdicts against the whole read's (decision 0089 ruling
//! 5): over every journal `load` refuses, the opening refuses with the
//! same variant and the same words, and over every journal it reads, the
//! opening stands at the head `load` read to.

use std::path::Path;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{ChainError, EventEnvelope, EventType};
use serde_json::error::Category;
use serde_json::json;

use crate::test_support::{plant_event, plant_run};
use crate::{Opened, Store, StoreError};

/// A verdict, comparable: the head opened, or the refusal's variant with
/// what it names, and its words.
#[derive(Debug, PartialEq)]
enum Verdict {
    Opened(Opened),
    Chain(ChainError, String),
    Json((Category, usize, usize), String),
    NotFound(String, String),
    Unread(String),
}

fn verdict(read: Result<Opened, StoreError>) -> Verdict {
    let error = match read {
        Ok(opened) => return Verdict::Opened(opened),
        Err(error) => error,
    };
    let words = error.to_string();
    match error {
        StoreError::Chain(chain) => Verdict::Chain(chain, words),
        StoreError::Json(json) => {
            Verdict::Json((json.classify(), json.line(), json.column()), words)
        }
        StoreError::RunNotFound(run_id) => Verdict::NotFound(run_id, words),
        StoreError::Sqlite(_) => Verdict::Unread(words),
        other => panic!("no verdict reads {other:?}"),
    }
}

/// `load`'s verdict on `run_id`, as the head it read to.
fn loaded(store: &Store, run_id: &str) -> Verdict {
    verdict(store.load(run_id).map(|events| match events.last() {
        Some(last) => Opened {
            seq: last.seq,
            hash: last.event_hash.clone(),
        },
        None => Opened {
            seq: 0,
            hash: ZERO_HASH.to_string(),
        },
    }))
}

/// A run of four events, `run/started` and three phases, appended by the
/// store's own writer to `run_id`, and its events.
fn appended(store: &mut Store, run_id: &str) -> Vec<EventEnvelope> {
    store
        .create_run(run_id, "feat", "self", &json!({}))
        .unwrap();
    let run = [
        (
            EventType::RunStarted,
            json!({"feature": "feat", "manifest": {}}),
        ),
        (EventType::PhaseEntered, json!({"phase": "intake"})),
        (EventType::PhaseEntered, json!({"phase": "implement"})),
        (EventType::PhaseEntered, json!({"phase": "review"})),
    ];
    run.into_iter()
        .map(|(kind, payload)| store.append_next(run_id, kind, payload, None, None))
        .collect::<Result<_, _>>()
        .unwrap()
}

/// What a planted row holds: an envelope's text, or the SQL value of
/// bytes that are not text at all, or not UTF-8.
enum Row {
    Text(String),
    Raw(&'static str),
}

/// `event` as the store serializes it.
fn text(event: &EventEnvelope) -> Row {
    Row::Text(serde_json::to_string(event).unwrap())
}

/// `event` changed by `edit` and sealed again over its new content.
fn resealed(event: &EventEnvelope, edit: impl FnOnce(&mut EventEnvelope)) -> Row {
    let mut edited = event.clone();
    edit(&mut edited);
    text(&edited.sealed())
}

/// Plant run `run_id` in the journal at `db`, its rows at seq 1 onward.
fn plant(db: &Path, run_id: &str, rows: Vec<Row>) {
    let conn = rusqlite::Connection::open(db).unwrap();
    plant_run(&conn, run_id, "{}").unwrap();
    for (seq, row) in (1i64..).zip(rows) {
        match row {
            Row::Text(text) => plant_event(&conn, run_id, &seq, "x", &text).unwrap(),
            Row::Raw(value) => conn
                .execute(
                    &format!(
                        "INSERT INTO events (run_id, seq, event_hash, envelope)
                         VALUES (?1, ?2, 'x', {value})"
                    ),
                    rusqlite::params![run_id, seq],
                )
                .unwrap(),
        };
    }
}

/// A journal at `db` holding run `whole` as its writer appended it, run
/// `empty` with no event, and beside them each run `load` refuses, planted
/// from `whole`'s events; and `whole`'s events.
fn journal(db: &Path) -> Vec<EventEnvelope> {
    let mut writer = Store::open(db).unwrap();
    let events = appended(&mut writer, "whole");
    writer
        .create_run("empty", "feat", "self", &json!({}))
        .unwrap();
    let entered = serde_json::to_string(&events[1]).unwrap();
    let edited = Row::Text(entered.replace("intake", "edited"));
    let unparsed = || Row::Text("not an envelope".to_string());
    let journals = [
        (
            "gap",
            vec![text(&events[0]), text(&events[1]), text(&events[3])],
        ),
        (
            "broken",
            vec![
                text(&events[0]),
                text(&events[1]),
                resealed(&events[2], |event| event.previous_hash = ZERO_HASH.into()),
            ],
        ),
        (
            "edited",
            vec![text(&events[0]), edited, text(&events[2]), text(&events[3])],
        ),
        (
            "foreign",
            vec![
                text(&events[0]),
                text(&events[1]),
                resealed(&events[2], |event| event.run_id = "other".into()),
            ],
        ),
        (
            "schema",
            vec![
                text(&events[0]),
                resealed(&events[1], |event| event.event_schema_version = 2),
            ],
        ),
        (
            "unparsed-after-a-defect",
            vec![
                text(&events[0]),
                resealed(&events[1], |event| event.previous_hash = ZERO_HASH.into()),
                text(&events[2]),
                unparsed(),
            ],
        ),
        (
            "unparsed-twice",
            vec![text(&events[0]), unparsed(), Row::Text("{".to_string())],
        ),
        (
            "unread",
            vec![text(&events[0]), unparsed(), Row::Raw("X'00'")],
        ),
        (
            "not-utf8",
            vec![text(&events[0]), Row::Raw("CAST(X'FF' AS TEXT)")],
        ),
    ];
    for (run_id, rows) in journals {
        plant(db, run_id, rows);
    }
    events
}

/// The binding of ruling 5's refusal: every journal `load` refuses, the
/// opening refuses with the same variant naming the same seqs, in the same
/// words — a gap, a broken link, an event edited below the head, a foreign
/// run, a schema version not supported, a run the journal lacks, and a row
/// that does not parse or cannot be read, as a blob or as text that is not
/// UTF-8, which outranks a chain defect
/// before it as `load`'s order has it — and every journal `load` reads,
/// the opening stands at the head it read to: the run's last event, or
/// the empty head for a run with no event. The opening that keeps what it
/// verifies gives the same verdicts and keeps the events `load` reads.
/// Read-only, as every readout opens a run (decision 0014).
#[test]
fn a_run_is_opened_as_the_whole_read_reads_it() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let events = journal(&db);
    let store = Store::open_read_only(&db).unwrap();
    let chain = |defect: ChainError| verdict(Err(StoreError::Chain(defect)));
    let unparsed = || {
        let error = serde_json::from_str::<EventEnvelope>("not an envelope").unwrap_err();
        let words = StoreError::Json(error).to_string();
        Verdict::Json((Category::Syntax, 1, 2), words)
    };
    let last = &events[3];
    let expected = [
        (
            "whole",
            Verdict::Opened(Opened {
                seq: 4,
                hash: last.event_hash.clone(),
            }),
        ),
        (
            "empty",
            Verdict::Opened(Opened {
                seq: 0,
                hash: ZERO_HASH.to_string(),
            }),
        ),
        (
            "absent",
            Verdict::NotFound("absent".into(), "run 'absent' not found".into()),
        ),
        (
            "gap",
            chain(ChainError::SeqGap {
                seq: 4,
                expected: 3,
            }),
        ),
        (
            "broken",
            chain(ChainError::BrokenChain {
                seq: 3,
                prev_seq: 2,
            }),
        ),
        ("edited", chain(ChainError::BadHash { seq: 2 })),
        ("foreign", chain(ChainError::ForeignRun { seq: 3 })),
        (
            "schema",
            chain(ChainError::BadSchemaVersion { seq: 2, found: 2 }),
        ),
        ("unparsed-after-a-defect", unparsed()),
        ("unparsed-twice", unparsed()),
        (
            "unread",
            Verdict::Unread("sqlite: Invalid column type Blob at index: 0, name: envelope".into()),
        ),
        (
            "not-utf8",
            Verdict::Unread(
                "sqlite: Conversion error from type Text at index: 0, \
                 invalid utf-8 sequence of 1 bytes from index 0"
                    .into(),
            ),
        ),
    ];
    for (run_id, verdict_of) in expected {
        let opened = verdict(store.open_run(run_id));
        assert_eq!(opened, verdict_of, "{run_id}");
        assert_eq!(opened, loaded(&store, run_id), "{run_id} against load");
        let kept = store.open_kept(run_id).map(|(opened, kept)| {
            let text = |events: &[EventEnvelope]| serde_json::to_string(events).unwrap();
            let loaded = store.load(run_id).unwrap();
            assert_eq!(text(&kept), text(&loaded), "{run_id} kept");
            opened
        });
        assert_eq!(verdict(kept), verdict_of, "{run_id} kept");
    }
    assert_eq!(
        store.head_hash("whole").unwrap(),
        (4, last.event_hash.clone())
    );
}

/// What a contended read names: its operation, and the words after the
/// time it waited, which varies.
fn contention(read: Result<impl std::fmt::Debug, StoreError>) -> (&'static str, String) {
    let error = read.unwrap_err();
    let words = error.to_string();
    let StoreError::Contended { operation, .. } = error else {
        panic!("not contention: {words}");
    };
    let (_, after) = words.split_once("ms of ").unwrap();
    (operation, after.to_string())
}

/// The opening waits out a peer's lock under the store's patience as
/// `load` does, and answers typed contention named `load`'s, in `load`'s
/// words, so a readout that moves from `load` to the opening refuses the
/// same. The journal is in rollback mode, where a peer's exclusive lock
/// shuts readers out.
#[test]
fn the_opening_contends_as_load_does() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    appended(&mut Store::open(&db).unwrap(), "whole");
    rusqlite::Connection::open(&db)
        .unwrap()
        .pragma_update(None, "journal_mode", "DELETE")
        .unwrap();
    let mut reader = Store::open_read_only(&db).unwrap();
    reader
        .set_patience(std::time::Duration::from_millis(50))
        .unwrap();
    let holder = rusqlite::Connection::open(&db).unwrap();
    holder.execute_batch("BEGIN EXCLUSIVE").unwrap();
    let words = ("load", "load; nothing was written".to_string());
    assert_eq!(contention(reader.open_run("whole")), words);
    assert_eq!(contention(reader.open_kept("whole")), words);
    assert_eq!(contention(reader.load("whole")), words);
    drop(holder);
    assert_eq!(reader.open_run("whole").unwrap().seq, 4);
}
