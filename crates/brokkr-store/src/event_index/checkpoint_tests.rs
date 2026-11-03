//! The checkpoints' dated facts (decision 0089 ruling 7): each
//! `effect/checkpointed` row keeps its time, phase, cost and whether it was
//! reported, its token counts and whether it is its site's session record,
//! ruling 4's comparison holds each column to brokkr-core's one reading, a
//! later session record moves the flag, a writer from before them leaves
//! the row for the open's walk, and a table from before them is altered in
//! place.

use super::checkpoint::{backfilled, checkpointed, COLUMNS, WRITE};
use super::*;
use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::summary::{Bounded, NAME_BYTES};
use rusqlite::params;
use serde_json::{json, Value as Json};

use crate::test_support::{
    append, at, every_run_reads_as_its_fold, lags, opened, plant_older_writer, refolded, text,
};
use crate::tests::envelope_builder::EnvelopeBuilder;
use crate::tests::store;
use crate::Store;

/// A checkpoint row's facts, by [`COLUMNS`]' order.
fn kept(store: &Store, run_id: &str, seq: u64) -> Vec<Value> {
    crate::test_support::kept_facts(store, run_id, seq, &COLUMNS)
}

/// A checkpoint of attempt `a`, naming `member`, its record `record`.
fn checkpoint(member: Option<&str>, record: Json) -> Json {
    let mut checkpoint = record;
    if let (Some(member), Some(object)) = (member, checkpoint.as_object_mut()) {
        object.insert("member".into(), json!(member));
    }
    json!({"effect_id": "fx", "attempt_id": "a", "checkpoint": checkpoint})
}

/// A run of `implement` with one checkpoint of `record` appended.
fn one_checkpoint(store: &mut Store, record: Json) -> EventEnvelope {
    opened(store, "r", "implement", &[]);
    append(
        store,
        "r",
        EventType::EffectCheckpointed,
        checkpoint(Some("m"), record),
    )
}

/// The row a checkpoint appended to `store`'s run `r` keeps.
fn checkpointed_row(store: &mut Store, record: Json) -> Vec<Value> {
    let event = append(
        store,
        "r",
        EventType::EffectCheckpointed,
        checkpoint(Some("m"), record),
    );
    kept(store, "r", event.seq)
}

#[test]
fn each_append_keeps_the_checkpoint_its_record_states() {
    let (_dir, mut store) = store();
    opened(&mut store, "r", "implement", &[]);
    let event = append(
        &mut store,
        "r",
        EventType::EffectCheckpointed,
        checkpoint(
            Some("m"),
            json!({
                "step": "seat-turn",
                "total_cost_usd": 0.5,
                "input_tokens": 10,
                "output_tokens": 4,
                "cache_read_tokens": 3,
                "cache_write_tokens": 2,
            }),
        ),
    );
    assert_eq!(
        kept(&store, "r", event.seq),
        [
            Value::Integer(at(&event)),
            text("implement"),
            Value::Real(0.5),
            Value::Integer(1),
            Value::Integer(10),
            Value::Integer(4),
            Value::Integer(3),
            Value::Integer(2),
            Value::Null,
            Value::Integer(0),
            Value::Integer(1),
        ]
    );
    // A checkpoint reporting nothing keeps no cost and no tokens, and its
    // session flag is written as 0, not left NULL.
    let bare = checkpointed_row(&mut store, json!({"step": "tool-use"}));
    assert_eq!(bare[2..4], [Value::Null, Value::Integer(0)]);
    assert_eq!(
        bare[4..10],
        [
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Integer(0)
        ]
    );
}

/// Ruling 4's comparison holds every checkpoint column to brokkr-core's
/// reading of the run: an edit of any one is named by that column, and a
/// row whose facts are not written is a lag, not a disagreement.
#[test]
fn each_checkpoint_column_that_is_not_its_fold_s_is_named() {
    let (_dir, mut store) = store();
    let event = one_checkpoint(
        &mut store,
        json!({"step": "seat-turn", "total_cost_usd": 0.25}),
    );
    let held = kept(&store, "r", event.seq);
    assert_eq!(held.last(), Some(&Value::Integer(1)));
    let named = crate::test_support::each_column_named(
        &store,
        "r",
        event.seq,
        &COLUMNS,
        &held,
        Some("facts"),
    );
    assert_eq!(
        named,
        COLUMNS
            .iter()
            .map(|column| column.to_string())
            .collect::<Vec<_>>()
    );
    store
        .conn
        .execute(
            "UPDATE event_index SET at_ms = 1 WHERE run_id = 'r' AND seq = ?1",
            params![event.seq as i64],
        )
        .unwrap();
    assert_eq!(
        store.summary_disagreements().unwrap()[0].to_string(),
        format!(
            "run 'r': the event index row of its event {} is not the event's in its at_ms",
            event.seq
        )
    );
    store
        .conn
        .execute(
            "UPDATE event_index SET at_ms = ?1 WHERE run_id = 'r' AND seq = ?2",
            params![held[0], event.seq as i64],
        )
        .unwrap();
}

