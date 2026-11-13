//! The run lock's checks (decision 0108 ruling 1): a second driver is
//! refused naming the holder, a killed driver's lock goes with it, the
//! directory and file refuse links and other modes by name, and a run id
//! that would escape the directory never reaches a path.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rustix::fs::{flock, FlockOperation};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::*;
use crate::lock_file;

/// Set, to the journal's path, in a re-executed test binary that is to
/// hold the run until it is killed.
const CHILD: &str = "BROKKR_TEST_RUN_LOCK_CHILD";

/// The lock directory beside `journal`.
fn runs(journal: &Path) -> PathBuf {
    let mut name = std::ffi::OsString::from(journal);
    name.push(DIRECTORY);
    PathBuf::from(name)
}

/// The lock file of `run` beside `journal`.
fn lock(journal: &Path, run: &str) -> PathBuf {
    beside(journal, run)
}

/// The holder facts `run`'s lock records.
fn holder(journal: &Path, run: &str) -> Holder {
    serde_json::from_str(&std::fs::read_to_string(lock(journal, run)).unwrap()).unwrap()
}

/// The words a holder is refused with.
fn driven(run: &str, holder: &Holder) -> String {
    format!(
        "run '{run}' is driven by pid {} on this host since {} (engine {})",
        holder.pid, holder.since, holder.engine
    )
}

/// A lock held in one handle refuses a second driver at once, naming the
/// holder; with the first dropped, the next takes it.
#[test]
fn a_second_driver_is_refused_naming_the_holder() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let store = Store::open(&path).unwrap();
    assert!(
        !runs(&path).exists(),
        "no lock file before the first driver"
    );
    let held = take(&store, "run").unwrap();
    let refused = take(&store, "run").unwrap_err();
    let recorded = holder(&path, "run");
    assert_eq!(
        refused,
        RunLockError::Driven {
            run_id: "run".into(),
            holder: Some(recorded.clone()),
        }
    );
    assert_eq!(refused.to_string(), driven("run", &recorded));
    assert_eq!(
        refused.to_string(),
        format!(
            "run 'run' is driven by pid {} on this host since {} (engine {})",
            std::process::id(),
            recorded.since,
            ENGINE_VERSION
        )
    );
    drop(held);
    assert!(take(&store, "run").is_ok(), "the next driver takes it");
}

/// A taken lock records the pid, the start of the hold, the store's host
/// fingerprint and the engine's version, and makes its directory 0700.
#[test]
fn a_taken_lock_records_the_holders_facts() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let store = Store::open(&path).unwrap();
    let _held = take(&store, "run").unwrap();
    let recorded = holder(&path, "run");
    assert_eq!(recorded.pid, std::process::id());
    assert_eq!(recorded.engine, ENGINE_VERSION);
    assert_eq!(recorded.host, store.host_fingerprint());
    assert!(
        OffsetDateTime::parse(&recorded.since, &Rfc3339).is_ok(),
        "the start is an RFC 3339 time: {}",
        recorded.since
    );
    let mode = std::fs::metadata(runs(&path)).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o700, "the lock directory is this user's own");
    let mode = std::fs::metadata(lock(&path, "run"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600, "the lock file is this user's own");
}

/// The directory refuses a regular file, a link and a mode others may
/// enter, by its own name; the file refuses a link, a second link and a
/// mode others may read, by its own name.
#[test]
fn the_directory_and_file_refuse_links_other_modes_and_owners() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let store = Store::open(&path).unwrap();
    let at = runs(&path);
    let elsewhere = dir.path().join("elsewhere");
    std::fs::create_dir(&elsewhere).unwrap();

    std::fs::write(&at, "").unwrap();
    let refused = take(&store, "run").unwrap_err();
    assert_eq!(refused, RunLockError::Directory(at.clone()));
    assert_eq!(
        refused.to_string(),
        format!(
            "run lock: the directory {} is not this user's own directory of mode 0700 \
             (decision 0108)",
            at.display()
        )
    );
    std::fs::remove_file(&at).unwrap();

    std::os::unix::fs::symlink(&elsewhere, &at).unwrap();
    assert_eq!(
        take(&store, "run").unwrap_err(),
        RunLockError::Directory(at.clone())
    );
    std::fs::remove_file(&at).unwrap();

    std::fs::create_dir(&at).unwrap();
    std::fs::set_permissions(&at, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        take(&store, "run").unwrap_err(),
        RunLockError::Directory(at.clone())
    );
    std::fs::set_permissions(&at, std::fs::Permissions::from_mode(0o700)).unwrap();
    drop(take(&store, "run").unwrap());

    let file = lock(&path, "run");
    let other = dir.path().join("other");
    std::fs::write(&other, "").unwrap();
    std::fs::set_permissions(&other, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::remove_file(&file).unwrap();
    std::os::unix::fs::symlink(&other, &file).unwrap();
    let refused = take(&store, "run").unwrap_err();
    assert_eq!(refused, RunLockError::File(file.clone()));
    assert_eq!(
        refused.to_string(),
        format!(
            "run lock: the lock {} is not this user's own single file (decision 0108)",
            file.display()
        )
    );
    std::fs::remove_file(&file).unwrap();
    std::fs::write(&file, "").unwrap();
    std::fs::hard_link(&file, dir.path().join("second")).unwrap();
    assert_eq!(
        take(&store, "run").unwrap_err(),
        RunLockError::File(file.clone())
    );
    std::fs::remove_file(dir.path().join("second")).unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o640)).unwrap();
    assert_eq!(take(&store, "run").unwrap_err(), RunLockError::File(file));
}

