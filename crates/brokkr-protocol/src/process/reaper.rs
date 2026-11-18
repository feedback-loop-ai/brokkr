//! The driver's reaper (#1133). The engine makes every driver a child
//! subreaper (`attempts::detach`), and the setting survives the exec into
//! `brokkr driver`, so every orphan of the harness's tree is reparented to
//! the driver. The driver waits only on the children it spawned; this
//! reaper reaps the rest, so an adopted orphan does not hold its pid until
//! the attempt ends.
//!
//! The registry holds every child a serving driver holds a handle to:
//! each spawns through [`spawn`] (`process::spawned`), which registers it
//! under the registry's lock, in the critical section of the spawn itself,
//! and each sweep takes the same lock, so a child that exits at once is
//! never seen unregistered. A registered child is left to its handle,
//! whose `wait` must still return the child's own status. It is held by
//! what the spawn read while the driver still owned it, before any wait:
//! a pidfd, released once the kernel says its process is no longer the
//! driver's child, which only its handle's reap makes so, and which no
//! reuse of its pid can move. Where no pidfd opens, it is held by its
//! start, released once its row is gone or names a process that started
//! at another time. A `waitpid(-1)` loop would take a handle's status, and
//! is not one.
//!
//! A zombie is reaped on the sweep after the one that first saw it, so a
//! child spawned past the door is still left a moment to its own handle.
//! Reaping is best effort: a read that fails is skipped, and tried again
//! on the next interval. Linux only: off Linux no driver is a subreaper
//! (decision 0063).

use std::collections::{BTreeMap, BTreeSet};
use std::io::Result;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::process::{Child, Command};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use rustix::io::Errno;
use rustix::process::{getpid, pidfd_open, waitid, Pid, PidfdFlags, WaitId, WaitIdOptions};

use super::table::{self, Entry, Identity};

/// How long the reaper sleeps between sweeps.
const INTERVAL: Duration = Duration::from_secs(1);

/// The children a serving driver holds handles to, and the unregistered
/// zombies the last sweep saw.
#[derive(Debug, Default)]
struct Handled {
    /// Whether this process serves as a driver: only then is a child held,
    /// so no other process keeps a registry no sweep empties.
    serving: bool,
    /// Each held child by pid, with what its spawn read of it.
    held: BTreeMap<i32, Hold>,
    seen: BTreeSet<Identity>,
}

/// What a held child is known by: read at its spawn, while the driver
/// still owned it and no wait could have reaped it.
#[derive(Debug)]
enum Hold {
    /// A pidfd: the process itself, whatever later reuses its pid.
    Pidfd(OwnedFd),
    /// Its start, where no pidfd opened.
    Start(String),
    /// Neither read: held until its row is gone.
    Unread,
}

static HANDLED: Mutex<Handled> = Mutex::new(Handled {
    serving: false,
    held: BTreeMap::new(),
    seen: BTreeSet::new(),
});

/// The registry, whatever a panic under its lock left: it holds no
/// invariant a half-made change could break.
fn handled() -> MutexGuard<'static, Handled> {
    HANDLED.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Spawn `builder` as a held child, under the registry's lock.
pub(super) fn spawn(builder: &mut Command) -> Result<Child> {
    handled().spawn(builder, pidfd, table::stat_of)
}

/// A pidfd of `pid`, the driver's own child.
fn pidfd(pid: Pid) -> Result<OwnedFd> {
    Ok(pidfd_open(pid, PidfdFlags::empty())?)
}

/// Whether the process `pidfd` names is no longer the driver's child: its
/// handle reaped it. The wait neither blocks nor reaps; a wait that fails
/// otherwise says nothing, and keeps it.
fn reaped(pidfd: BorrowedFd<'_>) -> bool {
    let options = WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT;
    waitid(WaitId::PidFd(pidfd), options).err() == Some(Errno::CHILD)
}

/// Start the reaper, once per process however often a driver serves
/// (`adapters::serve`). A thread the host refuses leaves the driver
/// unreaped, as it was before.
pub(crate) fn start() {
    if handled().serve() {
        let _ = std::thread::Builder::new()
            .name("reaper".to_string())
            .spawn(run);
    }
}

/// The reaper thread's body: a sweep every [`INTERVAL`], for as long as
/// the driver runs.
fn run() {
    loop {
        std::thread::sleep(INTERVAL);
        sweep();
    }
}

/// One sweep, under the registry's lock: release each child its handle
/// reaped, read the driver's children, and reap each zombie
/// [`Handled::doomed`] names.
fn sweep() {
    let mut handled = handled();
    handled.retire(table::stat_of);
    let rows = table::children_of(getpid().as_raw_pid());
    handled
        .doomed(rows.as_deref())
        .into_iter()
        .for_each(table::reap);
}

impl Handled {
    /// Mark this process a serving driver: whether it was not one yet, so
    /// its reaper is still to start.
    fn serve(&mut self) -> bool {
        !std::mem::replace(&mut self.serving, true)
    }

    /// Spawn `builder` and, in a serving driver, hold the child before
    /// `self`'s lock is released: by the pidfd `open` gives, or else by its
    /// start as `stat` reads it.
    fn spawn(
        &mut self,
        builder: &mut Command,
        open: impl Fn(Pid) -> Result<OwnedFd>,
        stat: impl Fn(i32) -> Result<Option<Entry>>,
    ) -> Result<Child> {
        let child = builder.spawn()?;
        if self.serving {
            let pid = Pid::from_child(&child);
            let hold = match open(pid) {
                Ok(pidfd) => Hold::Pidfd(pidfd),
                Err(_) => match stat(pid.as_raw_pid()) {
                    Ok(Some(row)) => Hold::Start(row.id.start),
                    _ => Hold::Unread,
                },
            };
            self.held.insert(pid.as_raw_pid(), hold);
        }
        Ok(child)
    }

    /// Release each held child whose handle reaped it: by its pidfd, or by
    /// `stat`, its own row now, gone or another process's. A read that
    /// fails keeps it.
    fn retire(&mut self, stat: impl Fn(i32) -> Result<Option<Entry>>) {
        self.held.retain(|&pid, hold| match hold {
            Hold::Pidfd(pidfd) => !reaped(pidfd.as_fd()),
            Hold::Start(start) => match stat(pid) {
                Ok(Some(row)) => *start == row.id.start,
                Ok(None) => false,
                Err(_) => true,
            },
            Hold::Unread => !matches!(stat(pid), Ok(None)),
        });
    }

    /// The pids to reap, given `rows`, the driver's children: each zombie
    /// no handle holds that the last sweep saw too. `None`, a read that
    /// failed, reaps nothing and changes nothing.
    fn doomed(&mut self, rows: Option<&[Entry]>) -> Vec<i32> {
        let Some(rows) = rows else {
            return Vec::new();
        };
        let zombies: BTreeSet<Identity> = rows
            .iter()
            .filter(|row| row.zombie && !self.held.contains_key(&row.id.pid))
            .map(|row| row.id.clone())
            .collect();
        let doomed = zombies.intersection(&self.seen).map(|id| id.pid).collect();
        self.seen = zombies;
        doomed
    }
}

#[cfg(test)]
mod tests;
