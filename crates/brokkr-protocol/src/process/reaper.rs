//! The driver's reaper (#1133). The engine makes every driver a child
//! subreaper (`attempts::detach`), and the setting survives the exec into
//! `brokkr driver`, so every orphan of the harness's tree is reparented to
//! the driver. The driver waits only on the harness it spawned; this
//! reaper reaps the rest, so an adopted orphan does not hold its pid until
//! the attempt ends.
//!
//! The registry holds the pid of every harness the driver holds a handle
//! to. [`spawn`] registers it under the registry's lock, in the critical
//! section of the spawn itself, and each sweep takes the same lock, so a
//! harness that exits at once is never seen unregistered. A registered pid
//! is left to its handle, whose `wait` must still return the harness's own
//! status; it is released once a sweep no longer finds it among the
//! driver's children, which only its handle's reap can do. A `waitpid(-1)`
//! loop would take that status, and is not one.
//!
//! The driver also runs short commands of its own outside the registry (a
//! harness's `--version`, dsh's composition probe), each reaped by its own
//! `output()`. A zombie is therefore reaped only on the sweep after the one
//! that first saw it: an unregistered child its own handle reaps at once
//! is gone by then. Reaping is best effort: a read that fails is skipped,
//! and tried again on the next interval. Linux only: off Linux no driver
//! is a subreaper (decision 0063).

use std::collections::BTreeSet;
use std::process::{Child, Command};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use rustix::process::{getpid, Pid};

use super::table::{self, Entry, Identity};

/// How long the reaper sleeps between sweeps.
const INTERVAL: Duration = Duration::from_secs(1);

/// The pids the driver holds handles to, and the unregistered zombies the
/// last sweep saw.
#[derive(Debug, Default)]
struct Handled {
    pids: BTreeSet<i32>,
    seen: BTreeSet<Identity>,
}

static HANDLED: Mutex<Handled> = Mutex::new(Handled {
    pids: BTreeSet::new(),
    seen: BTreeSet::new(),
});

/// The registry, whatever a panic under its lock left: it holds no
/// invariant a half-made change could break.
fn handled() -> MutexGuard<'static, Handled> {
    HANDLED.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Spawn `builder` as a handled child: its pid is registered before the
/// lock is released, so no sweep can reap it.
pub(crate) fn spawn(builder: &mut Command) -> std::io::Result<Child> {
    let mut handled = handled();
    let child = builder.spawn()?;
    handled.pids.insert(Pid::from_child(&child).as_raw_pid());
    Ok(child)
}

/// Start the reaper, once per driver (`adapters::serve`). A thread the
/// host refuses leaves the driver unreaped, as it was before.
pub(crate) fn start() {
    let _ = std::thread::Builder::new()
        .name("reaper".to_string())
        .spawn(run);
}

/// The reaper thread's body: a sweep every [`INTERVAL`], for as long as
/// the driver runs.
fn run() {
    loop {
        std::thread::sleep(INTERVAL);
        sweep();
    }
}

/// One sweep, under the registry's lock: read the driver's children, and
/// reap each zombie [`Handled::doomed`] names.
fn sweep() {
    let mut handled = handled();
    let rows = table::children_of(getpid().as_raw_pid());
    handled
        .doomed(rows.as_deref())
        .into_iter()
        .for_each(table::reap);
}

impl Handled {
    /// The pids to reap, given `rows`, the driver's children: each zombie
    /// no handle holds that the last sweep saw too. A registered pid no
    /// longer among them was reaped by its handle, and is released. `None`,
    /// a read that failed, reaps nothing and changes nothing.
    fn doomed(&mut self, rows: Option<&[Entry]>) -> Vec<i32> {
        let Some(rows) = rows else {
            return Vec::new();
        };
        self.pids
            .retain(|pid| rows.iter().any(|row| row.id.pid == *pid));
        let zombies: BTreeSet<Identity> = rows
            .iter()
            .filter(|row| row.zombie && !self.pids.contains(&row.id.pid))
            .map(|row| row.id.clone())
            .collect();
        let doomed = zombies.intersection(&self.seen).map(|id| id.pid).collect();
        self.seen = zombies;
        doomed
    }
}

#[cfg(test)]
mod tests;
