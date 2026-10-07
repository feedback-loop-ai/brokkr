//! A hands session's scratch tree, and the reaper for a tree whose owner
//! died (#415).
//!
//! A hands server, an exec box and doctor's probe each hold a session
//! directory, `brokkr-hands-<label>-<pid>-<uuid>` under the temporary
//! directory, for a call's scratch and, in a server's, what outlives one
//! call: overlay upper layers. An exec box writes its overlays to RAM
//! inside the box (#504). The owner
//! holds an advisory lock on the tree for its whole life and removes the
//! tree when it ends, on a termination signal included. A session that
//! cannot take its lock refuses to exist. A SIGKILLed owner can do
//! neither, and on a RAM-backed `/tmp` its tree costs memory until
//! something removes it: every run, resume and rerun reaps before it
//! drives. A tree is reaped only when BOTH the pid its name records reads
//! dead here AND no process holds its lock, so a LOCKED tree is never
//! touched, whether its owner's pid was reused or it runs in a pid
//! namespace whose pids mean nothing here. A session's own tree is never
//! seen unlocked: it is built under a name the reaper does not read and
//! renamed into place once its lock is held.
//!
//! A tree with NO lock file is not protected. It was left by a server
//! from before #415, which took no lock, and it is reaped when its
//! recorded pid reads dead here. That covers a live pre-#415 server in a
//! foreign pid namespace sharing the temporary directory: its tree can be
//! removed under it. The operator's ruling of 2026-09-28 accepted that
//! risk for a one-release transition, from 0.12.0 until 0.13.0 ends it;
//! from then on a lockless tree is kept and said, never reaped. A lock
//! the reaper cannot probe is no answer: it keeps the tree and says why.

use std::fmt;
use std::fs::File;
use std::os::fd::{AsFd, BorrowedFd};
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};

use rustix::fs::{FileType, FlockOperation, Mode, OFlags};
use rustix::io::Errno;
use rustix::process::Pid;
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use thiserror::Error;

const PREFIX: &str = "brokkr-hands-";
/// What a session's tree is named before its lock is held: a prefix the
/// reaper never reads as a session's.
const STAGING: &str = ".staging-";
/// The file inside the tree whose lock the owner holds for its life.
const LOCK: &str = ".owner.lock";
/// The signals that end a server by default and that it can observe.
const TERMINATION: [i32; 3] = [SIGTERM, SIGINT, SIGHUP];

