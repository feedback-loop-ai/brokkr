//! The checkpoints' dated facts (decision 0089 ruling 7): each
//! `effect/checkpointed` row keeps its time, phase, cost and whether it was
//! reported, its token counts and whether it is its site's session record,
//! ruling 4's comparison holds each column to brokkr-core's one reading, a
//! later session record moves the flag, a writer from before them leaves
//! the row for the open's walk, and a table from before them is altered in
//! place.

use super::checkpoint::{backfilled, checkpointed, COLUMNS, SITE, WRITE, WRITE_SESSION};
use super::*;
use brokkr_core::envelope::{ChainError, EventEnvelope, EventType};
use brokkr_core::summary::{Bounded, Checkpoint, NAME_BYTES};
use rusqlite::params;
use serde_json::{json, Value as Json};

use crate::summary::Compared;
use crate::test_support::{
    append, at, disagreements, every_run_reads_as_its_fold, failed, lags, lane, opened,
    plant_envelope, plant_older_writer, plant_summary_writer, planted_fixtures, refolded, started,
    text,
};
use crate::tests::envelope_builder::EnvelopeBuilder;
use crate::tests::store;
use crate::Store;

/// A checkpoint row's facts, by [`COLUMNS`]' order.
fn kept(store: &Store, run_id: &str, seq: u64) -> Vec<Value> {
    crate::test_support::kept_facts(store, run_id, seq, &COLUMNS)
}

/// Write `held`'s [`COLUMNS`] to the checkpoint row at `seq` through the
/// pre-stamp writer's column-only update: the facts stand, and `facts_seq`
/// does not move.
fn write_pre_stamp_facts(store: &Store, run_id: &str, seq: u64, held: &[Value]) {
    let assignments: Vec<String> = COLUMNS
        .iter()
        .enumerate()
        .map(|(at, column)| format!("{column} = ?{}", at + 3))
        .collect();
    let sql = format!(
        "UPDATE event_index SET {} WHERE run_id = ?1 AND seq = ?2",
        assignments.join(", ")
    );
    let mut bound: Vec<Value> = vec![Value::Text(run_id.into()), Value::Integer(seq as i64)];
    bound.extend_from_slice(held);
    store
        .conn
        .execute(&sql, rusqlite::params_from_iter(bound))
        .unwrap();
}

/// A checkpoint row's session stamp: its own seq once core's reading has
/// computed its session, NULL while it is behind.
fn stamped(store: &Store, run_id: &str, seq: u64) -> Value {
    crate::test_support::kept_facts(store, run_id, seq, &["facts_seq"]).remove(0)
}

/// A finishing checkpoint of site (a, m) chained onto `first` at its next
/// seq.
fn finishing_after(first: &EventEnvelope, event_id: &str, at: &str) -> EventEnvelope {
    EnvelopeBuilder::new(
        EventType::EffectCheckpointed,
        checkpoint(Some("m"), json!({"step": "claude-session-finished"})),
    )
    .run("r")
    .seq(first.seq + 1)
    .event_id(event_id)
    .at(at)
    .previous(first.event_hash.clone())
    .sealed()
}

/// `seq`'s session on `store`'s run `r`.
fn session(store: &Store, seq: u64) -> Value {
    kept(store, "r", seq)[9].clone()
}

/// The journal before its next open: `seq`'s session, the run left behind
/// on purpose, and the doctor naming no disagreement.
fn before_open(store: &mut Store, seq: u64, session: Value) {
    assert_eq!(kept(store, "r", seq)[9], session);
    assert_eq!(disagreements(store), Vec::<String>::new());
    lags(store, &["r"]);
}

/// The pass left the backfill unfinished, and the next open's walk completes
/// it.
fn unfinished_then_backfilled(store: Store, moved: bool) -> Store {
    assert!(moved);
    assert!(!backfilled(&store.conn).unwrap());
    let store = refolded(store);
    assert!(backfilled(&store.conn).unwrap());
    store
}

/// The SQL a 4ebca69a0 writer runs when it finds the checkpoint triggers
/// this build retired missing: its moving and finishing triggers decide
/// `session` in SQL again, which is the defect this repair answers. The
/// expressions are that revision's, rebuilt from the helpers it shared
/// with this one.
fn retired_checkpoint_triggers() -> String {
    let checkpointed = crate::event_index::CHECKPOINTED;
    let record = crate::event_index::record("NEW");
    let payload = crate::event_index::payload("NEW");
    let finished = format!(
        "COALESCE((SELECT CASE WHEN value LIKE '%-session-finished' THEN 1 ELSE 0 END
              FROM {record} WHERE key = 'step' AND type = 'text'
              ORDER BY id DESC LIMIT 1), 0)"
    );
    let moved = |attempt: &str, member: &str| {
        format!(
            "UPDATE event_index SET session = 0
                WHERE run_id = NEW.run_id
                  AND attempt_id IS {attempt}
                  AND member IS {member}
                  AND seq < NEW.seq
                  AND type = '{checkpointed}' AND session = 1"
        )
    };
    format!(
        "DROP TRIGGER IF EXISTS event_index_checkpoint_moved;
         CREATE TRIGGER event_index_checkpoint_moved AFTER UPDATE OF session ON event_index
             WHEN NEW.type = '{checkpointed}' AND NEW.session = 1 BEGIN
             {checkpoint_moved};
         END;
         DROP TRIGGER IF EXISTS event_index_checkpoint_finished;
         CREATE TRIGGER event_index_checkpoint_finished AFTER INSERT ON events
             WHEN {kind} = '{checkpointed}' AND {attempt} IS NOT NULL
               AND {finished} = 1 BEGIN
             {session_moved};
         END;",
        kind = crate::summary::text_at("NEW.envelope", "$.type"),
        attempt = crate::event_index::last_text(&payload, "attempt_id"),
        finished = finished,
        checkpoint_moved = moved("NEW.attempt_id", "NEW.member"),
        session_moved = moved(
            &crate::event_index::last_text(&payload, "attempt_id"),
            &crate::event_index::last_text(&record, "member"),
        ),
    )
}

/// The journal reopened writable, with `seq`'s session and the run reading
/// as its fold with no disagreement.
fn reopened(store: Store, seq: u64, session: Value) -> Store {
    let store = refolded(store);
    assert_eq!(kept(&store, "r", seq)[9], session);
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
    assert_eq!(disagreements(&store), Vec::<String>::new());
    store
}

