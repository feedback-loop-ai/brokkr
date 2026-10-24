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
use brokkr_core::envelope::EventEnvelope;
use brokkr_core::summary::{Account, Summary};
use rusqlite::{params, Connection, ToSql};

use crate::import::INSERT_EVENT;
use crate::summary::{Compared, Listed};
use crate::{Disagreement, Store};

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

/// Take the seq guard (#354) off a journal, leaving it as a journal
/// written before the guard existed: one that may hold a row at a seq
/// that is not a position. Opening the journal read-write restores it.
pub fn drop_seq_guard(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("DROP TRIGGER IF EXISTS events_seq_is_a_position")
}
