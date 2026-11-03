//! Rows planted past every writer the store keeps, for tests (#354).
//!
//! A test that needs a journal the store would never write — a broken
//! chain, a record from before a fence, a row an older binary left —
//! plants it here rather than in its own SQL, so the statements a planted
//! row goes through are the ones the store's own writers use. The `runs`
//! and `events` tables admit anything the schema's guards do; nothing
//! here chains, seals or validates.
//!
//! Compiled for this crate's own tests and behind the `test-support`
//! feature, which only dev-dependencies enable.

use std::collections::BTreeSet;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::summary::{Account, Summary};
use rusqlite::types::Value;
use rusqlite::{params, Connection, ToSql};

use crate::import::INSERT_EVENT;
use crate::summary::{Compared, Listed};
use crate::{Disagreement, IndexFault, Store};

mod generated;

pub use generated::{fleet, generate, Generated, Standing, Width};

/// Every run's summary row and seat rows stand at its head and read as its
/// fold (decision 0089 ruling 4): each run folded whole, and its rows
/// compared with the projections of that fold, column for column.
/// Stricter than `brokkr doctor`, which passes a lag: a test's writers
/// each hand over their fold, so here a row or seat rows behind its head
/// are named, as is a run with no row, a run that does not load, and a
/// row's first differing column.
/// A journal the comparison cannot read panics, naming why.
pub fn every_run_reads_as_its_fold(store: &Store) -> Vec<String> {
    let mut found = Vec::new();
    let compared =
        store.each_run_compared(|run_id, compared| found.extend(named(run_id, compared)));
    read(compared);
    found
}

/// What `compared` names `run_id` by, `run: what its row reads as`, or
/// `None` where the row reads as its fold. A seat head of no run is never
/// a run's comparison, only the doctor's sweep's, and is named for the
/// match to be whole.
pub(crate) fn named(run_id: &str, compared: Compared) -> Option<String> {
    let words = match compared {
        Compared::Agrees => return None,
        Compared::Behind => "behind its head".to_string(),
        Compared::SeatsBehind => "seat rows behind its head".to_string(),
        Compared::Disagrees(Disagreement::Missing { .. }) => "no summary row".to_string(),
        Compared::Disagrees(Disagreement::Column { column, .. }) => column.to_string(),
        Compared::Disagrees(Disagreement::Unread { error, .. }) => {
            format!("does not load: {error}")
        }
        Compared::Disagrees(Disagreement::Indexed { seq, fault, .. }) => {
            format!("event {seq} indexed {fault:?}")
        }
        Compared::Disagrees(Disagreement::Seated { row, fault, .. }) => {
            format!("seat row {row} {fault:?}")
        }
        Compared::Disagrees(Disagreement::SeatHead { .. }) => "a seat head of no run".to_string(),
    };
    Some(format!("{run_id}: {words}"))
}

/// What the comparison read, or a panic naming why it could not.
fn read<T>(compared: Result<T, crate::StoreError>) -> T {
    compared.unwrap_or_else(|error| panic!("the summary comparison reads: {error}"))
}

/// What a store is held to when it drops: whether it was ever handed a
/// writer's fold, the runs it wrote, by creating or appending, and those
/// its test names as lagging.
#[derive(Debug, Default)]
pub(crate) struct Binding {
    carried: bool,
    wrote: BTreeSet<String>,
    lagging: BTreeSet<String>,
}

impl Binding {
    /// Record that the store was handed a writer's fold.
    pub(crate) fn carried(&mut self) {
        self.carried = true;
    }

    /// Record that the store wrote `run_id`.
    pub(crate) fn wrote(&mut self, run_id: &str) {
        self.wrote.insert(run_id.to_string());
    }

    /// Record that an append of `landed` events to `run_id` committed: a
    /// write when any landed, and none when a chain landed nothing, which
    /// leaves its run as it found it.
    pub(crate) fn appended(&mut self, run_id: &str, landed: usize) {
        if landed > 0 {
            self.wrote(run_id);
        }
    }
}

