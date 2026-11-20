//! Large journals planted in one transaction (#1130). One `append_next`
//! per event commits, and so syncs the WAL, once per event, and judges
//! the writer's carried account each time: most of a large fixture's time
//! on a disk-backed TMPDIR. [`plant_after`] takes the writer's own steps
//! instead, each row fenced, sealed and inserted as `append_once` does it,
//! and the carried account judged once over them all as `chain_once`
//! judges a chain, in one transaction. [`assert_planted_rows`] holds the
//! planted rows to the ones `append_next` leaves, for each suite that
//! plants through it. [`plant_older_at_once`] plants an older writer's
//! rows in one transaction too.

use std::collections::HashMap;

use brokkr_core::envelope::{EventEnvelope, EventType};
use rusqlite::types::Value;
use rusqlite::{Connection, TransactionBehavior};
use serde_json::{json, Value as Json};

use crate::chain::{fence, seal_and_insert};
use crate::test_support::{every_run_reads_as_its_fold, plant_older_writer};
use crate::{head_of, summary, Store, StoreError};

/// `events` appended to `run_id` after its head in one transaction: each
/// fenced and sealed with no cause and no attempt, chained onto the one
/// before it, and the summary, the seat rows and the indexes judged once
/// over them all, as `append_next` would leave them one by one.
pub(crate) fn plant_after(store: &mut Store, run_id: &str, events: Vec<(EventType, Json)>) {
    let carried = store.carried.clone();
    let tx = store
        .conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    let found = head_of(&tx, run_id).unwrap();
    let mut head = found.clone();
    let mut sealed = Vec::with_capacity(events.len());
    for (event_type, payload) in events {
        fence(&tx, run_id, event_type, &payload, head.0 + 1)
            .unwrap()
            .unwrap();
        let event = sealed_into(&tx, run_id, &head, event_type, payload);
        head = (event.seq, event.event_hash.clone());
        sealed.push(event);
    }
    let settled = summary::judge(&tx, carried.as_ref(), (found.0, &found.1), &sealed).unwrap();
    tx.commit().unwrap();
    store.settle((run_id, sealed.len()), settled);
}

/// `events` planted as [`plant_older_writer`] plants them, inside one
/// transaction: its run row and each event row otherwise commit, and so
/// sync, one by one.
pub(crate) fn plant_older_at_once(store: &Store, run_id: &str, events: &[EventEnvelope]) {
    let tx = store.conn.unchecked_transaction().unwrap();
    plant_older_writer(store, run_id, events);
    tx.commit().unwrap();
}

/// `payload` sealed as `run_id`'s next event after `head` and inserted
/// inside `tx` by `chain::seal_and_insert`, with no cause and no attempt;
/// a seq already taken is a panic naming it, for a plant appends after
/// the head it read.
fn sealed_into(
    tx: &Connection,
    run_id: &str,
    head: &(u64, String),
    event_type: EventType,
    payload: Json,
) -> EventEnvelope {
    match seal_and_insert(tx, run_id, head.clone(), event_type, payload, None, None) {
        Ok(envelope) => envelope,
        Err(StoreError::AppendConflict { seq }) => panic!("seq {seq} is the run's next"),
        Err(error) => panic!("seq {} seals: {error}", head.0 + 1),
    }
}

/// The columns two journals of the same events may differ in past their
/// stamps: `meta`'s values, which hold the journal's own id, and each
/// seat row's digest, a seal of its own values, stamps among them, which
/// a seat read checks against those values and [`assert_planted_rows`]
/// holds by [`every_run_reads_as_its_fold`].
const DIFFERS: [(&str, &str); 2] = [("meta", "value"), ("seat_summary", "digest")];

/// What `text` reads as once `stamps` names each event's id, hash and
/// stamp by its seq: an envelope with those fields named, a hash or id by
/// its seq's name, a stamp as `stamp`, and any other text as itself.
fn named(text: &str, stamps: &HashMap<String, String>) -> Value {
    if let Some(name) = stamps.get(text) {
        return Value::Text(name.clone());
    }
    match serde_json::from_str::<Json>(text) {
        Ok(Json::Object(mut envelope)) if envelope.contains_key("event_hash") => {
            for field in ["event_id", "recorded_at", "previous_hash", "event_hash"] {
                if let Some(Json::String(value)) = envelope.get(field) {
                    let name = stamps.get(value).cloned().unwrap_or_else(|| value.clone());
                    envelope.insert(field.to_string(), json!(name));
                }
            }
            Value::Text(Json::Object(envelope).to_string())
        }
        _ => Value::Text(text.to_string()),
    }
}

/// What two journals of the same events stamp differently, named alike:
/// each event's id and hash by its run and seq, and every clock reading,
/// each run's `created_at` and each event's `recorded_at`, as text and as
/// the milliseconds the index keeps of it (`at_ms`), as `stamp`.
struct Stamps {
    text: HashMap<String, String>,
    ms: Vec<i64>,
}

/// `store`'s [`Stamps`] over `runs`.
fn stamps(store: &Store, runs: &[String]) -> Stamps {
    use time::format_description::well_known::Rfc3339;
    let mut stamps = Stamps {
        text: HashMap::new(),
        ms: Vec::new(),
    };
    let stamp = |stamps: &mut Stamps, text: String| {
        let stamped = time::OffsetDateTime::parse(&text, &Rfc3339).unwrap();
        stamps
            .ms
            .push((stamped.unix_timestamp_nanos() / 1_000_000) as i64);
        stamps.text.insert(text, "stamp".to_string());
    };
    for run_id in runs {
        let created: String = store
            .conn
            .query_row(
                "SELECT created_at FROM runs WHERE run_id = ?1",
                [run_id],
                |row| row.get(0),
            )
            .unwrap();
        stamp(&mut stamps, created);
        for event in store.load(run_id).unwrap() {
            let at = format!("{run_id}@{}", event.seq);
            stamps.text.insert(event.event_id, format!("id {at}"));
            stamps.text.insert(event.event_hash, format!("hash {at}"));
            stamp(&mut stamps, event.recorded_at);
        }
    }
    stamps
}