/// A phase past the name bound is written cut and reads as its fold's: the
/// writer's row and the comparison's named row take the cut from one
/// derivation, so a whole phase can stand on neither side of the check.
#[test]
fn a_phase_past_its_bound_is_cut_where_it_is_stored() {
    let (_dir, mut store) = store();
    let phase = "p".repeat(NAME_BYTES + 20);
    opened(&mut store, "r", &phase, &[]);
    let event = append(
        &mut store,
        "r",
        EventType::EffectCheckpointed,
        checkpoint(Some("m"), json!({"step": "seat-turn"})),
    );
    assert_eq!(
        kept(&store, "r", event.seq)[1],
        text(&"p".repeat(NAME_BYTES))
    );
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
    assert_eq!(
        store
            .summary_disagreements()
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        Vec::<String>::new()
    );
}

/// Every fixture journal keeps each checkpoint column for column what
/// brokkr-core reads of its run whole (decision 0089 ruling 7), and names
/// the same rows.
#[test]
fn every_fixture_keeps_what_the_checkpoints_read() {
    let (_dir, mut store) = store();
    let fixtures = crate::test_support::planted_fixtures(&store);
    store.checkpoint_behind_with(|_| {}).unwrap();
    let mut wrong = Vec::new();
    for (run_id, events) in fixtures {
        let account = Account::whole(&run_id, &events);
        let found = crate::test_support::indexed_seqs(&store, &run_id, "effect/checkpointed");
        if found.as_slice() != account.checkpoint_rows() {
            wrong.push(format!(
                "{run_id}: checkpoints {found:?}, read {:?}",
                account.checkpoint_rows()
            ));
            continue;
        }
        for checkpoint in account.checkpoints() {
            let row = kept(&store, &run_id, checkpoint.seq);
            let mut want = super::checkpoint::facts(Some(&checkpoint)).to_vec();
            if let Some(phase) = checkpoint.phase.as_deref() {
                want[1] = text(&Bounded::within(phase, NAME_BYTES).cut);
            }
            if row != want {
                wrong.push(format!(
                    "{run_id}: checkpoint {} {row:?}, read {want:?}",
                    checkpoint.seq
                ));
            }
        }
    }
    assert_eq!(wrong, Vec::<String>::new());
    assert!(backfilled(&store.conn).unwrap());
}

