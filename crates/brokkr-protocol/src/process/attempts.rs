//! The engine's live attempts (#403): each attempt's process group and
//! the descendants recorded under it while its tree is alive, so both
//! the attempt's own end and the engine's stop reach all of it.
//!
//! Starting the first attempt makes the engine a child subreaper on
//! Linux, starts the thread that records every live attempt's
//! descendants and attributes the orphans the engine adopts, and installs
//! the stop handler: SIGINT, SIGTERM, SIGHUP and SIGQUIT end every live
//! attempt before the engine exits. The driver leads a session of its
//! own, so a signal to the engine's group, a terminal's Ctrl-C, Ctrl-\ or
//! hangup, no longer reaches it; the handler is what does. Nor does a
//! SIGKILL to the engine's group, which no handler sees.
//!
//! Every read of the table is taken under the registry's lock, so reads
//! are observed in the order they were taken, and "the read before" names
//! one read.

use std::collections::{BTreeMap, BTreeSet};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, Once, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use rustix::process::{getpid, Pid};
use signal_hook::consts::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
use signal_hook::iterator::Signals;

use super::table::{self, Entry, Identity, TableError};
use super::tree::{self, Bounds, Host, Unsettled};

/// How often every live attempt's tree is read and recorded.
const TRACK: Duration = Duration::from_millis(100);

/// The engine's exit status when its stop could not prove every live
/// attempt over; 128 plus the signal says it could.
const UNPROVEN_STOP: i32 = 125;

struct Live {
    group: Pid,
    /// Is the leader unreaped, so its pid still names this group? Cleared
    /// by `close`, before the reap, under the lock the stop handler takes.
    open: bool,
    recorded: BTreeSet<Identity>,
    /// Every process the table listed before this attempt began: none of
    /// them is its.
    before: BTreeSet<Identity>,
    /// Orphans the engine adopted that this attempt and another could
    /// each have left: ended with it, and it parks on them.
    doubted: BTreeSet<Identity>,
    /// The youngest birth the first read that found nothing of the
    /// attempt running showed; `None` while something of it runs. An
    /// orphan born later is not its.
    ended: Option<u64>,
}

type Table = fn() -> Result<Vec<Entry>, TableError>;

/// The engine's live attempts, and what its reads of the table found.
struct Registry {
    attempts: BTreeMap<u64, Live>,
    /// Orphans the engine adopted that no attempt could have left: the
    /// engine's own.
    unowned: BTreeSet<Identity>,
    /// The last read, which every caller that asked for one before it
    /// began may take instead of reading again.
    latest: Option<Read>,
}

struct Read {
    table: Table,
    began: Instant,
    entries: Vec<Entry>,
}

static LIVE: Mutex<Registry> = Mutex::new(Registry {
    attempts: BTreeMap::new(),
    unowned: BTreeSet::new(),
    latest: None,
});
static NEXT: AtomicU64 = AtomicU64::new(0);
static START: Once = Once::new();
static MISSING: OnceLock<Option<Unsettled>> = OnceLock::new();

fn live() -> MutexGuard<'static, Registry> {
    LIVE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// One live attempt. Dropping it forgets the attempt.
pub(super) struct Attempt {
    key: u64,
}

/// Why a driver was not spawned.
#[derive(Debug)]
pub(super) enum Unspawned {
    /// The read before it could not be taken: without it, nothing would
    /// tell the attempt's orphans from what ran before it.
    Table(Unsettled),
    Spawn(std::io::Error),
}

impl Attempt {
    /// Spawn the driver, leading a session of its own and its tree's
    /// subreaper, and register its group under one lock, so the stop
    /// handler never misses a driver that is already running. The first
    /// attempt's `host` serves the tracker and the stop handler.
    pub(super) fn spawn(builder: &mut Command, host: Host) -> Result<(Child, Attempt), Unspawned> {
        START.call_once(|| start(host));
        // SAFETY: `detach` runs between fork and exec, and makes system
        // calls and allocates nothing.
        unsafe { std::os::unix::process::CommandExt::pre_exec(builder, detach) };
        let (child, key) = live().admit(host.table, builder)?;
        Ok((child, Attempt { key }))
    }