/// A `between` callback that plants `peer` on its first call, the instant a
/// peer's append lands while a run is read.
fn peer_once<'a>(
    peer: &'a EventEnvelope,
    moved: &'a mut bool,
) -> impl FnMut(&rusqlite::Connection) + 'a {
    move |conn| {
        if *moved {
            return;
        }
        *moved = true;
        crate::test_support::plant_envelope(conn, peer).unwrap();
    }
}

/// `run_id`'s `events` planted and their checkpoint facts written, and the
/// last event's hash.
fn planted(store: &mut Store, run_id: &str, events: &[EventEnvelope]) -> String {
    plant_older_writer(store, run_id, events);
    store.checkpoint_behind_with(|_| {}).unwrap();
    events.last().unwrap().event_hash.clone()
}

/// A checkpoint of attempt `a`, naming `member`, its record `record`.
fn checkpoint(member: Option<&str>, record: Json) -> Json {
    checkpoint_at("a", member, record)
}

/// A checkpoint of `attempt`, naming `member`, its record `record`.
fn checkpoint_at(attempt: &str, member: Option<&str>, record: Json) -> Json {
    let mut checkpoint = record;
    if let (Some(member), Some(object)) = (member, checkpoint.as_object_mut()) {
        object.insert("member".into(), json!(member));
    }
    json!({"effect_id": "fx", "attempt_id": attempt, "checkpoint": checkpoint})
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

/// A run `r` with one appended checkpoint whose payload names no attempt:
/// a row no site names, which core reads no session for.
fn one_attemptless(store: &mut Store) -> EventEnvelope {
    opened(store, "r", "implement", &[]);
    append(
        store,
        "r",
        EventType::EffectCheckpointed,
        json!({"effect_id": "fx", "checkpoint": {"member": "m", "step": "seat-turn"}}),
    )
}

/// Append a checkpoint of site (a, m) whose record is `record`.
fn append_checkpoint(store: &mut Store, record: Json) -> EventEnvelope {
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

/// `run_id`'s run started, a phase entered and one checkpoint of site
/// (a, m), chained.
fn checkpoint_run(run_id: &str) -> Vec<EventEnvelope> {
    chained(
        run_id,
        vec![
            (
                EventType::RunStarted,
                json!({"feature": "f", "manifest": {}}),
            ),
            (EventType::PhaseEntered, json!({"phase": "implement"})),
            (
                EventType::EffectCheckpointed,
                checkpoint(Some("m"), json!({"step": "seat-turn"})),
            ),
        ],
    )
}

/// `n` chained checkpoints of `attempt` and `member`, at one site.
fn chained_checkpoints(run_id: &str, attempt: &str, member: &str, n: u64) -> Vec<EventEnvelope> {
    chained(
        run_id,
        (1..=n)
            .map(|_| {
                (
                    EventType::EffectCheckpointed,
                    checkpoint_at(attempt, Some(member), json!({"step": "seat-turn"})),
                )
            })
            .collect(),
    )
}

/// Insert `seq`'s checkpoint through the older writer's statement and return
/// the VM steps SQLite took, the cost of the triggers an append fires.
fn insert_checkpoint(
    store: &Store,
    run_id: &str,
    seq: u64,
    previous: &str,
    attempt: &str,
    member: &str,
) -> i64 {
    use crate::import::INSERT_EVENT;
    use rusqlite::StatementStatus;

    let event = EnvelopeBuilder::new(
        EventType::EffectCheckpointed,
        checkpoint_at(attempt, Some(member), json!({"step": "seat-turn"})),
    )
    .run(run_id)
    .seq(seq)
    .event_id(format!("{run_id}-{seq}"))
    .at("2026-01-01T00:00:00Z")
    .previous(previous)
    .sealed();
    let envelope = serde_json::to_string(&event).unwrap();
    let mut insert = store.conn.prepare(INSERT_EVENT).unwrap();
    insert.reset_status(StatementStatus::VmStep);
    insert
        .execute(params![run_id, seq as i64, &event.event_hash, &envelope])
        .unwrap();
    i64::from(insert.reset_status(StatementStatus::VmStep))
}

/// The rows the refold's candidate search selects at `run_id`'s site when
/// the append starts at `from`, and the VM steps it took: the selection's
/// own cost, separated from the envelope reads and writes that follow it.
fn site_candidates(
    store: &Store,
    run_id: &str,
    attempt: &str,
    member: &str,
    from: u64,
) -> (Vec<u64>, i64) {
    use rusqlite::StatementStatus;

    let mut statement = store.conn.prepare(&SITE).unwrap();
    statement.reset_status(StatementStatus::VmStep);
    let found: Vec<u64> = statement
        .query_map(params![run_id, attempt, member, from as i64], |row| {
            row.get(0)
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    (
        found,
        i64::from(statement.reset_status(StatementStatus::VmStep)),
    )
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
            Value::Integer(0),
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
    let second = append_checkpoint(&mut store, json!({"step": "claude-session-finished"}));
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

/// A re-opened attempt is one participant, as main reads it: the refold's
/// participant rows alone move the earlier session record's flag off when
/// the later one lands, and an open's behind refold reads the same, without
/// the attempt's starts (decision 0089 rulings 4 and 7).
#[test]
fn a_reopened_participant_s_earlier_session_record_is_moved() {
    let (_dir, mut store) = store();
    opened(&mut store, "r", "implement", &[]);
    started(&mut store, "r", "a", Some("m"), "anthropic");
    let first = append_checkpoint(&mut store, json!({"step": "claude-session-finished"}));
    failed(&mut store, "r", "a");
    started(&mut store, "r", "a", Some("m"), "anthropic");
    let second = append_checkpoint(&mut store, json!({"step": "claude-session-finished"}));
    assert_eq!(session(&store, first.seq), Value::Integer(0));
    assert_eq!(session(&store, second.seq), Value::Integer(1));
    let events = store.load("r").unwrap();
    let read: Vec<Value> = brokkr_core::summary::checkpoints(&events)
        .into_iter()
        .map(|checkpoint| Value::Integer(i64::from(checkpoint.session)))
        .collect();
    assert_eq!(read, [Value::Integer(0), Value::Integer(1)]);
    let store = refolded(store);
    assert_eq!(session(&store, first.seq), Value::Integer(0));
    assert_eq!(session(&store, second.seq), Value::Integer(1));
}

/// A step whose suffix differs only in case finishes no session in
/// brokkr-core's reading, so the earlier record keeps the flag; no SQL
/// decides it (decision 0089 rulings 4 and 7; decision 0071 ruling 5).
#[test]
fn an_upper_case_suffix_does_not_move_the_session() {
    let (_dir, mut store) = store();
    let first = one_checkpoint(&mut store, json!({"step": "claude-session-finished"}));
    let second = append_checkpoint(&mut store, json!({"step": "claude-SESSION-FINISHED"}));
    assert_eq!(session(&store, first.seq), Value::Integer(1));
    assert_eq!(session(&store, second.seq), Value::Integer(0));
    before_open(&mut store, second.seq, Value::Integer(0));
    let store = reopened(store, second.seq, Value::Integer(0));
    assert_eq!(session(&store, first.seq), Value::Integer(1));
}

/// A record that repeats `step` keeps the last copy, as serde reads it: the
/// duplicate null finishes no session, so the earlier record keeps the
/// flag, after the append and after a reopen (decision 0089 rulings 4 and
/// 7; decision 0071 ruling 5).
#[test]
fn a_last_step_key_that_is_null_keeps_the_session() {
    let (_dir, mut store) = store();
    let first = one_checkpoint(&mut store, json!({"step": "claude-session-finished"}));
    let raw = format!(
        r#"{{"run_id":"r","seq":{},"event_id":"r-duplicate","event_schema_version":1,"type":"effect/checkpointed","payload":{{"effect_id":"fx","attempt_id":"a","checkpoint":{{"member":"m","step":"claude-session-finished","step":null}}}},"causation_id":null,"correlation_id":"r","recorded_at":"2026-01-01T00:00:09Z","previous_hash":"{}","event_hash":""}}"#,
        first.seq + 1,
        first.event_hash
    );
    let parsed: EventEnvelope = serde_json::from_str(&raw).unwrap();
    let hash = parsed.compute_hash();
    let sealed = raw.replace(r#""event_hash":"""#, &format!(r#""event_hash":"{hash}""#));
    crate::test_support::plant_event(&store.conn, "r", &((first.seq + 1) as i64), &hash, &sealed)
        .unwrap();
    assert_eq!(session(&store, first.seq), Value::Integer(1));
    assert_eq!(session(&store, first.seq + 1), Value::Null);
    before_open(&mut store, first.seq + 1, Value::Null);
    let store = reopened(store, first.seq + 1, Value::Integer(0));
    assert_eq!(session(&store, first.seq), Value::Integer(1));
}

/// A finishing record whose stamp does not parse is ignored by
/// brokkr-core, so its site's earlier session record keeps the flag, after
/// the append and after a reopen (decision 0089 rulings 4 and 7; decision
/// 0071 ruling 5).
#[test]
fn an_unreadable_stamp_keeps_the_session() {
    let (_dir, mut store) = store();
    let first = one_checkpoint(&mut store, json!({"step": "claude-session-finished"}));
    let second = finishing_after(&first, "r-unreadable", "not a stamp");
    plant_envelope(&store.conn, &second).unwrap();
    assert_eq!(session(&store, first.seq), Value::Integer(1));
    assert_eq!(session(&store, second.seq), Value::Null);
    before_open(&mut store, second.seq, Value::Null);
    let store = reopened(store, second.seq, Value::Null);
    assert_eq!(session(&store, first.seq), Value::Integer(1));
}

/// A 4ebca69a0 writer reopens the journal, finds the checkpoint triggers
/// this build retired missing, and makes them again; its facts backfill
/// writes the pre-stamp columns and leaves the current stamp standing. The
/// next current open must drop the retired triggers and send the
/// checkpoints behind, so the site's flag is core's reading and a later
/// upper-case suffix cannot clear it in SQL (decision 0089 rulings 4 and
/// 7; decision 0071 ruling 5).
#[test]
fn a_reopened_journal_drops_the_retired_checkpoint_triggers_and_refolds_the_session() {
    let (_dir, mut store) = store();
    let first = one_checkpoint(&mut store, json!({"step": "claude-session-finished"}));
    let held = kept(&store, "r", first.seq);
    assert_eq!(session(&store, first.seq), Value::Integer(1));
    // The older writer's migration makes its own triggers again, resets
    // the facts its repair found and rewrites the pre-stamp columns from
    // the record; the current writer's stamp stays, so nothing reads
    // behind until the retired trigger moves a flag.
    store
        .conn
        .execute_batch(&retired_checkpoint_triggers())
        .unwrap();
    store
        .conn
        .execute(
            "UPDATE event_index SET facts = NULL WHERE type = 'effect/checkpointed'",
            [],
        )
        .unwrap();
    write_pre_stamp_facts(&store, "r", first.seq, &held);
    assert_eq!(
        stamped(&store, "r", first.seq),
        Value::Integer(first.seq as i64)
    );

    let mut store = refolded(store);
    assert!(crate::event_index::index_intact(&store.conn).unwrap());
    let second = append_checkpoint(&mut store, json!({"step": "claude-SESSION-FINISHED"}));
    assert_eq!(session(&store, first.seq), Value::Integer(1));
    assert_eq!(session(&store, second.seq), Value::Integer(0));
    for name in [
        "event_index_checkpoint_moved",
        "event_index_checkpoint_finished",
    ] {
        let present: bool = store
            .conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM sqlite_master
                                WHERE type = 'trigger' AND name = ?1)",
                [name],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!present, "the retired trigger {name} survived the repair");
    }
    assert_eq!(disagreements(&store), Vec::<String>::new());
    let store = reopened(store, second.seq, Value::Integer(0));
    assert_eq!(session(&store, first.seq), Value::Integer(1));
}

/// The repair sends behind the checkpoints a retired trigger could have
/// moved even when their stamps still stand: a later open must refold the
/// site from core's reading, never trust a flag the retired SQL wrote
/// (decision 0089 rulings 4 and 7).
#[test]
fn a_reopened_journal_refolds_a_flag_the_retired_trigger_moved() {
    let (_dir, mut store) = store();
    let first = one_checkpoint(&mut store, json!({"step": "claude-session-finished"}));
    let second = append_checkpoint(&mut store, json!({"step": "claude-session-finished"}));
    assert_eq!(session(&store, first.seq), Value::Integer(0));
    assert_eq!(session(&store, second.seq), Value::Integer(1));
    // The older writer's trigger is made again, and its moving SQL leaves
    // both stamps standing while it writes a flag core does not read.
    store
        .conn
        .execute_batch(&retired_checkpoint_triggers())
        .unwrap();
    store
        .conn
        .execute(
            "UPDATE event_index SET session = 1 WHERE run_id = 'r' AND seq = ?1",
            [first.seq as i64],
        )
        .unwrap();
    assert_eq!(session(&store, first.seq), Value::Integer(1));
    assert_eq!(
        stamped(&store, "r", first.seq),
        Value::Integer(first.seq as i64)
    );

    lags(&mut store, &["r"]);
    let store = refolded(store);
    assert_eq!(session(&store, first.seq), Value::Integer(0));
    assert_eq!(session(&store, second.seq), Value::Integer(1));
    assert_eq!(disagreements(&store), Vec::<String>::new());
}

/// A writer from before the facts inserts a finishing record without them:
/// the trigger sends both rows behind, the site is a lag rather than a
/// false disagreement, and the open's walk writes both (decision 0089
/// rulings 4 and 7).
#[test]
fn an_older_writer_s_second_session_record_moves_the_first_flag() {
    let (_dir, mut store) = store();
    let first = one_checkpoint(&mut store, json!({"step": "claude-session-finished"}));
    let second = finishing_after(&first, "r-older", "2026-01-01T00:00:09Z");
    plant_envelope(&store.conn, &second).unwrap();
    assert_eq!(session(&store, first.seq), Value::Integer(1));
    assert_eq!(kept(&store, "r", second.seq)[11], Value::Null);
    assert_eq!(stamped(&store, "r", first.seq), Value::Null);
    assert_eq!(stamped(&store, "r", second.seq), Value::Null);
    before_open(&mut store, second.seq, Value::Null);
    let store = reopened(store, second.seq, Value::Integer(1));
    assert_eq!(session(&store, first.seq), Value::Integer(0));
}

/// A writer from before the session stamp writes the record's facts but
/// leaves `facts_seq` NULL: the site is behind, the summary row is already
/// at its head, and the next writable open refolds the site from core's
/// reading through the stream index, never the run (decision 0089 rulings
/// 4 and 7).
#[test]
fn a_writer_from_before_the_stamp_leaves_the_site_behind() {
    let (_dir, mut store) = store();
    let first = one_checkpoint(&mut store, json!({"step": "claude-session-finished"}));
    let second = finishing_after(&first, "r-stampless", "2026-01-01T00:00:09Z");
    // The writer from before the stamp wrote the records' facts and no
    // stamp: two session rows stand, and the summary row is current.
    plant_summary_writer(&store, "r", std::slice::from_ref(&second));
    let checkpoint = brokkr_core::summary::Checkpoint::read(&second, Some("implement"));
    let facts = super::checkpoint::facts(checkpoint.as_ref()).unwrap();
    write_pre_stamp_facts(&store, "r", second.seq, &facts);
    assert_eq!(session(&store, first.seq), Value::Integer(1));
    assert_eq!(session(&store, second.seq), Value::Integer(1));
    assert_eq!(stamped(&store, "r", first.seq), Value::Null);
    assert_eq!(stamped(&store, "r", second.seq), Value::Null);
    let read = store.compared("r", || {}).unwrap();
    assert!(
        matches!(
            read,
            crate::summary::Compared::Behind | crate::summary::Compared::SeatsBehind
        ),
        "{read:?}"
    );
    before_open(&mut store, second.seq, Value::Integer(1));
    let store = reopened(store, second.seq, Value::Integer(1));
    assert_eq!(session(&store, first.seq), Value::Integer(0));
    assert_eq!(
        stamped(&store, "r", first.seq),
        Value::Integer(first.seq as i64)
    );
    assert_eq!(
        stamped(&store, "r", second.seq),
        Value::Integer(second.seq as i64)
    );
}

/// A checkpoint whose payload names no attempt is no site: it keeps no
/// session, is stamped processed, and the append's refold skips it. A
/// writer from before the stamp leaves such a row behind, and the open
/// stamps it without inventing a site (decision 0089 rulings 4 and 7).
#[test]
fn a_checkpoint_with_no_attempt_is_stamped_without_a_site() {
    let (_dir, mut store) = store();
    let event = one_attemptless(&mut store);
    assert_eq!(kept(&store, "r", event.seq)[9], Value::Null);
    assert_eq!(
        stamped(&store, "r", event.seq),
        Value::Integer(event.seq as i64)
    );
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
    store
        .conn
        .execute(
            "UPDATE event_index SET facts_seq = NULL WHERE run_id = 'r' AND seq = ?1",
            params![event.seq as i64],
        )
        .unwrap();
    before_open(&mut store, event.seq, Value::Null);
    let store = reopened(store, event.seq, Value::Null);
    assert_eq!(
        stamped(&store, "r", event.seq),
        Value::Integer(event.seq as i64)
    );
}

/// The session send searches the session partial index for the site's
/// earlier session rows, and the stream index for the appended row; the
/// facts' write searches the row's key. No write walks the run's
/// checkpoints.
#[test]
fn the_session_and_row_sends_search_their_indexes() {
    let (_dir, store) = store();
    let bound = rusqlite::params_from_iter(std::iter::repeat_n(Value::Null, 15));
    assert_eq!(
        crate::test_support::plan(&store, &WRITE, bound).unwrap(),
        ["SEARCH event_index USING PRIMARY KEY (run_id=? AND seq=?)"]
    );
    let statements = crate::test_support::trigger_statements(
        &store,
        "event_index_checkpoint_behind",
        &[
            ("NEW.run_id", "'r'"),
            ("NEW.attempt_id", "'a'"),
            ("NEW.member", "'m'"),
            ("NEW.seq", "1000"),
        ],
    );
    assert_eq!(statements.len(), 2);
    assert_eq!(
        crate::test_support::plan(&store, &statements[0], []).unwrap(),
        [
            "SEARCH event_index USING PRIMARY KEY (run_id=? AND seq=?)",
            "LIST SUBQUERY 1",
            "SEARCH event_index USING COVERING INDEX event_index_checkpoint_session (run_id=? AND attempt_id=? AND member=? AND seq<?)"
        ]
    );
    assert_eq!(
        crate::test_support::plan(&store, &statements[1], []).unwrap(),
        [
            "SEARCH event_index USING PRIMARY KEY (run_id=? AND seq=?)",
            "LIST SUBQUERY 1",
            "SEARCH event_index USING INDEX event_index_stream (run_id=? AND attempt_id=? AND member=? AND seq=?)"
        ]
    );
}

/// The retired start lookup, kept here so the cost test measures the work a
/// refold that reads the attempt's starts would add: the same SQL text and
/// bind order, so `prepare_cached` hands that refold this statement.
const RETIRED_STARTS: &str = "SELECT seq FROM event_index INDEXED BY event_index_stream
         WHERE run_id = ?1 AND attempt_id = ?2
           AND type = 'effect/started' AND seq <= ?3
         ORDER BY seq";

/// Appending a checkpoint to a run with N checkpoints costs the same VM
/// steps at every N, at 2,000 and 20,000, measured over everything the
/// append runs inside the bound: the event's insert and its triggers, the
/// facts' write, and the refold's candidate search, envelope reads and
/// session writes. A refold that reads the attempt's starts would add the
/// retired lookup's history scan, which the same count holds to zero
/// (decision 0089 rulings 7 and 9).
#[test]
fn an_appended_checkpoint_costs_the_same_at_any_history() {
    use crate::import::INSERT_EVENT;
    use rusqlite::StatementStatus;

    let (_dir, mut store) = store();
    let mut steps = Vec::new();
    for n in [2_000u64, 20_000] {
        let run_id = format!("r{n}");
        let events = chained(
            &run_id,
            std::iter::once((
                EventType::EffectStarted,
                json!({"effect_id": "fx", "attempt_id": "a",
                       "provenance": [{"member": "m", "provider": "anthropic"}]}),
            ))
            .chain((1..=n).map(|_| {
                (
                    EventType::EffectCheckpointed,
                    checkpoint(Some("m"), json!({"step": "seat-turn"})),
                )
            }))
            .collect(),
        );
        let previous = planted(&mut store, &run_id, &events);
        let next = EnvelopeBuilder::new(
            EventType::EffectCheckpointed,
            checkpoint(Some("m"), json!({"step": "seat-turn"})),
        )
        .run(&run_id)
        .seq(n + 2)
        .event_id(format!("{run_id}-{}", n + 2))
        .at("2026-01-01T00:00:01Z")
        .previous(previous)
        .sealed();
        let envelope = serde_json::to_string(&next).unwrap();
        let mut insert = store.conn.prepare(INSERT_EVENT).unwrap();
        insert.reset_status(StatementStatus::VmStep);
        insert
            .execute(params![
                &run_id,
                (n + 2) as i64,
                &next.event_hash,
                &envelope
            ])
            .unwrap();
        let inserted = insert.reset_status(StatementStatus::VmStep);
        let account = Account::whole(&run_id, &store.load(&run_id).unwrap());
        // Reset each statement's count and drop it, so the append's own
        // `prepare_cached` finds it in the cache and the count is the
        // append's, not a second statement's.
        let reset = |sql: &str| {
            let statement = store.conn.prepare_cached(sql).unwrap();
            statement.reset_status(StatementStatus::VmStep);
        };
        for sql in [
            &*WRITE,
            &*SITE,
            super::ENVELOPE,
            WRITE_SESSION,
            RETIRED_STARTS,
        ] {
            reset(sql);
        }
        super::checkpoint::write_appended(&store.conn, &account, std::slice::from_ref(&next))
            .unwrap();
        let steps_of = |sql: &str| {
            let statement = store.conn.prepare_cached(sql).unwrap();
            i64::from(statement.reset_status(StatementStatus::VmStep))
        };
        let facts = steps_of(&WRITE);
        let refold = [&*SITE, super::ENVELOPE, WRITE_SESSION, RETIRED_STARTS]
            .into_iter()
            .map(steps_of)
            .sum::<i64>();
        steps.push((inserted, facts, refold));
    }
    assert_eq!(
        steps[0], steps[1],
        "appending costs the checkpoint: {steps:?}"
    );
}

/// Appending a checkpoint visits its own site's rows and no other's: the
/// trigger's send and the refold's candidate search cost the same at 200
/// and 400 checkpoints at the appended site, and a run whose other sites
/// hold many checkpoints visits none of them (decision 0089 rulings 4 and
/// 7; #912's park send).
#[test]
fn sending_a_checkpoint_behind_visits_only_its_site_s_rows() {
    let (_dir, mut store) = store();
    let mut same = Vec::new();
    let mut other = Vec::new();
    let mut selected_same = Vec::new();
    let mut selected_other = Vec::new();
    for n in [200u64, 400] {
        let run_id = format!("same{n}");
        let events = chained_checkpoints(&run_id, "a", "m", n);
        let previous = planted(&mut store, &run_id, &events);
        let (found, steps) = site_candidates(&store, &run_id, "a", "m", n);
        assert_eq!(found, vec![n], "the new row is the only candidate");
        selected_same.push(steps);
        same.push(insert_checkpoint(
            &store,
            &run_id,
            n + 1,
            &previous,
            "a",
            "m",
        ));

        let run_id = format!("other{n}");
        let events = chained_checkpoints(&run_id, "b", "m2", n);
        let previous = planted(&mut store, &run_id, &events);
        let (found, steps) = site_candidates(&store, &run_id, "a", "m", n);
        assert_eq!(found, Vec::<u64>::new(), "another site holds no rows");
        selected_other.push(steps);
        other.push(insert_checkpoint(
            &store,
            &run_id,
            n + 1,
            &previous,
            "a",
            "m",
        ));
    }
    assert_eq!(same[0], same[1], "the appended site's rows alone: {same:?}");
    // The ordinal the writer keeps seeks the appended site's stream index
    // once, and that empty-range seek crosses one more page between 200 and
    // 400 checkpoints; the invariant is that no other site's row is visited,
    // which a scan of them would fail by hundreds of steps, not one.
    assert!(
        other[1].abs_diff(other[0]) <= 1,
        "another site's rows are never visited: {other:?}"
    );
    assert_eq!(
        selected_same[0], selected_same[1],
        "the refold selects its site's rows alone: {selected_same:?}"
    );
    assert_eq!(
        selected_other[0], selected_other[1],
        "the refold visits no other site's rows: {selected_other:?}"
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
        .execute_batch("DROP TRIGGER event_index_checkpoint_behind")
        .unwrap();
    let second = finishing_after(&first, "r-older", "2026-01-01T00:00:09Z");
    // An older writer appends with the trigger gone: neither row is sent
    // behind, and the open's repair finds the trigger missing.
    plant_envelope(&store.conn, &second).unwrap();
    assert_eq!(kept(&store, "r", first.seq)[9], Value::Integer(1));
    assert_eq!(kept(&store, "r", second.seq)[11], Value::Null);
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
         DROP INDEX event_index_checkpoint_site_stale;
         DROP TRIGGER event_index_checkpoint_behind;
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
            Value::Null,
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

/// A checkpoint row whose facts or stamp are not written is a lag, not a
/// disagreement: the comparison compares its immutable columns and leaves
/// its dated ones for the refold that writes them (decision 0089 ruling 4).
#[test]
fn a_checkpoint_marker_off_its_fold_is_a_lag_not_a_disagreement() {
    for column in ["facts", "facts_seq", "repeat"] {
        let (_dir, mut store) = store();
        let (event, _held) = one_kept(&mut store, json!({"step": "seat-turn"}));
        let edit =
            format!("UPDATE event_index SET {column} = NULL WHERE run_id = 'r' AND seq = ?1");
        store
            .conn
            .execute(&edit, params![event.seq as i64])
            .unwrap();
        assert!(
            matches!(store.compared("r", || {}).unwrap(), Compared::Behind),
            "{column}"
        );
        lags(&mut store, &["r"]);
    }
}

/// The refusal an append of one checkpoint whose `column` is past the
/// signed bound returns: the run `r` of `implement`, written nothing.
fn refused_count(store: &mut Store, column: &str) -> crate::StoreError {
    let mut record = json!({"step": "seat-turn"});
    record[column] = Json::from(9_223_372_036_854_775_808u64);
    store
        .append_next(
            "r",
            EventType::EffectCheckpointed,
            checkpoint(Some("m"), record),
            None,
            None,
        )
        .unwrap_err()
}

/// The run `r` is still at its head and the refused append wrote no
/// checkpoint row.
fn assert_nothing_written(store: &Store) {
    assert_eq!(store.head_hash("r").unwrap().0, 2);
    assert_eq!(
        crate::test_support::indexed_seqs(store, "r", "effect/checkpointed"),
        Vec::<u64>::new()
    );
}

/// A token count past the signed 64-bit integer the index keeps is refused
/// by name, not wrapped into a negative fact, and the append writes
/// nothing (decision 0089 ruling 7).
#[test]
fn a_token_count_past_the_index_s_bound_is_refused_by_name() {
    let (_dir, mut store) = store();
    opened(&mut store, "r", "implement", &[]);
    let refused = refused_count(&mut store, "input_tokens");
    assert!(matches!(
        refused,
        crate::StoreError::TokenCountOutOfRange(3, "input_tokens", 9_223_372_036_854_775_808)
    ));
    assert_eq!(
        refused.to_string(),
        "checkpoint 3 reports input_tokens 9223372036854775808 past the index's 64-bit \
         bound; nothing was written"
    );
    assert_nothing_written(&store);
}

/// The other token counts are held to the same signed bound, each refused
/// by its own column: a cached count past it is a typed refusal, never a
/// negative fact (decision 0089 ruling 7).
#[test]
fn every_token_count_past_the_index_s_bound_is_refused_by_name() {
    let (_dir, mut store) = store();
    opened(&mut store, "r", "implement", &[]);
    for column in ["output_tokens", "cache_read_tokens", "cache_write_tokens"] {
        let refused = refused_count(&mut store, column);
        assert!(
            matches!(
                refused,
                crate::StoreError::TokenCountOutOfRange(
                    3,
                    found,
                    9_223_372_036_854_775_808
                ) if found == column
            ),
            "{column}: {refused:?}"
        );
        assert_eq!(
            refused.to_string(),
            format!(
                "checkpoint 3 reports {column} 9223372036854775808 past the index's 64-bit \
                 bound; nothing was written"
            )
        );
    }
    assert_nothing_written(&store);
}

/// A reasoning count past the bound is refused by the index's one reading
/// even though an append's seat-record contract refuses that record first:
/// no writer wraps the count into a negative fact (decision 0089 ruling 7).
#[test]
fn a_reasoning_count_past_the_index_s_bound_is_refused_by_name() {
    let events = chained(
        "r",
        vec![(
            EventType::EffectCheckpointed,
            json!({"effect_id": "fx", "attempt_id": "a", "checkpoint": {
                "member": "m", "step": "seat-turn",
                "reasoning_output_tokens": 9_223_372_036_854_775_808u64,
            }}),
        )],
    );
    let checkpoint = Checkpoint::read(&events[0], Some("implement")).unwrap();
    let refused = super::checkpoint::facts(Some(&checkpoint)).unwrap_err();
    assert!(
        matches!(
            refused,
            crate::StoreError::TokenCountOutOfRange(
                1,
                "reasoning_output_tokens",
                9_223_372_036_854_775_808
            )
        ),
        "{refused:?}"
    );
    assert_eq!(
        refused.to_string(),
        "checkpoint 1 reports reasoning_output_tokens 9223372036854775808 past the index's \
         64-bit bound; nothing was written"
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
    let named = store.summary_disagreements().unwrap();
    assert_eq!(named.len(), 1, "the healthy run still reads: {named:?}");
    assert!(
        matches!(
            &named[0],
            crate::Disagreement::Unread {
                run_id,
                error: crate::StoreError::TokenCountOutOfRange(
                    5,
                    "input_tokens",
                    9_223_372_036_854_775_808
                ),
            } if run_id == "legacy"
        ),
        "{named:?}"
    );
    // The next open refuses the same run by name and still opens.
    let path = store.path().to_path_buf();
    drop(store);
    let store = Store::open(&path).unwrap();
    assert!(!backfilled(&store.conn).unwrap());
    assert_eq!(kept(&store, "healthy", healthy.seq)[11], Value::Integer(1));
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
    let store = unfinished_then_backfilled(store, moved);
    assert_eq!(kept(&store, "r", first.seq)[11], Value::Integer(1));
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// A peer's unwritten checkpoint landing while the session refold reads the
/// site is left for the next open, and this pass claims no completion
/// (decision 0089 rulings 4 and 8).
#[test]
fn a_session_refold_whose_peer_writes_facts_stays_unfinished() {
    let (_dir, mut store) = store();
    let first = one_checkpoint(&mut store, json!({"step": "claude-session-finished"}));
    store
        .conn
        .execute(
            "UPDATE event_index SET facts_seq = NULL WHERE run_id = 'r' AND seq = ?1",
            params![first.seq as i64],
        )
        .unwrap();
    lags(&mut store, &["r"]);
    let peer = finishing_after(&first, "r-peer", "2026-01-01T00:00:09Z");
    let mut moved = false;
    let staged = peer_once(&peer, &mut moved);
    store.checkpoint_behind_with(staged).unwrap();
    let store = unfinished_then_backfilled(store, moved);
    assert_eq!(session(&store, first.seq), Value::Integer(0));
    assert_eq!(session(&store, peer.seq), Value::Integer(1));
}

/// A checkpoint whose record names no attempt has no site to refold: its
/// stamp is the row's own seq. An update the journal refuses returns the
/// store's error, leaving the row behind for the next open.
#[test]
fn a_named_checkpoint_stamp_the_index_refuses_errors() {
    let (_dir, mut store) = store();
    let event = one_attemptless(&mut store);
    store
        .conn
        .execute(
            "UPDATE event_index SET facts_seq = NULL WHERE run_id = 'r' AND seq = ?1",
            params![event.seq as i64],
        )
        .unwrap();
    lags(&mut store, &["r"]);
    store
        .conn
        .execute_batch(&format!(
            "CREATE TRIGGER refusing BEFORE UPDATE OF facts_seq ON event_index
             WHEN OLD.run_id = 'r' AND OLD.seq = {}
             BEGIN SELECT RAISE(ABORT, 'the index refuses the stamp'); END;",
            event.seq
        ))
        .unwrap();
    let refused = store.checkpoint_behind_with(|_| {}).unwrap_err();
    assert!(matches!(refused, StoreError::Sqlite(_)), "{refused:?}");
    assert_eq!(refused.to_string(), "sqlite: the index refuses the stamp");
    assert_eq!(stamped(&store, "r", event.seq), Value::Null);
}

/// A checkpoint write the journal refuses fails the run's refold with the
/// store's error rather than passing it like a count the index cannot
/// hold, and the next open after the fault is gone refolds the run.
#[test]
fn a_refused_checkpoint_write_fails_the_refold() {
    let (_dir, mut store) = store();
    opened(&mut store, "r", "implement", &[]);
    let first = append(
        &mut store,
        "r",
        EventType::EffectCheckpointed,
        checkpoint(Some("m"), json!({"step": "seat-turn"})),
    );
    let moved = EnvelopeBuilder::new(EventType::PhaseEntered, json!({"phase": "verify"}))
        .run("r")
        .seq(first.seq + 1)
        .event_id("r-moved")
        .at("2026-01-01T00:00:09Z")
        .previous(first.event_hash.clone())
        .sealed();
    plant_envelope(&store.conn, &moved).unwrap();
    assert_eq!(lane(&store, "r").as_deref(), Some("behind"));
    store
        .conn
        .execute_batch(
            "CREATE TRIGGER refusing BEFORE UPDATE OF at_ms ON event_index
             WHEN OLD.run_id = 'r'
             BEGIN SELECT RAISE(ABORT, 'the index refuses the checkpoint'); END;",
        )
        .unwrap();
    let refused = store.catch_up().unwrap_err();
    assert!(matches!(refused, StoreError::Sqlite(_)), "{refused:?}");
    assert_eq!(
        refused.to_string(),
        "sqlite: the index refuses the checkpoint"
    );
    assert_eq!(lane(&store, "r").as_deref(), Some("behind"));
    store.conn.execute_batch("DROP TRIGGER refusing").unwrap();
    store.catch_up().unwrap();
    assert_ne!(lane(&store, "r").as_deref(), Some("behind"));
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// A run whose chain does not load is left for a reader to read whole and
/// the doctor to name: the checkpoint backfill writes nothing of it, walks
/// past it, and claims no completion.
#[test]
fn a_run_that_does_not_load_keeps_its_checkpoints_for_the_doctor() {
    let (_dir, mut store) = store();
    let events = checkpoint_run("broken");
    // The third event chains onto no hash of the second.
    plant_older_writer(&store, "broken", &events[..2]);
    crate::test_support::plant_broken_link(&store.conn, &events[2], 3).unwrap();
    let refused = store.load("broken").unwrap_err();
    assert!(
        matches!(
            &refused,
            StoreError::Chain(ChainError::BrokenChain {
                seq: 3,
                prev_seq: 2
            })
        ),
        "{refused:?}"
    );
    assert_eq!(kept(&store, "broken", 3)[11], Value::Null);
    store.checkpoint_behind_with(|_| {}).unwrap();
    assert!(!backfilled(&store.conn).unwrap());
    assert_eq!(kept(&store, "broken", 3)[11], Value::Null);
}

/// A checkpoint write the journal refuses fails the checkpoint backfill by
/// the store's own error, not passed like a count the index cannot hold:
/// the run stays for the next open, and the next pass writes it.
#[test]
fn a_refused_checkpoint_backfill_errors() {
    let (_dir, mut store) = store();
    let events = checkpoint_run("r");
    plant_older_writer(&store, "r", &events);
    store
        .conn
        .execute_batch(
            "CREATE TRIGGER refusing BEFORE UPDATE OF at_ms ON event_index
             WHEN OLD.run_id = 'r'
             BEGIN SELECT RAISE(ABORT, 'the index refuses the backfill'); END;",
        )
        .unwrap();
    let refused = store.checkpoint_behind_with(|_| {}).unwrap_err();
    assert!(matches!(refused, StoreError::Sqlite(_)), "{refused:?}");
    assert_eq!(
        refused.to_string(),
        "sqlite: the index refuses the backfill"
    );
    assert!(!backfilled(&store.conn).unwrap());
    assert_eq!(kept(&store, "r", 3)[11], Value::Null);
    store.conn.execute_batch("DROP TRIGGER refusing").unwrap();
    store.checkpoint_behind_with(|_| {}).unwrap();
    assert!(backfilled(&store.conn).unwrap());
    assert_eq!(kept(&store, "r", 3)[11], Value::Integer(1));
}

/// A checkpoint row's `repeat` flag, as its writer wrote it.
fn repeated(store: &Store, run_id: &str, seq: u64) -> Value {
    crate::test_support::kept_facts(store, run_id, seq, &["repeat"]).remove(0)
}

/// A streamed message's turns repeat the usage of the site's last metered
/// turn and are flagged 1 after the first; a turn with another usage, or
/// another site's turn, is 0 (decision 0089 ruling 7).
#[test]
fn a_turn_repeating_its_site_s_last_message_is_flagged() {
    let (_dir, mut store) = store();
    opened(&mut store, "r", "implement", &[]);
    let first = append_checkpoint(
        &mut store,
        json!({"step": "seat-turn", "input_tokens": 10, "output_tokens": 4}),
    );
    let second = append_checkpoint(
        &mut store,
        json!({"step": "seat-turn", "input_tokens": 10, "output_tokens": 4}),
    );
    let third = append_checkpoint(
        &mut store,
        json!({"step": "seat-turn", "input_tokens": 12, "output_tokens": 5}),
    );
    let fourth = append_checkpoint(
        &mut store,
        json!({"step": "seat-turn", "input_tokens": 12, "output_tokens": 5}),
    );
    assert_eq!(repeated(&store, "r", first.seq), Value::Integer(0));
    assert_eq!(repeated(&store, "r", second.seq), Value::Integer(1));
    assert_eq!(repeated(&store, "r", third.seq), Value::Integer(0));
    assert_eq!(repeated(&store, "r", fourth.seq), Value::Integer(1));
    // A turn carrying no usage is no message again, and moves nothing.
    let bare = append_checkpoint(&mut store, json!({"step": "tool-use"}));
    assert_eq!(repeated(&store, "r", bare.seq), Value::Integer(0));
    let again = append_checkpoint(
        &mut store,
        json!({"step": "seat-turn", "input_tokens": 12, "output_tokens": 5}),
    );
    assert_eq!(repeated(&store, "r", again.seq), Value::Integer(1));
    // Another site's identical turn is its own first message.
    let other = append(
        &mut store,
        "r",
        EventType::EffectCheckpointed,
        checkpoint_at(
            "b",
            Some("m"),
            json!({"step": "seat-turn", "input_tokens": 10, "output_tokens": 4}),
        ),
    );
    assert_eq!(repeated(&store, "r", other.seq), Value::Integer(0));
}

/// Every fixture journal's `repeat` flags are the ones the messages' reading
/// gives: a turn whose usage its site's last metered turn carried is the
/// message again (decision 0089 ruling 7).
#[test]
fn every_fixture_flags_the_turns_its_site_s_last_message_repeats() {
    use brokkr_core::seat::meter::once;
    use brokkr_core::summary::{checkpoints, Checkpoint};
    use std::collections::BTreeMap;

    let (_dir, mut store) = store();
    let mut fixtures = planted_fixtures(&store);
    // A streamed message: the same usage on two turns of one site, which
    // the fixtures may hold none of, so the reading is proven past them.
    let message = || {
        checkpoint(
            Some("m"),
            json!({"step": "seat-turn", "input_tokens": 10, "output_tokens": 4}),
        )
    };
    let streamed = chained(
        "streamed",
        vec![
            (EventType::EffectCheckpointed, message()),
            (EventType::EffectCheckpointed, message()),
            (
                EventType::EffectCheckpointed,
                checkpoint(
                    Some("m"),
                    json!({"step": "seat-turn", "input_tokens": 12, "output_tokens": 5}),
                ),
            ),
        ],
    );
    plant_older_writer(&store, "streamed", &streamed);
    fixtures.push(("streamed".to_string(), streamed));
    store.checkpoint_behind_with(|_| {}).unwrap();
    let mut wrong = Vec::new();
    for (run_id, events) in fixtures {
        let read = checkpoints(&events);
        let mut sites: BTreeMap<(String, Option<String>), Vec<&Checkpoint>> = BTreeMap::new();
        for checkpoint in &read {
            sites
                .entry((checkpoint.attempt_id.clone(), checkpoint.member.clone()))
                .or_default()
                .push(checkpoint);
        }
        let mut expected = BTreeMap::new();
        for site in sites.values() {
            let before: Vec<Option<_>> = site
                .iter()
                .map(|checkpoint| checkpoint.meter.usage().cloned())
                .collect();
            let mut meters: Vec<_> = site
                .iter()
                .map(|checkpoint| checkpoint.meter.clone())
                .collect();
            once(meters.iter_mut());
            for (at, checkpoint) in site.iter().enumerate() {
                let after = meters[at].usage().is_some();
                expected.insert(checkpoint.seq, before[at].is_some() && !after);
            }
        }
        for checkpoint in &read {
            let want = expected[&checkpoint.seq];
            let found =
                repeated(&store, &run_id, checkpoint.seq) == Value::Integer(i64::from(want));
            if !found {
                wrong.push(format!(
                    "{run_id}: checkpoint {} repeat {:?}, read {want}",
                    checkpoint.seq,
                    repeated(&store, &run_id, checkpoint.seq)
                ));
            }
        }
    }
    assert_eq!(wrong, Vec::<String>::new());
}

/// A start naming an attempt already started opens another site, and its
/// first turn repeating the earlier site's usage is not the message again:
/// `repeat` follows the site's boundaries, as `meter::once` does
/// (decision 0089 rulings 4 and 7).
#[test]
fn a_reopened_site_s_first_turn_does_not_repeat_the_earlier_site_s() {
    let (_dir, mut store) = store();
    opened(&mut store, "r", "implement", &[]);
    append(
        &mut store,
        "r",
        EventType::EffectRequested,
        json!({"effect_id": "fx", "seat": "worker", "phase": "implement"}),
    );
    started(&mut store, "r", "a", Some("m"), "anthropic");
    let first = append_checkpoint(
        &mut store,
        json!({"step": "seat-turn", "input_tokens": 10, "output_tokens": 4}),
    );
    failed(&mut store, "r", "a");
    started(&mut store, "r", "a", Some("m"), "anthropic");
    let second = append_checkpoint(
        &mut store,
        json!({"step": "seat-turn", "input_tokens": 10, "output_tokens": 4}),
    );
    assert_eq!(repeated(&store, "r", first.seq), Value::Integer(0));
    assert_eq!(repeated(&store, "r", second.seq), Value::Integer(0));
    let events = store.load("r").unwrap();
    let sites = brokkr_core::summary::sites(&events);
    assert_eq!(sites.len(), 2);
    assert_eq!(
        sites[1].metered[0].meter.usage().unwrap().input_tokens,
        Some(10)
    );
}