/// Name `run_ids` as runs this test leaves behind their heads, or not
/// loading, on purpose: the drop check passes them, and holds every
/// other run the store wrote to [`every_run_reads_as_its_fold`].
pub fn lags(store: &mut Store, run_ids: &[&str]) {
    let named = run_ids.iter().map(|run_id| run_id.to_string());
    store.binding.lagging.extend(named);
}

/// The lane `run_id`'s summary row is read from (decision 0089 ruling 6),
/// or `None` for a run with no row: how a test sees that a writer left the
/// row at its head rather than behind it.
pub fn lane(store: &Store, run_id: &str) -> Option<String> {
    store
        .conn
        .query_row(
            "SELECT lane FROM run_summary WHERE run_id = ?1",
            [run_id],
            |row| row.get(0),
        )
        .ok()
}

/// After every test that wrote through a store a writer's fold was ever
/// handed to, whether or not it still carries it, an engine scenario or
/// an operator verb among them, every run that store wrote stands at its
/// head and reads as its fold, as [`every_run_reads_as_its_fold`] reads
/// it, but the runs the test named by [`lags`]: a write the writer did
/// not hand its fold to leaves its run behind, and is named. A store no
/// fold was handed to writes as a binary from before the summary does,
/// and is not held. Not while a test is already failing: its own panic
/// is the one to report.
impl Drop for Store {
    fn drop(&mut self) {
        if !self.binding.carried || std::thread::panicking() {
            return;
        }
        let binding = &self.binding;
        let checked = binding.wrote.difference(&binding.lagging);
        let disagreeing: Vec<String> = checked
            .filter_map(|run_id| named(run_id, read(self.compared(run_id, || {}))))
            .collect();
        assert!(
            disagreeing.is_empty(),
            "a summary row disagrees with its run's fold: {disagreeing:?}"
        );
    }
}

/// `event_type` with `payload` appended by this store to `run_id`, as the
/// writer's own append: the call every test that drives a run makes.
pub fn append(
    store: &mut Store,
    run_id: &str,
    event_type: EventType,
    payload: serde_json::Value,
) -> EventEnvelope {
    store
        .append_next(run_id, event_type, payload, None, None)
        .unwrap()
}

/// `value` as SQLite holds text.
pub fn text(value: &str) -> Value {
    Value::Text(value.to_string())
}

/// Plant a `runs` row carrying this manifest text verbatim, valid JSON or
/// not: a run as a foreign or older writer left it, with no origin and no
/// arrival. Inside a transaction a test holds open, it is also how the
/// test takes the journal's write lock.
pub fn plant_run(conn: &Connection, run_id: &str, manifest: &str) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at)
         VALUES (?1, 'feat', 'self', ?2, '2026-01-01T00:00:00Z')",
        params![run_id, manifest],
    )
}

/// Plant an event row exactly as given — any seq SQLite will bind, any
/// hash, any envelope text — past the append fence and the chain.
pub fn plant_event(
    conn: &Connection,
    run_id: &str,
    seq: &dyn ToSql,
    event_hash: &str,
    envelope: &str,
) -> rusqlite::Result<usize> {
    conn.execute(INSERT_EVENT, params![run_id, seq, event_hash, envelope])
}

/// Plant a sealed envelope under its own run, seq and hash, serialized
/// the way the store serializes the envelopes it writes.
pub fn plant_envelope(conn: &Connection, envelope: &EventEnvelope) -> rusqlite::Result<usize> {
    let text = serde_json::to_string(envelope).expect("an envelope serializes");
    plant_event(
        conn,
        &envelope.run_id,
        &(envelope.seq as i64),
        &envelope.event_hash,
        &text,
    )
}

