//! A server box's entry (decision 0065 slice two, U6c6b; MB3, MB5): the
//! waiting bootstrap the box runs, and the launch that starts it. The
//! observer that built the box hands its entry to the broker, each mounted
//! source named by its place among the handles it hands back beside it, so
//! the broker holding those handles launches the very box observed. The
//! entry crosses as one closed [`ServerEntry`], decoded once where the
//! observer's record is, which owns the generated identity's tree it hands
//! over and removes it once dropped where it is this process's, whatever
//! the broker answers. The launch runs the checked launcher through its
//! own handle, and carries in the bootstrap's two control pipes and
//! bubblewrap's info pipe beside the mounted handles, and nothing else;
//! bubblewrap names the box's first process, by its host pid, on the info
//! pipe (`--info-fd`). The launcher's own start is bounded by the startup's
//! absolute deadline and cancellation, as every later wait is (MB3). The
//! box stays in the broker's process group and session, and dies with its
//! launcher (`--die-with-parent`).

use std::ffi::OsStr;
#[cfg(target_os = "linux")]
use std::io::{PipeReader, PipeWriter, Read, Write};
use std::os::fd::{AsRawFd, RawFd};
#[cfg(target_os = "linux")]
use std::os::fd::{BorrowedFd, OwnedFd};
#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
#[cfg(target_os = "linux")]
use std::process::{Child, Command, Stdio};
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "linux")]
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::ServerBox;
#[cfg(target_os = "linux")]
use super::{namespace_path, Namespace};
use crate::broker::Refusal;
#[cfg(target_os = "linux")]
use crate::hands::SANDBOX_HOME;

/// The flag that mounts a source from a descriptor; the number after it is
/// the observer's, so the entry names the handle instead.
const DESCRIPTOR_MOUNT: &str = "--ro-bind-fd";

/// What enters the bootstrap: the system set's `env`, unsetting `PWD`.
/// bubblewrap sets `PWD` to the directory it entered after clearing the
/// environment, whatever `--clearenv` and `--chdir` say, and the bootstrap
/// admits MB4's fixed names alone (MB4).
#[cfg(target_os = "linux")]
const UNSET_PWD: [&str; 3] = ["/usr/bin/env", "-u", "PWD"];

/// The longest one wait of a launch's watcher lasts before the deadline
/// and the cancellation are read again.
#[cfg(target_os = "linux")]
const SLICE: Duration = Duration::from_millis(10);

/// The byte that releases a started launcher to its exec; any other fails
/// its start.
#[cfg(target_os = "linux")]
const RELEASE: u8 = 1;

/// The startup a launch is bounded by: its absolute deadline, and whether
/// the attempt is cancelled, which its watcher asks from a thread of its
/// own.
#[cfg(target_os = "linux")]
type Startup<'a> = (Instant, &'a (dyn Fn() -> bool + Sync));

/// What kills a start that overran.
#[cfg(target_os = "linux")]
const KILL: rustix::process::Signal = rustix::process::Signal::KILL;

/// One word of a box's argv: text, or the handle at a place among the box's
/// handles, which only the process holding them can number.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
enum Word {
    Text(String),
    Handle(usize),
}

/// A server box's entry (U6c6b): the bubblewrap argv that builds it, up to
/// its command, every mounted source a handle by its place; the checked
/// launcher's place among the same handles; the bootstrap it enters,
/// mounted read-only at its own path; and the private session tree its
/// generated identity lies in, handed over with it. bubblewrap mounts a
/// descriptor by the path it names, so that tree outlives the observer
/// until the box stands, and goes once the entry does (MB5).
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerEntry {
    argv: Vec<Word>,
    launcher: usize,
    bootstrap: PathBuf,
    identity: Identity,
}

/// How a session tree's name spells its owner `pid`.
fn owned_by(pid: u32) -> String {
    format!("-{pid}-")
}