/// Take a file's exclusive lock without waiting. The one seam a test
/// fails: a filesystem that refuses locks cannot be planted.
type Flock = fn(BorrowedFd<'_>) -> Result<(), Errno>;

fn flock(fd: BorrowedFd<'_>) -> Result<(), Errno> {
    rustix::fs::flock(fd, FlockOperation::NonBlockingLockExclusive)
}

/// Create a session's lock file. The one seam a test fails: a
/// filesystem that refuses the create — `ENOSPC` or `EDQUOT` on a full
/// RAM `/tmp`, `EMFILE` — cannot be planted.
type Create = fn(&Path) -> std::io::Result<File>;

fn create_lock(path: &Path) -> std::io::Result<File> {
    File::create(path)
}

/// Make one of a call's private directories in the session's scratch
/// tree, owner-only whatever the caller's umask (#570). A
/// `create_dir_all` directory takes the umask's mode: under 002, Ubuntu's
/// default, the box's `/tmp` and its home would be group-writable 775,
/// and the capability broker's ancestry guard refuses every plan whose
/// path walks a group-writable directory. The mode goes to mkdir(2)
/// itself, which no umask can widen.
pub(super) fn private_dir(path: &Path) -> std::io::Result<()> {
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
}

/// An errno in the kernel's words, as `std::io::Error` prints it.
fn said(errno: Errno) -> std::io::Error {
    std::io::Error::from_raw_os_error(errno.raw_os_error())
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("hands session: {0}")]
    Io(#[from] std::io::Error),
    #[error("hands session: cannot lock {}: {cause}", path.display())]
    Lock {
        path: PathBuf,
        cause: std::io::Error,
    },
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
    lock: File,
}

impl Session {
    /// A new session under the temporary directory (`TMPDIR` honoured).
    pub fn create(label: &str) -> Result<Session, SessionError> {
        Session::create_in(&std::env::temp_dir(), label, flock, create_lock)
    }

    fn create_in(
        tmp: &Path,
        label: &str,
        lock: Flock,
        create: Create,
    ) -> Result<Session, SessionError> {
        let name = format!(
            "{PREFIX}{label}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        );
        // Built under a name `owner_pid` rejects, so no reaper considers
        // the tree before its lock is held, and renamed into place after.
        let staged = tmp.join(format!("{STAGING}{name}"));
        let dir = tmp.join(name);
        std::fs::create_dir_all(&staged)?;
        let lock_file = create(&staged.join(LOCK)).inspect_err(|_| {
            // The directory removed is the one this call made a line
            // above, under a name `owner_pid` rejects, so no reaper
            // would ever collect it: a lock file that cannot be created
            // must not leave it behind. `remove_dir` needs no descriptor,
            // so it clears the empty tree even when the create failed for
            // want of one (`EMFILE`), which the walk `remove_dir_all`
            // opens would not survive; the walk is the fallback for a
            // planted directory at `.owner.lock`, which leaves the tree
            // non-empty. The create's own error is the one returned.
            if std::fs::remove_dir(&staged).is_err() {
                let _ = std::fs::remove_dir_all(&staged);
            }
        })?;
        let mut session = Session {
            lock: lock_file,
            dir: staged,
        };
        // A session holding no lock would read as unowned the moment its
        // pid reads as dead, as it does from another pid namespace: it
        // refuses instead, and dropping it removes the tree. The refusal
        // names the tree the operator would have met, not its staged name.
        lock(session.lock.as_fd()).map_err(|errno| SessionError::Lock {
            path: dir.join(LOCK),
            cause: said(errno),
        })?;
        std::fs::rename(&session.dir, &dir)?;
        session.dir = dir;
        Ok(session)
    }

    pub fn path(&self) -> &Path {
        &self.dir
    }

    /// Remove the tree when SIGTERM, SIGINT or SIGHUP ends the process,
    /// which then exits with the code `code` gives the signal, so the
    /// binary's one table of exit codes holds this one too.
    /// For a server blocked on its stdin: its `Drop` never runs then.
    pub fn remove_on_termination(&self, code: fn(i32) -> i32) -> Result<(), SessionError> {
        let mut signals = Signals::new(TERMINATION)?;
        let dir = self.dir.clone();
        std::thread::spawn(move || {
            // `forever` ends only when the handle is closed, and nothing
            // closes it: the first item is the first signal.
            let signal = signals.forever().next().unwrap_or(SIGTERM);
            let _ = std::fs::remove_dir_all(&dir);
            std::process::exit(code(signal));
        });
        Ok(())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Why a reaped tree was removed, as the start prints it.
const REAPED_BECAUSE: &str = "its owner is dead and holds no lock";

/// What one reaping did: the trees it removed, and the trees it kept for
/// want of an answer from their locks. Displayed one line per tree, for
/// the start to print on stderr; nothing is journaled.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Reaped {
    removed: Vec<PathBuf>,
    kept: Vec<(PathBuf, Unprobed)>,
}

impl Reaped {
    /// The reaping said on lines, one per tree, each tree's path through
    /// `safe` before the lines are joined: the one derivation of the
    /// line. `Display` paints it with the identity, and the start's
    /// renderer passes the safe one, so a newline inside a tree's path is
    /// stripped with its control and directional bytes and only the
    /// renderer's own separators remain (#468).
    pub fn lines_with(&self, safe: fn(&str) -> String) -> String {
        let mut said = String::new();
        for tree in &self.removed {
            let tree = safe(&tree.display().to_string());
            said.push_str(&format!("hands: reaped {tree}: {REAPED_BECAUSE}\n"));
        }
        for (tree, why) in &self.kept {
            let tree = safe(&tree.display().to_string());
            said.push_str(&format!(
                "hands: kept {tree}: its lock cannot be probed: {why}\n"
            ));
        }
        said
    }
}

impl fmt::Display for Reaped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.lines_with(|tree| tree.to_owned()))
    }
}

/// Why the reaper could not tell whether a tree's lock is held.
#[derive(Debug, Error, PartialEq, Eq)]
enum Unprobed {
    #[error("{LOCK} is not a regular file")]
    NotRegular,
    #[error("reading {LOCK}: {}", said(*.0))]
    Stat(Errno),
    #[error("opening {LOCK}: {}", said(*.0))]
    Open(Errno),
    #[error("locking {LOCK}: {}", said(*.0))]
    Lock(Errno),
}

