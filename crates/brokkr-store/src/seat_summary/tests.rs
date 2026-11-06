//! The seat summary (decision 0089, slice 2): an append writes the seat
//! rows it moved and the seat head in its own transaction, a writer that
//! hands over no fold writes none, a writer from before the table leaves
//! the rows stale and a writable open refolds them, a backfill cut short
//! resumes at the next run, and a run's rows are read only at its head.

use super::*;
use crate::test_support::{
    disagreements, every_run_reads_as_its_fold, failed, import_fixtures, lags, opened,
    plant_envelope, plant_summary_writer, planted_fixtures, refolded, started, unseat,
};
use crate::tests::envelope_builder::EnvelopeBuilder;
use crate::tests::walk::cut_short;
use crate::tests::{store, write_lock_on};
use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::seat::row::{facts, Outcome, Provider};
use serde_json::{json, Value as Json};

fn append(store: &mut Store, run_id: &str, event_type: EventType, payload: Json) {
    store
        .append_next(run_id, event_type, payload, None, None)
        .unwrap();
}

fn checkpoint(member: Option<&str>, turn: u64) -> Json {
    let mut checkpoint = json!({"step": "seat-turn", "turn": turn, "model": "opus",
                                "input_tokens": 10 * turn, "output_tokens": turn});
    if let Some(member) = member {
        checkpoint["member"] = json!(member);
    }
    json!({"effect_id": "fx", "attempt_id": "a", "checkpoint": checkpoint})
}

/// `run_id` created by this store, which carries its account, its phase
/// entered, an effect requested and started, and a checkpoint of the
/// seat's: two rows, the request's own and its attempt's.
fn seated(store: &mut Store, run_id: &str) {
    store
        .create_run(run_id, "a feature", "self", &json!({}))
        .unwrap();
    let opened = json!({"feature": "a feature", "manifest": {}});
    append(store, run_id, EventType::RunStarted, opened);
    append(
        store,
        run_id,
        EventType::PhaseEntered,
        json!({"phase": "work"}),
    );
    let requested = json!({"effect_id": "fx", "seat": "worker", "phase": "work"});
    append(store, run_id, EventType::EffectRequested, requested);
    let provenance = json!([{"member": null, "provider": "anthropic"}]);
    let started = json!({"effect_id": "fx", "attempt_id": "a", "provenance": provenance});
    append(store, run_id, EventType::EffectStarted, started);
    append(
        store,
        run_id,
        EventType::EffectCheckpointed,
        checkpoint(None, 1),
    );
}

/// `run_id`'s rows as its events read whole fold them.
fn folded_rows(store: &Store, run_id: &str) -> Vec<SeatRow> {
    let events = store.load(run_id).unwrap();
    Account::whole(run_id, &events).seats().to_vec()
}

/// `run_id`'s rows, read as current.
fn read(store: &Store, run_id: &str) -> Vec<SeatRow> {
    match store.seat_rows(run_id).unwrap() {
        SeatRows::Current(rows) => rows,
        SeatRows::Stale => panic!("{run_id}'s seat rows read as stale"),
    }
}

/// `run_id`'s seat head as SQLite holds it.
fn head(store: &Store, run_id: &str) -> Option<(u64, String)> {
    let folded = folded(&store.conn, run_id).unwrap();
    folded.map(|(seq, hash, _)| (seq, hash))
}

fn sql(store: &Store, statements: &str) {
    store.conn.execute_batch(statements).unwrap();
}

/// The next event of `run_id`, sealed onto its head and not appended.
fn next(store: &Store, run_id: &str, event_type: EventType, payload: Json) -> EventEnvelope {
    let (seq, previous) = store.head_hash(run_id).unwrap();
    EnvelopeBuilder::new(event_type, payload)
        .run(run_id)
        .seq(seq + 1)
        .event_id(format!("{run_id}-{}", seq + 1))
        .previous(previous)
        .sealed()
}

/// The next event of `run_id`, stamped with a time no reader parses and
/// sealed over it, as an older writer leaves it.
fn unreadable(store: &Store, run_id: &str, event_type: EventType, payload: Json) -> EventEnvelope {
    let mut event = next(store, run_id, event_type, payload);
    event.recorded_at = "not a stamp".into();
    event.event_hash = event.compute_hash();
    event
}

/// Plant `event` as an older writer leaves it, mark the run a lag, and
/// open the journal again to refold it.
fn planted_and_refolded(mut store: Store, run_id: &str, event: &EventEnvelope) -> Store {
    plant_envelope(&store.conn, event).unwrap();
    lags(&mut store, &[run_id]);
    refolded(store)
}

#[test]
fn a_new_run_has_its_seat_head_and_no_row() {
    let (_dir, mut store) = store();
    store.create_run("r", "f", "self", &json!({})).unwrap();
    assert_eq!(head(&store, "r"), Some((0, ZERO.to_string())));
    assert_eq!(store.seat_rows("r").unwrap(), SeatRows::Current(Vec::new()));
}

const ZERO: &str = brokkr_core::canonical::ZERO_HASH;

/// A checkpoint's append writes its row and the seat head in its own
/// transaction, and only the rows its event moved: a row it did not move
/// keeps what the table held, here an edit the read and the binding then
/// name, and which, undone, leaves every row its fold's.
#[test]
fn a_checkpoint_append_writes_the_rows_it_moved_and_the_seat_head() {
    let (_dir, mut store) = store();
    seated(&mut store, "r");
    assert_eq!(read(&store, "r"), folded_rows(&store, "r"));
    assert_eq!(read(&store, "r").len(), 2);
    // The seat's attempt row: a member's checkpoint moves the request's
    // own row, which keeps the effect's accounting, and not this one.
    sql(
        &store,
        "UPDATE seat_summary SET seat = 'edited' WHERE opened = 1",
    );
    append(
        &mut store,
        "r",
        EventType::EffectCheckpointed,
        checkpoint(Some("m"), 2),
    );
    let head_now = store.head_hash("r").unwrap();
    assert_eq!(head(&store, "r"), Some(head_now));
    assert_eq!(store.seat_rows("r").unwrap(), SeatRows::Stale);
    assert_eq!(
        every_run_reads_as_its_fold(&store),
        ["r: seat row 1 Column(\"seat\")"]
    );
    sql(
        &store,
        "UPDATE seat_summary SET seat = 'worker' WHERE opened = 1",
    );
    assert_eq!(read(&store, "r"), folded_rows(&store, "r"));
    assert_eq!(read(&store, "r").len(), 3);
}

/// A run whose live start names an effect no request has asked for keeps
/// no seat head, its rows not keeping that start's live block: they read
/// stale, and the binding holds them to their fold all the same, an edited
/// row or a seat head kept for them named behind. A writable open refolds
/// the run and keeps none still. The terminal ending that start keeps the
/// head again, every row its fold's.
#[test]
fn a_live_start_no_row_is_of_keeps_no_seat_head() {
    let (_dir, mut store) = store();
    seated(&mut store, "r");
    let unasked = json!({"effect_id": "other", "attempt_id": "b"});
    append(&mut store, "r", EventType::EffectStarted, unasked.clone());
    assert_eq!(head(&store, "r"), None);
    assert_eq!(read_raw(&store, "r"), 2);
    assert_eq!(store.seat_rows("r").unwrap(), SeatRows::Stale);
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
    let behind = ["r: seat rows behind its head"];
    sql(
        &store,
        "UPDATE seat_summary SET seat = 'x' WHERE opened = 0",
    );
    assert_eq!(every_run_reads_as_its_fold(&store), behind);
    sql(
        &store,
        "UPDATE seat_summary SET seat = 'worker' WHERE opened = 0",
    );
    let (seq, hash) = store.head_hash("r").unwrap();
    write_head(&store.conn, "r", (seq, &hash)).unwrap();
    assert_eq!(every_run_reads_as_its_fold(&store), behind);
    sql(&store, "DELETE FROM seat_head WHERE run_id = 'r'");
    let peer = Store::open(store.path()).unwrap();
    assert_eq!(head(&peer, "r"), None);
    append(&mut store, "r", EventType::EffectSucceeded, unasked);
    assert_eq!(head(&store, "r"), Some(store.head_hash("r").unwrap()));
    assert_eq!(read(&store, "r"), folded_rows(&store, "r"));
}

/// An append through a store that carries no fold writes no seat row and
/// leaves the seat head behind; so does one whose fold stands at another
/// head than the append found, and the rows read as stale.
#[test]
fn an_append_with_no_fold_or_one_at_another_head_writes_no_seat_row() {
    let (_dir, mut store) = store();
    seated(&mut store, "r");
    let before = head(&store, "r");
    let mut peer = Store::open(store.path()).unwrap();
    append(
        &mut peer,
        "r",
        EventType::EffectCheckpointed,
        checkpoint(None, 2),
    );
    assert_eq!(head(&store, "r"), before);
    append(
        &mut store,
        "r",
        EventType::EffectCheckpointed,
        checkpoint(None, 3),
    );
    assert!(store.carried.is_none());
    assert_eq!(head(&store, "r"), before);
    assert_eq!(read_raw(&store, "r"), 2);
    assert_eq!(store.seat_rows("r").unwrap(), SeatRows::Stale);
    assert_eq!(every_run_reads_as_its_fold(&store), ["r: behind its head"]);
    lags(&mut store, &["r"]);
}