/// A run id that would reach outside the lock directory is refused by
/// name, before any path is made.
#[test]
fn a_run_id_that_would_escape_the_directory_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let store = Store::open(&path).unwrap();
    for run in ["../escape", "a/b", "a/", "/absolute", ".", "..", ""] {
        let refused = take(&store, run).unwrap_err();
        assert_eq!(refused, RunLockError::UnsafeId(run.to_string()));
        assert_eq!(
            refused.to_string(),
            format!("run '{run}' is not a safe file name, so it has no run lock")
        );
    }
    assert!(!path.parent().unwrap().join("escape").exists());
    assert!(!runs(&path).exists(), "no directory was made for a refusal");
}

/// A lock file that is held but has recorded nothing refuses as an
/// unrecorded holder rather than guessing its facts.
#[test]
fn a_held_lock_that_recorded_nothing_is_refused_as_that() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let store = Store::open(&path).unwrap();
    let at = directory(&path).unwrap();
    let held = lock_file::opened(&at, "run.lock").unwrap();
    flock(&held, FlockOperation::NonBlockingLockExclusive).unwrap();
    let refused = take(&store, "run").unwrap_err();
    assert_eq!(
        refused,
        RunLockError::Driven {
            run_id: "run".into(),
            holder: None,
        }
    );
    assert_eq!(
        refused.to_string(),
        "run 'run' is driven by another process on this host, which has not recorded its facts \
         yet"
    );
    drop(held);
}

/// A driver in another process holds the run, and a second is refused
/// naming that process; once it is killed, with no lease to expire and
/// nothing else done, the next takes the run.
#[test]
fn a_killed_drivers_lock_lets_the_next_take_the_run() {
    if let Some(journal) = std::env::var_os(CHILD) {
        let journal = PathBuf::from(journal);
        return hold_until_killed(&journal);
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let store = Store::open(&path).unwrap();
    let vars = [(CHILD, path.as_os_str())];
    let marker = dir.path().join("held");
    let mut child = crate::child::spawn_holding(
        "run_lock::tests::a_killed_drivers_lock_lets_the_next_take_the_run",
        &vars,
        &marker,
    );
    let recorded = holder(&path, "run");
    assert_eq!(recorded.pid, child.id());
    let refused = take(&store, "run").unwrap_err();
    assert_eq!(refused.to_string(), driven("run", &recorded));
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(take(&store, "run").is_ok(), "the next driver takes it");
}

/// The re-executed child: take the run's lock and hold it until killed.
fn hold_until_killed(journal: &Path) {
    let store = Store::open(journal).unwrap();
    let _held = take(&store, "run").unwrap();
    std::fs::write(journal.with_file_name("held"), "").unwrap();
    std::thread::sleep(Duration::from_secs(300));
}

/// A lock directory that cannot even be made refuses by its own path.
#[test]
fn a_lock_directory_that_cannot_be_made_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing/forge.db");
    let facts = Facts {
        host: None,
        since: "2026-11-13T00:00:00Z".into(),
        engine: ENGINE_VERSION.into(),
    };
    let refused = take_at(&path, "run", &facts).unwrap_err();
    assert_eq!(refused, RunLockError::Directory(runs(&path)));
}
