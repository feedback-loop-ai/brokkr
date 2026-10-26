//! The aggregate columns (decision 0089 ruling 7): each append keeps them
//! as its fold states them, ruling 4's comparison holds every one of them,
//! a writer from before them sends its row behind, and a table from
//! before them is made anew.

use brokkr_core::envelope::EventType;
use brokkr_core::stamp::parse_millis;
use rusqlite::types::Value;
use serde_json::{json, Value as Json};

use super::COLUMNS;
use crate::summary::summary_intact;
use crate::test_support::{every_run_reads_as_its_fold, lags, lane, plant_envelope};
use crate::tests::envelope_builder::EnvelopeBuilder;
use crate::tests::store;
use crate::{Disagreement, Store};

/// Every aggregate column of `run_id`'s row, in the row's order.
fn kept(store: &Store, run_id: &str) -> Vec<Value> {
    let names: Vec<&str> = COLUMNS.iter().map(|(name, _)| *name).collect();
    let select = format!(
        "SELECT {} FROM run_summary WHERE run_id = ?1",
        names.join(", ")
    );
    let read = |row: &rusqlite::Row<'_>| (0..names.len()).map(|at| row.get(at)).collect();
    store.conn.query_row(&select, [run_id], read).unwrap()
}

fn text(value: &str) -> Value {
    Value::Text(value.to_string())
}

/// `run_id` created, started under a manifest that pins realm `alpha`
/// and names recipe `self`, and carried through `steps`: the last event's
/// time in milliseconds, and the first's.
fn driven(store: &mut Store, run_id: &str, steps: &[(EventType, Json)]) -> (i64, i64) {
    store
        .create_run(run_id, "a feature", "self", &json!({}))
        .unwrap();
    let manifest = json!({
        "bundle_name": "self",
        "realms": {"source": "/work/realms.json", "realm": "alpha",
                   "map": {"realms": [{"name": "alpha", "path": "repos/alpha"}]}},
    });
    let started = json!({"feature": "a feature", "manifest": manifest});
    let mut appended = vec![(EventType::RunStarted, started)];
    appended.extend(steps.iter().cloned());
    let times: Vec<i64> = appended
        .into_iter()
        .map(|(event_type, payload)| {
            let event = store.append_next(run_id, event_type, payload, None, None);
            parse_millis(&event.unwrap().recorded_at).unwrap()
        })
        .collect();
    (times[0], times[times.len() - 1])
}

/// A phase entered, its effect run, and a ruling from it to `next` by
/// `rule`.
fn turn(phase: &str, effect: &str, rule: &str, next: &str) -> Vec<(EventType, Json)> {
    vec![
        (EventType::PhaseEntered, json!({ "phase": phase })),
        (
            EventType::EffectRequested,
            json!({"effect_id": effect, "seat": phase}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": effect, "attempt_id": format!("{effect}-a")}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": effect, "result": {"result": "pass"}}),
        ),
        (
            EventType::TransitionDecided,
            json!({"from": phase, "result": "pass", "rule_id": rule, "next": next}),
        ),
    ]
}

/// An implement, a review that sends the run back, an implement again,
/// and the run's `end` there.
fn reworked(end: Vec<(EventType, Json)>) -> Vec<(EventType, Json)> {
    let mut steps = turn("implement", "i1", "IMPL-DONE", "review");
    steps.extend(turn("review", "r1", "REVIEW-BACK", "implement"));
    steps.extend(turn("implement", "i2", "IMPL-DONE", "stop"));
    steps.extend(end);
    steps
}

