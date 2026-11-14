//! Fixtures for decision 0108 ruling 3's release, shared by the exclusion's
//! and the settle verb's tests: one host, one acknowledged shape (B) run
//! whose attempt stands, and the child process groups the proof reads. One
//! home for them keeps the two suites from restating the same setup.

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};

use brokkr_core::envelope::EventType;
use brokkr_store::Store;
use serde_json::json;

use super::{boxed, claim, Coordinated, Host};
use crate::engine::broker::admission::{Held, Protects, Witnessed};
use crate::engine::broker::settlement::Driver;

/// A real child process leading a process group of its own, alive until
/// dropped: the attempt's process group a test keeps standing on Linux and
/// macOS alike (decision 0063; decision 0108 ruling 3).
pub(in crate::engine) struct LiveGroup(std::process::Child);

impl LiveGroup {
    pub(in crate::engine) fn new() -> LiveGroup {
        let child = std::process::Command::new("sleep")
            .arg("30")
            .process_group(0)
            .spawn()
            .expect("a child leading its own process group");
        LiveGroup(child)
    }

    /// Its process group's id: the child's own pid, which it leads.
    pub(in crate::engine) fn group(&self) -> u32 {
        self.0.id()
    }
}

impl Drop for LiveGroup {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The id of a process group that has ended: a child that led its own group
/// and was reaped, so the host no longer shows a member.
pub(in crate::engine) fn dead_group() -> u32 {
    let mut child = std::process::Command::new("true")
        .process_group(0)
        .spawn()
        .expect("a child leading its own process group");
    let group = child.id();
    child.wait().unwrap();
    group
}

/// A run of `worktree` whose attempt `attempt` stands, its one `work` site
/// boxed to it: a writer that cannot reach a journal beside the HOME, so the
/// release's guarded read is trustworthy (R2).
pub(in crate::engine) fn boxed_claim(
    worktree: &Path,
    ids: [&str; 2],
) -> crate::engine::broker::admission::Claim {
    claim(
        worktree,
        ids,
        Protects::Nothing,
        &[boxed("work", &[worktree])],
    )
}

/// A run of `worktree` whose attempt `attempt` stands in `host`'s root with
/// the driver records and process group a test names: what the release proof
/// reads (decision 0108 ruling 3). Its attempt-level cleanup stays Settling,
/// so recovery leaves it standing for the proof.
pub(in crate::engine) fn planted(
    host: &Host,
    worktree: &Path,
    [run, attempt]: [&str; 2],
    drivers: &[Witnessed],
    group: Option<u32>,
    journal: &Path,
) -> Held {
    let mut claim = boxed_claim(worktree, [run, attempt]);
    claim.journal = journal.to_path_buf();
    claim.reserved.drivers = drivers
        .iter()
        .map(|cleanup| Driver {
            site: "work".to_string(),
            begun: None,
            cleanup: *cleanup,
        })
        .collect();
    claim.reserved.group = group;
    host.admit(&claim)
        .expect("the planted reservation is admitted")
}

/// Run `run`'s journal at `db`, stopped at seq 5.
pub(in crate::engine) fn concluded(db: &Path, run: &str) {
    let mut store = crate::engine::tests::in_flight_store(db, run);
    store
        .append_next(
            run,
            EventType::RunStopped,
            json!({"reason": "concluded"}),
            None,
            None,
        )
        .unwrap();
}

/// Shape (B) planted behind `db`'s run `run`: the stop command and
/// acceptance 0.12.0 left after the conclusion, chained onto the head the
/// store read. Returned opened writable, the run then not folding.
pub(in crate::engine) fn broken(db: &Path, run: &str) -> Store {
    let store = Store::open(db).unwrap();
    let head = store.load(run).unwrap().pop().unwrap();
    drop(store);
    let connection = rusqlite::Connection::open(db).unwrap();
    brokkr_store::test_support::plant_chained(&connection, run, &head, &crate::late::late_stop())
        .unwrap();
    drop(connection);
    Store::open(db).unwrap()
}

/// Acknowledge the quarantined run `run` where it stands.
pub(in crate::engine) fn acknowledge(store: &mut Store, run: &str) {
    let (seq, hash) = store.head_hash(run).unwrap();
    let by = brokkr_store::Attribution {
        operator: "operator",
        reason: "0.12.0 left a stop after the conclusion",
    };
    store
        .acknowledge_quarantine(run, (seq, &hash), by)
        .expect("the broken run is acknowledged where it stands");
}

/// A host, its worktree and journal, and an acknowledged shape (B) run
/// `run-a` whose attempt `a1` stands with the driver records and process
/// group a test names.
pub(in crate::engine) struct Fixture {
    pub(in crate::engine) host: Host,
    _stood: Coordinated,
    pub(in crate::engine) work: PathBuf,
    pub(in crate::engine) db: PathBuf,
    pub(in crate::engine) store: Store,
}

/// [`Fixture`] whose run is acknowledged.
pub(in crate::engine) fn quarantined(drivers: &[Witnessed], group: Option<u32>) -> Fixture {
    let mut fixture = unacknowledged(drivers, group);
    acknowledge(&mut fixture.store, "run-a");
    fixture
}

/// [`Fixture`] whose run carries a terminal event and a shape (B) tail but
/// no acknowledgement: what the release refuses whatever else holds.
pub(in crate::engine) fn unacknowledged(drivers: &[Witnessed], group: Option<u32>) -> Fixture {
    let host = Host::new();
    let work = host.worktree("work");
    let db = host.journal("run-a.db");
    planted(&host, &work, ["run-a", "a1"], drivers, group, &db);
    concluded(&db, "run-a");
    let store = broken(&db, "run-a");
    let stood = Coordinated::at(host.home());
    Fixture {
        host,
        _stood: stood,
        work,
        db,
        store,
    }
}
