//! Doctor's version probes on the host: a tool asked `--version`, with a
//! bound on how long it may take and on what it may leave behind (#1137).
//!
//! A probe runs the tool in a process group of its own with stdin null,
//! and waits at most [`VERSION_BOUND`] for it to exit. Then, while the
//! leader is still unreaped, the whole group is killed, whether the tool
//! answered or not, and so is the leader, in case it left the group; the
//! leader is then reaped within [`SETTLE`], and by the same settle the
//! kernel must answer that the group is gone ([`group_gone`]). After the
//! reap no signal but signal 0 is sent. A tool that never answers, or a shim
//! that resolves a tool to itself and forks without end, costs doctor the
//! bound and the settle and no more, and leaves no process of its group
//! behind it. A tool that did not answer within the bound, whose group
//! the kernel still holds a member of, or that was not reaped within the
//! settle, reads as one that gave no version.
//!
//! The kernel decides "gone", not a process table: under `hidepid` a
//! member that runs as another uid has no row in `/proc`. The probe reads
//! no table. `kill(-pgid, 0)` answers ESRCH only once no member of the
//! group lives, whoever it runs as and whatever a table shows; any other
//! answer, success or EPERM, is a member left.
//!
//! What a process group cannot do: a descendant that leaves the group
//! (`setsid`, `setpgid`) is out of the group kill's reach and survives it.
//! It may hold the tool's standard output open. The read ends at the bound
//! all the same, whatever holds the pipe, so it never holds doctor past
//! the bound, and a tool whose output did not close gave no version.
//!
//! SIGCHLD is at its default for a probe's life ([`ChildrenWaited`]), so
//! the kernel never reaps the leader itself: it stays the probe's zombie,
//! and its id the probe's, until doctor reaps it after both kills. A
//! host that refuses the default gets no probe: nothing is spawned, and
//! the tool gave no version. A disposition that could not be put back is
//! a line of doctor's report ([`restored`]). A leader that is no child to
//! wait for all the same (ECHILD) left no status to read, and its id may
//! already name another process: it is sent neither kill, and gave no
//! version.
//!
//! Every wait is polled by [`before`], which reads the clock before each
//! poll and again when the poll answers, so nothing first observed past a
//! deadline is accepted, however late a poll begun in time answers. The
//! one observation that vetoes rather than accepts, a leader that is no
//! child (ECHILD), is looked for once more just before the kills, whatever
//! the clock reads.

use std::io::Read;
use std::os::fd::AsFd;
use std::os::unix::process::CommandExt;
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use brokkr_protocol::adapters::DshInvocation;
use rustix::event::PollFlags;
use rustix::io::Errno;
use rustix::process::{
    kill_process, kill_process_group, test_kill_process_group, waitid, Pid, Signal, WaitId,
    WaitIdOptions,
};

use super::{safe_line, Report};
use crate::broker::ready;

/// How long doctor waits for a tool on the host to answer. A version
/// banner is milliseconds' work; a tool still running at this bound is
/// not going to answer.
pub(super) const VERSION_BOUND: Duration = Duration::from_secs(5);

/// How long doctor waits, after the kill, for the killed leader to be
/// reaped and the kernel to answer its group gone. A leader the kill
/// could not reach is left unreaped past it.
const SETTLE: Duration = Duration::from_secs(1);

/// The most of a tool's output a probe keeps: the report keeps one line,
/// and a tool that writes without end costs no more than this. What it
/// writes past the cap is read and discarded, so its pipe never ends it.
const READ_CAP: usize = 4096;

/// How often the wait looks whether the tool has exited.
const POLL: Duration = Duration::from_millis(10);

pub(super) fn tool_version(program: &str) -> Option<String> {
    let mut command = Command::new(program);
    // POSIX `sh` has no portable `--version`; dash exits 2 for it even
    // though the executable is healthy. The exec adapter needs presence,
    // not a shell brand, so probe the one operation every `sh` promises.
    if program == "sh" {
        command.args(["-c", "printf 'POSIX shell'"]);
        return answered(command, VERSION_BOUND).map(|out| String::from_utf8_lossy(&out).into());
    }
    command.arg("--version");
    answered(command, VERSION_BOUND).map(|out| safe_line(&out))
}

