//! The host's process tree, as the attempt's tree is read from it (#403,
//! #918). Linux has two reads: the tracker's 100 ms tick walks the
//! engine's own subtree through `/proc`'s `children` files only to ask
//! whether the registry already knows everything in it and whether any row
//! changed since the last walk, and a new identity, a changed row, a doubt
//! or no previous walk makes the tick read the whole table, which alone
//! attributes; admission and every kill read the whole table too, so
//! `before` names every process that already ran. Everything settlement can
//! attribute is a descendant of the engine, and a tick that walked the
//! whole host would pay for every process on it. macOS reads the whole
//! table with `ps`, where there is no subreaper. A process is named by its
//! pid AND its start stamp, so a pid the kernel has since handed to another
//! process never reads as the one recorded. A table that cannot be read
//! whole is no table: a row the engine cannot read could be the attempt's,
//! so it fails the read rather than reading as gone.

#[cfg(target_os = "linux")]
use std::collections::{BTreeMap, BTreeSet};

use rustix::process::{Pid, Signal};
use thiserror::Error;

/// One process, by pid and the kernel's start stamp for it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Identity {
    pub(super) pid: i32,
    pub(super) start: String,
}

impl Identity {
    /// When the process was born, in clock ticks since boot: on Linux
    /// the start stamp, so stamps order births. macOS's stamp is a date,
    /// and orders nothing.
    pub(super) fn born(&self) -> Option<u64> {
        self.start.parse().ok()
    }
}

/// One row of a table read. A zombie has exited: it holds a pid until its
/// parent reaps it, and it runs nothing, so it counts as gone. An exiting
/// process is one BSD `ps` flags `E`, trying to exit but not yet a
/// zombie: Darwin's group signal skips it (`tree::group_refused`). Linux
/// lists no such state, and reads none exiting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Entry {
    pub(super) id: Identity,
    pub(super) ppid: i32,
    pub(super) pgid: i32,
    pub(super) zombie: bool,
    pub(super) exiting: bool,
}

/// Why a table read failed.
#[derive(Debug, Error)]
pub(super) enum TableError {
    #[cfg(target_os = "linux")]
    #[error("/proc could not be listed: {0}")]
    List(std::io::Error),
    #[cfg(target_os = "linux")]
    #[error("the row of process {pid} could not be read: {error}")]
    Row { pid: i32, error: std::io::Error },
    #[cfg(not(target_os = "linux"))]
    #[error("ps could not be run: {0}")]
    Ps(std::io::Error),
    #[cfg(any(test, not(target_os = "linux")))]
    #[error("ps exited {0}")]
    Status(std::process::ExitStatus),
    #[cfg(any(test, not(target_os = "linux")))]
    #[error("the ps row {row:?} could not be read")]
    Unparsed { row: String },
    #[error("the table has no row for the engine itself (pid {pid})")]
    NoSelf { pid: i32 },
}

#[cfg(target_os = "linux")]
const PROC: &str = "/proc";

/// The two row states that decide whether anything of a live attempt still
/// runs: a zombie holds a pid but runs nothing, and an exiting process is on
/// its way to one. Linux lists no exiting process, so there it is always
/// false; both are compared so a state a Linux table never lists could not
/// change unreported.
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct State {
    zombie: bool,
    exiting: bool,
}

/// One tick's walk as the tracker remembers it: each identity the walk saw
/// and its state. `attempts::Registry` only compares it with the next
/// walk; it is never passed to `Live::record`, `observe`, `recorded`,
/// `doubted` or a kill.
#[cfg(target_os = "linux")]
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Sighted {
    rows: BTreeMap<Identity, State>,
}

/// What one tick's walk of the engine's own tree saw: the rows it read, and
/// whether anything kept it from reading the tree whole. It is a sighting,
/// not a table read: the tracker asks whether it shows an identity the
/// registry does not know or a row that changed since the last walk, and
/// the full snapshot alone attributes. Nothing here reaches `Live::record`,
/// `observe`, `recorded`, `doubted` or a kill.
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub(super) struct Sighting {
    entries: Vec<Entry>,
    /// The walk named a process or thread that vanished or was replaced
    /// under it, or could not read part of the tree: it may have missed
    /// something, so the tick reads the full table rather than trust it.
    doubt: bool,
}