/// The session tree `dir`, which this process made and names its owner,
/// renamed to name `owner` instead: what a dead-session reaper reads, so
/// the tree outlives this process while `owner` lives.
fn handed(dir: &Path, owner: u32) -> Result<PathBuf, Refusal> {
    let name = dir.file_name().and_then(OsStr::to_str);
    let name = name.ok_or(Refusal::Establishment)?;
    let creator = owned_by(std::process::id());
    let renamed = dir.with_file_name(name.replacen(&creator, &owned_by(owner), 1));
    std::fs::rename(dir, &renamed).or(Err(Refusal::Establishment))?;
    Ok(renamed)
}

/// Whether `tree` is one this process may remove once its box is settled:
/// a session tree directly in the temporary directory sessions are made
/// in, its name naming this process its owner, as [`handed`] names the
/// launching process. Any other is never removed.
fn ours(tree: &Path) -> bool {
    let named = tree.file_name().and_then(OsStr::to_str);
    let owner = named.is_some_and(|name| name.contains(&owned_by(std::process::id())));
    owner & (tree.parent() == Some(std::env::temp_dir().as_path()))
}

/// A box's generated identity tree, as its entry hands it over: removed
/// when the entry is dropped where it is this process's ([`ours`]), or in
/// the process that made the entry (`made`, never on the wire), which
/// forgets an entry it has handed over and so drops only one it could not;
/// any other process an entry names a tree to leaves it.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
struct Identity {
    tree: PathBuf,
    #[serde(skip)]
    made: bool,
}

impl Drop for Identity {
    fn drop(&mut self) {
        if self.made | ours(&self.tree) {
            std::fs::remove_dir_all(&self.tree).ok();
        }
    }
}

/// A launched box: the launcher's process; the control pipe's write end,
/// on which the broker writes the bootstrap's intent and whose close
/// releases it; the ready pipe's read end; the info pipe's read end, on
/// which bubblewrap names the box's first process; and each mounted source,
/// by the place of its handle and where it stands in the box. Dropping it
/// settles the box: the launcher is killed and reaped, which the box it
/// started does not outlive, and the pipes are closed. It borrows the
/// entry it launched, so that entry, and the generated identity tree it
/// removes, outlives the box.
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct Launched<'e> {
    pub child: Child,
    pub control: PipeWriter,
    pub ready: PipeReader,
    pub info: PipeReader,
    pub mounts: Vec<(usize, String)>,
    _entry: &'e ServerEntry,
}

#[cfg(target_os = "linux")]
impl Drop for Launched<'_> {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

impl ServerBox {
    /// The bubblewrap argv that builds the box, up to its entry, which
    /// [`ServerBox::entry`] names for the launch.
    pub(in crate::hands) fn argv(&self) -> &[String] {
        &self.intent.argv
    }

    /// The box's entry, for the broker that launches it: its argv with
    /// each mounted descriptor named by its place among
    /// [`ServerBox::handles`], the checked launcher's place there, and the
    /// generated identity's tree, which this box hands over to the process
    /// `owner` rather than removes: renamed to name `owner` its owner, so
    /// no reaper takes it while `owner` lives and one may once it has
    /// ended, its handles' paths renamed with it. The entry removes it,
    /// dropped in `owner`, or here, where it is not handed over and so is
    /// dropped rather than forgotten. A descriptor the box does not hold leaves
    /// identity unprotected; a launcher it holds no handle on is
    /// unavailable, as one never found is; a tree already handed over, or
    /// one that cannot be renamed, leaves the box not established.
    pub fn entry(&mut self, owner: u32) -> Result<ServerEntry, Refusal> {
        let numbers: Vec<RawFd> = self.handles.iter().map(|(_, fd)| fd.as_raw_fd()).collect();
        let mut argv = Vec::with_capacity(self.argv().len());
        let mut words = self.argv().iter();
        while let Some(word) = words.next() {
            argv.push(Word::Text(word.clone()));
            if word == DESCRIPTOR_MOUNT {
                let number = words.next().and_then(|number| number.parse().ok());
                let place = numbers.iter().position(|held| Some(*held) == number);
                argv.push(Word::Handle(place.ok_or(Refusal::Identity)?));
            }
        }
        let mut paths = self.handles.iter().map(|(path, _)| Some(path));
        let launcher = paths.position(|path| path == self.launcher.as_ref());
        let launcher = launcher.ok_or(Refusal::Unavailable)?;
        let scratch = self.scratch.take().ok_or(Refusal::Establishment)?;
        let identity = handed(scratch.path(), owner)?;
        for (path, _) in &mut self.handles {
            if let Ok(within) = path.strip_prefix(scratch.path()) {
                *path = identity.join(within);
            }
        }
        // Handed to the entry: the session's drop would remove the tree, by
        // its old name, which is its new one where `owner` is this process.
        // Its lock is released when this process ends.
        std::mem::forget(scratch);
        Ok(ServerEntry {
            argv,
            launcher,
            bootstrap: self.bootstrap.clone(),
            identity: Identity {
                tree: identity,
                made: true,
            },
        })
    }

