//! A hands session's scratch tree, and the reaper for a tree whose owner
//! died (#415).
//!
//! A hands server, an exec box and doctor's probe each hold a session
//! directory, `brokkr-hands-<label>-<pid>-<uuid>` under the temporary
//! directory, for what outlives one call: overlay upper layers. The owner
//! holds an advisory lock on the tree for its whole life and removes the
//! tree when it ends, on a termination signal included. A SIGKILLed owner
//! can do neither, and on a RAM-backed `/tmp` its tree costs memory until
//! something removes it: the engine reaps at every drive. A tree is
//! reaped only when BOTH the pid its name records is dead AND no process
//! holds its lock, so a live owner's tree is never touched, whether its
//! pid was reused, it is a pre-#415 server that took no lock, or it runs
//! in a pid namespace whose pids mean nothing here.

use std::fs::File;
use std::path::{Path, PathBuf};

use rustix::fs::FlockOperation;
use rustix::process::Pid;
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use thiserror::Error;

const PREFIX: &str = "brokkr-hands-";
/// The file inside the tree whose lock the owner holds for its life.
const LOCK: &str = ".owner.lock";
/// The signals that end a server by default and that it can observe.
const TERMINATION: [i32; 3] = [SIGTERM, SIGINT, SIGHUP];

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("hands session: {0}")]
    Io(#[from] std::io::Error),
}

/// The hands module's callers report in text; this is where a session's
/// error becomes it, once.
impl From<SessionError> for String {
    fn from(error: SessionError) -> String {
        error.to_string()
    }
}

/// A session tree, locked while this value lives and removed when it
/// drops.
#[derive(Debug)]
pub struct Session {
    dir: PathBuf,
    _lock: File,
}

impl Session {
    /// A new session under the temporary directory (`TMPDIR` honoured).
    pub fn create(label: &str) -> Result<Session, SessionError> {
        Session::create_in(&std::env::temp_dir(), label)
    }

    fn create_in(tmp: &Path, label: &str) -> Result<Session, SessionError> {
        let dir = tmp.join(format!(
            "{PREFIX}{label}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir)?;
        let lock = File::create(dir.join(LOCK))?;
        // Not `?`: an exclusive lock on a file this process just created
        // cannot be contended, and a filesystem refusing locks altogether
        // still leaves the tree guarded by this live pid in its name.
        let _ = rustix::fs::flock(&lock, FlockOperation::NonBlockingLockExclusive);
        Ok(Session { dir, _lock: lock })
    }

    pub fn path(&self) -> &Path {
        &self.dir
    }

    /// Remove the tree when SIGTERM, SIGINT or SIGHUP ends the process,
    /// which then exits `128 + signal` as a shell reports a signal death.
    /// For a server blocked on its stdin: its `Drop` never runs then.
    pub fn remove_on_termination(&self) -> Result<(), SessionError> {
        let mut signals = Signals::new(TERMINATION)?;
        let dir = self.dir.clone();
        std::thread::spawn(move || {
            // `forever` ends only when the handle is closed, and nothing
            // closes it: the first item is the first signal.
            let signal = signals.forever().next().unwrap_or(SIGTERM);
            let _ = std::fs::remove_dir_all(&dir);
            std::process::exit(128 + signal);
        });
        Ok(())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Remove every session tree under the temporary directory whose owner
/// is dead, returning the trees removed.
pub fn reap_dead_sessions() -> Vec<PathBuf> {
    reap_dead_sessions_in(&std::env::temp_dir())
}

fn reap_dead_sessions_in(tmp: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(tmp)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .filter(|tree| orphaned(tree))
        .filter(|tree| std::fs::remove_dir_all(tree).is_ok())
        .collect()
}

/// A tree this module named, whose owner pid is dead and whose lock no
/// process holds. A name it did not write is never an orphan.
fn orphaned(tree: &Path) -> bool {
    let owner = tree
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(owner_pid);
    owner.is_some_and(|pid| !alive(pid)) && unlocked(tree)
}

/// The pid in `brokkr-hands-<label>-<pid>-<uuid>`.
fn owner_pid(name: &str) -> Option<Pid> {
    let rest = name.strip_prefix(PREFIX)?;
    let at = rest.len().checked_sub(uuid::fmt::Hyphenated::LENGTH)?;
    uuid::Uuid::try_parse(rest.get(at..)?).ok()?;
    let (_label, pid) = rest.get(..at)?.strip_suffix('-')?.rsplit_once('-')?;
    Pid::from_raw(pid.parse().ok()?)
}

/// Dead only when the kernel says no such process: a pid owned by
/// another user answers EPERM, and is alive.
fn alive(pid: Pid) -> bool {
    rustix::process::test_kill_process(pid) != Err(rustix::io::Errno::SRCH)
}

/// No process holds the tree's lock: it takes the lock, or the tree has
/// no lock file (a pre-#415 owner, or one killed before it locked). Any
/// other failure to open it is not an answer, and keeps the tree.
fn unlocked(tree: &Path) -> bool {
    match File::open(tree.join(LOCK)) {
        Ok(lock) => rustix::fs::flock(&lock, FlockOperation::NonBlockingLockExclusive).is_ok(),
        Err(error) => error.kind() == std::io::ErrorKind::NotFound,
    }
}

#[cfg(test)]
mod tests;