#[cfg(target_os = "linux")]
impl Sighting {
    /// Does this sighting show an identity `known` does not? A doubt counts
    /// as one: a partial walk proves nothing either way.
    pub(super) fn unseen(&self, known: impl Fn(&Identity) -> bool) -> bool {
        self.doubt || self.entries.iter().any(|entry| !known(&entry.id))
    }

    /// The sighting as the next tick compares it: each identity and its
    /// states. An identity that appears or disappears, or a row that turns
    /// zombie or exiting, is a change the tick must take a snapshot for.
    pub(super) fn sighted(&self) -> Sighted {
        Sighted {
            rows: self
                .entries
                .iter()
                .map(|entry| {
                    (
                        entry.id.clone(),
                        State {
                            zombie: entry.zombie,
                            exiting: entry.exiting,
                        },
                    )
                })
                .collect(),
        }
    }

    /// A sighting of `entries`, for the tests that plant a walk.
    #[cfg(test)]
    pub(super) fn new(entries: Vec<Entry>, doubt: bool) -> Sighting {
        Sighting { entries, doubt }
    }

    /// The pids it saw, for the tests that hold the real host's walk to the
    /// engine's own tree.
    #[cfg(test)]
    pub(super) fn pids(&self) -> BTreeSet<i32> {
        self.entries.iter().map(|entry| entry.id.pid).collect()
    }
}

/// The engine's own tree: itself and every descendant, read through
/// `/proc/<pid>/task/<tid>/children`. The engine is a child subreaper on
/// Linux (`attempts`), so a descendant that leaves its attempt's group,
/// or whose parent exits, is reparented to the engine and stays in this
/// subtree; every process settlement must attribute is therefore here.
///
/// The walk reads the subtree, never the host: unrelated host processes
/// are not visited, and one read costs the engine's own process count,
/// not the host's. It answers two questions for the tick: is there an
/// identity here the registry does not already know or a row that changed
/// since the last walk, and could the walk not tell?
#[cfg(target_os = "linux")]
pub(super) fn own_tree() -> Result<Sighting, TableError> {
    sighting_in(
        std::path::Path::new(PROC),
        rustix::process::getpid().as_raw_pid(),
    )
}

/// [`own_tree`] in `proc`, for the planted trees the tests read.
#[cfg(target_os = "linux")]
fn sighting_in(proc: &std::path::Path, root: i32) -> Result<Sighting, TableError> {
    let walk = walk_in(proc, root)?;
    Ok(Sighting {
        entries: walk.entries,
        doubt: walk.doubt,
    })
}

/// The raw walk: its rows, and whether a named process or thread vanished
/// or was replaced while it ran. The walk skips what exited so that it can
/// finish, and `doubt` is how the tick knows its sighting was partial.
#[cfg(target_os = "linux")]
struct Walk {
    entries: Vec<Entry>,
    doubt: bool,
}

/// The tree rooted at `root` in `proc`, root included, found by the
/// `children` files of every thread of every process in it. A child that
/// exits mid-walk is skipped, so the walk still finishes; `doubt` says it
/// did, and a tick then reads the full table instead of trusting a partial
/// walk. A child that exits before its own `children` are read leaves its
/// children to be reparented to the engine, where a later read sees them.
///
/// An entry keeps the parent and start stamp its own `stat` reports. A
/// pid its parent's `children` file named may have been reused by another
/// process after the child exited; that process's row names a different
/// parent, and it is dropped from this walk rather than given the old
/// edge, as a doubt. Its subtree goes with it. A process that truly moved
/// is found under its real parent on the next read, as an orphan is today.
/// The walk never assigns a parent from the listing.
#[cfg(target_os = "linux")]
fn walk_in(proc: &std::path::Path, root: i32) -> Result<Walk, TableError> {
    let entry = read(proc, root)
        .map_err(|error| TableError::Row { pid: root, error })?
        .ok_or(TableError::NoSelf { pid: root })?;
    let mut seen = BTreeSet::from([root]);
    let mut frontier = vec![root];
    let mut entries = vec![entry];
    let mut doubt = false;
    while let Some(pid) = frontier.pop() {
        let (children, vanished) =
            children(proc, pid).map_err(|error| TableError::Row { pid, error })?;
        doubt |= vanished;
        for child in children {
            if !seen.insert(child) {
                continue;
            }
            match read(proc, child).map_err(|error| TableError::Row { pid: child, error })? {
                // The pid was reused by a process with another parent, or
                // the child exited before its row: the walk cannot follow
                // this edge, and a snapshot must decide what it was.
                Some(entry) if entry.ppid == pid => {
                    entries.push(entry);
                    frontier.push(child);
                }
                Some(_) | None => doubt = true,
            }
        }
    }
    Ok(Walk { entries, doubt })
}

