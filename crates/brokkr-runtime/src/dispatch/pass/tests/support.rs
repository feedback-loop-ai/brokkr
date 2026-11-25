//! The fixtures both of the dispatcher's test modules read the journal and
//! the machine with: its commands, claims and refusals row for row, a host
//! that measures plenty, and seams that always stand and start one run.

use std::ffi::OsString;
use std::path::Path;

use brokkr_store::Store;

use super::super::super::{pass, Ended, Ending, Passed, Seams, Spawn, SpawnFailed, Spawned};
use crate::admission::Host;
use crate::dispatch::pass::PassSeams;
use crate::dispatch::Dispatcher;

/// The host `file` declares, with a temp directory the tests own and every
/// filesystem reported plenty.
pub(crate) fn host(file: &Path) -> Host<'_> {
    Host {
        file,
        temp: Path::new("/brokkr-test-temp"),
        free: &|_: &Path| Ok(1 << 40),
        machine: None,
    }
}

/// Run one pass over `store`, with the machine `file` names and `seams`.
pub(crate) fn run_pass<S, P, C>(
    store: &mut Store,
    dispatcher: &Dispatcher,
    file: &Path,
    seams: Seams<S, P, C>,
) -> Result<Passed, Box<Ended>>
where
    S: FnMut() -> Result<(), Ending>,
    P: FnMut(&Spawn) -> Result<Spawned, SpawnFailed>,
    C: FnMut() -> bool,
{
    pass(store, dispatcher, &host(file), OsString::new(), seams)
}

/// A stand that always stands.
pub(crate) fn stands_ok() -> Result<(), Ending> {
    Ok(())
}

/// A stop that never comes: every claim may go ahead.
pub(crate) fn claims_ok() -> bool {
    true
}

/// A spawn that starts one run with pid 1.
pub(crate) fn spawned_one(_: &Spawn) -> Result<Spawned, SpawnFailed> {
    Ok(Spawned { pid: 1 })
}

/// The seam a test uses when it neither counts a stand nor records a
/// spawn.
type Quiet =
    PassSeams<'static, fn() -> Result<(), Ending>, fn(&Spawn) -> Result<Spawned, SpawnFailed>>;

/// A seam that always stands, claims and starts one run.
pub(crate) fn quiet() -> Quiet {
    Seams {
        stands: stands_ok,
        spawn: spawned_one,
        claims: always_claims(),
    }
}

/// [`claims_ok`] as the pass's own steps take it. A function item has no
/// size, so the box it is leaked from allocates nothing.
pub(crate) fn always_claims() -> &'static mut dyn FnMut() -> bool {
    Box::leak(Box::new(claims_ok))
}

/// The queue commands the journal holds, in order.
pub(crate) fn commands(store: &Store) -> Vec<(i64, String, String, String)> {
    let conn = rusqlite::Connection::open(store.path()).unwrap();
    let mut rows = conn
        .prepare("SELECT seq, command, operator, reason FROM queue_commands ORDER BY seq")
        .unwrap();
    let mapped = rows.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    });
    mapped.unwrap().collect::<Result<_, _>>().unwrap()
}

/// How many commands of `word` the journal holds.
pub(crate) fn count(store: &Store, word: &str) -> usize {
    commands(store)
        .into_iter()
        .filter(|(_, command, ..)| command == word)
        .count()
}

/// The refusals `queue_refusals` holds.
pub(crate) fn refusals(store: &Store) -> Vec<String> {
    let conn = rusqlite::Connection::open(store.path()).unwrap();
    let read = |row: &rusqlite::Row<'_>| row.get(0);
    let mut rows = conn.prepare("SELECT refusal FROM queue_refusals").unwrap();
    rows.query_map([], read)
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// The claims the journal holds, by entry, generation and run.
pub(crate) fn claims(store: &Store) -> Vec<(i64, i64, i64, Option<String>)> {
    let conn = rusqlite::Connection::open(store.path()).unwrap();
    let mut statement = conn
        .prepare("SELECT seq, entry_id, generation, run_id FROM queue_claims ORDER BY seq")
        .unwrap();
    let mut claims = Vec::new();
    let mut rows = statement.query([]).unwrap();
    while let Some(row) = rows.next().unwrap() {
        claims.push((
            row.get(0).unwrap(),
            row.get(1).unwrap(),
            row.get(2).unwrap(),
            row.get(3).unwrap(),
        ));
    }
    claims
}

/// The run ids the journal holds.
pub(crate) fn runs(store: &Store) -> Vec<String> {
    store
        .list_runs()
        .unwrap()
        .into_iter()
        .map(|(run, ..)| run)
        .collect()
}

/// A write lock held on `store`'s journal, released by its drop or a
/// rollback.
pub(crate) fn holding(store: &Store) -> rusqlite::Connection {
    let holder = rusqlite::Connection::open(store.path()).unwrap();
    holder
        .busy_timeout(std::time::Duration::from_millis(50))
        .unwrap();
    holder.execute_batch("BEGIN IMMEDIATE").unwrap();
    holder
}