/// A read is one snapshot: a peer that appends to the run and refolds it
/// after the read took the run's last event leaves the read at that
/// event, with that head's rows, never naming one instant's head beside
/// another's seat head.
#[test]
fn a_seat_read_is_one_snapshot_whatever_a_peer_writes_meanwhile() {
    let (_dir, mut store) = store();
    seated(&mut store, "r");
    let before = folded_rows(&store, "r");
    let mut peer = Store::open(store.path()).unwrap();
    let found = store.seat_rows_with("r", || {
        let more = checkpoint(Some("m"), 2);
        append(&mut peer, "r", EventType::EffectCheckpointed, more);
        peer.refold("r", &mut |_| {}).unwrap();
    });
    assert_eq!(found.unwrap(), SeatRows::Current(before));
    assert_eq!(read(&store, "r"), folded_rows(&store, "r"));
    assert_eq!(read(&store, "r").len(), 3);
}

/// A participant's costs summed past `f64::MAX` keep their sum, an
/// infinity, which its seat row reads back current and as its fold.
#[test]
fn a_cost_summed_to_an_infinity_reads_current_as_its_fold() {
    let (_dir, mut store) = store();
    seated(&mut store, "r");
    for turn in [2, 3] {
        let priced = json!({"step": "seat-turn", "turn": turn, "total_cost_usd": 1e308});
        let payload = json!({"effect_id": "fx", "attempt_id": "a", "checkpoint": priced});
        append(&mut store, "r", EventType::EffectCheckpointed, payload);
    }
    let rows = read(&store, "r");
    assert_eq!(rows, folded_rows(&store, "r"));
    let costs: Vec<Option<f64>> = brokkr_core::seat::row::facts(&rows)
        .iter()
        .map(|facts| facts.cost)
        .collect();
    assert_eq!(costs, [Some(f64::INFINITY)]);
}

/// How many seat rows `run_id` holds, whatever its seat head.
fn read_raw(store: &Store, run_id: &str) -> i64 {
    let count = "SELECT count(*) FROM seat_summary WHERE run_id = ?1";
    store
        .conn
        .query_row(count, [run_id], |row| row.get(0))
        .unwrap()
}

/// A writer that hands over the fold of a run whose seat head an older
/// writer left behind writes every row, not only those it moved.
#[test]
fn an_append_onto_seat_rows_that_lag_writes_every_row() {
    let (_dir, mut store) = store();
    seated(&mut store, "r");
    sql(
        &store,
        "UPDATE seat_head SET folded_seq = 1; UPDATE seat_summary SET seat = 'x'",
    );
    append(
        &mut store,
        "r",
        EventType::EffectCheckpointed,
        checkpoint(None, 2),
    );
    assert_eq!(read(&store, "r"), folded_rows(&store, "r"));
}