    /// The deadline's kill, for the watchdog: read the table, then kill
    /// the group and every identity the attempt `key` owns.
    pub(super) fn kill(key: u64, host: Host) -> Result<(), Unsettled> {
        read_and_kill(key, host, |_| {})
    }

    pub(super) fn key(&self) -> u64 {
        self.key
    }

    /// The same kill, then close the group to signals: its leader is
    /// about to be reaped.
    pub(super) fn close(&self, host: Host) -> Result<(), Unsettled> {
        read_and_kill(self.key, host, |this| this.open = false)
    }

    /// What of the attempt a fresh read shows still running, once what it
    /// shows is recorded and attributed and every identity the attempt
    /// owns that still runs is signalled again.
    pub(super) fn survivors(&self, host: Host) -> Result<(), Unsettled> {
        let since = Instant::now();
        let mut live = live();
        let entries = live.read_since(host.table, since)?;
        live.attempts[&self.key].running(&entries, host.kill)
    }

    /// Once nothing of the attempt runs: what still casts doubt on it.
    pub(super) fn doubts(&self, missing: Option<Unsettled>) -> Result<(), Unsettled> {
        live().attempts[&self.key].doubts(missing)
    }
}

impl Drop for Attempt {
    fn drop(&mut self) {
        live().attempts.remove(&self.key);
    }
}

/// Read the table, record and attribute what it shows, and SIGKILL what
/// the attempt `key` owns. The read at the kill is what the kill knows of
/// what left the group since the tracker last looked, and on macOS, where
/// no orphan comes back to the engine, it is the last look there is. A
/// read that fails is carried after the kill.
fn read_and_kill(key: u64, host: Host, then: fn(&mut Live)) -> Result<(), Unsettled> {
    let since = Instant::now();
    let mut live = live();
    let read = live.read_since(host.table, since);
    let this = live.attempts.get_mut(&key).expect("a registered attempt");
    let killed = this.kill(host);
    then(this);
    killed.and(read.map(drop))
}

impl Registry {
    /// Read `table` afresh, spawn the driver `builder`, and register its
    /// group. The read is never an earlier one shared: an orphan of the
    /// engine's own born after that read and before the spawn would be
    /// missing from `before`, and were the driver to exit before the next
    /// read, it would be attributed to this attempt and killed with it.
    /// What remains is one born between this read and the fork itself.
    fn admit(&mut self, table: Table, builder: &mut Command) -> Result<(Child, u64), Unspawned> {
        let before = self.read(table).map_err(Unspawned::Table)?;
        let child = builder.spawn().map_err(Unspawned::Spawn)?;
        let key = NEXT.fetch_add(1, Ordering::Relaxed);
        let attempt = Live {
            group: Pid::from_child(&child),
            open: true,
            recorded: BTreeSet::new(),
            before: before.into_iter().map(|entry| entry.id).collect(),
            doubted: BTreeSet::new(),
            ended: None,
        };
        self.attempts.insert(key, attempt);
        Ok((child, key))
    }

    /// Read the table, and observe what it shows.
    fn read(&mut self, table: Table) -> Result<Vec<Entry>, Unsettled> {
        let began = Instant::now();
        let read = tree::read(table);
        read.iter().for_each(|entries| self.observe(entries));
        self.latest = read.as_ref().ok().map(|entries| Read {
            table,
            began,
            entries: entries.clone(),
        });
        read
    }

    /// A read of `table` that began no earlier than `since`: the last
    /// one, when it did, and otherwise a new one. Callers that queued for
    /// the lock while one read was taken share the next.
    fn read_since(&mut self, table: Table, since: Instant) -> Result<Vec<Entry>, Unsettled> {
        let fresh = self
            .latest
            .as_ref()
            .filter(|latest| latest.began >= since && std::ptr::fn_addr_eq(latest.table, table));
        match fresh {
            Some(latest) => Ok(latest.entries.clone()),
            None => self.read(table),
        }
    }

    /// Record what `entries` shows of every live attempt's tree, attribute
    /// each orphan the engine adopted that none explains, once, as it
    /// first appears, note which attempts ended, and reap the adopted
    /// zombies.
    fn observe(&mut self, entries: &[Entry]) {
        for live in self.attempts.values_mut() {
            live.record(entries);
        }
        self.unowned
            .retain(|id| entries.iter().any(|entry| entry.id == *id));
        let strays: Vec<Identity> = strays(entries, self)
            .map(|entry| entry.id.clone())
            .collect();
        for id in strays {
            self.attribute(id, entries);
        }
        let youngest = entries.iter().filter_map(|entry| entry.id.born()).max();
        for live in self.attempts.values_mut() {
            live.note(entries, youngest);
        }
        self.reap(entries);
    }