/// The walk's rows, with the doubt a tick uses dropped: the table tests
/// hold the walk itself to what it reads.
#[cfg(all(target_os = "linux", test))]
fn own_tree_in(proc: &std::path::Path, root: i32) -> Result<Vec<Entry>, TableError> {
    walk_in(proc, root).map(|walk| walk.entries)
}

/// The children of every thread of `pid`, from the `children` file each
/// thread's `/proc/<pid>/task/<tid>` holds, and whether any of them exited
/// while it was read. A process or thread that exits while it is read is
/// gone, and lists no children here.
#[cfg(target_os = "linux")]
fn children(proc: &std::path::Path, pid: i32) -> std::io::Result<(Vec<i32>, bool)> {
    let tasks = match std::fs::read_dir(proc.join(pid.to_string()).join("task")) {
        Err(error) if gone(&error) => return Ok((Vec::new(), true)),
        tasks => tasks?,
    };
    let mut children = Vec::new();
    let mut doubt = false;
    for task in tasks {
        let task = task?;
        let text = match std::fs::read_to_string(task.path().join("children")) {
            // A task that exited between the listing and its read took
            // its children with it: what it forked was reparented, and
            // the next read finds it with the engine.
            Err(error) if gone(&error) => {
                doubt = true;
                continue;
            }
            text => text?,
        };
        for child in text.split_whitespace() {
            children.push(child.parse().map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unparsed child pid {child:?}"),
                )
            })?);
        }
    }
    Ok((children, doubt))
}

/// Is `error` a process or thread that exited while it was read?
#[cfg(target_os = "linux")]
fn gone(error: &std::io::Error) -> bool {
    matches!(error.raw_os_error(), Some(libc::ENOENT | libc::ESRCH))
}

/// Every process the host lists, for admission's `before`, the read at a
/// kill, and the tests that plant a `/proc` tree.
#[cfg(target_os = "linux")]
pub(super) fn snapshot() -> Result<Vec<Entry>, TableError> {
    snapshot_in(std::path::Path::new(PROC))
}

/// Every process `proc` lists. A process that exits between the listing
/// and its read is gone, and is left out as gone; a row that cannot be
/// read for any other reason, or read whole, fails the snapshot.
#[cfg(target_os = "linux")]
fn snapshot_in(proc: &std::path::Path) -> Result<Vec<Entry>, TableError> {
    let mut entries = Vec::new();
    for dir in std::fs::read_dir(proc).map_err(TableError::List)? {
        let name = dir.map_err(TableError::List)?.file_name();
        if let Some(pid) = name.to_str().and_then(|name| name.parse::<i32>().ok()) {
            entries.extend(read(proc, pid).map_err(|error| TableError::Row { pid, error })?);
        }
    }
    Ok(entries)
}

#[cfg(all(target_os = "linux", test))]
thread_local! {
    /// How many stat rows this thread has read: a planted `/proc` counts
    /// the files one tracker read opens.
    static STAT_READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(all(target_os = "linux", test))]
fn count_stat() {
    STAT_READS.with(|reads| reads.set(reads.get() + 1));
}

/// How many stat rows this thread has read since [`clear_stat_reads`].
#[cfg(all(target_os = "linux", test))]
fn stat_reads() -> usize {
    STAT_READS.with(std::cell::Cell::get)
}

#[cfg(all(target_os = "linux", test))]
fn clear_stat_reads() {
    STAT_READS.with(|reads| reads.set(0));
}

/// The row of `pid`, or `None` when the process is gone: its directory
/// went with it (`ENOENT`), or it exited while its `stat` was read
/// (`ESRCH`).
#[cfg(target_os = "linux")]
fn read(proc: &std::path::Path, pid: i32) -> std::io::Result<Option<Entry>> {
    #[cfg(test)]
    count_stat();
    let stat = match std::fs::read_to_string(proc.join(pid.to_string()).join("stat")) {
        Err(error) if gone(&error) => return Ok(None),
        stat => stat?,
    };
    parse_stat(pid, &stat).map(Some).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("unparsed stat {stat:?}"),
        )
    })
}