/// A writer from between the run summary and the seat summary leaves the
/// summary row at its head and the seat rows behind it: they read as
/// stale, a read-only open migrates and refolds nothing, and a writable
/// open refolds them.
#[test]
fn an_older_writer_leaves_seat_rows_stale_until_a_writable_open() {
    let (_dir, mut store) = store();
    seated(&mut store, "r");
    let stale = head(&store, "r");
    let more = next(
        &store,
        "r",
        EventType::EffectCheckpointed,
        checkpoint(Some("m"), 2),
    );
    plant_summary_writer(&store, "r", &[more]);
    assert_eq!(
        crate::test_support::lane(&store, "r").as_deref(),
        Some("running")
    );
    assert_eq!(store.seat_rows("r").unwrap(), SeatRows::Stale);
    assert_eq!(
        every_run_reads_as_its_fold(&store),
        ["r: seat rows behind its head"]
    );
    let path = store.path().to_path_buf();
    lags(&mut store, &["r"]);
    drop(store);
    let reader = Store::open_read_only(&path).unwrap();
    assert_eq!(reader.seat_rows("r").unwrap(), SeatRows::Stale);
    assert_eq!(head(&reader, "r"), stale);
    let store = Store::open(&path).unwrap();
    assert_eq!(read(&store, "r"), folded_rows(&store, "r"));
    assert_eq!(read(&reader, "r").len(), 3);
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// Three seated runs, left as a binary from before the seat summary
/// leaves them: no table of it.
fn older_journal() -> (tempfile::TempDir, std::path::PathBuf) {
    let (dir, mut store) = store();
    for run_id in ["a", "b", "c"] {
        seated(&mut store, run_id);
    }
    let path = store.path().to_path_buf();
    drop(store);
    unseat(&rusqlite::Connection::open(&path).unwrap()).unwrap();
    (dir, path)
}

fn heads(store: &Store) -> Vec<bool> {
    let runs = ["a", "b", "c"];
    runs.iter().map(|run| head(store, run).is_some()).collect()
}

#[test]
fn a_journal_from_before_the_seat_summary_reads_stale_until_a_writable_open() {
    let (_dir, path) = older_journal();
    let reader = Store::open_read_only(&path).unwrap();
    assert!(!seats_intact(&reader.conn).unwrap());
    assert_eq!(reader.seat_rows("a").unwrap(), SeatRows::Stale);
    let store = Store::open(&path).unwrap();
    assert!(finished(&store.conn, BACKFILLED).unwrap());
    for run_id in ["a", "b", "c"] {
        assert_eq!(
            read(&reader, run_id),
            folded_rows(&store, run_id),
            "{run_id}"
        );
    }
}

/// A backfill a peer's lock cuts short writes the runs it reached, leaves
/// the journal's seat rows read as stale, and the next writable open
/// resumes at the next run.
#[test]
fn a_seat_backfill_cut_short_resumes_at_the_next_run() {
    let (_dir, path) = older_journal();
    migrate_seats(&rusqlite::Connection::open(&path).unwrap()).unwrap();
    let lock = write_lock_on(&path);
    let mut store = Store::open(&path).unwrap();
    lock.execute_batch("ROLLBACK").unwrap();
    assert_eq!(heads(&store), [false, false, false]);
    let peer = cut_short(&mut store, &path, |store, stage| {
        store.seat_behind_with(stage)
    });
    assert_eq!(heads(&store), [true, false, false]);
    assert!(!finished(&store.conn, BACKFILLED).unwrap());
    assert_eq!(store.seat_rows("a").unwrap(), SeatRows::Stale);
    drop(store);
    peer.execute_batch("ROLLBACK").unwrap();
    let store = Store::open(&path).unwrap();
    assert!(finished(&store.conn, BACKFILLED).unwrap());
    assert_eq!(heads(&store), [true, true, true]);
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// Each of `runs`, by how many seat rows it reads as current, each the
/// fold's, or `None` where they read as stale.
fn counted(store: &Store, runs: &[String]) -> Vec<(String, Option<usize>)> {
    let counted = runs.iter().map(|run| {
        let rows = match store.seat_rows(run).unwrap() {
            SeatRows::Current(rows) => Some(rows),
            SeatRows::Stale => None,
        };
        if let Some(rows) = &rows {
            assert_eq!(*rows, folded_rows(store, run), "{run}");
        }
        (run.clone(), rows.map(|rows| rows.len()))
    });
    counted.collect()
}

/// An import writes each fixture's rows as the projection gives them, at
/// its head. A fixture the import refuses is planted as an older writer's
/// and reads as stale until a writable open folds it; the broken chain
/// does not load, and is never folded.
#[test]
fn an_import_writes_the_rows_the_projection_gives() {
    let (_dir, mut store) = store();
    let (runs, _) = import_fixtures(&mut store);
    let count = |run: &str, rows| (run.to_string(), rows);
    let mut expected = [
        count("conclude-already-done-hand-built", Some(2)),
        count("conclude-broken-chain-hand-built", None),
        count("conclude-parked-hand-built", Some(2)),
        count("conclude-stopped-mid-effect-hand-built", Some(4)),
        count("reforging-the-road-back-hand-built", None),
        count("tui-graph-the-selection-box-gets-80f98deb", Some(6)),
    ];
    assert_eq!(counted(&store, &runs), expected);
    let path = store.path().to_path_buf();
    drop(store);
    let store = Store::open(&path).unwrap();
    expected[4].1 = Some(22);
    assert_eq!(counted(&store, &runs), expected);
}

/// A row that holds what no writer writes, a blob, a real, a word no
/// outcome is or an outcome its column's writer never writes, reads as
/// stale, never as a row.
#[test]
fn a_row_no_writer_writes_reads_as_stale() {
    let (_dir, mut store) = store();
    seated(&mut store, "r");
    for edit in [
        "UPDATE seat_summary SET seat = x'00' WHERE opened = 1",
        "UPDATE seat_summary SET seat = 'worker', turns = 1.5 WHERE opened = 1",
        "UPDATE seat_summary SET turns = 1, ended = 'done' WHERE opened = 1",
        "UPDATE seat_summary SET ended_seq = 6, ended = 'carried' WHERE opened = 1",
    ] {
        sql(&store, edit);
        assert_eq!(store.seat_rows("r").unwrap(), SeatRows::Stale, "{edit}");
    }
    lags(&mut store, &["r"]);
}

/// A row at a place its writer never gives, before the first, past a
/// gap, or not a number, a text not UTF-8 among them, reads as stale; and a cost no record states or
/// a sum no costs reach, a NaN, a turns' meter no turn writes, a session
/// meter that is a turn's or settles no output, or a meter's total its
/// input and output do not sum to, reads as stale, never as a row.
#[test]
fn a_row_at_a_place_cost_or_meter_no_writer_writes_reads_as_stale() {
    let finished = json!({"step": "exec-session-finished", "total_cost_usd": 1.0});
    let payload = json!({"effect_id": "fx", "attempt_id": "a", "checkpoint": finished});
    let nan = f64::NAN.to_bits() as i64;
    for edit in [
        "UPDATE seat_summary SET opened = -1 WHERE opened = 1".to_string(),
        "UPDATE seat_summary SET opened = 7 WHERE opened = 1".to_string(),
        "UPDATE seat_summary SET opened = 'x' WHERE opened = 1".to_string(),
        "UPDATE seat_summary SET opened = CAST(x'ff' AS TEXT) WHERE opened = 1".to_string(),
        format!("UPDATE seat_summary SET session_cost = {nan} WHERE opened = 1"),
        format!("UPDATE seat_summary SET spent = {nan} WHERE opened = 1"),
        "UPDATE seat_summary SET metered_turn = 0 WHERE opened = 1".to_string(),
        "UPDATE seat_summary SET metered_settles = 1 WHERE opened = 1".to_string(),
        "UPDATE seat_summary SET metered_input_tokens = NULL, metered_output_tokens = NULL,
             metered_cache_read_tokens = NULL, metered_cache_write_tokens = NULL,
             metered_reasoning_output_tokens = NULL, metered_total_tokens = NULL
             WHERE opened = 1"
            .to_string(),
        "UPDATE seat_summary SET metered_total_tokens = 999 WHERE opened = 1".to_string(),
        "UPDATE seat_summary SET metered_input_tokens = NULL, metered_output_tokens = NULL
             WHERE opened = 1"
            .to_string(),
        "UPDATE seat_summary SET session_total_tokens = 999 WHERE opened = 1".to_string(),
        "UPDATE seat_summary SET session_turn = 1 WHERE opened = 1".to_string(),
        "UPDATE seat_summary SET session_settles = 1 WHERE opened = 1".to_string(),
    ] {
        let (_dir, mut store) = store();
        seated(&mut store, "r");
        append(
            &mut store,
            "r",
            EventType::EffectCheckpointed,
            payload.clone(),
        );
        assert_eq!(read(&store, "r").len(), 2, "{edit}");
        sql(&store, &edit);
        assert_eq!(store.seat_rows("r").unwrap(), SeatRows::Stale, "{edit}");
        lags(&mut store, &["r"]);
    }
}

/// A row edited into a shape every guard of the read admits and no
/// writer writes, a boundary or a stamp no boundary is, a pin or a carry
/// on a seat's own row, an end on a member's, a start without its last,
/// a first checkpoint without its last, a session's cost with no sum
/// spent, reads as stale by its digest, and the doctor names its first
/// column that is not its fold's; so does a digest that is another row's,
/// not of its row, or of no shape a digest is, and a text, a column's or
/// the digest's, that is not UTF-8.
#[test]
fn a_row_its_digest_is_not_of_reads_as_stale_and_is_named() {
    let finished = json!({"step": "exec-session-finished", "total_cost_usd": 1.0});
    let payload = json!({"effect_id": "fx", "attempt_id": "a", "checkpoint": finished});
    let another = "digest = (SELECT digest FROM seat_summary AS o WHERE o.opened = 2)";
    for (row, set, named) in [
        (
            1,
            "boundary_seq = 4, boundary = 'not-a-boundary'",
            "boundary_seq",
        ),
        (1, "stamp_seq = 6, stamp = 'not-a-boundary'", "stamp_seq"),
        (1, "pin_seq = 6, pin = 'failed'", "pin_seq"),
        (
            1,
            "carried_seq = 4, carried_attempt_id = 'a'",
            "carried_seq",
        ),
        (2, "ended_seq = 6, ended = 'failed'", "ended_seq"),
        (1, "since = NULL", "since"),
        (1, "last_seq = NULL, last_at = NULL", "last_seq"),
        (1, "spent_seq = NULL, spent = NULL", "spent_seq"),
        (1, another, "digest"),
        (1, "digest = upper(digest)", "digest"),
        (1, "digest = x'00'", "digest"),
        (1, "effect_id = CAST(x'ff' AS TEXT)", "effect_id"),
        (1, "digest = CAST(x'ff' AS TEXT)", "digest"),
    ] {
        let (_dir, mut store) = store();
        seated(&mut store, "r");
        let member = checkpoint(Some("m"), 2);
        append(&mut store, "r", EventType::EffectCheckpointed, member);
        let session = payload.clone();
        append(&mut store, "r", EventType::EffectCheckpointed, session);
        assert_eq!(read(&store, "r").len(), 3, "{set}");
        sql(
            &store,
            &format!("UPDATE seat_summary SET {set} WHERE opened = {row}"),
        );
        assert_eq!(store.seat_rows("r").unwrap(), SeatRows::Stale, "{set}");
        let expected = format!("r: seat row {row} Column({named:?})");
        assert_eq!(every_run_reads_as_its_fold(&store), [expected], "{set}");
        lags(&mut store, &["r"]);
    }
}

/// A row's digest is of its run, its place and every column, each value
/// by its type and its length: any one of them changed changes it, and
/// no two rows share an encoding.
#[test]
fn a_row_s_digest_is_of_its_run_its_place_and_every_column() {
    let (_dir, mut store) = store();
    seated(&mut store, "r");
    let row = &folded_rows(&store, "r")[1];
    let data = row.data();
    let sealed = digest("r", 1, &data);
    let last = written("r", 1, row).pop();
    assert_eq!(last, Some((DIGEST, Value::Text(sealed.clone()))));
    assert_ne!(digest("s", 1, &data), sealed);
    assert_ne!(digest("r", 2, &data), sealed);
    for (at, (column, _)) in SeatRow::COLUMNS.iter().enumerate() {
        let mut changed = data.clone();
        changed[at] = match &changed[at] {
            Datum::Null => Datum::Integer(0),
            Datum::Integer(value) => Datum::Integer(value ^ 1),
            Datum::Text(value) => Datum::Text(format!("{value}.")),
        };
        assert_ne!(digest("r", 1, &changed), sealed, "{column}");
    }
    let pair = |a: &str, b: &str| [Datum::Text(a.into()), Datum::Text(b.into())];
    assert_ne!(
        digest("r", 1, &pair("ab", "c")),
        digest("r", 1, &pair("a", "bc"))
    );
    let zeros = "\0".repeat(8);
    let one = |datum: Datum| digest("r", 1, &[datum]);
    let typed = [
        one(Datum::Null),
        one(Datum::Integer(0)),
        one(Datum::Text(String::new())),
        one(Datum::Text(zeros)),
    ];
    let distinct: BTreeSet<&String> = typed.iter().collect();
    assert_eq!(distinct.len(), 4);
}

/// [`seated`]'s run with a second effect asked and started after it, and
/// then a member's checkpoint of the first: rows 0 and 1 the first
/// effect's, 2 and 3 the second's, and 4 the first's again.
fn interleaved(store: &mut Store, run_id: &str) {
    seated(store, run_id);
    let requested = json!({"effect_id": "fy", "seat": "worker", "phase": "work"});
    append(store, run_id, EventType::EffectRequested, requested);
    let started = json!({"effect_id": "fy", "attempt_id": "b"});
    append(store, run_id, EventType::EffectStarted, started);
    let member = checkpoint(Some("m"), 2);
    append(store, run_id, EventType::EffectCheckpointed, member);
}

/// The statements that swap the rows at places `a` and `b`.
fn swapped(a: usize, b: usize) -> String {
    format!(
        "UPDATE seat_summary SET opened = 99 WHERE opened = {a};
         UPDATE seat_summary SET opened = {a} WHERE opened = {b};
         UPDATE seat_summary SET opened = {b} WHERE opened = 99;"
    )
}

/// The statements that copy the row at place `from` to place 5.
fn copied(from: usize) -> String {
    format!(
        "CREATE TEMP TABLE copied AS SELECT * FROM seat_summary WHERE opened = {from};
         UPDATE copied SET opened = 5;
         INSERT INTO seat_summary SELECT * FROM copied;"
    )
}

/// Rows in an order the projection opens, a request's row after a later
/// request's, read as current; rows no projection opens read as stale: a
/// key twice, an attempt's row before its request's own, a request before
/// one asked earlier, an attempt's row of a request not yet asked, a row
/// naming another effect than its request's, and a request with no row
/// of its own.
#[test]
fn rows_out_of_the_order_the_projection_opens_read_as_stale() {
    let key: Vec<&str> = SeatRow::COLUMNS[..4]
        .iter()
        .map(|(name, _)| *name)
        .collect();
    assert_eq!(key, ["request_seq", "effect_id", "attempt_id", "member"]);
    let (_kept, mut kept) = store();
    interleaved(&mut kept, "r");
    let rows = read(&kept, "r");
    assert_eq!(rows, folded_rows(&kept, "r"));
    assert_eq!(rows.len(), 5);
    for edit in [
        copied(1),
        copied(0),
        swapped(0, 1),
        swapped(0, 2),
        swapped(2, 4),
        "UPDATE seat_summary SET effect_id = 'fy' WHERE opened = 4".to_string(),
        "DELETE FROM seat_summary WHERE opened = 2;
         UPDATE seat_summary SET opened = 2 WHERE opened = 3;
         UPDATE seat_summary SET opened = 3 WHERE opened = 4;"
            .to_string(),
    ] {
        let (_dir, mut store) = store();
        interleaved(&mut store, "r");
        sql(&store, &edit);
        assert_eq!(store.seat_rows("r").unwrap(), SeatRows::Stale, "{edit}");
        lags(&mut store, &["r"]);
    }
}

/// `run_id`'s folded rows planted by SQL at the places `order` gives, the
/// row at each place the folded row `order` names there, each sealed at
/// its place as a writer seals it ([`written`]).
fn resealed(store: &Store, run_id: &str, order: &[usize]) {
    let rows = folded_rows(store, run_id);
    let mut insert = store.conn.prepare(&INSERT).unwrap();
    for (opened, from) in order.iter().enumerate() {
        let key = [
            Value::Text(run_id.to_string()),
            Value::Integer(opened as i64),
        ];
        let values = written(run_id, opened, &rows[*from]);
        let values = values.into_iter().map(|(_, value)| value);
        insert
            .execute(params_from_iter(key.into_iter().chain(values)))
            .unwrap();
    }
}

/// Rows each sealed at its place, so only the order the projection opens
/// tells them from a writer's, read as current in that order and as stale
/// out of it: an attempt's row before its request's own, and an attempt's
/// row of a request asked before the request that stands before it.
#[test]
fn sealed_rows_out_of_the_order_the_projection_opens_read_as_stale() {
    let (_kept, mut kept) = store();
    interleaved(&mut kept, "r");
    resealed(&kept, "r", &[0, 1, 2, 3, 4]);
    assert_eq!(read(&kept, "r"), folded_rows(&kept, "r"));
    for order in [[1, 0, 2, 3, 4], [2, 1, 0, 3, 4]] {
        let (_dir, mut store) = store();
        interleaved(&mut store, "r");
        resealed(&store, "r", &order);
        assert_eq!(store.seat_rows("r").unwrap(), SeatRows::Stale, "{order:?}");
        lags(&mut store, &["r"]);
    }
}

/// A row whose key is another's reads as stale, though it is sealed at
/// its own place as a writer seals a row: the projection opens each key
/// once. The same rows sealed again at their places read as current.
#[test]
fn a_key_twice_sealed_at_its_own_place_reads_as_stale() {
    let (_dir, mut store) = store();
    interleaved(&mut store, "r");
    resealed(&store, "r", &[0, 1, 2, 3, 4]);
    assert_eq!(read(&store, "r"), folded_rows(&store, "r"));
    resealed(&store, "r", &[0, 1, 2, 3, 4, 1]);
    assert_eq!(store.seat_rows("r").unwrap(), SeatRows::Stale);
    lags(&mut store, &["r"]);
}

/// A journal that lost one table of the seat summary, or the rows'
/// digest an unreleased build of it made them without, loses both
/// tables at a writable open, so no seat head that survived vouches for
/// rows that did not, and the walk refolds every run; until then its rows
/// read as stale. A repair that finds both tables already made drops
/// nothing.
#[test]
fn a_journal_that_lost_a_seat_table_reads_stale_until_a_writable_open_refolds_it() {
    for lost in [
        "DROP TABLE seat_summary",
        "DROP TABLE seat_head",
        "ALTER TABLE seat_summary DROP COLUMN digest",
        "ALTER TABLE seat_head DROP COLUMN shape",
    ] {
        let (_dir, mut store) = store();
        seated(&mut store, "r");
        let path = store.path().to_path_buf();
        drop(store);
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch(lost).unwrap();
        let reader = Store::open_read_only(&path).unwrap();
        assert_eq!(reader.seat_rows("r").unwrap(), SeatRows::Stale, "{lost}");
        let store = Store::open(&path).unwrap();
        assert_eq!(read(&store, "r"), folded_rows(&store, "r"), "{lost}");
        assert_eq!(read(&store, "r").len(), 2, "{lost}");
        migrate_seats(&store.conn).unwrap();
        assert_eq!(read(&store, "r").len(), 2, "{lost}");
    }
}

/// A journal whose rows' table an unreleased build made without a column
/// slice 2b keeps reads stale, refusing its reader nothing, and a writable
/// open makes both tables anew and refolds every run, the column's facts
/// included: a turn's tool and target, a finish's time and outcome, a
/// session's id, a sequence's finished steps, a member's tag, the models
/// reported, the members tagged and the live lines opened before their
/// effect's request, and which rows are of the run's last start.
#[test]
fn a_seat_table_without_a_column_slice_2b_keeps_is_rebuilt_at_a_writable_open() {
    let unkept = [
        "finish_at",
        "finish_outcome",
        "session_id",
        "seat_turn_seq",
        "seat_turn",
        "seat_turn_tool",
        "seat_turn_target",
        "steps",
        "tagged_seq",
        "models",
        "early_tags",
        "early_lines",
        "live",
    ];
    for column in unkept {
        let (_dir, mut store) = store();
        seated(&mut store, "r");
        let turn = json!({"step": "seat-turn", "turn": 2, "tool": "Edit", "target": "a.rs"});
        let turned = json!({"effect_id": "fx", "attempt_id": "a", "checkpoint": turn});
        append(&mut store, "r", EventType::EffectCheckpointed, turned);
        let step = json!({"step": "sequence-step-finished", "step_name": "draft"});
        let stepped = json!({"effect_id": "fx", "attempt_id": "a", "checkpoint": step});
        append(&mut store, "r", EventType::EffectCheckpointed, stepped);
        let tagged = checkpoint(Some("b"), 1);
        append(&mut store, "r", EventType::EffectCheckpointed, tagged);
        let early = json!({"member": "c", "step": "seat-turn", "turn": 1, "tool": "Read"});
        let early = json!({"effect_id": "fy", "attempt_id": "z", "checkpoint": early});
        append(&mut store, "r", EventType::EffectCheckpointed, early);
        let asked = json!({"effect_id": "fy", "seat": "other"});
        append(&mut store, "r", EventType::EffectRequested, asked);
        let started = json!({"effect_id": "fy", "attempt_id": "z"});
        append(&mut store, "r", EventType::EffectStarted, started);
        let path = store.path().to_path_buf();
        drop(store);
        let raw = rusqlite::Connection::open(&path).unwrap();
        let dropped = format!("ALTER TABLE seat_summary DROP COLUMN {column}");
        raw.execute_batch(&dropped).unwrap();
        let reader = Store::open_read_only(&path).unwrap();
        assert!(!seats_intact(&reader.conn).unwrap(), "{column}");
        assert_eq!(reader.seat_rows("r").unwrap(), SeatRows::Stale, "{column}");
        let store = Store::open(&path).unwrap();
        let rows = store.seat_rows("r").unwrap();
        assert_eq!(
            rows,
            SeatRows::Current(folded_rows(&store, "r")),
            "{column}"
        );
        let read = read(&store, "r");
        let facts = brokkr_core::seat::row::facts(&read);
        let target = facts[0].target.as_deref();
        assert_eq!(target, Some("a.rs"), "{column}");
        let listed = brokkr_core::seat::row::inner(&read, "fx");
        let words = |word: &str| vec![word.to_string()];
        assert_eq!(listed, (words("draft"), words("b")), "{column}");
        let models = brokkr_core::seat::row::models(&read, 0, u64::MAX);
        assert_eq!(models.into_iter().collect::<Vec<_>>(), words("opus"));
        let turned = Some(("1".to_string(), "Read".to_string()));
        let live = brokkr_core::seat::row::live(&read);
        assert_eq!(live, [("?:c".to_string(), turned)], "{column}");
    }
}

/// A journal the seat view's build wrote, its rows' table without the
/// columns an effect's accounting is kept in (decision 0089 slice 3),
/// reads stale, refusing its reader nothing, and a writable open makes
/// both tables anew and refolds every run, its accounting included: the
/// rows read back cost as `seat_costs` gives the journal. A column of it
/// edited past its writer is named by the doctor.
#[test]
fn a_seat_table_without_the_accounting_s_columns_gains_them_at_a_writable_open() {
    let costs = SeatRow::COLUMNS.iter().map(|(name, _)| *name);
    let costs = costs.filter(|name| name.starts_with("costs_"));
    let unkept: String = costs
        .map(|column| format!("ALTER TABLE seat_summary DROP COLUMN {column};"))
        .collect();
    let (_dir, mut store) = store();
    seated(&mut store, "r");
    let path = store.path().to_path_buf();
    drop(store);
    rusqlite::Connection::open(&path)
        .and_then(|raw| raw.execute_batch(&unkept))
        .unwrap();
    let reader = Store::open_read_only(&path).unwrap();
    let count = "SELECT count(*) FROM pragma_table_info('seat_summary')";
    let columns: i64 = reader.conn.query_row(count, [], |row| row.get(0)).unwrap();
    // Its run and place, the seat view's 69 columns, the seats' six dated
    // ones, and the digest.
    assert_eq!(columns, 2 + 69 + 6 + 1);
    assert!(!seats_intact(&reader.conn).unwrap());
    assert_eq!(reader.seat_rows("r").unwrap(), SeatRows::Stale);
    let mut store = Store::open(&path).unwrap();
    let rows = read(&store, "r");
    assert_eq!(rows, folded_rows(&store, "r"));
    let (seats, total) = brokkr_core::seat::row::costs(&rows);
    assert_eq!(
        serde_json::to_value(&seats).unwrap(),
        json!({"worker": {"attempts": 1, "turns": 0, "cost_usd": 0.0, "model": "opus",
                          "boundary": "not recorded", "effort": "not reported",
                          "input_tokens": 10, "output_tokens": 1}})
    );
    let events = store.load("r").unwrap();
    assert_eq!((seats, total), brokkr_core::seat::seat_costs(&events));
    sql(
        &store,
        "UPDATE seat_summary SET costs_attempts = 9 WHERE run_id = 'r' AND opened = 0",
    );
    let found: Vec<String> = store
        .summary_disagreements()
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        found,
        [
            "run 'r': its seat rows stand at its head, and the costs_attempts of its seat row 0 \
          is not its fold's"
        ]
    );
    lags(&mut store, &["r"]);
}

/// `run_id`'s rows and seat head at its head as the seat view's build
/// writes them into a table that keeps the accounting's columns: its 69
/// columns and a digest of them, the accounting's left NULL, and a head
/// of the three columns it names.
fn written_by_the_seat_view_s_build(store: &Store, run_id: &str) {
    let columns = SeatRow::COLUMNS.iter().map(|(name, _)| *name);
    let older: Vec<&str> = columns
        .take_while(|name| !name.starts_with("costs_"))
        .collect();
    assert_eq!(older.len(), 69);
    let bound: Vec<String> = (1..=older.len() + 3).map(|at| format!("?{at}")).collect();
    let insert = format!(
        "INSERT OR REPLACE INTO seat_summary (run_id, opened, {}, digest) VALUES ({})",
        older.join(", "),
        bound.join(", ")
    );
    for (opened, row) in folded_rows(store, run_id).iter().enumerate() {
        let data = &row.data()[..older.len()];
        let sealed = digest(run_id, opened as i64, data);
        let key = [Value::Text(run_id.into()), Value::Integer(opened as i64)];
        let values = data.iter().cloned().map(value).chain([Value::Text(sealed)]);
        let all = params_from_iter(key.into_iter().chain(values));
        store.conn.execute(&insert, all).unwrap();
    }
    let (seq, hash) = store.head_hash(run_id).unwrap();
    store
        .conn
        .execute(
            "INSERT OR REPLACE INTO seat_head (run_id, folded_seq, folded_hash) VALUES (?1, ?2, ?3)",
            params![run_id, seq as i64, hash],
        )
        .unwrap();
}

/// Rows another build wrote at the run's head, the seat view's, which
/// names none of the accounting's columns and no shape of its head, lag
/// for this build as rows behind the head do: a shapeless head reads
/// stale over any rows, the doctor names the run behind, this build's
/// next append writes every row, and where the other build appended, the
/// next writable open refolds the run whole, its accounting what
/// `seat_costs` gives.
#[test]
fn rows_another_build_wrote_at_the_run_s_head_lag_until_this_build_writes_them() {
    let (_dir, mut store) = store();
    seated(&mut store, "r");
    sql(&store, "UPDATE seat_head SET shape = NULL");
    assert_eq!(store.seat_rows("r").unwrap(), SeatRows::Stale);
    written_by_the_seat_view_s_build(&store, "r");
    assert_eq!(store.seat_rows("r").unwrap(), SeatRows::Stale);
    let behind = ["r: seat rows behind its head"];
    assert_eq!(every_run_reads_as_its_fold(&store), behind);
    // A member's turn, which moves the request's own row and not its
    // attempt's: the append writes that one too.
    let turn = checkpoint(Some("m"), 2);
    append(&mut store, "r", EventType::EffectCheckpointed, turn);
    assert_eq!(read(&store, "r"), folded_rows(&store, "r"));
    let more = next(
        &store,
        "r",
        EventType::EffectCheckpointed,
        checkpoint(Some("m"), 3),
    );
    plant_summary_writer(&store, "r", &[more]);
    written_by_the_seat_view_s_build(&store, "r");
    let (seq, hash) = store.head_hash("r").unwrap();
    assert_eq!(folded(&store.conn, "r").unwrap(), Some((seq, hash, false)));
    assert_eq!(every_run_reads_as_its_fold(&store), behind);
    let path = store.path().to_path_buf();
    lags(&mut store, &["r"]);
    drop(store);
    let store = Store::open(&path).unwrap();
    let rows = read(&store, "r");
    assert_eq!(rows, folded_rows(&store, "r"));
    let events = store.load("r").unwrap();
    let costs = brokkr_core::seat::row::costs(&rows);
    assert_eq!(costs, brokkr_core::seat::seat_costs(&events));
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// Seat rows and a seat head of a run the journal does not hold read as
/// stale, whatever head they claim, and the doctor's sweep names each as
/// a stray; a run created under that id later reads none of them.
#[test]
fn seat_rows_and_heads_of_no_run_read_stale_and_are_named_as_strays() {
    let (_dir, mut store) = store();
    seated(&mut store, "r");
    sql(
        &store,
        &format!(
            "CREATE TEMP TABLE copied AS SELECT * FROM seat_summary WHERE run_id = 'r';
             UPDATE copied SET run_id = 'ghost';
             INSERT INTO seat_summary SELECT * FROM copied;
             INSERT INTO seat_head (run_id, folded_seq, folded_hash)
                 VALUES ('ghost', 0, '{ZERO}'), ('hollow', 0, '{ZERO}');"
        ),
    );
    assert_eq!(store.seat_rows("ghost").unwrap(), SeatRows::Stale);
    let found: Vec<String> = store
        .summary_disagreements()
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        found,
        [
            "run 'ghost': the seat summary holds a row at 0, where its fold holds none",
            "run 'ghost': the seat summary holds a seat head of it, where the journal holds no \
             such run",
            "run 'hollow': the seat summary holds a seat head of it, where the journal holds no \
             such run",
        ]
    );
    store.create_run("ghost", "f", "self", &json!({})).unwrap();
    assert_eq!(
        store.seat_rows("ghost").unwrap(),
        SeatRows::Current(Vec::new())
    );
}

/// The binding names a seat head of no run, which no run's comparison
/// gives, by its own words.
#[test]
fn the_binding_names_a_seat_head_of_no_run() {
    let head = Disagreement::SeatHead {
        run_id: "ghost".to_string(),
    };
    let named = crate::test_support::named("ghost", Compared::Disagrees(head));
    assert_eq!(named.as_deref(), Some("ghost: a seat head of no run"));
}

/// A reported cost of -0.0, which the seat-record fence refuses today and
/// a journal from before it may hold, is refolded and read back as -0.0,
/// bit for bit, as the fold gives it: SQLite keeps a whole-valued REAL as
/// an integer and would read it back as +0.0.
#[test]
fn a_negative_zero_cost_reads_back_bit_for_bit() {
    let (_dir, mut store) = store();
    seated(&mut store, "r");
    let finished = json!({"step": "exec-session-finished", "total_cost_usd": -0.0});
    let payload = json!({"effect_id": "fx", "attempt_id": "a", "checkpoint": finished});
    let unfenced = next(&store, "r", EventType::EffectCheckpointed, payload);
    plant_summary_writer(&store, "r", &[unfenced]);
    let path = store.path().to_path_buf();
    lags(&mut store, &["r"]);
    drop(store);
    let store = Store::open(&path).unwrap();
    let data = |rows: Vec<SeatRow>| rows.iter().map(SeatRow::data).collect::<Vec<_>>();
    let rows = read(&store, "r");
    assert_eq!(data(rows.clone()), data(folded_rows(&store, "r")));
    let costs: Vec<Option<u64>> = brokkr_core::seat::row::facts(&rows)
        .iter()
        .map(|facts| facts.last_attempt_cost.map(f64::to_bits))
        .collect();
    assert_eq!(costs, [Some((-0.0f64).to_bits())]);
}

/// A seat row that is not its fold's is named by its place and how: a
/// row missing, a column that differs, a row the fold holds none at, one
/// at a place below zero, of a fold with rows or with none, and of a run
/// the journal does not hold, at that place as it stands.
#[test]
fn seat_rows_that_part_from_the_fold_are_named_by_their_place_and_how() {
    let (_dir, mut store) = store();
    for run_id in ["missing", "edited", "stray", "under"] {
        seated(&mut store, run_id);
    }
    store.create_run("empty", "f", "self", &json!({})).unwrap();
    sql(
        &store,
        "INSERT INTO seat_summary (run_id, opened, request_seq, effect_id, digest)
             VALUES ('empty', -1, 1, 'fx', '');
         DELETE FROM seat_summary WHERE run_id = 'missing' AND opened = 1;
         UPDATE seat_summary SET model = 'other' WHERE run_id = 'edited' AND opened = 1;
         INSERT INTO seat_summary (run_id, opened, request_seq, effect_id, digest)
             SELECT 'stray', 2, request_seq, effect_id, digest FROM seat_summary
             WHERE run_id = 'stray' AND opened = 0;
         UPDATE seat_summary SET opened = -1 WHERE run_id = 'under' AND opened = 1;
         INSERT INTO seat_summary (run_id, opened, request_seq, effect_id, digest)
             VALUES ('ghost', -2, 1, 'fx', ''), ('ghost', 0, 1, 'fx', '');",
    );
    let found: Vec<String> = store
        .summary_disagreements()
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        found,
        [
            "run 'edited': its seat rows stand at its head, and the model of its seat row 1 is \
             not its fold's",
            "run 'empty': the seat summary holds a row at -1, where its fold holds none",
            "run 'missing': its fold's seat row 1 has no seat summary row",
            "run 'stray': the seat summary holds a row at 2, where its fold holds none",
            "run 'under': the seat summary holds a row at -1, where its fold holds none",
            "run 'ghost': the seat summary holds a row at -2, where its fold holds none",
        ]
    );
    for run_id in ["under", "empty"] {
        assert_eq!(
            store.seat_rows(run_id).unwrap(),
            SeatRows::Stale,
            "{run_id}"
        );
        let place = match store.compared(run_id, || {}).unwrap() {
            Compared::Disagrees(Disagreement::Seated { row, .. }) => row,
            other => panic!("{other:?}"),
        };
        assert_eq!(place, (-1i64) as u64, "{run_id}");
    }
    lags(
        &mut store,
        &["missing", "edited", "stray", "under", "empty"],
    );
}