#[test]
fn each_append_keeps_the_aggregates_its_fold_states() {
    let (_dir, mut store) = store();
    let stop = (EventType::PhaseEntered, json!({"phase": "stop"}));
    let ended = |ending: EventType| (ending, json!({}));
    let (started, finished) = driven(
        &mut store,
        "by-rule",
        &reworked(vec![stop.clone(), ended(EventType::RunStopped)]),
    );
    let at = |ms: i64| Value::Integer(ms);
    let flags = [Value::Integer(1), Value::Integer(0), Value::Integer(1)];
    let expected = |finished: Value, ending: &str, rule: Value, seq: i64| {
        let mut columns = vec![text("alpha"), text("self"), at(started)];
        columns.extend([finished, text(ending), rule, Value::Integer(2)]);
        columns.extend(flags.clone());
        columns.push(Value::Integer(seq));
        columns
    };
    let rule = text("IMPL-DONE");
    assert_eq!(
        kept(&store, "by-rule"),
        expected(at(finished), "by_rule", rule, 18)
    );
    let (started, finished) = driven(
        &mut store,
        "shipped",
        &reworked(vec![ended(EventType::RunCompleted)]),
    );
    let mut shipped = expected(at(finished), "shipped", Value::Null, 17);
    shipped[2] = at(started);
    assert_eq!(kept(&store, "shipped"), shipped);
    let (started, finished) = driven(
        &mut store,
        "by-operator",
        &reworked(vec![ended(EventType::RunStopped)]),
    );
    let mut stopped = expected(at(finished), "by_operator", Value::Null, 17);
    stopped[2] = at(started);
    assert_eq!(kept(&store, "by-operator"), stopped);
    let (started, _) = driven(&mut store, "open", &turn("implement", "i1", "R", "review"));
    assert_eq!(
        kept(&store, "open"),
        [
            text("alpha"),
            text("self"),
            at(started),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Integer(1),
            Value::Integer(0),
            Value::Integer(0),
            Value::Integer(0),
            Value::Integer(6),
        ]
    );
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// Ruling 4's comparison, which `brokkr doctor` and every writer store's
/// drop make, holds each aggregate column to the run's fold: an edit of
/// any one is named by that column.
#[test]
fn each_aggregate_column_that_is_not_its_fold_s_is_named() {
    let (_dir, mut store) = store();
    let stop = (EventType::PhaseEntered, json!({"phase": "stop"}));
    driven(
        &mut store,
        "r",
        &reworked(vec![stop, (EventType::RunStopped, json!({}))]),
    );
    let held = kept(&store, "r");
    let mut named = Vec::new();
    for (at, (column, _)) in COLUMNS.iter().enumerate() {
        let edit = format!("UPDATE run_summary SET {column} = ?1 WHERE run_id = 'r'");
        let wrong = match &held[at] {
            Value::Text(_) => text("edited"),
            _ => Value::Integer(99),
        };
        store.conn.execute(&edit, [&wrong]).unwrap();
        let found = store.summary_disagreements().unwrap();
        named.extend(found.iter().map(|disagreement| match disagreement {
            Disagreement::Column { column, .. } => column.to_string(),
            other => format!("{other}"),
        }));
        store.conn.execute(&edit, [&held[at]]).unwrap();
    }
    let columns: Vec<&str> = COLUMNS.iter().map(|(name, _)| *name).collect();
    assert_eq!(named, columns);
    store
        .conn
        .execute("UPDATE run_summary SET ending_rule = 'IMPL-HELD'", [])
        .unwrap();
    assert_eq!(
        store.summary_disagreements().unwrap()[0].to_string(),
        "run 'r': its summary row stands at its head, and its ending_rule is not its fold's"
    );
    assert_eq!(every_run_reads_as_its_fold(&store), ["r: ending_rule"]);
    lags(&mut store, &["r"]);
}

/// A binary from before the aggregates writes the folded columns and not
/// their stamp: its row is sent behind, read whole, and refolded by the
/// next writable open, never left at its head with an earlier head's
/// aggregates. A writer that stamps them stays at its head.
#[test]
fn a_writer_from_before_the_aggregates_sends_its_row_behind() {
    let (_dir, mut store) = store();
    driven(&mut store, "r", &turn("implement", "i1", "R", "review"));
    let (seq, previous) = store.head_hash("r").unwrap();
    let entered = EnvelopeBuilder::new(EventType::PhaseEntered, json!({"phase": "review"}))
        .run("r")
        .seq(seq + 1)
        .event_id("r-older")
        .at("2026-01-01T00:00:09Z")
        .previous(previous)
        .sealed();
    plant_envelope(&store.conn, &entered).unwrap();
    let older = "UPDATE run_summary SET folded_seq = head_seq, folded_hash = head_hash";
    store.conn.execute(older, []).unwrap();
    assert_eq!(lane(&store, "r").as_deref(), Some("behind"));
    let folded: Option<i64> = store
        .conn
        .query_row("SELECT folded_seq FROM run_summary", [], |row| row.get(0))
        .unwrap();
    assert_eq!(folded, None);
    lags(&mut store, &["r"]);
    let path = store.path().to_path_buf();
    drop(store);
    let store = Store::open(&path).unwrap();
    assert_eq!(lane(&store, "r").as_deref(), Some("running"));
    assert_eq!(kept(&store, "r")[10], Value::Integer(7));
    let stamped = "UPDATE run_summary SET folded_hash = head_hash, aggregated_seq = head_seq";
    store.conn.execute(stamped, []).unwrap();
    assert_eq!(lane(&store, "r").as_deref(), Some("running"));
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// A journal whose summary a build from before the aggregates made reads
/// whole until a writer opens it, which makes the table anew and refolds
/// every run into it (ruling 8).
#[test]
fn a_summary_from_before_the_aggregates_is_made_anew_and_refolded() {
    let (_dir, mut store) = store();
    driven(&mut store, "r", &turn("implement", "i1", "R", "review"));
    let held = kept(&store, "r");
    let path = store.path().to_path_buf();
    drop(store);
    let older = rusqlite::Connection::open(&path).unwrap();
    older
        .execute_batch("ALTER TABLE run_summary DROP COLUMN review_back")
        .unwrap();
    drop(older);
    let reader = Store::open_read_only(&path).unwrap();
    assert!(!summary_intact(&reader.conn).unwrap());
    assert!(!reader.pages().unwrap());
    drop(reader);
    let store = Store::open(&path).unwrap();
    assert!(summary_intact(&store.conn).unwrap());
    assert!(store.pages().unwrap());
    assert_eq!(lane(&store, "r").as_deref(), Some("running"));
    assert_eq!(kept(&store, "r"), held);
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}