/// Plant a copy of `event` at `seq` that chains onto no event — its
/// `previous_hash` is [`ZERO_HASH`] and its recorded hash still `event`'s
/// — as a chain broken behind the store's back.
pub fn plant_broken_link(
    conn: &Connection,
    event: &EventEnvelope,
    seq: u64,
) -> rusqlite::Result<usize> {
    let broken = EventEnvelope {
        seq,
        previous_hash: ZERO_HASH.to_string(),
        ..event.clone()
    };
    plant_envelope(conn, &broken)
}

/// Where the exported fixture journals are, read and never regenerated.
pub fn fixture_journals() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/journals")
}

/// `events` planted as a writer from before the summary leaves them: the
/// run and its events, and nothing folded.
pub fn plant_older_writer(store: &Store, run_id: &str, events: &[EventEnvelope]) {
    plant_run(&store.conn, run_id, "{}").expect("a run plants");
    for event in events {
        plant_envelope(&store.conn, event).expect("an event plants");
    }
}

/// `events` appended to `run_id` as a writer from between the summary and
/// the seat summary (decision 0089 slices 1 and 2) leaves them: the run's
/// summary row folded at its new head, and its seat rows and seat head
/// where they stood.
pub fn plant_summary_writer(store: &Store, run_id: &str, events: &[EventEnvelope]) {
    for event in events {
        plant_envelope(&store.conn, event).expect("an event plants");
    }
    let whole = store.load(run_id).expect("the run loads");
    let listed = Listed::of(&store.conn, run_id).expect("the run is listed");
    let summary = Summary::of(&Account::whole(run_id, &whole), listed.listing());
    crate::summary::write(&store.conn, run_id, &summary).expect("the summary writes");
}

/// Every exported journal under `fixtures/journals/`, in name order: its
/// path, its lines and the events they hold, read whole, chain unjudged.
pub(crate) fn fixture_events() -> Vec<(std::path::PathBuf, String, Vec<EventEnvelope>)> {
    let mut journals: Vec<_> = std::fs::read_dir(fixture_journals())
        .expect("the fixture journals are listed")
        .map(|entry| entry.expect("a fixture entry reads").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "ndjson")
        })
        .collect();
    journals.sort();
    journals
        .into_iter()
        .map(|journal| {
            let ndjson = std::fs::read_to_string(&journal).expect("a fixture journal reads");
            let events = ndjson
                .lines()
                .map(|line| serde_json::from_str(line).expect("a fixture event is an envelope"))
                .collect();
            (journal, ndjson, events)
        })
        .collect()
}

/// Every exported journal under `fixtures/journals/`, in name order,
/// imported into `store`, and planted as [`plant_older_writer`] plants it
/// where the import refuses it: each journal's run id, and the words of
/// each refusal.
pub fn import_fixtures(store: &mut Store) -> (Vec<String>, Vec<String>) {
    let (mut runs, mut refused) = (Vec::new(), Vec::new());
    for (journal, ndjson, events) in fixture_events() {
        let sidecar = std::fs::read_to_string(journal.with_extension("manifest.json"));
        let manifest = sidecar.map_or_else(
            |_| serde_json::json!({}),
            |text| serde_json::from_str(&text).expect("a sidecar manifest is JSON"),
        );
        let run_id = events[0].run_id.clone();
        if let Err(error) = store.import_run(&ndjson, &manifest, &journal) {
            refused.push(error.to_string());
            plant_older_writer(store, &run_id, &events);
        }
        runs.push(run_id);
    }
    (runs, refused)
}

/// Leave a journal as a binary from before the summary (decision 0089)
/// left it: none of the summary's table, index or triggers, none of the
/// event index's or the seat summary's, and no backfill recorded. Opening
/// it read-write migrates it again.
pub fn unmigrate(conn: &Connection) -> rusqlite::Result<()> {
    let unsummarized = conn.execute_batch(
        "DROP TABLE run_summary;
         DROP TRIGGER run_summary_of_a_run;
         DROP TRIGGER run_summary_of_an_event;
         DROP TRIGGER run_summary_of_an_acknowledgement;
         DELETE FROM meta WHERE key = 'run_summary_backfill';",
    );
    unsummarized?;
    unindex(conn)?;
    unseat(conn)
}