// ------------------------------------------------- the seats' dated facts

/// A row's column by name, as its writer's data holds it.
fn column(row: &SeatRow, name: &str) -> Datum {
    let at = SeatRow::COLUMNS
        .iter()
        .position(|(column, _)| *column == name)
        .unwrap_or_else(|| panic!("{name} is a column"));
    row.data()[at].clone()
}

/// A row's section of text, where its data holds one.
fn held(row: &SeatRow, name: &str) -> Option<String> {
    match column(row, name) {
        Datum::Text(text) => Some(text),
        Datum::Null => None,
        Datum::Integer(value) => panic!("{name} is text, not {value}"),
    }
}

/// A row's integer column, where its data holds one.
fn numeric(row: &SeatRow, name: &str) -> Option<i64> {
    match column(row, name) {
        Datum::Integer(value) => Some(value),
        Datum::Null => None,
        Datum::Text(text) => panic!("{name} is an integer, not {text}"),
    }
}

/// The row of the attempt `attempt` naming `member`, `None` for the seat's
/// own.
fn attempted<'a>(rows: &'a [SeatRow], attempt: &str, member: Option<&str>) -> &'a SeatRow {
    let found = rows.iter().find(|row| {
        held(row, "attempt_id").as_deref() == Some(attempt)
            && held(row, "member").as_deref() == member
    });
    found.unwrap_or_else(|| panic!("attempt {attempt} names {member:?}"))
}

