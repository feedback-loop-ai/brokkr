//! The far run's journal, built once for the test binary (#978): the
//! checkpoint counts that reach the `u16` display clamp are 65,600 and
//! 66,000 rows, and planting them again in every test that reads them was
//! the shard's cost. One `OnceLock` per size holds the planted journal,
//! its events and their whole view; a test that only reads uses it as it
//! stands, and a test that appends takes a copy. The three journals are
//! removed at process exit, so a run leaves no fixture behind.

use std::sync::OnceLock;

use super::*;

/// The one far run's name: the sizes keep their own journals, and no
/// assertion reads the name.
const FAR_RUN: &str = "far work";

/// The statement [`planted_far`] prepares: a copy of `brokkr-store`'s
/// private `INSERT_EVENT`, held to the source by
/// [`the_fixture_inserts_the_store_s_events`] (decision 0071 ruling 5).
const INSERT_EVENT: &str =
    "INSERT INTO events (run_id, seq, event_hash, envelope) VALUES (?1, ?2, ?3, ?4)";

/// A far run's journal, built once for the test binary: the tempdir that
/// holds it, its path, the run's name, its events and their whole view.
pub(super) struct FarWork {
    _dir: tempfile::TempDir,
    pub(super) db: std::path::PathBuf,
    pub(super) run: &'static str,
    pub(super) events: Vec<EventEnvelope>,
    pub(super) whole: RunView,
}

impl FarWork {
    fn build(run: &'static str, checkpoints: u64) -> FarWork {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("forge.db");
        let events = at_work(run, checkpoints, true);
        planted_far(&db, run, &events);
        let whole = brokkr_view::run_view(&events, fold(&events).as_ref().ok());
        FarWork {
            _dir: dir,
            db,
            run,
            events,
            whole,
        }
    }

    /// A per-test copy of the fixture's journal, in its own tempdir: a
    /// test that appends owns the copy and never the shared file. The
    /// fixture is closed after it is planted, so its WAL is checkpointed
    /// into the one file.
    pub(super) fn copy(&self) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("forge.db");
        std::fs::copy(&self.db, &db).unwrap();
        (dir, db)
    }
}

/// The 65,600-event journal, built once for the test binary.
static FAR_65_600: OnceLock<FarWork> = OnceLock::new();
/// The 66,000-event journal, built once for the test binary.
static FAR_66_000: OnceLock<FarWork> = OnceLock::new();
/// The 1,000-event journal, built once for the test binary.
static FAR_1_000: OnceLock<FarWork> = OnceLock::new();

/// `checkpoints`'s shared fixture, built once for the test binary. One
/// `OnceLock` per size the tests read, so a 66,000-event journal is
/// planted and folded once whatever number of tests asks for it.
pub(super) fn far_work_of(checkpoints: u64) -> &'static FarWork {
    #[cfg(unix)]
    register_cleanup();
    match checkpoints {
        65_600 => FAR_65_600.get_or_init(|| FarWork::build(FAR_RUN, 65_600)),
        66_000 => FAR_66_000.get_or_init(|| FarWork::build(FAR_RUN, 66_000)),
        1_000 => FAR_1_000.get_or_init(|| FarWork::build(FAR_RUN, 1_000)),
        other => panic!("no shared far fixture of {other} checkpoints"),
    }
}

/// [`far_work_of`] at 65,600 checkpoints.
pub(super) fn far_work() -> &'static FarWork {
    far_work_of(65_600)
}

/// Register [`remove_fixtures`] once, so the process's exit removes the
/// journals a `OnceLock` would otherwise leave in the tempdir forever.
#[cfg(unix)]
fn register_cleanup() {
    static REGISTERED: OnceLock<()> = OnceLock::new();
    REGISTERED.get_or_init(|| {
        // SAFETY: `atexit` is libc's; it stores the `extern "C"` callback
        // and returns zero, and the callback touches only these statics
        // and the filesystem. The supported hosts are Linux and macOS.
        let registered = unsafe { libc_atexit(remove_fixtures) };
        assert_eq!(registered, 0, "the fixture cleanup registers at exit");
    });
}

// Read `atexit` from libc without adding a crate: `libc` is already in the
// graph, so the symbol is declared here as a test does elsewhere.
#[cfg(unix)]
unsafe extern "C" {
    #[link_name = "atexit"]
    fn libc_atexit(callback: extern "C" fn()) -> std::os::raw::c_int;
}

/// Remove whichever far journals this binary built. `OnceLock` contents
/// are never dropped, so `TempDir`'s own cleanup does not run; this does.
#[cfg(unix)]
extern "C" fn remove_fixtures() {
    for cell in [&FAR_65_600, &FAR_66_000, &FAR_1_000] {
        if let Some(far) = cell.get() {
            let _ = std::fs::remove_dir_all(far._dir.path());
        }
    }
}

/// `run`'s `events` planted into a fresh journal at `path` as an older
/// writer leaves them, every row in one transaction with the events
/// statement prepared once: the shared planted helper prepares it again
/// per row, which is the whole cost of the 65,600-checkpoint fixture; the
/// rows and the store's catch-up are the same.
fn planted_far(path: &Path, run: &str, events: &[EventEnvelope]) {
    drop(Store::open(path).unwrap());
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    let transaction = conn.unchecked_transaction().unwrap();
    brokkr_store::test_support::plant_run(&transaction, run, "{}").unwrap();
    {
        let mut insert = transaction.prepare_cached(INSERT_EVENT).unwrap();
        for event in events {
            insert
                .execute(rusqlite::params![
                    event.run_id,
                    event.seq as i64,
                    event.event_hash,
                    serde_json::to_string(event).unwrap(),
                ])
                .unwrap();
        }
    }
    transaction.commit().unwrap();
    drop(conn);
    drop(Store::open(path).unwrap());
}

/// [`INSERT_EVENT`] is `brokkr-store`'s statement, not a copy left to
/// drift: the store keeps the constant private, so the source file is the
/// binding (decision 0071 ruling 5).
#[test]
fn the_fixture_inserts_the_store_s_events() {
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/brokkr-store/src/import.rs"),
    )
    .unwrap();
    assert!(
        source.contains(INSERT_EVENT),
        "the fixture's INSERT_EVENT is the store's statement"
    );
}