/// A site's second finishing session record moves the flag off the first;
/// a later checkpoint that is not one leaves the last session record's flag
/// standing.
#[test]
fn a_site_s_second_session_record_flips_the_first_to_zero() {
    let (_dir, mut store) = store();
    let first = one_checkpoint(&mut store, json!({"step": "claude-session-finished"}));
    let second = append(
        &mut store,
        "r",
        EventType::EffectCheckpointed,
        checkpoint(Some("m"), json!({"step": "claude-session-finished"})),
    );
    assert_eq!(kept(&store, "r", first.seq)[9], Value::Integer(0));
    assert_eq!(kept(&store, "r", second.seq)[9], Value::Integer(1));
    // Another site's finishing record leaves this site's last one alone.
    let other = append(
        &mut store,
        "r",
        EventType::EffectCheckpointed,
        json!({"effect_id": "fx", "attempt_id": "b", "checkpoint": {"member": "m",
            "step": "claude-session-finished"}}),
    );
    assert_eq!(kept(&store, "r", second.seq)[9], Value::Integer(1));
    assert_eq!(kept(&store, "r", other.seq)[9], Value::Integer(1));
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// The session flag's one mover searches the partial index of its site's
/// later session rows, and the facts' write searches the row's key; no
/// write walks the run's checkpoints.
#[test]
fn the_session_mover_and_the_facts_write_search_their_indexes() {
    let (_dir, store) = store();
    let bound = rusqlite::params_from_iter(std::iter::repeat_n(Value::Null, 13));
    assert_eq!(
        crate::test_support::plan(&store, &WRITE, bound).unwrap(),
        ["SEARCH event_index USING PRIMARY KEY (run_id=? AND seq=?)"]
    );
    let update = crate::test_support::trigger_body(
        &store,
        "event_index_checkpoint_moved",
        &[
            ("NEW.run_id", "'r'"),
            ("NEW.attempt_id", "'a'"),
            ("NEW.member", "'m'"),
            ("NEW.seq", "1000"),
        ],
    );
    assert_eq!(
        crate::test_support::plan(&store, &update, []).unwrap(),
        [
            "SEARCH event_index USING COVERING INDEX event_index_checkpoint_session (run_id=? AND \
          attempt_id=? AND member=? AND seq<?)"
        ]
    );
}

/// Appending a checkpoint to a run with N checkpoints visits the same VM
/// steps at every N: the facts' write names one row and the trigger's
/// session search visits none of them.
#[test]
fn an_appended_checkpoint_visits_the_same_steps_at_every_n() {
    use crate::import::INSERT_EVENT;
    use rusqlite::{params_from_iter, StatementStatus};

    let (_dir, mut store) = store();
    let mut steps = Vec::new();
    for n in [200u64, 400] {
        let run_id = format!("r{n}");
        let mut previous = brokkr_core::canonical::ZERO_HASH.to_string();
        let mut events = Vec::new();
        for seq in 1..=n {
            let event = EnvelopeBuilder::new(
                EventType::EffectCheckpointed,
                checkpoint(Some("m"), json!({"step": "seat-turn"})),
            )
            .run(&run_id)
            .seq(seq)
            .event_id(format!("{run_id}-{seq}"))
            .at("2026-01-01T00:00:00Z")
            .previous(previous.clone())
            .sealed();
            previous.clone_from(&event.event_hash);
            events.push(event);
        }
        plant_older_writer(&store, &run_id, &events);
        store.checkpoint_behind_with(|_| {}).unwrap();
        let next = EnvelopeBuilder::new(
            EventType::EffectCheckpointed,
            checkpoint(Some("m"), json!({"step": "seat-turn"})),
        )
        .run(&run_id)
        .seq(n + 1)
        .event_id(format!("{run_id}-{}", n + 1))
        .at("2026-01-01T00:00:01Z")
        .previous(previous)
        .sealed();
        let envelope = serde_json::to_string(&next).unwrap();
        let mut insert = store.conn.prepare(INSERT_EVENT).unwrap();
        insert.reset_status(StatementStatus::VmStep);
        insert
            .execute(params![
                &run_id,
                (n + 1) as i64,
                &next.event_hash,
                &envelope
            ])
            .unwrap();
        let inserted = insert.reset_status(StatementStatus::VmStep);
        let account = Account::whole(&run_id, &store.load(&run_id).unwrap());
        let mut write = store.conn.prepare(&WRITE).unwrap();
        write.reset_status(StatementStatus::VmStep);
        let facts = super::checkpoint::facts(account.checkpoint(n + 1));
        let mut bound: Vec<Value> =
            vec![Value::Text(run_id.clone()), Value::Integer((n + 1) as i64)];
        bound.extend_from_slice(&facts);
        write.execute(params_from_iter(bound)).unwrap();
        steps.push((inserted, write.reset_status(StatementStatus::VmStep)));
    }
    assert_eq!(
        steps[0], steps[1],
        "appending costs the checkpoint: {steps:?}"
    );
}

/// A trigger restored after it was gone reconciles the session flag a
/// writer moved while it was missing: the repair sends the checkpoints
/// behind and the open's walk writes their flags again.
#[test]
fn restoring_the_checkpoint_trigger_sends_the_facts_it_missed_behind() {
    let (_dir, mut store) = store();
    let first = one_checkpoint(&mut store, json!({"step": "claude-session-finished"}));
    store
        .conn
        .execute_batch("DROP TRIGGER event_index_checkpoint_moved")
        .unwrap();
    let second = append(
        &mut store,
        "r",
        EventType::EffectCheckpointed,
        checkpoint(Some("m"), json!({"step": "claude-session-finished"})),
    );
    // The trigger was gone, so the earlier record still holds the flag.
    assert_eq!(kept(&store, "r", first.seq)[9], Value::Integer(1));
    assert_eq!(kept(&store, "r", second.seq)[9], Value::Integer(1));
    lags(&mut store, &["r"]);
    let store = refolded(store);
    assert_eq!(kept(&store, "r", first.seq)[9], Value::Integer(0));
    assert_eq!(kept(&store, "r", second.seq)[9], Value::Integer(1));
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// A table from before the checkpoints keeps its rows and gains the
/// columns at a writable open, which then writes their facts.
#[test]
fn an_index_from_before_the_checkpoints_is_altered_and_refolded() {
    let (_dir, mut store) = store();
    let event = one_checkpoint(
        &mut store,
        json!({"step": "seat-turn", "total_cost_usd": 1.5}),
    );
    let held = kept(&store, "r", event.seq);
    let path = store.path().to_path_buf();
    lags(&mut store, &["r"]);
    drop(store);
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch(
        "DROP INDEX event_index_checkpoint;
         DROP INDEX event_index_checkpoint_stale;
         DROP INDEX event_index_checkpoint_session;
         DROP TRIGGER event_index_checkpoint_moved;
         ALTER TABLE event_index DROP COLUMN facts;",
    )
    .unwrap();
    let rows: i64 = conn
        .query_row("SELECT count(*) FROM event_index", [], |row| row.get(0))
        .unwrap();
    drop(conn);
    let reader = Store::open_read_only(&path).unwrap();
    assert!(!index_intact(&reader.conn).unwrap());
    assert!(!checkpointed(&reader.conn).unwrap());
    assert!(!reader.reads_index().unwrap());
    drop(reader);
    let store = Store::open(&path).unwrap();
    assert!(checkpointed(&store.conn).unwrap());
    assert!(store.reads_index().unwrap());
    assert!(backfilled(&store.conn).unwrap());
    let after: i64 = store
        .conn
        .query_row("SELECT count(*) FROM event_index", [], |row| row.get(0))
        .unwrap();
    assert_eq!(after, rows, "the altered table keeps its rows");
    assert_eq!(kept(&store, "r", event.seq), held);
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}