/// What the site a row states metering by: the rule `Metering` indexes.
fn rule(row: &SeatRow) -> i64 {
    numeric(row, "metering").expect("a row states a metering rule")
}

/// `name` cut to `bytes`, as a datum.
fn rule_of(metering: brokkr_core::seat::meter::Metering) -> i64 {
    match metering {
        brokkr_core::seat::meter::Metering::Session => 0,
        brokkr_core::seat::meter::Metering::Turns => 1,
        brokkr_core::seat::meter::Metering::Settled => 2,
    }
}

fn cut(name: Option<&str>, bytes: usize) -> Datum {
    name.map_or(Datum::Null, |name| {
        Datum::Text(
            brokkr_core::summary::Bounded::within(name, bytes)
                .cut
                .to_string(),
        )
    })
}

/// A run whose attempt `a` starts with the provider `provider`, after
/// entering `phase`, and whose seat is `worker`.
fn sited(store: &mut Store, run_id: &str, phase: &str, provider: &str) {
    store
        .create_run(run_id, "a feature", "self", &json!({}))
        .unwrap();
    append(
        store,
        run_id,
        EventType::RunStarted,
        json!({"feature": "a feature", "manifest": {}}),
    );
    append(
        store,
        run_id,
        EventType::PhaseEntered,
        json!({"phase": phase}),
    );
    append(
        store,
        run_id,
        EventType::EffectRequested,
        json!({"effect_id": "fx", "seat": "worker", "phase": phase}),
    );
    append(
        store,
        run_id,
        EventType::EffectStarted,
        json!({"effect_id": "fx", "attempt_id": "a",
               "provenance": [{"member": null, "provider": provider}]}),
    );
}

