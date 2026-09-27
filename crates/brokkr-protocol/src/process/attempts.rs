//! The engine's live attempts (#403): each attempt's process group and
//! the descendants recorded under it while its tree is alive, so both
//! the attempt's own end and the engine's stop reach all of it.
//!
//! Starting the first attempt makes the engine a child subreaper on
//! Linux, starts the thread that records every live attempt's
//! descendants, and installs the stop handler: SIGINT, SIGTERM and SIGHUP
//! end every live attempt before the engine exits. The driver leads a
//! group of its own, so a signal to the engine's group, a terminal's
//! Ctrl-C or hangup, no longer reaches it; the handler is what does.

use std::collections::{BTreeMap, BTreeSet};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, Once, PoisonError};
use std::time::Duration;

use rustix::io::Errno;
use rustix::process::{getpid, Pid};
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

use super::table::{self, Entry, Identity};
use super::tree::{self, Unsettled};

/// How often every live attempt's tree is read and recorded.
const TRACK: Duration = Duration::from_millis(100);

struct Live {
    group: Pid,
    /// Is the leader unreaped, so its pid still names this group? Cleared
    /// by `close`, before the reap, under the lock the stop handler takes.
    open: bool,
    recorded: BTreeSet<Identity>,
    /// Orphans the engine had adopted before this attempt began: not its.
    before: BTreeSet<Identity>,
}

static LIVE: Mutex<BTreeMap<u64, Live>> = Mutex::new(BTreeMap::new());
static NEXT: AtomicU64 = AtomicU64::new(0);
static START: Once = Once::new();

fn live() -> MutexGuard<'static, BTreeMap<u64, Live>> {
    LIVE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// One live attempt. Dropping it forgets the attempt.
pub(super) struct Attempt {
    key: u64,
}

impl Attempt {
    /// Spawn the driver and register its group under one lock, so the
    /// stop handler never misses a driver that is already running.
    pub(super) fn spawn(builder: &mut Command) -> std::io::Result<(Child, Attempt)> {
        START.call_once(start);
        let before = table::snapshot().unwrap_or_default();
        let before = adopted(&before).map(|entry| entry.id.clone()).collect();
        let mut live = live();
        let child = builder.spawn()?;
        let key = NEXT.fetch_add(1, Ordering::Relaxed);
        let group = Pid::from_child(&child);
        live.insert(
            key,
            Live {
                group,
                open: true,
                recorded: BTreeSet::new(),
                before,
            },
        );
        Ok((child, Attempt { key }))
    }

    /// SIGKILL what the attempt recorded, for the watchdog, which kills
    /// the group itself.
    pub(super) fn kill_recorded(key: u64) {
        live().get(&key).into_iter().for_each(|live| {
            live.recorded.iter().for_each(table::kill);
        });
    }

    pub(super) fn key(&self) -> u64 {
        self.key
    }

    /// Kill the group and every recorded descendant, then close the group
    /// to signals: its leader is about to be reaped. The group kill's
    /// refusal, if the kernel gave one.
    pub(super) fn close(&self, kill_group: fn(Pid) -> rustix::io::Result<()>) -> Option<Errno> {
        let mut live = live();
        let this = live.get_mut(&self.key).expect("a registered attempt");
        let refused = this.kill(kill_group);
        this.open = false;
        refused
    }

    /// What of the attempt `entries` shows still running: its group, its
    /// recorded descendants, and an adopted orphan nothing recorded. The
    /// attempt's own adopted zombies are reaped on the way.
    pub(super) fn survivors(&self, entries: &[Entry]) -> Result<(), Unsettled> {
        let live = live();
        let this = &live[&self.key];
        this.reap(entries);
        let group = this.group.as_raw_pid();
        let running = || entries.iter().filter(|entry| !entry.zombie);
        if running().any(|entry| entry.pgid == group) {
            return Err(Unsettled::Group { group });
        }
        let descendants = pids(running().filter(|entry| this.recorded.contains(&entry.id)));
        if !descendants.is_empty() {
            return Err(Unsettled::Descendants { pids: descendants });
        }
        let strays = pids(adopted(entries).filter(|entry| {
            !this.before.contains(&entry.id)
                && !live
                    .values()
                    .any(|other| other.recorded.contains(&entry.id))
        }));
        if !strays.is_empty() {
            return Err(Unsettled::Strays { pids: strays });
        }
        Ok(())
    }
}

