//! The ordinal tests' journals planted in one transaction (#1130). One
//! `append_next` per event commits, and so syncs the WAL, once per event,
//! and judges the writer's carried account each time: most of a large
//! fixture's time on a disk-backed TMPDIR. [`plant`] takes the writer's
//! own steps instead, each row fenced, sealed and inserted as
//! `append_once` does it, and the carried account judged once over them
//! all as `chain_once` judges a chain, in one transaction. The test below
//! holds the planted rows to the ones `append_next` leaves.

use std::collections::HashMap;

use brokkr_core::envelope::{EventEnvelope, EventType, EVENT_SCHEMA_VERSION};
use rusqlite::types::Value;
use rusqlite::{params, Connection, TransactionBehavior};
use serde_json::{json, Value as Json};

use crate::chain::fence;
use crate::test_support::{append, every_run_reads_as_its_fold, opened};
use crate::{head_of, now_rfc3339, summary, terminal, Store};

/// Run `run_id` opened in `implement` by [`opened`], then `events`
/// appended in one transaction: each fenced and sealed with no cause and
/// no attempt, chained onto the one before it, and the summary, the seat
/// rows and the indexes judged once over them all.
pub(super) fn plant(store: &mut Store, run_id: &str, events: Vec<(EventType, Json)>) {
    opened(store, run_id, "implement", &[]);
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

/// `payload` sealed as `run_id`'s next event after `head` and inserted
/// inside `tx`, as `chain::seal_and_insert` seals and inserts an append,
/// with no cause and no attempt, but through the connection's statement
/// cache: `seal_and_insert` prepares its insert, and with it every
/// trigger the insert fires, once per row, which costs a 20,000-event
/// plant three times its time here. Held to the writer's rows by
/// [`a_planted_journal_holds_the_rows_the_writer_appends`].
fn sealed_into(
    tx: &Connection,
    run_id: &str,
    (last_seq, previous_hash): &(u64, String),
    event_type: EventType,
    payload: Json,
) -> EventEnvelope {
    terminal::refuse(tx, run_id, event_type, *last_seq).unwrap();
    let envelope = EventEnvelope {
        run_id: run_id.to_string(),
        seq: last_seq + 1,
        event_id: uuid::Uuid::new_v4().to_string(),
        event_schema_version: EVENT_SCHEMA_VERSION,
        event_type,
        payload,
        causation_id: None,
        correlation_id: run_id.to_string(),
        attempt_id: None,
        recorded_at: now_rfc3339(),
        previous_hash: previous_hash.clone(),
        event_hash: String::new(),
    }
    .sealed();
    let serialized = serde_json::to_string(&envelope).unwrap();
    let inserted = tx
        .prepare_cached(
            "INSERT OR IGNORE INTO events (run_id, seq, event_hash, envelope)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .unwrap()
        .execute(params![
            run_id,
            envelope.seq as i64,
            envelope.event_hash,
            serialized
        ])
        .unwrap();
    assert_eq!(inserted, 1, "seq {} is the run's next", envelope.seq);
    envelope
}

/// The columns two journals of the same events may differ in past their
/// stamps: `meta`'s values, which hold the journal's own id, and each
/// seat row's digest, a seal of its own values, stamps among them, which
/// a seat read checks against those values and the test holds by
/// [`every_run_reads_as_its_fold`].
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

/// Every shape the ordinal tests plant, one run each: the planted
/// participants, H1's interleaving, two effects on one attempt, three
/// disjoint spans, result-only records on the target's attempt and on
/// another, a member's record, a NUL member and more streams than one.
fn shapes() -> Vec<Vec<(EventType, Json)>> {
    use super::*;
    let mut sequential = two_effects_on_one_attempt();
    sequential.extend(own("gx", "a", 2));
    let stale = |attempt: &str| {
        let mut events = asked_and_started("fx", "a");
        events.push(checked("fx", "a", Some("target"), 1));
        events.extend(results("fx", attempt, "m", 3));
        events.push(checked("fx", "a", Some("target"), 3));
        events
    };
    let mut recorded = interleaved_member();
    recorded.extend(results("fx", "b", "m", 1));
    recorded.push(checked("fx", "b", Some("m"), 2));
    let mut nul = asked_and_started("fx", "a");
    nul.extend(turns("fx", "a", Some("m\u{0}suffix"), 3));
    let mut streams = vec![(EventType::EffectRequested, json!({"effect_id": "fx"}))];
    for attempt in ["a0", "a1", "a2"] {
        streams.push(started("fx", attempt));
        streams.extend(own("fx", attempt, 2));
    }
    vec![
        planted(),
        h1_events(),
        sequential,
        three_spans(4),
        stale("a"),
        stale("b"),
        recorded,
        nul,
        streams,
    ]
}

/// A planted journal holds the rows the writer appends: every shape the
/// ordinal tests use, appended event by event into one journal and
/// planted into another, holds the same rows in every table, `events`,
/// `run_summary`, the seat rows and each index among them, once each
/// event's id, hash and stamp are named alike, but the columns
/// [`DIFFERS`] names; and every planted run reads as its fold.
#[test]
fn a_planted_journal_holds_the_rows_the_writer_appends() {
    let (_written_dir, mut written) = crate::tests::store();
    let (_planted_dir, mut planted) = crate::tests::store();
    let mut runs = Vec::new();
    for (at, events) in shapes().into_iter().enumerate() {
        let run_id = format!("r{at}");
        opened(&mut written, &run_id, "implement", &[]);
        for (kind, payload) in events.clone() {
            append(&mut written, &run_id, kind, payload);
        }
        plant(&mut planted, &run_id, events);
        runs.push(run_id);
    }
    let (planted_rows, written_rows) = (rows(&planted, &runs), rows(&written, &runs));
    let tables = |rows: &[(String, Vec<String>)]| -> Vec<String> {
        rows.iter().map(|(table, _)| table.clone()).collect()
    };
    assert_eq!(
        tables(&planted_rows),
        tables(&written_rows),
        "the same tables"
    );
    for ((table, planted), (_, written)) in planted_rows.iter().zip(&written_rows) {
        let only = |rows: &[String], other: &[String]| -> Vec<String> {
            rows.iter()
                .filter(|row| !other.contains(row))
                .cloned()
                .collect()
        };
        assert!(
            planted == written,
            "{table}: the planted rows are the writer's; planted only {:?}, written only {:?}",
            only(planted, written),
            only(written, planted)
        );
    }
    for table in [
        "event_index",
        "events",
        "run_summary",
        "seat_head",
        "seat_summary",
    ] {
        let (_, held) = planted_rows.iter().find(|(name, _)| name == table).unwrap();
        assert!(!held.is_empty(), "{table} holds the planted rows");
    }
    assert_eq!(
        every_run_reads_as_its_fold(&planted),
        Vec::<String>::new(),
        "every planted run's summary and seat rows, digests included, read as its fold"
    );
}
