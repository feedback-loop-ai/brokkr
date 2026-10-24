//! The seat summary (decision 0089, slice 2): an append writes the seat
//! rows it moved and the seat head in its own transaction, a writer that
//! hands over no fold writes none, a writer from before the table leaves
//! the rows stale and a writable open refolds them, a backfill cut short
//! resumes at the next run, and a run's rows are read only at its head.

use super::*;
use crate::test_support::{
    every_run_reads_as_its_fold, import_fixtures, lags, plant_summary_writer, unseat,
};
use crate::tests::envelope_builder::EnvelopeBuilder;
use crate::tests::walk::cut_short;
use crate::tests::{store, write_lock_on};
use brokkr_core::envelope::{EventEnvelope, EventType};
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
    folded(&store.conn, run_id).unwrap()
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
    sql(
        &store,
        "UPDATE seat_summary SET seat = 'edited' WHERE opened = 0",
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
        ["r: seat row 0 Column(\"seat\")"]
    );
    sql(
        &store,
        "UPDATE seat_summary SET seat = 'worker' WHERE opened = 0",
    );
    assert_eq!(read(&store, "r"), folded_rows(&store, "r"));
    assert_eq!(read(&store, "r").len(), 3);
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
/// session's id.
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
    ];
    for column in unkept {
        let (_dir, mut store) = store();
        seated(&mut store, "r");
        let turn = json!({"step": "seat-turn", "turn": 2, "tool": "Edit", "target": "a.rs"});
        let turned = json!({"effect_id": "fx", "attempt_id": "a", "checkpoint": turn});
        append(&mut store, "r", EventType::EffectCheckpointed, turned);
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
        let facts = brokkr_core::seat::row::facts(&read(&store, "r"));
        let target = facts[0].target.as_deref();
        assert_eq!(target, Some("a.rs"), "{column}");
    }
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
             INSERT INTO seat_head VALUES ('ghost', 0, '{ZERO}'), ('hollow', 0, '{ZERO}');"
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