    /// Launch the box `entry` names on its bootstrap, with `handles`, the
    /// box's handles in the observer's order: the checked launcher run
    /// through its own handle, from `/` under a cleared environment with no
    /// standard stream, in the broker's process group and session; the
    /// bootstrap entered from the private HOME with the arguments `command`
    /// gives for its control and ready descriptors. Each descriptor the box
    /// takes, a mounted handle, a bootstrap pipe's end or the info pipe's,
    /// is left open across the exec in the started process alone
    /// ([`inherit`]), so nothing else this process holds, the store's
    /// handle included, enters the box, and no other process this one
    /// starts meanwhile takes any of them. An entry that names an identity
    /// tree not handed to this process ([`ours`]), or a handle `handles`
    /// lacks, leaves identity unprotected; a launcher that cannot start
    /// leaves the box unavailable, and one whose start overruns `startup`
    /// ([`started`]) leaves it not established. The entry's identity tree
    /// goes with the entry, which the launched box borrows, so after the
    /// box is settled.
    #[cfg(target_os = "linux")]
    pub fn launch<'e>(
        entry: &'e ServerEntry,
        handles: &[BorrowedFd<'_>],
        command: impl FnOnce(RawFd, RawFd) -> Vec<String>,
        startup: Startup<'_>,
    ) -> Result<Launched<'e>, Refusal> {
        ours(&entry.identity.tree)
            .then_some(())
            .ok_or(Refusal::Identity)?;
        let (intent, control) = std::io::pipe().or(Err(Refusal::Establishment))?;
        let (ready, readied) = std::io::pipe().or(Err(Refusal::Establishment))?;
        let (info, informed) = std::io::pipe().or(Err(Refusal::Establishment))?;
        let carried = [
            intent.as_raw_fd(),
            readied.as_raw_fd(),
            informed.as_raw_fd(),
        ];
        let [intent_fd, ready_fd, info_fd] = carried;
        let mut carried = carried.to_vec();
        let mut argv = Vec::with_capacity(entry.argv.len());
        for word in &entry.argv {
            argv.push(match word {
                Word::Text(text) => text.clone(),
                Word::Handle(place) => {
                    let fd = handles.get(*place).ok_or(Refusal::Identity)?.as_raw_fd();
                    carried.push(fd);
                    fd.to_string()
                }
            });
        }
        let mut namespace = Namespace::entered(argv);
        namespace
            .argv
            .extend(["--info-fd".to_string(), info_fd.to_string()]);
        let bootstrap = [namespace_path(&entry.bootstrap)].into_iter();
        let entered = UNSET_PWD.map(String::from).into_iter().chain(bootstrap);
        let entered: Vec<String> = entered.chain(command(intent_fd, ready_fd)).collect();
        namespace.close(Path::new(SANDBOX_HOME), &entered);
        let launcher = handles.get(entry.launcher).ok_or(Refusal::Identity)?;
        let mut launch = Command::new(format!("/proc/self/fd/{}", launcher.as_raw_fd()));
        launch
            .args(&namespace.argv[1..])
            .env_clear()
            .current_dir("/")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let started = started(launch, inheritor(carried), startup);
        drop((intent, readied, informed));
        let mounts = entry.mounts().collect();
        Ok(Launched {
            child: started?,
            control,
            ready,
            info,
            mounts,
            _entry: entry,
        })
    }
}

