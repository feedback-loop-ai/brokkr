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
//! at another time; where the kernel will not wait on its pidfd, until its
//! row is gone. Whatever it is held by, its handle's [`Release`] releases
//! it when the handle goes. A `waitpid(-1)` loop would take a handle's
//! status, and is not one.
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
use std::thread::JoinHandle;
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
    held: BTreeMap<i32, Held>,
    seen: BTreeSet<Identity>,
    /// The last hold's token.
    tokens: u64,
}

/// A held child: the token its handle's [`Release`] names, so a handle
/// that outlives its reap never releases a later child of its pid, and
/// what the child is known by.
#[derive(Debug)]
struct Held {
    token: u64,
    hold: Hold,
}

/// What a held child is known by: read at its spawn, while the driver
/// still owned it and no wait could have reaped it.
#[derive(Debug)]
enum Hold {
    /// A pidfd: the process itself, whatever later reuses its pid.
    Pidfd(OwnedFd),
    /// Its start, where no pidfd opened.
    Start(String),
    /// Neither read, or a pidfd the kernel will not wait on: held until
    /// its row is gone.
    Unread,
}

static HANDLED: Mutex<Handled> = Mutex::new(Handled {
    serving: false,
    held: BTreeMap::new(),
    seen: BTreeSet::new(),
    tokens: 0,
});

/// The registry, whatever a panic under its lock left: it holds no
/// invariant a half-made change could break.
fn handled() -> MutexGuard<'static, Handled> {
    HANDLED.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A held child's handle's claim on its hold: dropped with the handle,
/// it releases the hold, whatever kind it is, so no hold outlasts the
/// handle that may wait on its pid. A child no serving driver held claims
/// nothing.
#[derive(Debug)]
pub(crate) struct Release(Option<(i32, u64)>);

impl Drop for Release {
    fn drop(&mut self) {
        if let Some((pid, token)) = self.0 {
            handled().release(pid, token);
        }
    }
}

/// Spawn `builder` as a held child, under the registry's lock.
pub(super) fn spawn(builder: &mut Command) -> Result<(Child, Release)> {
    let (child, token) = handled().spawn(builder, pidfd, table::stat_of)?;
    let pid = Pid::from_child(&child).as_raw_pid();
    Ok((child, Release(token.map(|token| (pid, token)))))
}

/// A pidfd of `pid`, the driver's own child.
fn pidfd(pid: Pid) -> Result<OwnedFd> {
    Ok(pidfd_open(pid, PidfdFlags::empty())?)
}

/// The kernel's wait on the process `pidfd` names, which neither blocks
/// nor reaps: `ECHILD` once it is no longer the driver's child, its
/// handle having reaped it.
fn waited(pidfd: BorrowedFd<'_>) -> rustix::io::Result<()> {
    let options = WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT;
    waitid(WaitId::PidFd(pidfd), options).map(drop)
}

/// Start the reaper, once per process however often a driver serves
/// (`adapters::serve`).
pub(crate) fn start() {
    handled().start(|run| {
        std::thread::Builder::new()
            .name("reaper".to_string())
            .spawn(run)
    });
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
    handled.retire(waited, table::stat_of);
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

    /// Serve, and start the reaper through `thread` if none runs yet. A
    /// thread the host refuses leaves the driver unreaped, as it was
    /// before, and not serving, so it holds nothing no sweep releases.
    fn start(&mut self, thread: impl FnOnce(fn()) -> Result<JoinHandle<()>>) {
        if self.serve() && thread(run).is_err() {
            self.serving = false;
        }
    }

    /// Spawn `builder` and, in a serving driver, hold the child before
    /// `self`'s lock is released: by the pidfd `open` gives, or else by its
    /// start as `stat` reads it. The token of the hold, where one is held.
    fn spawn(
        &mut self,
        builder: &mut Command,
        open: impl Fn(Pid) -> Result<OwnedFd>,
        stat: impl Fn(i32) -> Result<Option<Entry>>,
    ) -> Result<(Child, Option<u64>)> {
        let child = builder.spawn()?;
        if !self.serving {
            return Ok((child, None));
        }
        let pid = Pid::from_child(&child);
        let hold = match open(pid) {
            Ok(pidfd) => Hold::Pidfd(pidfd),
            Err(_) => match stat(pid.as_raw_pid()) {
                Ok(Some(row)) => Hold::Start(row.id.start),
                _ => Hold::Unread,
            },
        };
        self.tokens += 1;
        let token = self.tokens;
        self.held.insert(pid.as_raw_pid(), Held { token, hold });
        Ok((child, Some(token)))
    }

    /// Release `pid`'s hold, if it is still the one `token` names.
    fn release(&mut self, pid: i32, token: u64) {
        if self.held.get(&pid).is_some_and(|held| held.token == token) {
            self.held.remove(&pid);
        }
    }

    /// Release each held child whose handle reaped it: by its pidfd, as
    /// `wait` answers, or by `stat`, its own row now, gone or another
    /// process's. A read that fails keeps it. A pidfd `wait` refuses
    /// otherwise than with `ECHILD` cannot be waited on: it is closed, and
    /// its child held unread.
    fn retire(
        &mut self,
        wait: impl Fn(BorrowedFd<'_>) -> rustix::io::Result<()>,
        stat: impl Fn(i32) -> Result<Option<Entry>>,
    ) {
        self.held.retain(|&pid, held| match &held.hold {
            Hold::Pidfd(pidfd) => match wait(pidfd.as_fd()) {
                Ok(()) => true,
                Err(Errno::CHILD) => false,
                Err(_) => {
                    held.hold = Hold::Unread;
                    true
                }
            },
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