    /// Reap every zombie the engine adopted (`adopted`), whatever attempt
    /// explains it, or none: an orphan that exited before any read saw it
    /// run, and the engine's own once they exit, would otherwise hold
    /// their pids until the engine exits. A live attempt's leader is the
    /// engine's own child, reaped by its handle.
    fn reap(&self, entries: &[Entry]) {
        let leads = |entry: &Entry| {
            let pid = entry.id.pid;
            self.attempts
                .values()
                .any(|live| live.group.as_raw_pid() == pid)
        };
        adopted(entries)
            .filter(|entry| entry.zombie && !leads(entry))
            .for_each(|entry| table::reap(entry.id.pid));
    }

    /// Attribute the orphan `id` to the attempts that could have left it
    /// (`Live::left`): to that attempt's record when it is one, to each
    /// one's doubt when there are several, and to the engine when there
    /// are none.
    fn attribute(&mut self, id: Identity, entries: &[Entry]) {
        let mut witnesses: Vec<&mut Live> = self
            .attempts
            .values_mut()
            .filter(|live| live.left(&id, entries))
            .collect();
        match witnesses.as_mut_slice() {
            [] => {
                self.unowned.insert(id);
            }
            [one] => {
                one.recorded.insert(id);
            }
            several => several.iter_mut().for_each(|live| {
                live.doubted.insert(id.clone());
            }),
        }
    }
}

impl Live {
    /// SIGKILL the group while its leader is unreaped, and every identity
    /// recorded or doubted. The first refusal, once every kill was tried.
    fn kill(&self, host: Host) -> Result<(), Unsettled> {
        let group = self.group.as_raw_pid();
        let refused = self
            .open
            .then(|| tree::refused((host.kill_group)(self.group)))
            .flatten()
            .map(|errno| Unsettled::Kill {
                group,
                errno: errno.raw_os_error(),
            });
        let identities = signal(self.recorded.iter().chain(&self.doubted), host.kill);
        refused.map_or(Ok(()), Err).and(identities)
    }