/// The version of a SELECTED executable, asked as native asks it: the
/// candidate the seam's search found, under the name it was looked up
/// by. Probing the canonical target instead reported `env`'s version for
/// a `dsh` symlinked to it, where native exits 1 on the name mismatch
/// (review of run `124cca78`, R2). Nothing is searched for again: the
/// invocation's program is a path.
pub(super) fn invocation_version(invocation: &DshInvocation) -> Option<String> {
    let mut command = invocation.command();
    command.arg("--version");
    answered(command, VERSION_BOUND).map(|out| safe_line(&out))
}

/// What `command` wrote on its standard output, when it exited 0 within
/// `bound` and its output closed within it too, it was reaped within
/// [`SETTLE`], and the kernel answered its group gone by then.
fn answered(command: Command, bound: Duration) -> Option<Vec<u8>> {
    answered_with(command, bound, &mut sent, disposed)
}

/// [`answered`], with every kill and every question about the group put
/// to `call`, and SIGCHLD's disposition set through `set`: [`sent`] and
/// [`disposed`] on the host, a played answer in a test. A default that
/// `set` refuses spawns nothing.
fn answered_with(
    mut command: Command,
    bound: Duration,
    call: &mut Calls<'_>,
    set: Sigaction,
) -> Option<Vec<u8>> {
    let _waited = ChildrenWaited::held(set)?;
    let deadline = Instant::now() + bound;
    let spawned = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn();
    settled(spawned.ok()?, deadline, call)
}

/// What the spawned `child` answered by `deadline`, once its group and
/// it are killed and it is reaped. Both kills are sent before the reap,
/// while the leader is unreaped and its zombie pins the group's id, so
/// neither lands on a recycled id; a leader the probe cannot wait for is
/// sent neither. After the reap the group is only asked ([`group_gone`]).
///
/// One group kill is enough, so it is not repeated: once the kernel has
/// delivered SIGKILL to a group, no member of it can fork a new one. A
/// member doctor may not signal (EPERM) is not killed, keeps the group,
/// and so the read is no version: fail-closed.
fn settled(mut child: Child, deadline: Instant, call: &mut Calls<'_>) -> Option<Vec<u8>> {
    let read = drained(child.stdout.take(), deadline);
    let exited = match exited_by(&child, deadline) {
        Leader::Reaped => return None,
        Leader::Exited => true,
        Leader::Running => false,
    };
    let left = deadline.saturating_duration_since(Instant::now());
    let output = exited.then(|| read.recv_timeout(left).ok()).flatten();
    let leader = Pid::from_child(&child);
    // One last look just before the kills, whatever the clock reads: a
    // leader that is no child to wait for is sent neither, even when its
    // ECHILD was first seen past the deadline, which `before` refuses.
    if seen(leader) == Some(Leader::Reaped) {
        return None;
    }
    // Neither kill's own answer is read: the bounded reap shows whether
    // the leader ended, and the kernel's answer whether the group did.
    call(Call::KillGroup, leader).ok();
    call(Call::KillLeader, leader).ok();
    let settle = Instant::now() + SETTLE;
    // The group is asked after every reap, whatever the leader's status,
    // so a killed group's end is observed on the timed-out path too.
    let status = reaped(&mut child, settle);
    let gone = status.is_some() && group_gone(leader, settle, call);
    let answered = gone && status.is_some_and(|status| status.success());
    match output {
        Some(Answer::Closed(bytes)) => answered.then_some(bytes),
        Some(Answer::Expired) | None => None,
    }
}

/// What a probe asks of the kernel about the group its leader leads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Call {
    /// SIGKILL to every member of the group.
    KillGroup,
    /// SIGKILL to the leader by its pid, in case it left the group.
    KillLeader,
    /// `kill(-pgid, 0)`: does any member of the group live?
    AskGroup,
}

/// The closure seam a probe's calls go through ([`answered_with`]).
type Calls<'a> = dyn FnMut(Call, Pid) -> rustix::io::Result<()> + 'a;

/// `call` made of the kernel, for the group `leader` leads.
fn sent(call: Call, leader: Pid) -> rustix::io::Result<()> {
    match call {
        Call::KillGroup => kill_process_group(leader, Signal::KILL),
        Call::KillLeader => kill_process(leader, Signal::KILL),
        Call::AskGroup => test_kill_process_group(leader),
    }
}