/// A run whose site `a` starts under the provider `anthropic` and meters
/// one `opus` turn.
fn sited_turn(store: &mut Store, run_id: &str) {
    sited(store, run_id, "work", "anthropic");
    append(
        store,
        run_id,
        EventType::EffectCheckpointed,
        json!({"effect_id": "fx", "attempt_id": "a",
               "checkpoint": {"step": "seat-turn", "model": "opus", "input_tokens": 1}}),
    );
}

/// A run whose attempt `a` starts before its effect is asked, as main's
/// walk reads a start before its request: the request opens the site the
/// start named, and the attempt's checkpoint and failure follow it.
fn early_start(store: &mut Store, run_id: &str) {
    opened(store, run_id, "work", &[]);
    append(
        store,
        run_id,
        EventType::EffectStarted,
        json!({"effect_id": "fx", "attempt_id": "a",
               "provenance": [{"member": null, "provider": "anthropic"}]}),
    );
    append(
        store,
        run_id,
        EventType::EffectRequested,
        json!({"effect_id": "fx", "seat": "worker", "phase": "work"}),
    );
    append(
        store,
        run_id,
        EventType::EffectCheckpointed,
        checkpoint(None, 1),
    );
    append(
        store,
        run_id,
        EventType::EffectFailed,
        json!({"effect_id": "fx", "attempt_id": "a"}),
    );
}

/// `run_id`'s effect `fx` requested, its seat the worker's.
fn requested(store: &mut Store, run_id: &str) {
    append(
        store,
        run_id,
        EventType::EffectRequested,
        json!({"effect_id": "fx", "seat": "worker", "phase": "work"}),
    );
}

/// `run_id`'s events read whole, and the sites they read, with the rows
/// the store keeps of them.
fn sites_and_rows(store: &Store, run_id: &str) -> (Vec<brokkr_core::summary::Site>, Vec<SeatRow>) {
    let events = store.load(run_id).unwrap();
    (brokkr_core::summary::sites(&events), read(store, run_id))
}

/// The rows among `rows` that a site opened.
fn sited_rows(rows: &[SeatRow]) -> Vec<&SeatRow> {
    rows.iter()
        .filter(|row| numeric(row, "site").is_some())
        .collect()
}

/// Append a checkpoint of attempt `a` recording `record`.
fn recorded(store: &mut Store, run_id: &str, record: Json) {
    append(
        store,
        run_id,
        EventType::EffectCheckpointed,
        json!({"effect_id": "fx", "attempt_id": "a", "checkpoint": record}),
    );
}

/// A run whose attempt `a` starts under `anthropic`, meters a checkpoint,
/// fails, and is started again naming no provider: the participant's
/// current facts move to the restart while the earlier site keeps its own
/// dated facts.
fn providerless_restart(store: &mut Store, run_id: &str) {
    opened(store, run_id, "work", &[]);
    requested(store, run_id);
    started(store, run_id, "a", None, "anthropic");
    append(
        store,
        run_id,
        EventType::EffectCheckpointed,
        checkpoint(None, 1),
    );
    failed(store, run_id, "a");
    append(
        store,
        run_id,
        EventType::EffectStarted,
        json!({"effect_id": "fx", "attempt_id": "a",
               "provenance": [{"member": null, "provider": null}]}),
    );
}