/// The row of `id`, or `None` when it is gone or its pid now names
/// another process. The tree walk can miss a recorded process while it
/// is reparented, so settlement reads that identity's own row too
/// (`attempts::Live::rows`).
#[cfg(target_os = "linux")]
pub(super) fn row_of(id: &Identity) -> Result<Option<Entry>, TableError> {
    row_in(std::path::Path::new(PROC), id)
}

/// [`row_of`] in `proc`, for the planted trees the tests read.
#[cfg(target_os = "linux")]
fn row_in(proc: &std::path::Path, id: &Identity) -> Result<Option<Entry>, TableError> {
    let entry: Option<Entry> =
        read(proc, id.pid).map_err(|error| TableError::Row { pid: id.pid, error })?;
    Ok(entry.filter(|entry| entry.id == *id))
}

/// `/proc/<pid>/stat`: the command name is parenthesised and may hold any
/// byte, so fields are counted from its closing parenthesis — state,
/// ppid, pgrp, and the start time twenty fields on.
#[cfg(target_os = "linux")]
fn parse_stat(pid: i32, stat: &str) -> Option<Entry> {
    let (_, rest) = stat.rsplit_once(')')?;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    Some(Entry {
        id: Identity {
            pid,
            start: fields.get(19)?.to_string(),
        },
        ppid: fields.get(1)?.parse().ok()?,
        pgid: fields.get(2)?.parse().ok()?,
        zombie: ["Z", "X"].contains(fields.first()?),
        exiting: false,
    })
}

/// SIGKILL `id`, and nothing else. A pidfd pins the process it was opened
/// on, so when the start stamp read after opening it still matches, the
/// signal cannot reach a process that reused the pid. A process already
/// gone is not signalled; whether the signal ended it is read afterwards,
/// from the table, which is the proof `tree::end` settles on. A refusal,
/// or a row that cannot be read to confirm the stamp, is returned.
#[cfg(target_os = "linux")]
pub(super) fn kill(id: &Identity) -> std::io::Result<()> {
    Pid::from_raw(id.pid).map_or(Ok(()), |pid| signal(pid, id))
}

#[cfg(target_os = "linux")]
fn signal(pid: Pid, id: &Identity) -> std::io::Result<()> {
    use rustix::process::{pidfd_open, pidfd_send_signal, PidfdFlags};
    let pidfd = match pidfd_open(pid, PidfdFlags::empty()) {
        Ok(pidfd) => pidfd,
        Err(errno) => return refusal(Err(errno)),
    };
    let row = read(std::path::Path::new(PROC), id.pid)?;
    if row.is_some_and(|entry| entry.id == *id) {
        refusal(pidfd_send_signal(&pidfd, Signal::KILL))
    } else {
        Ok(())
    }
}

/// A signal's refusal; `ESRCH` is none, because the process is gone.
fn refusal(signalled: rustix::io::Result<()>) -> std::io::Result<()> {
    super::tree::refused(signalled)
        .map(std::io::Error::from)
        .map_or(Ok(()), Err)
}

/// Reap `pid`, a zombie the engine adopted as a subreaper (`attempts`),
/// so the pid is released. Never called on a child the engine spawned:
/// those are reaped by their own handle.
#[cfg(target_os = "linux")]
pub(super) fn reap(pid: i32) {
    use rustix::process::{waitpid, WaitOptions};
    let _ = Pid::from_raw(pid).map(|pid| waitpid(Some(pid), WaitOptions::NOHANG));
}

#[cfg(not(target_os = "linux"))]
pub(super) fn snapshot() -> Result<Vec<Entry>, TableError> {
    listed(ps(&["-axo", COLUMNS])?)
}

#[cfg(not(target_os = "linux"))]
const COLUMNS: &str = "pid=,ppid=,pgid=,stat=,lstart=";

#[cfg(not(target_os = "linux"))]
fn ps(args: &[&str]) -> Result<std::process::Output, TableError> {
    std::process::Command::new("ps")
        .args(args)
        .env("LC_ALL", "C")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .map_err(TableError::Ps)
}