/// Is the group `group` gone by `until`, by the kernel's own answer? It is
/// asked only once its leader is reaped, so no zombie leader answers for
/// it, and only with signal 0, which delivers nothing. ESRCH within the
/// settle is gone. Any other answer, success or EPERM, is a member left,
/// whether a table could see it or not, and at the settle's end the group
/// is not gone.
///
/// Why it sends no kill: the group's id is pinned only while the leader
/// is unreaped, as the engine's process tree rules (process/tree.rs,
/// lines 8 to 13: the group is signalled only while the leader is
/// unreaped, never a signal to a stranger). Once the leader is reaped, a
/// success or an EPERM pins the id only at the moment it is given: the
/// last member may end, and the id be handed to a new group, before the
/// next call. A kill then would reach a stranger; signal 0 harms none.
fn group_gone(group: Pid, until: Instant, call: &mut Calls<'_>) -> bool {
    let gone = before(until, |left| {
        let gone = call(Call::AskGroup, group) == Err(Errno::SRCH);
        napped(gone.then_some(()), left)
    });
    gone.is_some()
}

/// What `observe` first answers before `until`; `None` once the clock
/// reads `until`. The clock is read before each poll, so no poll begins
/// past the deadline, the rule of the tests' `exited_before`
/// (tests/support/reaped.rs), and again when a poll answers, so an
/// observation a poll begun in time first makes past the deadline is not
/// accepted either. Each poll is given what is left of the bound.
fn before<T>(until: Instant, mut observe: impl FnMut(Duration) -> Option<T>) -> Option<T> {
    loop {
        let left = until.checked_duration_since(Instant::now());
        if let Some(seen) = observe(left.filter(|left| !left.is_zero())?) {
            return (Instant::now() < until).then_some(seen);
        }
    }
}

/// `seen`, after a nap of at most `left` when there is nothing yet: the
/// pause between two polls that do not block.
fn napped<T>(seen: Option<T>, left: Duration) -> Option<T> {
    if seen.is_none() {
        std::thread::sleep(POLL.min(left));
    }
    seen
}

/// SIGCHLD held at its default disposition while a probe runs, and the
/// disposition it replaced put back once the last probe holding it ends.
/// Under SIGCHLD ignored the kernel reaps an exited child itself, and its
/// id is free at once; at the default, a leader stays the probe's zombie
/// until doctor reaps it, so no kill reaches a recycled id. The count lets
/// probes on several threads share the one process-wide disposition.
struct ChildrenWaited {
    /// How the disposition is put back.
    set: Sigaction,
}

/// The closure seam SIGCHLD's disposition is set through: it sets the one
/// given and answers the one it replaced. [`disposed`] on the host.
type Sigaction = fn(&Disposition) -> std::io::Result<Disposition>;

/// What the held [`ChildrenWaited`] share.
struct Waited {
    /// How many are held.
    held: usize,
    /// The disposition the first of them replaced.
    replaced: Option<Disposition>,
    /// Why the last to end could not put it back, until it is reported.
    unrestored: Option<std::io::Error>,
}

static WAITED: Mutex<Waited> = Mutex::new(Waited {
    held: 0,
    replaced: None,
    unrestored: None,
});

/// The shared [`Waited`], whatever a panicking holder left it.
fn waited() -> MutexGuard<'static, Waited> {
    WAITED.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A SIGCHLD disposition, as `sigaction` reads and sets it.
struct Disposition(libc::sigaction);

impl ChildrenWaited {
    /// The default held, or `None` when `set` refused it: the probe then
    /// spawns nothing and gives no version (fail-closed), since a leader
    /// the kernel may reap itself could hand its id to a stranger.
    fn held(set: Sigaction) -> Option<ChildrenWaited> {
        let mut waited = waited();
        if waited.held == 0 {
            waited.replaced = Some(set(&defaulted()).ok()?);
        }
        waited.held += 1;
        Some(ChildrenWaited { set })
    }
}

impl Drop for ChildrenWaited {
    fn drop(&mut self) {
        let mut waited = waited();
        waited.held -= 1;
        if waited.held == 0 {
            let restored = waited.replaced.take().map(|prior| (self.set)(&prior));
            if let Some(Err(error)) = restored {
                waited.unrestored = Some(error);
            }
        }
    }
}

/// `report`, with one warning when a probe could not put back the SIGCHLD
/// disposition it replaced; the failure is reported once.
pub(super) fn restored(mut report: Report) -> Report {
    if let Some(error) = waited().unrestored.take() {
        let detail = format!("a version probe could not put back its disposition: {error}");
        report.warn("SIGCHLD", detail);
    }
    report
}

