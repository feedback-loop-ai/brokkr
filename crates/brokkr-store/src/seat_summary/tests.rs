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
/// keeps what the table held, here an edit the binding then names.
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
    let rows = read(&store, "r");
    assert_eq!(rows[1..], folded_rows(&store, "r")[1..]);
    assert_eq!(rows.len(), 3);
    assert_eq!(
        every_run_reads_as_its_fold(&store),
        ["r: seat row 0 Column(\"seat\")"]
    );
    lags(&mut store, &["r"]);
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

/// A journal that lost one table of the seat summary loses the other with
/// it at a writable open, so no seat head that survived vouches for rows
/// that did not, and the walk refolds every run; until then its rows read
/// as stale. A repair that finds both tables already made drops nothing.
#[test]
fn a_journal_that_lost_a_seat_table_reads_stale_until_a_writable_open_refolds_it() {
    for lost in ["seat_summary", "seat_head"] {
        let (_dir, mut store) = store();
        seated(&mut store, "r");
        let path = store.path().to_path_buf();
        drop(store);
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch(&format!("DROP TABLE {lost}")).unwrap();
        let reader = Store::open_read_only(&path).unwrap();
        assert_eq!(reader.seat_rows("r").unwrap(), SeatRows::Stale, "{lost}");
        let store = Store::open(&path).unwrap();
        assert_eq!(read(&store, "r"), folded_rows(&store, "r"), "{lost}");
        assert_eq!(read(&store, "r").len(), 2, "{lost}");
        migrate_seats(&store.conn).unwrap();
        assert_eq!(read(&store, "r").len(), 2, "{lost}");
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
/// row missing, a column that differs, a row the fold holds none at.
#[test]
fn seat_rows_that_part_from_the_fold_are_named_by_their_place_and_how() {
    let (_dir, mut store) = store();
    for run_id in ["missing", "edited", "stray"] {
        seated(&mut store, run_id);
    }
    sql(
        &store,
        "DELETE FROM seat_summary WHERE run_id = 'missing' AND opened = 1;
         UPDATE seat_summary SET model = 'other' WHERE run_id = 'edited' AND opened = 1;
         INSERT INTO seat_summary (run_id, opened, request_seq, effect_id)
             SELECT 'stray', 2, request_seq, effect_id FROM seat_summary
             WHERE run_id = 'stray' AND opened = 0;",
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
            "run 'missing': its fold's seat row 1 has no seat summary row",
            "run 'stray': the seat summary holds a row at 2, where its fold holds none",
        ]
    );
    lags(&mut store, &["missing", "edited", "stray"]);
}
