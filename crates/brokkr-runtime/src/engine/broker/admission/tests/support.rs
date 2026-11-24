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
use crate::engine::broker::admission::{Held, Protects, Reservation, Witnessed};
use crate::engine::broker::settlement::Driver;

impl Host {
    /// The driver records `run` of `worktree` holds, in spawn order: each
    /// one's site, cleanup and own process group, as the dispatch door wrote
    /// them.
    pub(in crate::engine::broker) fn drivers(&self, worktree: &Path, run: &str) -> Vec<Driver> {
        let bytes = std::fs::read(self.reservation(worktree, run)).unwrap();
        let held: Reservation = serde_json::from_slice(&bytes).unwrap();
        held.attempts
            .into_values()
            .flat_map(|reserved| reserved.drivers)
            .collect()
    }
}

/// Whether the live-group test `test` may run here: it runs alone in its
/// own engine, so a sibling test's attempt tracker cannot take the child
/// whose group the proof asks about for an orphan and kill it.
pub(in crate::engine) fn isolated(test: &str) -> bool {
    crate::engine::tests::in_its_own_engine(test)
}

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

    /// End the child and reap it, so the host no longer shows its group.
    pub(in crate::engine) fn end(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl Drop for LiveGroup {
    fn drop(&mut self) {
        self.end();
    }
}

const LOCK_DB: &str = "BROKKR_RELEASE_LOCK_DB";
const LOCK_RUN: &str = "BROKKR_RELEASE_LOCK_RUN";
const LOCK_FILE: &str = "BROKKR_RELEASE_LOCK_FILE";

/// A child process holding the run `run`'s lock until dropped: the kernel's
/// exclusion across processes, not only across open descriptions (decision
/// 0108 ruling 1).
pub(in crate::engine) struct ChildLock(std::process::Child);

impl ChildLock {
    /// The holder's pid, which the refusal names.
    pub(in crate::engine) fn holder(&self) -> u32 {
        self.0.id()
    }
}

impl Drop for ChildLock {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// [`ChildLock`] holding `run`'s lock: the child takes it, records its
/// facts, and leaves a marker the parent waits on before it compares.
pub(in crate::engine) fn child_lock(store: &Store, run: &str) -> ChildLock {
    let marker = store.path().with_extension("locked");
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "engine::broker::admission::tests::support::lock_child",
            "--nocapture",
        ])
        .env(LOCK_DB, store.path())
        .env(LOCK_RUN, run)
        .env(LOCK_FILE, &marker)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("the lock holder starts");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::fs::metadata(&marker).is_err() {
        assert!(
            std::time::Instant::now() < deadline,
            "the lock holder never took the lock"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    ChildLock(child)
}

/// The child entry [`child_lock`] starts: takes the run's lock and holds it
/// until the parent's drop kills it or a minute passes. Inert without the
/// environment, so a plain suite run passes over it.
#[test]
fn lock_child() {
    let (Ok(db), Ok(run), Ok(marker)) = (
        std::env::var(LOCK_DB),
        std::env::var(LOCK_RUN),
        std::env::var(LOCK_FILE),
    ) else {
        return;
    };
    let store = Store::open(Path::new(&db)).unwrap();
    let _held = crate::run_lock::take(&store, &run).expect("the child takes the lock");
    std::fs::write(&marker, b"").unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while std::fs::metadata(&marker).is_ok() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

const DEAD_GROUP: &str = "BROKKR_DEAD_GROUP";

/// The child entry [`dead_group`] starts: spawns a short-lived child in its
/// own process group, waits for it to exit and be reaped, and prints its
/// group id. Spawning `.process_group(0)` directly in the shared test process
/// exposes the child to `attempts`'s orphan reaper, which adopts and reaps
/// non-engine-group zombies before `child.wait()` runs. Inert without the
/// environment, so a plain suite run passes over it.
#[test]
fn dead_group_child() {
    if std::env::var_os(DEAD_GROUP).is_none() {
        return;
    }
    let mut child = std::process::Command::new("true")
        .process_group(0)
        .spawn()
        .expect("a child leading its own process group");
    let group = child.id();
    let status = child.wait().expect("the child exits and is reaped");
    assert!(status.success(), "{status}");
    println!("{DEAD_GROUP}={group}");
}

/// The id of a process group that has ended: a child that led its own group
/// and was reaped, so the host no longer shows a member.
pub(in crate::engine) fn dead_group() -> u32 {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "engine::broker::admission::tests::support::dead_group_child",
            "--nocapture",
        ])
        .env(DEAD_GROUP, "1")
        .stdin(std::process::Stdio::null())
        .output()
        .expect("the dead_group helper runs");
    assert!(
        output.status.success(),
        "dead_group_child failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let prefix = format!("{DEAD_GROUP}=");
    stdout
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or_else(|| panic!("dead_group_child printed no group id:\n{stdout}"))
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
    ids: [&str; 2],
    drivers: &[Witnessed],
    group: Option<u32>,
    journal: &Path,
) -> Held {
    let drivers: Vec<(&str, Witnessed, Option<u32>)> = drivers
        .iter()
        .map(|cleanup| ("work", *cleanup, None))
        .collect();
    planted_drivers(host, worktree, ids, &drivers, group, journal)
}

/// [`planted`] with each driver's own site and process group: the shape the
/// dispatch door writes, one group per driver (decision 0108 ruling 3). The
/// `group` is 440d's single slot, kept where a test proves its migration.
pub(in crate::engine) fn planted_drivers(
    host: &Host,
    worktree: &Path,
    [run, attempt]: [&str; 2],
    drivers: &[(&str, Witnessed, Option<u32>)],
    group: Option<u32>,
    journal: &Path,
) -> Held {
    let mut claim = boxed_claim(worktree, [run, attempt]);
    claim.journal = journal.to_path_buf();
    claim.reserved.drivers = drivers
        .iter()
        .map(|(site, cleanup, group)| Driver {
            site: (*site).to_string(),
            begun: None,
            cleanup: *cleanup,
            group: *group,
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

/// A quarantined fixture whose attempt's drivers are the sites and cleanups
/// a panel test names, its lease returned so the test can record each
/// driver's own group through the dispatch door's own call.
pub(in crate::engine) fn quarantined_drivers(drivers: &[(&str, Witnessed)]) -> (Fixture, Held) {
    let host = Host::new();
    let work = host.worktree("work");
    let db = host.journal("run-a.db");
    let drivers: Vec<(&str, Witnessed, Option<u32>)> = drivers
        .iter()
        .map(|(site, cleanup)| (*site, *cleanup, None))
        .collect();
    let held = planted_drivers(&host, &work, ["run-a", "a1"], &drivers, None, &db);
    concluded(&db, "run-a");
    let mut store = broken(&db, "run-a");
    acknowledge(&mut store, "run-a");
    let stood = Coordinated::at(host.home());
    (
        Fixture {
            host,
            _stood: stood,
            work,
            db,
            store,
        },
        held,
    )
}