/// Leave a journal as a binary from before the seat summary (decision
/// 0089 slice 2) left it: neither of its tables, and no backfill
/// recorded.
pub fn unseat(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "DROP TABLE seat_summary;
         DROP TABLE seat_head;
         DELETE FROM meta WHERE key = 'seat_summary_backfill';",
    )
}

/// Leave a journal as a binary from before the event index (decision
/// 0089 slice 2) left it: no table, indexes or triggers of it, and no
/// backfill recorded.
pub fn unindex(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "DROP TABLE event_index;
         DROP TRIGGER event_index_of_an_event;
         DELETE FROM meta WHERE key = 'event_index_backfill';",
    )
}

/// Leave a journal as a binary from before journals named themselves
/// (decision 0095, #430) left it: no id. Opening it read-write names it.
pub fn unname(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("DELETE FROM meta WHERE key = 'journal_id'")
}

/// The plan SQLite takes for `run_id`'s trail read (decision 0089 ruling
/// 9), each step's detail in order: what a heap test asserts of the
/// journal it measures.
pub fn trail_plan(store: &Store, run_id: &str) -> rusqlite::Result<Vec<String>> {
    plan(store, &crate::event_index::TRAIL, [run_id])
}

/// The plan SQLite takes for `sql` bound to `bound`, each step's detail in
/// order.
pub(crate) fn plan(
    store: &Store,
    sql: &str,
    bound: impl rusqlite::Params,
) -> rusqlite::Result<Vec<String>> {
    let mut statement = store.conn.prepare(&format!("EXPLAIN QUERY PLAN {sql}"))?;
    let steps = statement.query_map(bound, |row| row.get(3))?;
    steps.collect()
}

/// The plan SQLite takes for a page of `run_id`'s checkpoints under
/// `attempt_id` that name `member` (decision 0089 ruling 9), each step's
/// detail in order: what the seat view's heap test asserts of the journal
/// it measures.
pub fn stream_plan(
    store: &Store,
    run_id: &str,
    attempt_id: &str,
    member: Option<&str>,
) -> rusqlite::Result<Vec<String>> {
    let bound = params![run_id, attempt_id, member, 1, i64::MAX, 51];
    plan(store, &crate::event_index::STREAM, bound)
}

/// The statements SQLite began on `store`'s connection while `read` ran,
/// counted by its trace, which it calls as each statement begins: what the
/// seat view's heap test holds a page's work to (decision 0089 ruling 9).
#[cfg(feature = "test-support")]
pub fn statements_begun<T>(store: &mut Store, read: impl FnOnce(&Store) -> T) -> (T, usize) {
    let (read, begun) = statements_traced(store, read);
    (read, begun.len())
}

/// The statements SQLite began on `store`'s connection while `read` ran,
/// in the order they began, each as its trace writes it, its bound values
/// in place: what the run view's whole read holds its statements to
/// (decision 0089 ruling 7).
#[cfg(feature = "test-support")]
pub fn statements_traced<T>(store: &mut Store, read: impl FnOnce(&Store) -> T) -> (T, Vec<String>) {
    use std::cell::RefCell;
    thread_local! {
        // SQLite calls a connection's trace on the thread running its
        // statement, so one collector per test thread keeps overlapping
        // collectors apart: the process-wide clear/take erased another
        // test's statements under CI's default parallelism (M5).
        static BEGUN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }
    fn begun(sql: &str) {
        BEGUN.with(|begun| begun.borrow_mut().push(sql.to_string()));
    }
    /// This thread's collector is cleared however `read` leaves: a panic
    /// unwinds through the guard rather than leaving a sibling test's
    /// thread-local history for the next call. The connection's trace is
    /// removed on the normal path; a panicking read drops its store with
    /// the trace still installed, which no later statement reads.
    struct Traced;
    impl Drop for Traced {
        fn drop(&mut self) {
            BEGUN.with(|begun| begun.borrow_mut().clear());
        }
    }
    BEGUN.with(|begun| begun.borrow_mut().clear());
    store.conn.trace(Some(begun));
    let guard = Traced;
    let read = read(store);
    let begun = BEGUN.with(|begun| std::mem::take(&mut *begun.borrow_mut()));
    drop(guard);
    store.conn.trace(None);
    (read, begun)
}

