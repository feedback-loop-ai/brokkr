//! One dispatcher per journal (decision 0095 ruling 2): a second is
//! refused at the lock, naming the holder, the lock goes with the process
//! that held it, a lock file that is not this user's own single file
//! refuses, and a dispatcher whose lock file was removed under it is
//! fenced out of the journal by the next.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use brokkr_store::{Attribution, Generation, NewEntry, QueueRefusal, Store, StoreError};
use rustix::fs::{flock, FlockOperation, CWD};

use super::*;
use crate::lock_file;

/// Set, to the journal's path, in a re-executed test binary that is to
/// hold the journal as its dispatcher until it is killed.
const CHILD: &str = "BROKKR_TEST_DISPATCH_CHILD";

/// A new journal in `dir`, and a store on it.
fn journal(dir: &Path) -> (PathBuf, Store) {
    let path = dir.join("forge.db");
    let store = Store::open(&path).unwrap();
    (path, store)
}

/// When the journal at `path` recorded the holder of `generation`.
fn since(path: &Path, generation: i64) -> String {
    let conn = rusqlite::Connection::open(path).unwrap();
    let query = "SELECT started_at FROM dispatch_holders WHERE generation = ?1";
    conn.query_row(query, [generation], |row| row.get(0))
        .unwrap()
}

/// The holder rows the journal at `path` holds.
fn holders(path: &Path) -> i64 {
    let conn = rusqlite::Connection::open(path).unwrap();
    let query = "SELECT COUNT(*) FROM dispatch_holders";
    conn.query_row(query, [], |row| row.get(0)).unwrap()
}

/// The refusal of a dispatcher of `path`, which `holder` holds.
fn dispatched(path: &Path, holder: &str) -> String {
    format!(
        "dispatch: the journal {} is dispatched by {holder}; one dispatcher per journal \
         (decision 0095)",
        path.display()
    )
}

/// A second dispatcher of a journal, through another store and another
/// open of its lock, is refused at once by the holder the journal's latest
/// row names, recording nothing; with the first dropped, it takes the
/// journal as the next generation. A lock held by a process that has not
/// recorded itself yet is refused as that.
#[test]
fn a_second_dispatcher_is_refused_naming_the_one_that_holds_the_journal() {
    let dir = tempfile::tempdir().unwrap();
    let (path, mut first) = journal(dir.path());
    let held = take_dispatch(&mut first).unwrap().unwrap();
    assert_eq!(held.generation, Generation(1));
    let mut second = Store::open(&path).unwrap();
    let refused = take_dispatch(&mut second).unwrap().unwrap_err();
    let holder = format!(
        "generation 1 (pid {} on this host since {})",
        std::process::id(),
        since(&path, 1)
    );
    assert!(matches!(&refused, DispatcherError::Dispatched(at, Some(_)) if at == &path));
    assert_eq!(refused.to_string(), dispatched(&path, &holder));
    assert_eq!(holders(&path), 1, "the refused one recorded nothing");
    drop(held);
    let next = take_dispatch(&mut second).unwrap().unwrap();
    assert_eq!(next.generation, Generation(2));
    drop(next);

    let fresh = tempfile::tempdir().unwrap();
    let (path, mut unrecorded) = journal(fresh.path());
    let lock = lock_file::opened(CWD, beside(&path)).unwrap();
    flock(&lock, FlockOperation::NonBlockingLockExclusive).unwrap();
    let refused = take_dispatch(&mut unrecorded).unwrap().unwrap_err();
    assert_eq!(refused, DispatcherError::Dispatched(path.clone(), None));
    let named = "a dispatcher that has not recorded itself yet";
    assert_eq!(refused.to_string(), dispatched(&path, named));
    assert_eq!(holders(&path), 0);
}

/// The dispatch lock of the journal at `path`, by the name it is given.
fn beside(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.dispatch.lock", path.display()))
}

/// The child's side: hold the journal at `journal` as its dispatcher, say
/// so by `held` beside it, and wait to be killed.
fn hold_until_killed(journal: &Path) {
    let mut store = Store::open(journal).unwrap();
    let _dispatcher = take_dispatch(&mut store).unwrap().unwrap();
    std::fs::write(journal.with_file_name("held"), "").unwrap();
    std::thread::sleep(Duration::from_secs(300));
}