impl Drop for Attempt {
    fn drop(&mut self) {
        live().remove(&self.key);
    }
}

impl Live {
    fn kill(&self, kill_group: fn(Pid) -> rustix::io::Result<()>) -> Option<Errno> {
        let refused = self.open.then(|| tree::refused(kill_group(self.group)));
        self.recorded.iter().for_each(table::kill);
        refused.flatten()
    }

    /// Record every descendant of the leader, and of what is already
    /// recorded, that `entries` shows running.
    fn record(&mut self, entries: &[Entry]) {
        let mut roots: BTreeSet<i32> = entries
            .iter()
            .filter(|entry| self.recorded.contains(&entry.id))
            .map(|entry| entry.id.pid)
            .collect();
        roots.extend(self.open.then_some(self.group.as_raw_pid()));
        loop {
            let found: Vec<&Entry> = entries
                .iter()
                .filter(|entry| {
                    !entry.zombie && roots.contains(&entry.ppid) && !roots.contains(&entry.id.pid)
                })
                .collect();
            if found.is_empty() {
                return;
            }
            for entry in found {
                roots.insert(entry.id.pid);
                self.recorded.insert(entry.id.clone());
            }
        }
    }

    /// Reap the zombies the engine adopted from this attempt: recorded,
    /// or left in its group. The leader is never one of them: it is the
    /// engine's own child, reaped by its handle.
    fn reap(&self, entries: &[Entry]) {
        let me = getpid().as_raw_pid();
        let group = self.group.as_raw_pid();
        entries
            .iter()
            .filter(|entry| entry.zombie && entry.ppid == me && entry.id.pid != group)
            .filter(|entry| entry.pgid == group || self.recorded.contains(&entry.id))
            .for_each(|entry| table::reap(entry.id.pid));
    }
}

fn pids<'a>(entries: impl Iterator<Item = &'a Entry>) -> Vec<i32> {
    entries.map(|entry| entry.id.pid).collect()
}

/// The orphans the engine adopted as a subreaper: its running children
/// in a session other than its own. A driver the engine spawned leads a
/// group, never a session, so it is never one of them.
fn adopted(entries: &[Entry]) -> impl Iterator<Item = &Entry> {
    let me = getpid().as_raw_pid();
    let session = entries
        .iter()
        .find(|entry| entry.id.pid == me)
        .map(|entry| entry.session.clone());
    entries.iter().filter(move |entry| {
        entry.ppid == me && !entry.zombie && Some(&entry.session) != session.as_ref()
    })
}

/// Once per engine, before its first attempt. A kernel older than 3.4
/// refuses the subreaper; the engine then has what macOS has.
fn start() {
    #[cfg(target_os = "linux")]
    let _ = rustix::process::set_child_subreaper(Some(getpid()));
    stop_on_signals();
    std::thread::spawn(|| loop {
        std::thread::sleep(TRACK);
        let busy = !live().is_empty();
        let entries = busy.then(table::snapshot).and_then(Result::ok);
        let entries = entries.unwrap_or_default();
        live().values_mut().for_each(|live| live.record(&entries));
    });
}

/// SIGINT, SIGTERM and SIGHUP end every live attempt, and the engine then
/// exits 128 plus the signal, as the signal's own default would have
/// ended it. A signal the engine inherited as ignored (`nohup`, a
/// shell's background job) stays ignored.
fn stop_on_signals() {
    let signals: Vec<i32> = [SIGINT, SIGTERM, SIGHUP]
        .into_iter()
        .filter(|signal| !ignored(*signal))
        .collect();
    let mut signals = Signals::new(signals).expect("the engine's stop signals register");
    std::thread::spawn(move || {
        let signal = signals.forever().next().unwrap_or(SIGTERM);
        // Held to the exit, so no attempt registers after the sweep.
        let live = live();
        live.values().for_each(|live| {
            live.kill(tree::kill_group);
        });
        std::process::exit(128 + signal)
    });
}

fn ignored(signal: i32) -> bool {
    let mut current = std::mem::MaybeUninit::<libc::sigaction>::zeroed();
    // SAFETY: a null new action only reads the current one, into memory
    // sized for it and zeroed, so it is initialised whatever the call does.
    unsafe {
        libc::sigaction(signal, std::ptr::null(), current.as_mut_ptr());
        current.assume_init().sa_sigaction == libc::SIG_IGN
    }
}

#[cfg(test)]
mod tests;