/// The plan SQLite takes for each statement begun on `store`'s connection
/// while `read` ran, each step's detail in the order begun, each statement
/// as its trace gives it, its parameters written in, but for the comment
/// it gives for a pragma, which plans nothing: what the `costs` heap test
/// asserts of the reads it measures (decision 0089 ruling 9).
#[cfg(feature = "test-support")]
pub fn plans_begun<T>(store: &mut Store, read: impl FnOnce(&Store) -> T) -> (T, Vec<String>) {
    let (read, begun) = statements_traced(store, read);
    let planned = begun.iter().filter(|sql| !sql.starts_with("--"));
    let plans = planned.map(|sql| plan(store, sql, []).unwrap());
    (read, plans.flatten().collect())
}

/// Take the seq guard (#354) off a journal, leaving it as a journal
/// written before the guard existed: one that may hold a row at a seq
/// that is not a position. Opening the journal read-write restores it.
pub fn drop_seq_guard(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("DROP TRIGGER IF EXISTS events_seq_is_a_position")
}

/// Plant a holder row of `generation` among the journal's dispatchers
/// (decision 0095 ruling 2), unless one stands at it: the dispatcher a
/// test's steps are taken by, the latest while none above it stands.
pub fn holding(store: &Store, generation: crate::Generation) {
    let plant = "INSERT OR IGNORE INTO dispatch_holders (generation, host, pid, started_at)
        VALUES (?1, NULL, 1, 'then')";
    store.conn.execute(plant, [generation.0]).unwrap();
}

/// Link queued `entry` to `run`, which the journal holds, through the
/// dispatcher's own claim and link (decision 0095 ruling 1), as
/// generation 1, its holder planted, claiming a bundle of digest `test`,
/// judging no run.
pub fn linked(store: &mut Store, entry: crate::EntryId, run: &str) {
    holding(store, crate::Generation(1));
    let read = store.queue_entry(entry).unwrap();
    let claimed = store.queue_claim_with(
        &read,
        i64::MAX,
        "test",
        crate::Generation(1),
        "test",
        |_, _, _| Ok::<(), ()>(()),
    );
    let claim = claimed.unwrap().unwrap();
    (store.queue_link(claim, crate::Generation(1), run, "test")).unwrap();
}

/// `columns` of one row of `run_id`, read by its key.
pub fn kept_facts(store: &Store, run_id: &str, seq: u64, columns: &[&str]) -> Vec<Value> {
    let select = format!(
        "SELECT {} FROM event_index WHERE run_id = ?1 AND seq = ?2",
        columns.join(", ")
    );
    store
        .conn
        .query_row(&select, params![run_id, seq as i64], |row| {
            (0..columns.len()).map(|at| row.get(at)).collect()
        })
        .unwrap()
}

/// The event's own time in milliseconds.
pub fn at(event: &EventEnvelope) -> i64 {
    brokkr_core::stamp::parse_millis(&event.recorded_at).unwrap()
}

/// A run created and carried by this store, started under a feature, with
/// `phase` entered, and then `steps`.
pub fn opened(
    store: &mut Store,
    run_id: &str,
    phase: &str,
    steps: &[(EventType, serde_json::Value)],
) {
    store
        .create_run(run_id, "a feature", "self", &serde_json::json!({}))
        .unwrap();
    append(
        store,
        run_id,
        EventType::RunStarted,
        serde_json::json!({"feature": "a feature", "manifest": {}}),
    );
    append(
        store,
        run_id,
        EventType::PhaseEntered,
        serde_json::json!({"phase": phase}),
    );
    for (event_type, payload) in steps {
        append(store, run_id, *event_type, payload.clone());
    }
}