/// The rows `ps` printed. A nonzero status is a table that was not read,
/// whatever it printed, and every row must parse. Compiled for Linux's
/// tests too, which stand it in as the table macOS reads.
#[cfg(any(test, not(target_os = "linux")))]
pub(super) fn listed(output: std::process::Output) -> Result<Vec<Entry>, TableError> {
    if !output.status.success() {
        return Err(TableError::Status(output.status));
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| {
            parse_ps(line).ok_or_else(|| TableError::Unparsed {
                row: line.to_string(),
            })
        })
        .collect()
}

/// One `ps` row: pid, ppid, pgid, state, then the start time, which is
/// several words long and is kept whole as the stamp. The state is a
/// letter, `Z` for a zombie, then flags, `E` among them for a process
/// trying to exit. A row without a
/// start time, or with one that is not `lstart`'s date (BSD `ps` prints
/// `-` for a process it cannot inspect), is not read whole: its stamp
/// names no process, so a recorded one would read as gone (#403).
#[cfg(any(test, not(target_os = "linux")))]
fn parse_ps(line: &str) -> Option<Entry> {
    let mut fields = line.split_whitespace();
    let pid = fields.next()?.parse().ok()?;
    let ppid = fields.next()?.parse().ok()?;
    let pgid = fields.next()?.parse().ok()?;
    let state = fields.next()?;
    let start: Vec<&str> = fields.collect();
    lstart(&start).then(|| Entry {
        id: Identity {
            pid,
            start: start.join(" "),
        },
        ppid,
        pgid,
        zombie: state.starts_with('Z'),
        exiting: state.chars().skip(1).any(|flag| flag == 'E'),
    })
}

/// Is `words` a start time as `ps` prints `lstart` under `LC_ALL=C`, by
/// `%c`: weekday, month, day, `hh:mm:ss`, year?
#[cfg(any(test, not(target_os = "linux")))]
fn lstart(words: &[&str]) -> bool {
    const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let [weekday, month, day, time, year] = words else {
        return false;
    };
    let clock = time
        .split(':')
        .map(str::parse)
        .collect::<Result<Vec<u8>, _>>();
    WEEKDAYS.contains(weekday)
        && MONTHS.contains(month)
        && day.parse().is_ok_and(|day: u8| (1..=31).contains(&day))
        && clock.is_ok_and(|clock| {
            matches!(clock[..], [hour, minute, second] if hour < 24 && minute < 60 && second <= 60)
        })
        && year.len() == 4
        && year.bytes().all(|digit| digit.is_ascii_digit())
}

/// SIGKILL `id` once `ps` has confirmed its start stamp. macOS has no
/// pidfd, so a pid reused between the read and the signal could be
/// signalled: a limit of the kill's precision there, not of settlement.
#[cfg(not(target_os = "linux"))]
pub(super) fn kill(id: &Identity) -> std::io::Result<()> {
    let pid = id.pid.to_string();
    kill_listed(
        id,
        ps(&["-o", COLUMNS, "-p", pid.as_str()]).and_then(listed),
    )
}

/// SIGKILL `id` when `rows`, `ps`'s listing of its pid, shows it running.
/// A listing that cannot be read, a row without its stamp among them, is
/// returned: it cannot confirm that the pid is not `id`'s.
#[cfg(any(test, not(target_os = "linux")))]
pub(super) fn kill_listed(
    id: &Identity,
    rows: Result<Vec<Entry>, TableError>,
) -> std::io::Result<()> {
    let rows = match rows {
        Ok(rows) => rows,
        // `ps -p` lists nothing, and exits 1, when the pid is gone.
        Err(TableError::Status(status)) if status.code() == Some(1) => Vec::new(),
        Err(error) => return Err(std::io::Error::other(error)),
    };
    if rows.iter().any(|entry| entry.id == *id && !entry.zombie) {
        let pid = Pid::from_raw(id.pid).expect("a listed pid is positive");
        refusal(rustix::process::kill_process(pid, Signal::KILL))
    } else {
        Ok(())
    }
}

/// launchd adopts every orphan on macOS; the engine never does.
#[cfg(not(target_os = "linux"))]
pub(super) fn reap(_pid: i32) {}

#[cfg(test)]
mod tests;