/// One table's rows in `store`, each row's columns but those [`DIFFERS`]
/// names, named by `stamps`, sorted.
fn table_rows(store: &Store, table: &str, stamps: &Stamps) -> Vec<String> {
    let mut select = store
        .conn
        .prepare(&format!("SELECT * FROM \"{table}\""))
        .unwrap();
    let columns: Vec<String> = select.column_names().map(str::to_string).collect();
    let mut found: Vec<String> = select
        .query_map([], |row| {
            let mut kept = Vec::new();
            for (at, column) in columns.iter().enumerate() {
                if DIFFERS.contains(&(table, column.as_str())) {
                    continue;
                }
                let value = match row.get::<_, Value>(at)? {
                    Value::Text(text) => named(&text, &stamps.text),
                    Value::Integer(ms) if stamps.ms.contains(&ms) => Value::Text("stamp".into()),
                    value => value,
                };
                kept.push(format!("{column}={value:?}"));
            }
            Ok(kept.join(" "))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    found.sort();
    found
}

/// Every table in `store`'s journal and its rows, as [`table_rows`]
/// reads them: what a read could find.
fn rows(store: &Store, runs: &[String]) -> Vec<(String, Vec<String>)> {
    let stamps = stamps(store, runs);
    let mut tables = store
        .conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap();
    let names: Vec<String> = tables
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    names
        .into_iter()
        .map(|table| {
            let found = table_rows(store, &table, &stamps);
            (table, found)
        })
        .collect()
}

/// `planted`, whose `runs` were planted through [`plant_after`], holds the
/// rows `written` holds, whose same runs were appended event by event:
/// the same rows in every table, once each event's id, hash and stamp are
/// named alike, but the columns [`DIFFERS`] names; and each of `held`
/// holds a row.
pub(crate) fn assert_same_rows(planted: &Store, written: &Store, runs: &[String], held: &[&str]) {
    let (planted_rows, written_rows) = (rows(planted, runs), rows(written, runs));
    let tables = |rows: &[(String, Vec<String>)]| -> Vec<String> {
        rows.iter().map(|(table, _)| table.clone()).collect()
    };
    assert_eq!(
        tables(&planted_rows),
        tables(&written_rows),
        "the same tables"
    );
    let only = |rows: &[String], other: &[String]| -> Vec<String> {
        rows.iter()
            .filter(|row| !other.contains(row))
            .cloned()
            .collect()
    };
    for ((table, planted), (_, written)) in planted_rows.iter().zip(&written_rows) {
        assert!(
            planted == written,
            "{table}: the planted rows are the writer's; planted only {:?}, written only {:?}",
            only(planted, written),
            only(written, planted)
        );
    }
    for table in held {
        let (_, rows) = planted_rows.iter().find(|(name, _)| name == table).unwrap();
        assert!(!rows.is_empty(), "{table} holds the planted rows");
    }
}

/// [`assert_same_rows`] for runs planted through [`plant_after`], whose
/// summary and seat rows, digests included, also read as their fold.
pub(crate) fn assert_planted_rows(
    planted: &Store,
    written: &Store,
    runs: &[String],
    held: &[&str],
) {
    assert_same_rows(planted, written, runs, held);
    assert_eq!(
        every_run_reads_as_its_fold(planted),
        Vec::<String>::new(),
        "every planted run's summary and seat rows, digests included, read as its fold"
    );
}

/// `run_id`'s events in seq order from 1, each chained onto the one before:
/// a test's journal, built the way the store's seal chains it.
pub(crate) fn chained(run_id: &str, events: Vec<(EventType, Json)>) -> Vec<EventEnvelope> {
    let mut previous = brokkr_core::canonical::ZERO_HASH.to_string();
    events
        .into_iter()
        .enumerate()
        .map(|(at, (event_type, payload))| {
            let seq = at as u64 + 1;
            let event = super::EnvelopeBuilder::new(event_type, payload)
                .run(run_id)
                .seq(seq)
                .event_id(format!("{run_id}-{seq}"))
                .at("2026-01-01T00:00:00Z")
                .previous(previous.clone())
                .sealed();
            previous.clone_from(&event.event_hash);
            event
        })
        .collect()
}

/// An older writer's run planted in one transaction holds the rows it
/// holds planted row by row, its checkpoint facts written after: the
/// planting `checkpoint_tests` gives its 20,000-event history.
#[test]
fn an_older_writer_planted_at_once_holds_the_rows_it_plants_one_by_one() {
    let (_one_dir, mut one) = super::store();
    let (_once_dir, mut once) = super::store();
    let runs = vec!["r".to_string()];
    let checkpoint = json!({"effect_id": "fx", "attempt_id": "a",
                            "checkpoint": {"step": "seat-turn", "member": "m"}});
    let mut events = vec![(
        EventType::EffectStarted,
        json!({"effect_id": "fx", "attempt_id": "a"}),
    )];
    events.extend(vec![(EventType::EffectCheckpointed, checkpoint); 6]);
    let events = chained("r", events);
    plant_older_writer(&one, "r", &events);
    plant_older_at_once(&once, "r", &events);
    for store in [&mut one, &mut once] {
        store.checkpoint_behind_with(|_| {}).unwrap();
    }
    assert_same_rows(&once, &one, &runs, &["event_index", "events", "runs"]);
}