/// `store`'s journal opened again read-write, which repairs its projections.
pub fn refolded(store: Store) -> Store {
    let path = store.path().to_path_buf();
    drop(store);
    Store::open(&path).unwrap()
}

/// What the doctor's sweep names, as words: empty where every run reads as
/// its fold.
pub fn disagreements(store: &Store) -> Vec<String> {
    store
        .summary_disagreements()
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// Every fixture journal planted as an older writer leaves it, each run's
/// id and events, in name order: the projections' backfills are the
/// caller's, so each walks only what it keeps.
pub fn planted_fixtures(store: &Store) -> Vec<(String, Vec<EventEnvelope>)> {
    fixture_events()
        .into_iter()
        .map(|(_, _, events)| {
            let run_id = events[0].run_id.clone();
            plant_older_writer(store, &run_id, &events);
            (run_id, events)
        })
        .collect()
}

/// Edit each of `columns` on `run_id`'s row at `seq` to a value that is
/// not the one `held` has, assert ruling 4's comparison names that column
/// or passes it as lagging when it is `behind`, and restore it. The names
/// are returned in the order they were checked.
pub fn each_column_named(
    store: &Store,
    run_id: &str,
    seq: u64,
    columns: &[&str],
    held: &[Value],
    behind: Option<&str>,
) -> Vec<String> {
    let mut named = Vec::new();
    for (at, column) in columns.iter().enumerate() {
        let edit = format!(
            "UPDATE event_index SET {column} = ?1 WHERE run_id = '{run_id}' AND seq = {seq}"
        );
        let wrong = match &held[at] {
            Value::Text(_) => text("edited"),
            Value::Real(_) => Value::Real(1.5),
            Value::Integer(1) => Value::Integer(2),
            _ => Value::Integer(1),
        };
        store.conn.execute(&edit, [&wrong]).unwrap();
        let read = store.compared(run_id, || {}).unwrap();
        if Some(*column) == behind {
            assert!(matches!(read, Compared::Behind), "{column}: {read:?}");
        } else {
            match read {
                Compared::Disagrees(Disagreement::Indexed {
                    fault: IndexFault::Column(found),
                    ..
                }) => assert_eq!(found, *column),
                other => panic!("{column}: {other:?}"),
            }
        }
        named.push(column.to_string());
        store.conn.execute(&edit, [&held[at]]).unwrap();
    }
    named
}

/// The seqs of `run_id`'s index rows of `event_type`, in order.
pub fn indexed_seqs(store: &Store, run_id: &str, event_type: &str) -> Vec<u64> {
    let mut statement = store
        .conn
        .prepare("SELECT seq FROM event_index WHERE run_id = ?1 AND type = ?2 ORDER BY seq")
        .unwrap();
    let seqs = statement
        .query_map(params![run_id, event_type], |row| row.get(0))
        .unwrap();
    seqs.map(|seq| seq.unwrap()).collect()
}

/// The body of the trigger `name`, its `BEGIN`-to-first-statement SQL,
/// with each `(from, to)` applied: a test's plan is the journal's own
/// trigger SQL, never a copy.
pub fn trigger_body(store: &Store, name: &str, bindings: &[(&str, &str)]) -> String {
    let trigger: String = store
        .conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'trigger' AND name = ?1",
            [name],
            |row| row.get(0),
        )
        .unwrap();
    let mut body = trigger
        .split_once("BEGIN")
        .and_then(|(_, rest)| rest.split_once(';'))
        .map(|(update, _)| update.to_string())
        .unwrap();
    for (from, to) in bindings {
        body = body.replace(from, to);
    }
    body
}