    /// Record every descendant of the leader, and of what is already
    /// recorded, that `entries` shows running, and every member of the
    /// group while its leader is unreaped: an orphan that stayed in the
    /// group is the attempt's, whoever adopted it.
    fn record(&mut self, entries: &[Entry]) {
        let mut roots: BTreeSet<i32> = entries
            .iter()
            .filter(|entry| self.recorded.contains(&entry.id))
            .map(|entry| entry.id.pid)
            .collect();
        let group = self.open.then_some(self.group.as_raw_pid());
        roots.extend(group);
        loop {
            let found: Vec<&Entry> = entries
                .iter()
                .filter(|entry| !entry.zombie && !roots.contains(&entry.id.pid))
                .filter(|entry| roots.contains(&entry.ppid) || Some(entry.pgid) == group)
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

    /// Does the leader still run, as its tree's subreaper? Not once the
    /// attempt is closed: its leader is killed and about to be reaped.
    fn leading(&self, entries: &[Entry]) -> bool {
        let group = self.group.as_raw_pid();
        self.open
            && entries
                .iter()
                .any(|entry| entry.id.pid == group && !entry.zombie)
    }

    /// Could this attempt have left the orphan `id`, which `entries` first
    /// shows with the engine? Not when `id` predates it. Not while its
    /// leader runs: the leader is its tree's subreaper, so its orphans go
    /// to the leader. And not when `id` was born after the attempt ended:
    /// an orphan of its tree was born while the tree ran, so it was alive,
    /// and among the births, at the read that first found the attempt
    /// ended. A birth that does not order (macOS) rules nothing out.
    fn left(&self, id: &Identity, entries: &[Entry]) -> bool {
        let born_before_the_end = self
            .ended
            .zip(id.born())
            .is_none_or(|(ended, born)| born <= ended);
        !self.before.contains(id) && !self.leading(entries) && born_before_the_end
    }

    /// Note, after attribution, whether `entries` shows anything of the
    /// attempt running, and when it first shows nothing, the `youngest`
    /// birth it shows.
    fn note(&mut self, entries: &[Entry], youngest: Option<u64>) {
        let runs = entries
            .iter()
            .any(|entry| !entry.zombie && self.explains(entry));
        self.ended = if runs { None } else { self.ended.or(youngest) };
    }

    /// Is `entry` this attempt's: its leader, in its group, recorded, or
    /// doubted?
    fn explains(&self, entry: &Entry) -> bool {
        let group = self.group.as_raw_pid();
        entry.id.pid == group
            || entry.pgid == group
            || self.recorded.contains(&entry.id)
            || self.doubted.contains(&entry.id)
    }

    /// What of the attempt `entries` shows running, in the order it is
    /// reported: its group, its recorded descendants, the orphans it
    /// doubts. Every identity still running is signalled again first,
    /// for what was attributed to it since its kill.
    fn running(
        &self,
        entries: &[Entry],
        kill: fn(&Identity) -> std::io::Result<()>,
    ) -> Result<(), Unsettled> {
        let running = || entries.iter().filter(|entry| !entry.zombie);
        let owned = |set: &BTreeSet<Identity>| -> Vec<&Identity> {
            running()
                .filter(|entry| set.contains(&entry.id))
                .map(|entry| &entry.id)
                .collect()
        };
        let (descendants, doubted) = (owned(&self.recorded), owned(&self.doubted));
        signal(descendants.iter().chain(&doubted).copied(), kill)?;
        let group = self.group.as_raw_pid();
        if running().any(|entry| entry.pgid == group) {
            return Err(Unsettled::Group { group });
        }
        if !descendants.is_empty() {
            return Err(Unsettled::Descendants {
                pids: pids(descendants),
            });
        }
        if !doubted.is_empty() {
            return Err(Unsettled::Strays {
                pids: pids(doubted),
            });
        }
        Ok(())
    }

    /// An attempt that doubted an orphan parks, even once it is gone: it
    /// may have been its. So does one that had descendants on a host
    /// that refused the engine a means.
    fn doubts(&self, missing: Option<Unsettled>) -> Result<(), Unsettled> {
        if !self.doubted.is_empty() {
            return Err(Unsettled::Strays {
                pids: pids(&self.doubted),
            });
        }
        self.unwatched(missing)
    }

    fn unwatched(&self, missing: Option<Unsettled>) -> Result<(), Unsettled> {
        missing
            .filter(|_| !self.recorded.is_empty())
            .map_or(Ok(()), Err)
    }
}

fn pids<'a>(ids: impl IntoIterator<Item = &'a Identity>) -> Vec<i32> {
    ids.into_iter().map(|id| id.pid).collect()
}

/// SIGKILL each identity. The first refusal, once every one was tried.
fn signal<'a>(
    ids: impl Iterator<Item = &'a Identity>,
    kill: fn(&Identity) -> std::io::Result<()>,
) -> Result<(), Unsettled> {
    ids.map(|id| {
        kill(id).map_err(|error| Unsettled::Signal {
            pid: id.pid,
            error: error.to_string(),
        })
    })
    .fold(Ok(()), Result::and)
}

/// The engine's children outside its own process group, whatever their
/// session: the orphans it adopted as a subreaper, and its attempts'
/// leaders. A child the engine spawns stays in the engine's group, and
/// nothing of an attempt can join it: its driver leads a session of its
/// own (`detach`), and `setpgid` refuses a group in another session.
fn adopted(entries: &[Entry]) -> impl Iterator<Item = &Entry> {
    let me = getpid().as_raw_pid();
    let mine = entries
        .iter()
        .find(|entry| entry.id.pid == me)
        .map(|entry| entry.pgid);
    entries
        .iter()
        .filter(move |entry| entry.ppid == me && Some(entry.pgid) != mine)
}

/// The running orphans the engine adopted that no live attempt explains
/// and that are not already the engine's own. A driver leads a group that
/// is a live attempt's.
fn strays<'a>(entries: &'a [Entry], registry: &'a Registry) -> impl Iterator<Item = &'a Entry> {
    adopted(entries)
        .filter(|entry| !entry.zombie)
        .filter(|entry| !registry.unowned.contains(&entry.id))
        .filter(|entry| !registry.attempts.values().any(|live| live.explains(entry)))
}

