//! The host's process table, as the attempt's tree is read from it (#403).
//! Linux reads `/proc`; macOS reads `ps`. A process is named by its pid
//! AND its start stamp, so a pid the kernel has since handed to another
//! process never reads as the one recorded. A table that cannot be read
//! whole is no table: a row the engine cannot read could be the
//! attempt's, so it fails the read rather than reading as gone.

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

/// One row of a snapshot. A zombie has exited: it holds a pid until its
/// parent reaps it, and it runs nothing, so it counts as gone. An exiting
/// process is inside the kernel's exit but not yet a zombie. BSD `ps`
/// flags it `E`, and Darwin's group signal skips it
/// (`tree::group_refused`). On Linux it is one whose every thread has
/// entered `do_exit` (`PF_EXITING`). Its exit can still take seconds,
/// for example while a pid namespace's init unmounts the box's overlay
/// and waits on the disk (#504). It runs no code of its own again, so it
/// cannot fork, signal or write anything new, and it counts as gone too
/// (`Entry::runs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Entry {
    pub(super) id: Identity,
    pub(super) ppid: i32,
    pub(super) pgid: i32,
    pub(super) zombie: bool,
    pub(super) exiting: bool,
}

impl Entry {
    /// Does the process still run? Neither a zombie nor an exiting
    /// process does.
    pub(super) fn runs(&self) -> bool {
        !self.zombie && !self.exiting
    }
}

/// Why the table could not be read whole.
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

/// Every process the host lists.
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

/// The row of `pid`, or `None` when the process is gone: its directory
/// went with it (`ENOENT`), or it exited while its `stat` was read
/// (`ESRCH`). The flag in `stat` is the leader thread's alone, so a
/// process reads exiting only when every thread of it does: a leader
/// that exits first leaves the others running.
#[cfg(target_os = "linux")]
fn read(proc: &std::path::Path, pid: i32) -> std::io::Result<Option<Entry>> {
    let dir = proc.join(pid.to_string());
    match stat_row(&dir, pid)? {
        Some(entry) if entry.exiting => Ok(Some(Entry {
            exiting: threads_exiting(&dir, pid)?,
            ..entry
        })),
        entry => Ok(entry),
    }
}

/// The row `dir/stat` holds, or `None` when it is gone.
#[cfg(target_os = "linux")]
fn stat_row(dir: &std::path::Path, pid: i32) -> std::io::Result<Option<Entry>> {
    let stat = match std::fs::read_to_string(dir.join("stat")) {
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

/// Has every thread `dir/task` lists entered its exit, or ended? A
/// thread, or the whole task list, gone between the listing and its read
/// has ended. A thread whose row cannot be read could still run, so it
/// fails the read.
#[cfg(target_os = "linux")]
fn threads_exiting(dir: &std::path::Path, pid: i32) -> std::io::Result<bool> {
    let tasks = match std::fs::read_dir(dir.join("task")) {
        Err(error) if gone(&error) => return Ok(true),
        tasks => tasks?,
    };
    for task in tasks {
        if stat_row(&task?.path(), pid)?.is_some_and(|thread| thread.runs()) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Did a read of `/proc` fail because what it named is gone?
#[cfg(target_os = "linux")]
fn gone(error: &std::io::Error) -> bool {
    matches!(error.raw_os_error(), Some(libc::ENOENT | libc::ESRCH))
}

/// The kernel's flag for a thread inside `do_exit`, set before it
/// releases anything (`include/linux/sched.h`).
#[cfg(target_os = "linux")]
const PF_EXITING: u32 = 0x0000_0004;

/// `/proc/<pid>/stat`: the command name is parenthesised and may hold any
/// byte, so fields are counted from its closing parenthesis — state,
/// ppid, pgrp, the kernel flags four fields on, and the start time
/// thirteen after them.
#[cfg(target_os = "linux")]
fn parse_stat(pid: i32, stat: &str) -> Option<Entry> {
    let (_, rest) = stat.rsplit_once(')')?;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    let flags: u32 = fields.get(6)?.parse().ok()?;
    Some(Entry {
        id: Identity {
            pid,
            start: fields.get(19)?.to_string(),
        },
        ppid: fields.get(1)?.parse().ok()?,
        pgid: fields.get(2)?.parse().ok()?,
        zombie: ["Z", "X"].contains(fields.first()?),
        exiting: flags & PF_EXITING != 0,
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