/// What the tree's lock answered.
#[derive(Debug, PartialEq, Eq)]
enum Probe {
    Free,
    Held,
    Unprobed(Unprobed),
}

/// Remove every session tree under the temporary directory whose owner
/// is dead, and keep, saying why, each one whose lock cannot be probed.
pub fn reap_dead_sessions() -> Reaped {
    reap_dead_sessions_in(&std::env::temp_dir(), flock)
}

fn reap_dead_sessions_in(tmp: &Path, lock: Flock) -> Reaped {
    let mut reaped = Reaped::default();
    let trees = std::fs::read_dir(tmp)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .filter(|tree| dead_owner(tree));
    for tree in trees {
        match probe(&tree, lock) {
            Probe::Free if std::fs::remove_dir_all(&tree).is_ok() => reaped.removed.push(tree),
            Probe::Free | Probe::Held => {}
            Probe::Unprobed(why) => reaped.kept.push((tree, why)),
        }
    }
    reaped
}

/// A tree this module named, whose owner pid is dead. A name it did not
/// write never has one.
fn dead_owner(tree: &Path) -> bool {
    tree.file_name()
        .and_then(|name| name.to_str())
        .and_then(owner_pid)
        .is_some_and(|pid| !alive(pid))
}

/// The pid in `brokkr-hands-<label>-<pid>-<uuid>`.
fn owner_pid(name: &str) -> Option<Pid> {
    let rest = name.strip_prefix(PREFIX)?;
    let at = rest.len().checked_sub(uuid::fmt::Hyphenated::LENGTH)?;
    uuid::Uuid::try_parse(rest.get(at..)?).ok()?;
    let (_label, pid) = rest.get(..at)?.strip_suffix('-')?.rsplit_once('-')?;
    Pid::from_raw(pid.parse().ok()?)
}

/// Dead only when the kernel says no such process, or that it is a
/// zombie: a pid owned by another user answers EPERM, and is alive.
fn alive(pid: Pid) -> bool {
    !zombie(pid) && rustix::process::test_kill_process(pid) != Err(Errno::SRCH)
}

/// Linux names a zombie by the state after the last `)` of
/// `/proc/<pid>/stat`; one that cannot be read is no zombie.
#[cfg(target_os = "linux")]
fn zombie(pid: Pid) -> bool {
    std::fs::read_to_string(format!("/proc/{}/stat", pid.as_raw_nonzero())).is_ok_and(|stat| {
        stat.rsplit_once(')')
            .is_some_and(|(_, rest)| rest.trim_start().starts_with('Z'))
    })
}

/// macOS has no `/proc`, and stays on `kill(pid, 0)`, which answers a
/// zombie as alive: its tree waits until its parent reaps it.
#[cfg(not(target_os = "linux"))]
fn zombie(_: Pid) -> bool {
    false
}

/// Whether a process holds the tree's lock. A tree with no lock file was
/// left by a pre-#415 server, which took no lock, and reads as free: it
/// is NOT protected, and a live one in a foreign pid namespace sharing the
/// temporary directory, whose pid reads dead here, loses its tree. The
/// operator's ruling of 2026-09-28 accepted that for one release; 0.13.0
/// ends the transition and keeps and says a lockless tree. A lock file
/// that is not a regular file, or that cannot be read, opened or locked
/// for any reason but another's hold, is no answer. Nothing here blocks:
/// the file is opened non-blocking, never through a symlink.
fn probe(tree: &Path, lock: Flock) -> Probe {
    let path = tree.join(LOCK);
    match rustix::fs::lstat(&path) {
        // TODO(0.13.0, #415): the lockless transition ends; keep and say.
        Err(Errno::NOENT) => return Probe::Free,
        Err(errno) => return Probe::Unprobed(Unprobed::Stat(errno)),
        Ok(stat) if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile => {
            return Probe::Unprobed(Unprobed::NotRegular)
        }
        Ok(_) => {}
    }
    let flags = OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    match rustix::fs::open(&path, flags, Mode::empty()) {
        Err(errno) => Probe::Unprobed(Unprobed::Open(errno)),
        Ok(fd) => match lock(fd.as_fd()) {
            Ok(()) => Probe::Free,
            Err(Errno::WOULDBLOCK) => Probe::Held,
            Err(errno) => Probe::Unprobed(Unprobed::Lock(errno)),
        },
    }
}

#[cfg(test)]
mod tests;