/// SIGCHLD's default disposition: no handler, no flags, an empty mask.
fn defaulted() -> Disposition {
    // SAFETY: `sigaction` is plain data, and all zeroes is SIG_DFL with
    // no flags and an empty mask on Linux and macOS.
    let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
    action.sa_sigaction = libc::SIG_DFL;
    Disposition(action)
}

/// Set SIGCHLD to `to`, answering the disposition it replaced, or the
/// error the host refused the call with.
fn disposed(to: &Disposition) -> std::io::Result<Disposition> {
    let mut prior = std::mem::MaybeUninit::<libc::sigaction>::zeroed();
    // SAFETY: `to` is a whole action, and `prior` is memory sized for one
    // and zeroed, so it is initialised whatever the call does.
    unsafe {
        let set = libc::sigaction(libc::SIGCHLD, &to.0, prior.as_mut_ptr());
        let error = std::io::Error::last_os_error();
        (set == 0)
            .then_some(Disposition(prior.assume_init()))
            .ok_or(error)
    }
}

/// The leader's status, once it is reaped by `until`; `None` when it is
/// not, so a leader the kill could not reach never holds doctor past it.
fn reaped(child: &mut Child, until: Instant) -> Option<ExitStatus> {
    before(until, |left| napped(child.try_wait().ok().flatten(), left))
}

/// How a probe's read of the tool's output ended. Only a closed read is
/// a version.
#[derive(Debug, PartialEq, Eq)]
enum Answer {
    /// Every holder closed the pipe: what it carried, to the cap.
    Closed(Vec<u8>),
    /// The deadline passed with the pipe still open, or a read failed.
    Expired,
}

/// How `pipe` ended by `deadline`, read on a thread of its own so the
/// wait keeps its bound.
fn drained(pipe: Option<ChildStdout>, deadline: Instant) -> Receiver<Answer> {
    let (sender, received) = channel();
    std::thread::spawn(move || {
        let answer = pipe.map_or(Answer::Expired, |pipe| kept(pipe, deadline));
        sender.send(answer).ok();
    });
    received
}

/// Read `pipe` until its end or `deadline`, keeping the first
/// [`READ_CAP`] bytes and discarding the rest, so a long answer is never
/// ended by its pipe and one without end costs no more than the cap. A
/// read waits only on what poll finds ready, so a pipe that a descendant
/// outside the group holds open ends the read at the deadline, expired,
/// and a close first seen at or past the deadline is expired too
/// ([`before`]).
fn kept(mut pipe: impl Read + AsFd, deadline: Instant) -> Answer {
    let mut bytes = Vec::new();
    let mut chunk = [0; READ_CAP];
    let ended = before(deadline, |left| {
        if !ready(&pipe, PollFlags::IN, left).unwrap_or(false) {
            return None;
        }
        match pipe.read(&mut chunk) {
            Ok(0) => Some(Answer::Closed(std::mem::take(&mut bytes))),
            Err(_) => Some(Answer::Expired),
            Ok(read) => {
                let room = READ_CAP - bytes.len();
                bytes.extend_from_slice(&chunk[..read.min(room)]);
                None
            }
        }
    });
    ended.unwrap_or(Answer::Expired)
}

/// What the wait saw of the probe's leader by its deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Leader {
    /// It exited and is unreaped, so its group id is still the probe's.
    Exited,
    /// It still runs.
    Running,
    /// It is no child doctor can wait for (ECHILD): something reaped it
    /// first, and its id may already be another process's.
    Reaped,
}

/// What `child` did by `deadline`, observed with `WNOWAIT`, so an exited
/// leader stays unreaped and its group id stays the probe's until the
/// kill.
fn exited_by(child: &Child, deadline: Instant) -> Leader {
    let leader = Pid::from_child(child);
    let exited = before(deadline, |left| napped(seen(leader), left));
    exited.unwrap_or(Leader::Running)
}

/// What one `WNOWAIT` poll sees of `leader` now, leaving it unreaped:
/// exited, no child to wait for, or nothing yet.
fn seen(leader: Pid) -> Option<Leader> {
    let options = WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT;
    match waitid(WaitId::Pid(leader), options) {
        Ok(Some(_)) => Some(Leader::Exited),
        Err(Errno::CHILD) => Some(Leader::Reaped),
        Ok(None) | Err(_) => None,
    }
}

#[cfg(test)]
mod tests;