/// The means this host refused the engine, if one: read once, as the
/// first attempt starts.
pub(super) fn missing() -> Option<Unsettled> {
    MISSING.get().cloned().flatten()
}

/// Once per engine, before its first attempt.
fn start(host: Host) {
    MISSING.get_or_init(probe);
    stop_on_signals(host);
    std::thread::spawn(move || loop {
        std::thread::sleep(TRACK);
        let mut live = live();
        let busy = !live.attempts.is_empty();
        drop(busy.then(|| live.read(host.table)));
    });
}

/// Run in each driver between its fork and its exec: lead a session of
/// its own, so no process of its tree can join the engine's group, and on
/// Linux adopt its own tree's orphans. Async-signal-safe: system calls,
/// and an error that allocates nothing.
fn detach() -> std::io::Result<()> {
    rustix::process::setsid()?;
    #[cfg(target_os = "linux")]
    subreaper()?;
    Ok(())
}

/// Make the calling process a child subreaper: the engine, and each
/// driver (`detach`), so a running leader adopts its own tree's orphans.
/// Async-signal-safe: one `prctl`, and an error that allocates nothing.
#[cfg(target_os = "linux")]
fn subreaper() -> std::io::Result<()> {
    rustix::process::set_child_subreaper(Some(getpid()))?;
    Ok(())
}

/// Become the subreaper, and open a pidfd on the engine itself: the
/// first refusal is the means this host lacks. A kernel older than 3.4
/// refuses the subreaper, one older than 5.3 the pidfd.
#[cfg(target_os = "linux")]
fn probe() -> Option<Unsettled> {
    use rustix::io::Errno;
    use rustix::process::{pidfd_open, PidfdFlags};
    let refused = subreaper().err();
    let refused = refused.as_ref().and_then(std::io::Error::raw_os_error);
    refused.map(Unsettled::Subreaper).or_else(|| {
        let pidfd = pidfd_open(getpid(), PidfdFlags::empty()).err();
        pidfd.map(Errno::raw_os_error).map(Unsettled::Pidfd)
    })
}

/// macOS has neither means, by design: the read at the kill stands in,
/// and the ruled residual (`tree`) is its limit.
#[cfg(not(target_os = "linux"))]
fn probe() -> Option<Unsettled> {
    None
}

/// SIGINT, SIGTERM, SIGHUP and SIGQUIT end every live attempt, and the
/// engine then exits 128 plus the signal, as the signal's own default
/// would have ended it, once the table reads every one of them gone;
/// otherwise it says why and exits `UNPROVEN_STOP`. A signal the engine
/// inherited as ignored (`nohup`, a shell's background job) stays
/// ignored.
fn stop_on_signals(host: Host) {
    let signals: Vec<i32> = [SIGINT, SIGTERM, SIGHUP, SIGQUIT]
        .into_iter()
        .filter(|signal| !ignored(*signal))
        .collect();
    let mut signals = Signals::new(signals).expect("the engine's stop signals register");
    std::thread::spawn(move || {
        let signal = signals.forever().next().unwrap_or(SIGTERM);
        // Held to the exit, so no attempt registers after the sweep.
        let mut live = live();
        let status = stop(&mut live, host).map_or_else(unproven, |()| 128 + signal);
        std::process::exit(status)
    });
}

/// Kill every live attempt, then wait up to the settle bound for the
/// table to read all of them gone.
fn stop(registry: &mut Registry, host: Host) -> Result<(), Unsettled> {
    let read = registry.read(host.table);
    let killed = registry
        .attempts
        .values()
        .map(|live| live.kill(host))
        .fold(Ok(()), Result::and);
    let gone = tree::wait(Instant::now() + Bounds::DEFAULT.settle, || {
        let entries = registry.read(host.table)?;
        registry
            .attempts
            .values()
            .try_for_each(|live| live.running(&entries, host.kill))
    });
    let missing = (host.missing)();
    let unwatched = registry
        .attempts
        .values()
        .try_for_each(|live| live.unwatched(missing.clone()));
    killed.and(read.map(drop)).and(gone).and(unwatched)
}

fn unproven(reason: Unsettled) -> i32 {
    eprintln!("brokkr: stopped with an attempt not proven over: {reason}");
    UNPROVEN_STOP
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