/// A run whose attempt `a`/`m` finishes a session, fails, and starts again
/// to finish another: main keys the session flag by the participant, so the
/// earlier record is not its session when the later one lands.
fn reopened_sessions(store: &mut Store, run_id: &str) {
    opened(store, run_id, "work", &[]);
    requested(store, run_id);
    started(store, run_id, "a", Some("m"), "anthropic");
    recorded(
        store,
        run_id,
        json!({"member": "m", "step": "seat-turn", "input_tokens": 5}),
    );
    recorded(
        store,
        run_id,
        json!({"member": "m", "step": "claude-session-finished", "output_tokens": 100}),
    );
    failed(store, run_id, "a");
    started(store, run_id, "a", Some("m"), "anthropic");
    recorded(
        store,
        run_id,
        json!({"member": "m", "step": "claude-session-finished", "output_tokens": 200}),
    );
}

/// The chief's cases, read as journals beside the fixtures: a start before
/// its request, a providerless restart, and a participant's second session.
fn chief_fixtures(store: &mut Store) -> Vec<(String, Vec<EventEnvelope>)> {
    type Build = fn(&mut Store, &str);
    let cases: [(&str, Build); 3] = [
        ("early-start", early_start),
        ("providerless-restart", providerless_restart),
        ("reopened-sessions", reopened_sessions),
    ];
    cases
        .into_iter()
        .map(|(run_id, build)| {
            build(store, run_id);
            (run_id.to_string(), store.load(run_id).unwrap())
        })
        .collect()
}

#[test]
fn every_fixture_keeps_the_seat_facts_the_run_reads() {
    let (_dir, mut store) = store();
    let mut fixtures = planted_fixtures(&store);
    fixtures.extend(chief_fixtures(&mut store));
    let store = refolded(store);
    let mut wrong = Vec::new();
    for (run_id, events) in fixtures {
        // A run whose chain does not load folds nowhere, so its rows read
        // stale and a reader reads it whole (decision 0089 ruling 8).
        if store.load(&run_id).is_err() {
            continue;
        }
        let account = Account::whole(&run_id, &events);
        let sites = brokkr_core::summary::sites(&events);
        let rows = match store.seat_rows(&run_id).unwrap() {
            SeatRows::Current(rows) => rows,
            SeatRows::Stale => {
                wrong.push(format!("{run_id}: seat rows read stale"));
                continue;
            }
        };
        let mut matched = vec![false; sites.len()];
        for row in &rows {
            let Some(_) = numeric(row, "site") else {
                continue;
            };
            // A site's own provider is its key: its whole provider may
            // have moved to none when its participant restarted.
            let provider = held(row, "provider_key").expect("a site's row states its provider key");
            let attempt = held(row, "attempt_id").expect("a site's row states its attempt");
            let member = held(row, "member");
            let found = sites.iter().enumerate().position(|(at, site)| {
                !matched[at]
                    && site.attempt_id == attempt
                    && site.member.as_deref() == member.as_deref()
            });
            let Some(at) = found else {
                wrong.push(format!(
                    "{run_id}: row {attempt}/{member:?} names provider {provider}, no site"
                ));
                continue;
            };
            matched[at] = true;
            let site = &sites[at];
            let want = [
                ("started_ms", Datum::Integer(site.at)),
                (
                    "phase_key",
                    cut(site.phase.as_deref(), brokkr_core::summary::NAME_BYTES),
                ),
                (
                    "provider_key",
                    cut(Some(&site.provider), brokkr_core::summary::NAME_BYTES),
                ),
                (
                    "model_key",
                    cut(site.model.as_deref(), brokkr_core::summary::MODEL_BYTES),
                ),
                ("metering", Datum::Integer(rule_of(site.metering))),
            ];
            for (name, want) in want {
                let got = column(row, name);
                if got != want {
                    wrong.push(format!(
                        "{run_id}: row {attempt}/{member:?} {name} {got:?}, read {want:?}"
                    ));
                }
            }
        }
        for (at, site) in sites.iter().enumerate() {
            if !matched[at] {
                wrong.push(format!(
                    "{run_id}: site {}/{:?} has no seat row",
                    site.attempt_id, site.member
                ));
            }
        }
        assert_eq!(rows, account.seats().to_vec(), "{run_id}");
    }
    assert_eq!(wrong, Vec::<String>::new());
}

/// A start before its request keeps its site row: the request it waited for
/// opens the site it named, as main's walk reads it, and the row is core's
/// own reading of the run.
#[test]
fn an_early_start_keeps_its_site_row() {
    let (_dir, mut store) = store();
    early_start(&mut store, "r");
    let (sites, rows) = sites_and_rows(&store, "r");
    let sited = sited_rows(&rows);
    assert_eq!(sites.len(), 1);
    assert_eq!(sited.len(), 1);
    assert_eq!(numeric(sited[0], "started_ms"), Some(sites[0].at));
    assert_eq!(
        held(sited[0], "provider_key").as_deref(),
        Some(sites[0].provider.as_str())
    );
    assert_eq!(
        rows,
        Account::whole("r", &store.load("r").unwrap())
            .seats()
            .to_vec()
    );
}

/// A providerless restart reports the participant at work, as main does: the
/// restart's seq is the attempt's current start, though no site opens and
/// the earlier site keeps its own provider key.
#[test]
fn a_providerless_restart_reports_working() {
    let (_dir, mut store) = store();
    providerless_restart(&mut store, "r");
    let rows = read(&store, "r");
    assert_eq!(facts(&rows)[0].outcome, Outcome::Working);
}

/// A providerless restart states the participant's provider as none, as
/// main does, while the earlier site's own provider key stays what its start
/// set.
#[test]
fn a_providerless_restart_clears_the_provider() {
    let (_dir, mut store) = store();
    providerless_restart(&mut store, "r");
    let rows = read(&store, "r");
    assert_eq!(facts(&rows)[0].provider, Some(Provider::Unnamed));
    let site = sited_rows(&rows)[0];
    assert_eq!(held(site, "provider_key").as_deref(), Some("anthropic"));
}

/// A site's start, phase, provider and model are written as their columns
/// state them, each name cut to its bound at a character boundary where it
/// is stored, the whole name last in the row.
#[test]
fn a_site_s_names_are_cut_at_their_bounds_where_they_are_stored() {
    let (_dir, mut store) = store();
    let phase = format!("{}é{}", "p".repeat(63), "p".repeat(1024 * 1024));
    let provider = format!("{}é{}", "q".repeat(63), "q".repeat(1024 * 1024));
    let model = format!("{}é{}", "m".repeat(79), "m".repeat(200));
    sited(&mut store, "r", &phase, &provider);
    // The seat record's fence refuses a model past its `maxLength`, so the
    // record is planted as an older writer leaves it and the open refolds
    // the row from it.
    let event = next(
        &store,
        "r",
        EventType::EffectCheckpointed,
        json!({"effect_id": "fx", "attempt_id": "a",
               "checkpoint": {"step": "seat-turn", "model": model,
                              "input_tokens": 1}}),
    );
    let store = planted_and_refolded(store, "r", &event);
    let rows = read(&store, "r");
    let row = attempted(&rows, "a", None);
    assert_eq!(held(row, "phase").as_deref(), Some(phase.as_str()));
    assert_eq!(held(row, "provider").as_deref(), Some(provider.as_str()));
    assert_eq!(held(row, "model").as_deref(), Some(model.as_str()));
    assert_eq!(held(row, "phase_key").as_deref(), Some(&phase[..63]));
    assert_eq!(held(row, "provider_key").as_deref(), Some(&provider[..63]));
    assert_eq!(held(row, "model_key").as_deref(), Some(&model[..79]));
    let events = store.load("r").unwrap();
    assert_eq!(
        numeric(row, "started_ms"),
        Some(brokkr_core::summary::sites(&events)[0].at)
    );
}

/// A start whose stamp does not parse is no site in brokkr-core's reading,
/// so it opens no site row and states no provider: no row is selected for a
/// site the reading never opened (decision 0089 rulings 4 and 7).
#[test]
fn an_unreadable_start_selects_no_site() {
    let (_dir, mut store) = store();
    opened(
        &mut store,
        "r",
        "implement",
        &[(
            EventType::EffectRequested,
            json!({"effect_id": "fx", "seat": "worker", "phase": "implement"}),
        )],
    );
    let unreadable = unreadable(
        &store,
        "r",
        EventType::EffectStarted,
        json!({"effect_id": "fx", "attempt_id": "a",
               "provenance": [{"member": null, "provider": "anthropic"}]}),
    );
    let store = planted_and_refolded(store, "r", &unreadable);
    let rows = read(&store, "r");
    let selected: Vec<&SeatRow> = rows
        .iter()
        .filter(|row| held(row, "provider").is_some())
        .collect();
    assert_eq!(selected, Vec::<&SeatRow>::new());
    let events = store.load("r").unwrap();
    assert_eq!(brokkr_core::summary::sites(&events), []);
    assert_eq!(disagreements(&store), Vec::<String>::new());
}

