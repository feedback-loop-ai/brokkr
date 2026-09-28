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
/// parent reaps it, and it runs nothing, so it counts as gone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Entry {
    pub(super) id: Identity,
    pub(super) ppid: i32,
    pub(super) pgid: i32,
    pub(super) zombie: bool,
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
    #[cfg(not(target_os = "linux"))]
    #[error("ps exited {0}")]
    Status(std::process::ExitStatus),
    #[cfg(not(target_os = "linux"))]
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
/// (`ESRCH`).
#[cfg(target_os = "linux")]
fn read(proc: &std::path::Path, pid: i32) -> std::io::Result<Option<Entry>> {
    let stat = match std::fs::read_to_string(proc.join(pid.to_string()).join("stat")) {
        Err(error) if matches!(error.raw_os_error(), Some(libc::ENOENT | libc::ESRCH)) => {
            return Ok(None)
        }
        stat => stat?,
    };
    parse_stat(pid, &stat).map(Some).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("unparsed stat {stat:?}"),
        )
    })
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

/// Reap `pid`, a zombie the engine adopted as the attempt's subreaper
/// (`attempts`), so the pid is released. Never called on a child the
/// engine spawned: those are reaped by their own handle.
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
/// whatever it printed, and every row must parse.
#[cfg(not(target_os = "linux"))]
fn listed(output: std::process::Output) -> Result<Vec<Entry>, TableError> {
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
/// several words long and is kept whole as the stamp.
#[cfg(not(target_os = "linux"))]
fn parse_ps(line: &str) -> Option<Entry> {
    let mut fields = line.split_whitespace();
    let pid = fields.next()?.parse().ok()?;
    let ppid = fields.next()?.parse().ok()?;
    let pgid = fields.next()?.parse().ok()?;
    let zombie = fields.next()?.starts_with('Z');
    let start = fields.collect::<Vec<_>>().join(" ");
    Some(Entry {
        id: Identity { pid, start },
        ppid,
        pgid,
        zombie,
    })
}

/// SIGKILL `id` once `ps` has confirmed its start stamp. macOS has no
/// pidfd, so a pid reused between the read and the signal could be
/// signalled: a limit of the kill's precision there, not of settlement.
#[cfg(not(target_os = "linux"))]
pub(super) fn kill(id: &Identity) -> std::io::Result<()> {
    let pid = id.pid.to_string();
    let rows = match ps(&["-o", COLUMNS, "-p", pid.as_str()]).and_then(listed) {
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