/// Liveness is the lock: a dispatcher in another process holds the
/// journal, and a second is refused naming that process; once it is
/// killed, with no lease to expire and nothing else done, the next takes
/// the journal as the next generation.
#[test]
fn a_killed_dispatchers_lock_lets_the_next_take_the_journal() {
    if let Some(journal) = std::env::var_os(CHILD) {
        return hold_until_killed(Path::new(&journal));
    }
    let dir = tempfile::tempdir().unwrap();
    let (path, mut store) = journal(dir.path());
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            // The full libtest path: under `--exact`, a bare name matches
            // nothing, and the child would hold nothing.
            "dispatch::tests::a_killed_dispatchers_lock_lets_the_next_take_the_journal",
            "--exact",
            "--test-threads=1",
        ])
        .env(CHILD, &path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    while !dir.path().join("held").exists() {
        assert!(
            Instant::now() < deadline,
            "the child never held the journal"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let refused = take_dispatch(&mut store).unwrap().unwrap_err();
    let holder = format!(
        "generation 1 (pid {} on this host since {})",
        child.id(),
        since(&path, 1)
    );
    assert_eq!(refused.to_string(), dispatched(&path, &holder));
    child.kill().unwrap();
    child.wait().unwrap();
    let next = take_dispatch(&mut store).unwrap().unwrap();
    assert_eq!(next.generation, Generation(2));
}

/// A dispatch lock that is a symlink, of two links, or one others may
/// read refuses the dispatcher by the lock's own path, recording nothing.
#[test]
fn a_lock_that_is_not_this_users_own_single_file_refuses_the_dispatcher() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let (path, mut store) = journal(dir.path());
    let lock = beside(&path);
    let refused = || {
        let text = format!(
            "dispatch: the lock {} is not this user's own single file, so the journal was not \
             dispatched (decision 0095)",
            lock.display()
        );
        (DispatcherError::Lock(lock.clone()), text)
    };
    // A link to a file that is this user's alone, so only the link refuses.
    let elsewhere = dir.path().join("elsewhere");
    std::fs::write(&elsewhere, "").unwrap();
    std::fs::set_permissions(&elsewhere, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &lock).unwrap();
    let symlinked = take_dispatch(&mut store).unwrap().unwrap_err();
    assert_eq!((symlinked.clone(), symlinked.to_string()), refused());
    std::fs::remove_file(&lock).unwrap();
    std::fs::write(&lock, "").unwrap();
    std::fs::hard_link(&lock, dir.path().join("second")).unwrap();
    let linked = take_dispatch(&mut store).unwrap().unwrap_err();
    assert_eq!((linked.clone(), linked.to_string()), refused());
    std::fs::remove_file(dir.path().join("second")).unwrap();
    std::fs::set_permissions(&lock, std::fs::Permissions::from_mode(0o644)).unwrap();
    let shared = take_dispatch(&mut store).unwrap().unwrap_err();
    assert_eq!((shared.clone(), shared.to_string()), refused());
    assert_eq!(holders(&path), 0);
}

/// A lock file removed and made again under a live dispatcher admits a
/// second, but not past the journal: the second's holder row fences the
/// first out, whose claim is refused by variant with nothing written,
/// while the second's lands.
#[test]
fn a_dispatcher_whose_lock_file_was_removed_is_fenced_out_by_the_next() {
    let dir = tempfile::tempdir().unwrap();
    let (path, mut store) = journal(dir.path());
    let by = Attribution {
        operator: "vy",
        reason: "w",
    };
    let new = NewEntry {
        payload: "launch",
        priority: 0,
        waits: &[],
    };
    let entry = store.queue_add(new, by).unwrap();
    let first = take_dispatch(&mut store).unwrap().unwrap();
    std::fs::remove_file(beside(&path)).unwrap();
    let mut other = Store::open(&path).unwrap();
    let second = take_dispatch(&mut other).unwrap().unwrap();
    assert_eq!(
        (first.generation, second.generation),
        (Generation(1), Generation(2))
    );
    let read = store.queue_entry(entry).unwrap();
    let judge = |_, _: &[_], _: &[_]| Ok::<(), ()>(());
    let fenced = store.queue_claim_with(&read, 0, "d", first.generation, "r", judge);
    let refused = QueueRefusal::Superseded(Generation(1));
    assert!(matches!(&fenced, Err(StoreError::Queue(q)) if q == &refused));
    assert_eq!(store.queue_entry(entry).unwrap(), read, "nothing written");
    let claimed = other.queue_claim_with(&read, 0, "d", second.generation, "r", judge);
    assert!(matches!(claimed, Ok(Ok(_))), "{claimed:?}");
}