impl ServerEntry {
    /// Each mounted source: the place of its handle and where it stands in
    /// the box, in mount order.
    #[cfg(target_os = "linux")]
    fn mounts(&self) -> impl Iterator<Item = (usize, String)> + '_ {
        self.argv.windows(3).filter_map(|words| match words {
            [Word::Text(flag), Word::Handle(place), Word::Text(target)] => {
                (flag == DESCRIPTOR_MOUNT).then(|| (*place, target.clone()))
            }
            _ => None,
        })
    }
}

#[cfg(target_os = "linux")]
impl Namespace {
    /// The namespace an entry's argv opened, to close on its bootstrap.
    fn entered(argv: Vec<String>) -> Namespace {
        Namespace {
            argv,
            paths: Vec::new(),
            targets: Vec::new(),
            binds: Vec::new(),
        }
    }
}

/// Leave each of `fds` open across an exec, and every other descriptor
/// above the standard streams closed by it, one this process inherited
/// without close-on-exec included: run in a launch's child between its fork
/// and its exec, so the process it starts, and no other, inherits them, and
/// nothing else. Async-signal-safe: one `close_range` and one `fcntl` each,
/// and an error that allocates nothing. A kernel without `close_range`'s
/// close-on-exec form (before 5.11) fails the start, with whatever
/// descriptor's flag was cleared meanwhile.
#[cfg(target_os = "linux")]
fn inherit(fds: &[RawFd]) -> std::io::Result<()> {
    // SAFETY: `close_range` sets one flag of every descriptor from 3 up,
    // whichever are open; it reads and writes no memory.
    let sealed = unsafe {
        libc::syscall(
            libc::SYS_close_range,
            3,
            libc::c_uint::MAX,
            libc::CLOSE_RANGE_CLOEXEC,
        )
    };
    // SAFETY: `fcntl` sets one flag of a descriptor by its number, which
    // the launch holds open for the child's whole start; it reads and
    // writes no memory.
    let carried = fds
        .iter()
        .all(|fd| unsafe { libc::fcntl(*fd, libc::F_SETFD, 0) } != -1);
    match (sealed, carried) {
        (0, true) => Ok(()),
        _ => Err(std::io::Error::last_os_error()),
    }
}

/// The hook a launch's child runs before its exec: [`inherit`] of `fds`.
#[cfg(target_os = "linux")]
pub(in crate::hands) fn inheritor(
    fds: Vec<RawFd>,
) -> impl FnMut() -> std::io::Result<()> + Send + Sync + 'static {
    move || inherit(&fds)
}

/// The hook a launch's started process runs first: [`reported`] on the
/// report and release pipes' `ends`.
#[cfg(target_os = "linux")]
pub(in crate::hands) fn reporter(
    ends: [RawFd; 2],
) -> impl FnMut() -> std::io::Result<()> + Send + Sync + 'static {
    move || reported(ends)
}

/// `launch` started with `hook`, its start, the fork and exec included,
/// bounded by `startup` (MB3). The start is made on this thread, which the
/// launcher's parent-death signal follows (bubblewrap's
/// `--die-with-parent` names the thread that started it, never the
/// process), while a thread of its own, joined before this returns,
/// watches it ([`watched`]): the started process first names itself and
/// waits to be released ([`reported`]), so that only that very process is
/// ever signalled, and only through a pidfd of it. A launcher that cannot
/// start in time is unavailable; a start that overruns is not established,
/// its process killed and reaped where it was started.
#[cfg(target_os = "linux")]
pub(in crate::hands) fn started(
    mut launch: Command,
    hook: impl FnMut() -> std::io::Result<()> + Send + Sync + 'static,
    startup: Startup<'_>,
) -> Result<Child, Refusal> {
    let (mut report, reporting) = std::io::pipe().or(Err(Refusal::Establishment))?;
    let (hold, mut releaser) = std::io::pipe().or(Err(Refusal::Establishment))?;
    let ends = [reporting.as_raw_fd(), hold.as_raw_fd()];
    // SAFETY: both hooks run between fork and exec, and make system calls
    // on descriptors and stack buffers alone ([`reported`], and `hook`'s
    // own); neither allocates.
    unsafe { launch.pre_exec(reporter(ends)).pre_exec(hook) };
    let ended = AtomicBool::new(false);
    let (started, timely) = std::thread::scope(|scope| {
        let watcher = scope.spawn(|| watched(&ended, &mut report, &mut releaser, startup));
        let started = launch.spawn().ok();
        ended.store(true, Ordering::Release);
        (started, watcher.join().unwrap_or(false))
    });
    // Every end stays open until the start has ended, so the numbers the
    // hook was given are never another descriptor's, and its report is
    // never written to a pipe no one reads.
    drop((report, reporting, hold, releaser));
    match (started, timely) {
        (Some(child), true) => Ok(child),
        (Some(mut child), false) => {
            child.kill().ok();
            child.wait().ok();
            Err(Refusal::Establishment)
        }
        (None, true) => Err(Refusal::Unavailable),
        (None, false) => Err(Refusal::Establishment),
    }
}

