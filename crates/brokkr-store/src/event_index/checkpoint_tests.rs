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
    append, at, every_run_reads_as_its_fold, lags, opened, plant_envelope, plant_older_writer,
    refolded, text,
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

/// A run `r` with one checkpoint of `record`, and the row it keeps.
fn one_kept(store: &mut Store, record: Json) -> (EventEnvelope, Vec<Value>) {
    let event = one_checkpoint(store, record);
    let held = kept(store, "r", event.seq);
    (event, held)
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

/// `run_id`'s events in seq order from 1, each chained onto the one before:
/// a test's journal, built the way the store's seal chains it.
fn chained(run_id: &str, events: Vec<(EventType, Json)>) -> Vec<EventEnvelope> {
    let mut previous = brokkr_core::canonical::ZERO_HASH.to_string();
    events
        .into_iter()
        .enumerate()
        .map(|(at, (event_type, payload))| {
            let seq = at as u64 + 1;
            let event = EnvelopeBuilder::new(event_type, payload)
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
    let (event, held) = one_kept(
        &mut store,
        json!({"step": "seat-turn", "total_cost_usd": 0.25}),
    );
    assert_eq!(held.last(), Some(&Value::Integer(1)));
    let named =
        crate::test_support::each_column_named(&store, "r", event.seq, &COLUMNS, &held, None);
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
        crate::test_support::disagreements(&store),
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
        for seq in account.checkpoint_rows() {
            let checkpoint = account
                .checkpoint(*seq)
                .expect("a checkpoint row has facts");
            let row = kept(&store, &run_id, checkpoint.seq);
            let mut want = super::checkpoint::facts(Some(checkpoint)).unwrap().to_vec();
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

/// One frozen fixture's checkpoint is pinned literally, so the fixture
/// comparison is not only the writer's own conversion read back: its time,
/// the run's phase, its cost and report, no tokens and its session flag
/// (decision 0089 ruling 7).
#[test]
fn a_fixture_checkpoint_keeps_the_facts_its_record_states() {
    let (_dir, mut store) = store();
    crate::test_support::planted_fixtures(&store);
    store.checkpoint_behind_with(|_| {}).unwrap();
    assert_eq!(
        kept(&store, "tui-graph-the-selection-box-gets-80f98deb", 25),
        [
            Value::Integer(1_788_127_917_000),
            text("intake"),
            Value::Real(0.8165567999999999),
            Value::Integer(1),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Integer(1),
            Value::Integer(1),
        ]
    );
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

/// A writer from before the facts inserts a finishing record without them:
/// the trigger moves the earlier flag as the event lands, the new row is a
/// lag rather than a false disagreement, and the open's walk writes both
/// (decision 0089 rulings 4 and 7).
#[test]
fn an_older_writer_s_second_session_record_moves_the_first_flag() {
    let (_dir, mut store) = store();
    let first = one_checkpoint(&mut store, json!({"step": "claude-session-finished"}));
    let second = EnvelopeBuilder::new(
        EventType::EffectCheckpointed,
        checkpoint(Some("m"), json!({"step": "claude-session-finished"})),
    )
    .run("r")
    .seq(first.seq + 1)
    .event_id("r-older")
    .at("2026-01-01T00:00:09Z")
    .previous(first.event_hash.clone())
    .sealed();
    plant_envelope(&store.conn, &second).unwrap();
    // The event's trigger moves the flag; its own row keeps no facts, so
    // the run is a lag and the doctor names no corruption.
    assert_eq!(kept(&store, "r", first.seq)[9], Value::Integer(0));
    assert_eq!(kept(&store, "r", second.seq)[10], Value::Null);
    assert_eq!(
        crate::test_support::disagreements(&store),
        Vec::<String>::new()
    );
    lags(&mut store, &["r"]);
    let store = refolded(store);
    assert_eq!(kept(&store, "r", first.seq)[9], Value::Integer(0));
    assert_eq!(kept(&store, "r", second.seq)[9], Value::Integer(1));
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
    let inserted = crate::test_support::trigger_body(
        &store,
        "event_index_checkpoint_finished",
        &[
            ("NEW.run_id", "'r'"),
            ("NEW.seq", "1000"),
            (
                "NEW.envelope",
                r#"'{"type":"effect/checkpointed","payload":{"attempt_id":"a",
                    "checkpoint":{"member":"m","step":"claude-session-finished"}}}'"#,
            ),
        ],
    );
    assert_eq!(
        crate::test_support::plan(&store, &inserted, []).unwrap()[0],
        "SEARCH event_index USING COVERING INDEX event_index_checkpoint_session (run_id=? AND \
         attempt_id=? AND member=? AND seq<?)"
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
        let events = chained(
            &run_id,
            (1..=n)
                .map(|_| {
                    (
                        EventType::EffectCheckpointed,
                        checkpoint(Some("m"), json!({"step": "seat-turn"})),
                    )
                })
                .collect(),
        );
        let previous = events.last().unwrap().event_hash.clone();
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
        let facts = super::checkpoint::facts(account.checkpoint(n + 1)).unwrap();
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

/// A no-carry append's phase lookup is keyed on the phase rows alone:
/// finding the run's phase behind N checkpoints visits the same VM steps at
/// every N, never scanning the checkpoints back to the last phase event
/// (decision 0089 rulings 7 and 9).
#[test]
fn the_no_carry_phase_lookup_visits_the_same_steps_at_every_n() {
    use super::checkpoint::PHASE_BEFORE;
    use rusqlite::StatementStatus;

    let (_dir, store) = store();
    let mut steps = Vec::new();
    for n in [200u64, 400] {
        let run_id = format!("p{n}");
        let events = chained(
            &run_id,
            std::iter::once((EventType::PhaseEntered, json!({"phase": "implement"})))
                .chain((2..=n + 1).map(|_| {
                    (
                        EventType::EffectCheckpointed,
                        checkpoint(Some("m"), json!({"step": "seat-turn"})),
                    )
                }))
                .collect(),
        );
        plant_older_writer(&store, &run_id, &events);
        let mut query = store.conn.prepare(&PHASE_BEFORE).unwrap();
        query.reset_status(StatementStatus::VmStep);
        let found: Option<String> = query
            .query_row(params![&run_id, (n + 2) as i64], |row| row.get(0))
            .unwrap();
        assert!(found.is_some());
        steps.push(query.reset_status(StatementStatus::VmStep));
    }
    assert_eq!(steps[0], steps[1], "the phase lookup is keyed: {steps:?}");
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
        .execute_batch(
            "DROP TRIGGER event_index_checkpoint_moved;
             DROP TRIGGER event_index_checkpoint_finished",
        )
        .unwrap();
    let second = append(
        &mut store,
        "r",
        EventType::EffectCheckpointed,
        checkpoint(Some("m"), json!({"step": "claude-session-finished"})),
    );
    // The triggers were gone, so the earlier record still holds the flag.
    assert_eq!(kept(&store, "r", first.seq)[9], Value::Integer(1));
    assert_eq!(kept(&store, "r", second.seq)[9], Value::Integer(1));
    lags(&mut store, &["r"]);
    let store = refolded(store);
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

/// A writer that carries no account of the run, as a second writable open
/// does, still writes the checkpoint it appends from the event alone, and
/// moves a site's session flag off its earlier record: no supported append
/// path leaves a valid journal looking corrupt (decision 0089 rulings 4
/// and 7).
#[test]
fn an_append_with_no_carried_account_keeps_its_facts_and_moves_the_session() {
    let (_dir, mut store) = store();
    let first = one_checkpoint(&mut store, json!({"step": "claude-session-finished"}));
    let mut store = refolded(store);
    let second = append(
        &mut store,
        "r",
        EventType::EffectCheckpointed,
        checkpoint(
            Some("m"),
            json!({
                "step": "claude-session-finished",
                "total_cost_usd": 0.5,
                "input_tokens": 10,
            }),
        ),
    );
    assert_eq!(kept(&store, "r", first.seq)[9], Value::Integer(0));
    assert_eq!(
        kept(&store, "r", second.seq),
        [
            Value::Integer(at(&second)),
            text("implement"),
            Value::Real(0.5),
            Value::Integer(1),
            Value::Integer(10),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Integer(1),
            Value::Integer(1),
        ]
    );
    // The append carried no fold, so the summary row is a lag, never a
    // disagreement: the doctor passes it, and the next writable open folds
    // it, with the facts the append wrote already standing.
    assert_eq!(
        crate::test_support::disagreements(&store),
        Vec::<String>::new()
    );
    let store = refolded(store);
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// An append keeps no earlier checkpoint in the carried account, so its
/// clone holds the checkpoint it writes and never the run's history
/// (decision 0089 ruling 9).
#[test]
fn an_append_keeps_no_earlier_checkpoint_in_the_carried_account() {
    let (_dir, mut store) = store();
    opened(&mut store, "r", "implement", &[]);
    assert_eq!(store.carried_checkpoints(), Some(0));
    for _ in 0..2 {
        append(
            &mut store,
            "r",
            EventType::EffectCheckpointed,
            checkpoint(Some("m"), json!({"step": "seat-turn"})),
        );
        assert_eq!(store.carried_checkpoints(), Some(0));
    }
}

/// A `facts` marker that is neither NULL nor 1 is an edit, not a lag: the
/// comparison names the column, and the cost beside it, instead of passing
/// every checkpoint column forever (decision 0089 ruling 4).
#[test]
fn a_facts_marker_a_writer_did_not_write_is_named_not_skipped() {
    let (_dir, mut store) = store();
    let (event, held) = one_kept(
        &mut store,
        json!({"step": "seat-turn", "total_cost_usd": 0.25}),
    );
    store
        .conn
        .execute(
            "UPDATE event_index SET facts = 2, usd = 999 WHERE run_id = 'r' AND seq = ?1",
            params![event.seq as i64],
        )
        .unwrap();
    assert_eq!(
        store.summary_disagreements().unwrap()[0].to_string(),
        format!(
            "run 'r': the event index row of its event {} is not the event's in its usd",
            event.seq
        )
    );
    store
        .conn
        .execute(
            "UPDATE event_index SET usd = ?1 WHERE run_id = 'r' AND seq = ?2",
            params![held[2], event.seq as i64],
        )
        .unwrap();
    assert_eq!(
        store.summary_disagreements().unwrap()[0].to_string(),
        format!(
            "run 'r': the event index row of its event {} is not the event's in its facts",
            event.seq
        )
    );
    store
        .conn
        .execute(
            "UPDATE event_index SET facts = 1 WHERE run_id = 'r' AND seq = ?1",
            params![event.seq as i64],
        )
        .unwrap();
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// A token count past the signed 64-bit integer the index keeps is refused
/// by name, not wrapped into a negative fact, and the append writes
/// nothing (decision 0089 ruling 7).
#[test]
fn a_token_count_past_the_index_s_bound_is_refused_by_name() {
    let (_dir, mut store) = store();
    opened(&mut store, "r", "implement", &[]);
    let mut record = json!({"step": "seat-turn"});
    record["input_tokens"] = Json::from(9_223_372_036_854_775_808u64);
    let refused = store
        .append_next(
            "r",
            EventType::EffectCheckpointed,
            checkpoint(Some("m"), record),
            None,
            None,
        )
        .unwrap_err();
    assert!(matches!(
        refused,
        crate::StoreError::TokenCountOutOfRange(3, "input_tokens", 9_223_372_036_854_775_808)
    ));
    assert_eq!(
        refused.to_string(),
        "checkpoint 3 reports input_tokens 9223372036854775808 past the index's 64-bit \
         bound; nothing was written"
    );
    assert_eq!(store.head_hash("r").unwrap().0, 2);
    assert_eq!(
        crate::test_support::indexed_seqs(&store, "r", "effect/checkpointed"),
        Vec::<u64>::new()
    );
}

/// A legacy checkpoint whose token count the index cannot hold refuses its
/// own run by name: the journal still opens, the healthy run still reads,
/// and the backfill stays unfinished (decision 0089 rulings 4 and 8).
#[test]
fn an_oversized_legacy_checkpoint_does_not_block_the_journal() {
    let (_dir, mut store) = store();
    opened(&mut store, "healthy", "implement", &[]);
    let healthy = append(
        &mut store,
        "healthy",
        EventType::EffectCheckpointed,
        checkpoint(Some("m"), json!({"step": "seat-turn"})),
    );
    let mut record = json!({"member": "m", "step": "seat-turn"});
    record["input_tokens"] = Json::from(9_223_372_036_854_775_808u64);
    let mut previous = brokkr_core::canonical::ZERO_HASH.to_string();
    let mut events = Vec::new();
    for (seq, event_type, payload) in [
        (
            1,
            EventType::RunStarted,
            json!({"feature": "a feature", "manifest": {}}),
        ),
        (2, EventType::PhaseEntered, json!({"phase": "implement"})),
        (
            3,
            EventType::EffectRequested,
            json!({"effect_id": "fx", "seat": "implement"}),
        ),
        (
            4,
            EventType::EffectStarted,
            json!({"effect_id": "fx", "attempt_id": "a1"}),
        ),
        (
            5,
            EventType::EffectCheckpointed,
            checkpoint(Some("m"), record.clone()),
        ),
    ] {
        let event = EnvelopeBuilder::new(event_type, payload)
            .run("legacy")
            .seq(seq)
            .event_id(format!("legacy-{seq}"))
            .at("2026-01-01T00:00:00Z")
            .previous(previous)
            .sealed();
        previous = event.event_hash.clone();
        events.push(event);
    }
    plant_older_writer(&store, "legacy", &events);
    let path = store.path().to_path_buf();
    drop(store);
    let store = Store::open(&path).unwrap();
    assert!(!backfilled(&store.conn).unwrap());
    let named = crate::test_support::disagreements(&store);
    assert!(
        named
            .iter()
            .any(|found| found.contains("legacy") && found.contains("input_tokens")),
        "{named:?}"
    );
    assert!(
        !named.iter().any(|found| found.contains("healthy")),
        "the healthy run still reads: {named:?}"
    );
    // The next open refuses the same run by name and still opens.
    let path = store.path().to_path_buf();
    drop(store);
    let store = Store::open(&path).unwrap();
    assert!(!backfilled(&store.conn).unwrap());
    assert_eq!(kept(&store, "healthy", healthy.seq)[10], Value::Integer(1));
}

/// A run a peer moves while the checkpoint backfill reads it is left for
/// the next open, and the backfill is not marked finished over it: the
/// completion claim stays true (decision 0089 rulings 4 and 8).
#[test]
fn a_backfill_whose_head_moved_stays_unfinished() {
    let (_dir, mut store) = store();
    let first = one_checkpoint(&mut store, json!({"step": "seat-turn"}));
    store
        .conn
        .execute(
            "UPDATE event_index SET facts = NULL WHERE run_id = 'r' AND seq = ?1",
            params![first.seq as i64],
        )
        .unwrap();
    lags(&mut store, &["r"]);
    let mut moved = false;
    store
        .checkpoint_behind_with(|conn| {
            if moved {
                return;
            }
            moved = true;
            let (seq, previous) = crate::head_of(conn, "r").unwrap();
            let peer = EnvelopeBuilder::new(
                EventType::EffectCheckpointed,
                checkpoint(Some("m"), json!({"step": "seat-turn"})),
            )
            .run("r")
            .seq(seq + 1)
            .event_id("r-peer")
            .at("2026-01-01T00:00:09Z")
            .previous(previous)
            .sealed();
            crate::test_support::plant_envelope(conn, &peer).unwrap();
        })
        .unwrap();
    assert!(moved);
    assert!(!backfilled(&store.conn).unwrap());
    let path = store.path().to_path_buf();
    drop(store);
    let store = Store::open(&path).unwrap();
    assert!(backfilled(&store.conn).unwrap());
    assert_eq!(kept(&store, "r", first.seq)[10], Value::Integer(1));
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}
