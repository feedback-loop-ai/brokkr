//! The host's process table, as the attempt's tree is read from it (#403).
//! Linux reads `/proc`; macOS reads `ps`. A process is named by its pid
//! AND its start stamp, so a pid the kernel has since handed to another
//! process never reads as the one recorded.

use rustix::process::{Pid, Signal};

/// One process, by pid and the kernel's start stamp for it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Identity {
    pub(super) pid: i32,
    pub(super) start: String,
}

/// One row of a snapshot. A zombie has exited: it holds a pid until its
/// parent reaps it, and it runs nothing, so it counts as gone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Entry {
    pub(super) id: Identity,
    pub(super) ppid: i32,
    pub(super) pgid: i32,
    /// The session, as a token compared only with another row's. macOS
    /// leaves it empty: no process is adopted there (no subreaper), and
    /// the session is read only to tell an adopted orphan apart.
    pub(super) session: String,
    pub(super) zombie: bool,
}

/// Every process the host lists. A process that exits between the listing
/// and its read is simply not in the snapshot.
#[cfg(target_os = "linux")]
pub(super) fn snapshot() -> std::io::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for dir in std::fs::read_dir("/proc")? {
        let name = dir?.file_name();
        let pid = name.to_str().and_then(|name| name.parse::<i32>().ok());
        entries.extend(pid.and_then(read));
    }
    Ok(entries)
}

#[cfg(target_os = "linux")]
fn read(pid: i32) -> Option<Entry> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    parse_stat(pid, &stat)
}

/// `/proc/<pid>/stat`: the command name is parenthesised and may hold any
/// byte, so fields are counted from its closing parenthesis — state,
/// ppid, pgrp, session, and the start time nineteen fields on.
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
        session: fields.get(3)?.to_string(),
        zombie: ["Z", "X"].contains(fields.first()?),
    })
}

/// SIGKILL `id`, and nothing else. A pidfd pins the process it was opened
/// on, so when the start stamp read after opening it still matches, the
/// signal cannot reach a process that reused the pid. A process already
/// gone is not signalled; whether the signal ended it is read afterwards,
/// from the table, which is the proof `tree::end` settles on.
#[cfg(target_os = "linux")]
pub(super) fn kill(id: &Identity) {
    use rustix::process::{pidfd_open, pidfd_send_signal, PidfdFlags};
    let pidfd = Pid::from_raw(id.pid).map(|pid| pidfd_open(pid, PidfdFlags::empty()));
    if let Some(Ok(pidfd)) = pidfd {
        if read(id.pid).is_some_and(|entry| entry.id == *id) {
            let _ = pidfd_send_signal(&pidfd, Signal::KILL);
        }
    }
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
pub(super) fn snapshot() -> std::io::Result<Vec<Entry>> {
    ps(&["-axo", COLUMNS])
}

#[cfg(not(target_os = "linux"))]
const COLUMNS: &str = "pid=,ppid=,pgid=,stat=,lstart=";

#[cfg(not(target_os = "linux"))]
fn ps(args: &[&str]) -> std::io::Result<Vec<Entry>> {
    let output = std::process::Command::new("ps")
        .args(args)
        .env("LC_ALL", "C")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_ps)
        .collect())
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
        session: String::new(),
        zombie,
    })
}

/// SIGKILL `id` once `ps` has confirmed its start stamp. macOS has no
/// pidfd, so a pid reused between the read and the signal is the named
/// residual of this platform.
#[cfg(not(target_os = "linux"))]
pub(super) fn kill(id: &Identity) {
    let pid = id.pid.to_string();
    let rows = ps(&["-o", COLUMNS, "-p", pid.as_str()]).unwrap_or_default();
    if rows.iter().any(|entry| entry.id == *id && !entry.zombie) {
        let _ = Pid::from_raw(id.pid).map(|pid| rustix::process::kill_process(pid, Signal::KILL));
    }
}

/// launchd adopts every orphan on macOS; the engine never does.
#[cfg(not(target_os = "linux"))]
pub(super) fn reap(_pid: i32) {}

#[cfg(test)]
mod tests;