/// Whether a start ended, as `ended` says once it has, within `startup`:
/// its process read on `report` by its pid, held by a pidfd and then
/// released on `releaser` ([`released`]), each wait at most [`SLICE`]. One
/// that overruns before it is released, or that cannot be held, is
/// answered with any other byte and fails its own start; one that overruns
/// after, in its exec or stopped, is killed through its pidfd.
#[cfg(target_os = "linux")]
fn watched(
    ended: &AtomicBool,
    report: &mut PipeReader,
    releaser: &mut PipeWriter,
    (deadline, cancelled): Startup<'_>,
) -> bool {
    let mut pid = [0; 4];
    let mut held: Option<OwnedFd> = None;
    let mut unheld = rustix::io::ioctl_fionbio(&*report, true).is_err();
    while !ended.load(Ordering::Acquire) {
        if unheld | (Instant::now() >= deadline) | cancelled() {
            match &held {
                Some(pidfd) => rustix::process::pidfd_send_signal(pidfd, KILL).ok(),
                None => releaser.write_all(&[!RELEASE]).ok(),
            };
            return false;
        }
        if held.is_none() & (report.read(&mut pid).ok() == Some(pid.len())) {
            held = released(pid, releaser);
            unheld = held.is_none();
        }
        std::thread::sleep(SLICE);
    }
    true
}

/// The process the pid `named` names, held by a pidfd, then released on
/// `releaser`: none where it cannot be held, so it is never released.
#[cfg(target_os = "linux")]
fn released(named: [u8; 4], releaser: &mut PipeWriter) -> Option<OwnedFd> {
    let pid = rustix::process::Pid::from_raw(i32::from_ne_bytes(named));
    let flags = rustix::process::PidfdFlags::empty();
    let pidfd = pid.and_then(|pid| rustix::process::pidfd_open(pid, flags).ok());
    pidfd.filter(|_| releaser.write_all(&[RELEASE]).is_ok())
}

/// A launch's first step in its started process, before `hook`'s: its pid
/// written on the report pipe's end `reporter`, then one byte read from the
/// release pipe's end `hold`, which [`watched`] writes; anything but
/// [`RELEASE`] fails the start. Async-signal-safe: `getpid`, one `write`
/// and one `read` of stack buffers, and an error that allocates nothing.
#[cfg(target_os = "linux")]
fn reported([reporter, hold]: [RawFd; 2]) -> std::io::Result<()> {
    let mut answer = [0; 1];
    // SAFETY: `getpid` reads nothing; `write` and `read` take the stack
    // buffers they are given and descriptors the launch holds open for the
    // whole start.
    let (wrote, read) = unsafe {
        let pid = libc::getpid().to_ne_bytes();
        let wrote = libc::write(reporter, pid.as_ptr().cast(), pid.len());
        (wrote, libc::read(hold, answer.as_mut_ptr().cast(), 1))
    };
    match (wrote, read, answer) {
        (4, 1, [RELEASE]) => Ok(()),
        _ => Err(std::io::Error::from_raw_os_error(libc::ECANCELED)),
    }
}