/// A checkpoint whose stamp does not parse is no record in brokkr-core's
/// reading, so it moves no site's model key while the whole participant
/// model keeps the last model any record names (decision 0089 rulings 4
/// and 7).
#[test]
fn an_unreadable_checkpoint_does_not_move_the_site_model_key() {
    let (_dir, mut store) = store();
    sited_turn(&mut store, "r");
    let unreadable = unreadable(
        &store,
        "r",
        EventType::EffectCheckpointed,
        json!({"effect_id": "fx", "attempt_id": "a",
               "checkpoint": {"step": "seat-turn", "model": "sonnet",
                              "input_tokens": 2}}),
    );
    let store = planted_and_refolded(store, "r", &unreadable);
    let rows = read(&store, "r");
    let row = attempted(&rows, "a", None);
    assert_eq!(held(row, "model").as_deref(), Some("sonnet"));
    assert_eq!(held(row, "model_key").as_deref(), Some("opus"));
    let events = store.load("r").unwrap();
    assert_eq!(
        brokkr_core::summary::sites(&events)[0].model.as_deref(),
        Some("opus")
    );
    assert_eq!(disagreements(&store), Vec::<String>::new());
}

/// A site's metering rule is the rule over all its checkpoints, and the
/// row is rewritten as each checkpoint moves it: its session record alone,
/// then its turns once a turn carries tokens, then settled once a settling
/// session record lands.
#[test]
fn a_site_s_metering_rule_moves_and_its_row_follows() {
    let (_dir, mut store) = store();
    sited(&mut store, "r", "work", "anthropic");
    let read_rule = |store: &Store| {
        let rows = read(store, "r");
        let row = attempted(&rows, "a", None);
        let account = Account::whole("r", &store.load("r").unwrap());
        let site = &account.sites()[0];
        (rule(row), rule_of(site.metering))
    };
    assert_eq!(read_rule(&store), (0, 0), "a session record alone");
    append(
        &mut store,
        "r",
        EventType::EffectCheckpointed,
        json!({"effect_id": "fx", "attempt_id": "a",
               "checkpoint": {"step": "seat-turn", "input_tokens": 10}}),
    );
    assert_eq!(read_rule(&store), (1, 1), "a turn carries tokens");
    append(
        &mut store,
        "r",
        EventType::EffectCheckpointed,
        json!({"effect_id": "fx", "attempt_id": "a",
               "checkpoint": {"step": "claude-session-finished", "output_tokens": 3}}),
    );
    assert_eq!(read_rule(&store), (2, 2), "a settling session record lands");
}

/// Ruling 4's comparison holds every seat column the dated facts added to
/// the fold's row: an edit of one is named by that column, never passed.
#[test]
fn each_seat_dated_column_that_is_not_its_fold_s_is_named() {
    let (_dir, mut store) = store();
    sited_turn(&mut store, "r");
    for name in [
        "started_ms",
        "phase_key",
        "provider_key",
        "model_key",
        "metering",
        "site",
    ] {
        let held = {
            let rows = read(&store, "r");
            column(attempted(&rows, "a", None), name)
        };
        let wrong = match &held {
            Datum::Integer(value) => Datum::Integer(value ^ 1),
            Datum::Text(text) => Datum::Text(format!("{text}.")),
            Datum::Null => Datum::Integer(0),
        };
        let edit = format!("UPDATE seat_summary SET {name} = ?1 WHERE run_id = 'r' AND opened = 1");
        store
            .conn
            .execute(&edit, [super::seal::value(wrong)])
            .unwrap();
        assert_eq!(
            disagreements(&store),
            [format!(
                "run 'r': its seat rows stand at its head, and the {name} of its seat row 1 \
                 is not its fold's"
            )],
            "{name}"
        );
        store
            .conn
            .execute(&edit, [super::seal::value(held)])
            .unwrap();
    }
    lags(&mut store, &["r"]);
}

/// A row whose dated column is not the one its fields derive reads as no
/// row: a start of another type or value, a key another name, and a rule
/// no [`Metering`] states.
#[test]
fn dated_seat_facts_no_row_writes_read_as_none() {
    let (_dir, mut store) = store();
    sited_turn(&mut store, "r");
    let rows = read(&store, "r");
    let row = attempted(&rows, "a", None);
    let at = |name: &str| {
        SeatRow::COLUMNS
            .iter()
            .position(|(column, _)| *column == name)
            .unwrap()
    };
    let cases = [
        ("started_ms", Datum::Text("1".into())),
        ("started_ms", Datum::Integer(0)),
        ("phase_key", Datum::Text("other".into())),
        ("phase_key", Datum::Null),
        ("provider_key", Datum::Integer(1)),
        ("metering", Datum::Integer(3)),
        ("metering", Datum::Text("turns".into())),
        ("site", Datum::Text("1".into())),
    ];
    for (name, wrong) in cases {
        let mut data = row.data();
        data[at(name)] = wrong.clone();
        assert!(SeatRow::of_data(data).is_none(), "{name} {wrong:?}");
    }
    // A site's provider key is its own start's, and its model key its site
    // reading's, which no row field holds once the participant moves, so
    // each is kept as the writer read it and a text edit is the digest's to
    // name; a row no site opened still checks its key against its whole
    // provider and model.
    let mut data = rows[0].data();
    data[at("model_key")] = Datum::Text("other".into());
    assert!(SeatRow::of_data(data).is_none(), "model_key");
    let mut data = row.data();
    data[at("model_key")] = Datum::Integer(1);
    assert!(SeatRow::of_data(data).is_none(), "model_key type");
}

/// A retry attempt's first turn that repeats the previous attempt's last
/// usage is its own site's, so it meters and its row states the site's
/// rule (decision 0089 rulings 4 and 7): the participant's last turn does
/// not swallow it.
#[test]
fn a_retry_s_first_turn_meters_its_own_site() {
    let (_dir, mut store) = store();
    sited(&mut store, "r", "work", "anthropic");
    append(
        &mut store,
        "r",
        EventType::EffectCheckpointed,
        json!({"effect_id": "fx", "attempt_id": "a",
               "checkpoint": {"step": "seat-turn", "input_tokens": 10}}),
    );
    failed(&mut store, "r", "a");
    started(&mut store, "r", "b", None, "anthropic");
    append(
        &mut store,
        "r",
        EventType::EffectCheckpointed,
        json!({"effect_id": "fx", "attempt_id": "b",
               "checkpoint": {"step": "seat-turn", "input_tokens": 10}}),
    );
    let rows = read(&store, "r");
    assert_eq!(numeric(attempted(&rows, "b", None), "metering"), Some(1));
    let account = Account::whole("r", &store.load("r").unwrap());
    assert_eq!(
        account
            .sites()
            .iter()
            .map(|site| rule_of(site.metering))
            .collect::<Vec<_>>(),
        [1, 1]
    );
}

/// A `session-started` after a settling finishing record leaves the
/// site's rule settled: the site's session record is its last finishing
/// one, as brokkr-core reads it, and the row states its rule.
#[test]
fn a_session_started_does_not_undo_its_site_s_settlement() {
    let (_dir, mut store) = store();
    sited_turn(&mut store, "r");
    append(
        &mut store,
        "r",
        EventType::EffectCheckpointed,
        json!({"effect_id": "fx", "attempt_id": "a",
               "checkpoint": {"step": "claude-session-finished", "output_tokens": 3}}),
    );
    append(
        &mut store,
        "r",
        EventType::EffectCheckpointed,
        json!({"effect_id": "fx", "attempt_id": "a",
               "checkpoint": {"step": "session-started", "session_id": "s"}}),
    );
    let rows = read(&store, "r");
    let account = Account::whole("r", &store.load("r").unwrap());
    assert_eq!(rule_of(account.sites()[0].metering), 2);
    assert_eq!(numeric(attempted(&rows, "a", None), "metering"), Some(2));
}

/// A start naming an attempt already started opens another site, and its
/// row: the first site's row keeps its own start, phase, provider and
/// rule, and the second's are the re-open's, one row per site.
#[test]
fn a_reopened_attempt_has_a_row_per_site() {
    let (_dir, mut store) = store();
    sited(&mut store, "r", "work", "anthropic");
    failed(&mut store, "r", "a");
    append(
        &mut store,
        "r",
        EventType::PhaseEntered,
        json!({"phase": "check"}),
    );
    started(&mut store, "r", "a", None, "openai");
    let (sites, rows) = sites_and_rows(&store, "r");
    let sited = sited_rows(&rows);
    assert_eq!(sites.len(), 2);
    assert_eq!(sited.len(), 2);
    for (row, site) in sited.iter().zip(&sites) {
        assert_eq!(numeric(row, "started_ms"), Some(site.at));
        assert_eq!(held(row, "phase_key").as_deref(), site.phase.as_deref());
        assert_eq!(
            held(row, "provider_key").as_deref(),
            Some(site.provider.as_str())
        );
        assert_eq!(numeric(row, "metering"), Some(rule_of(site.metering)));
    }
}
