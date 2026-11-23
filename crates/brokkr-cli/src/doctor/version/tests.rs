//! A test that can deliver a real signal, through the probe's own kills
//! or its own, runs in a PID namespace of its own (#1137g,
//! [`rerun_alone`]), so no pid or group id it names reaches a process
//! outside it, and the re-run proves it is isolated before it can deliver
//! anything (#1137h). Those tests are Linux only: macOS has no PID
//! namespace, and keeps the tests whose seam is inert. A test that only
//! needs a process ended lets it go through a FIFO ([`held`]) instead.

use super::*;
use crate::doctor::hermetic_tests::rerun;
#[cfg(target_os = "linux")]
use crate::doctor::hermetic_tests::{doctored_in, rerun_alone, rerun_alone_through, rerunning};
use rustix::io::Errno;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// How long a test waits for anything before it fails instead of hanging:
/// the bound and generous slack for a loaded host.
const PATIENCE: Duration = Duration::from_secs(60);

/// A shell script at `dir/name` running `body`, written by a child
/// process so no write descriptor of this test binary's is open on it
/// when it is executed (ETXTBSY, #255).
fn plant(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    let mut writer = Command::new("/bin/sh")
        .args(["-c", "printf '#!/bin/sh\\n%s\\n' \"$1\" > \"$0\""])
        .arg(&path)
        .arg(body)
        .spawn()
        .unwrap();
    let status = reaped(&mut writer, Instant::now() + PATIENCE);
    assert!(
        status.is_some_and(|status| status.success()),
        "planting {name}"
    );
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// The tool at `tool` probed by `tool_version`; see [`timed`].
#[cfg(target_os = "linux")]
fn probed(tool: &Path) -> (Option<String>, Duration) {
    let program = tool.to_str().unwrap().to_owned();
    timed(move || tool_version(&program))
}

/// `probe` run on a thread of its own, with how long it took; a probe
/// that has not answered within PATIENCE fails here.
fn timed<T: Send + 'static>(probe: impl FnOnce() -> T + Send + 'static) -> (T, Duration) {
    let (sender, received) = channel();
    std::thread::spawn(move || {
        let started = Instant::now();
        let version = probe();
        sender.send((version, started.elapsed())).ok();
    });
    received
        .recv_timeout(PATIENCE)
        .expect("the probe answered within PATIENCE")
}

/// The pid a planted tool wrote to `file`.
#[cfg(target_os = "linux")]
fn pid_in(file: &Path) -> Pid {
    let text = std::fs::read_to_string(file).unwrap();
    Pid::from_raw(text.trim().parse().unwrap()).unwrap()
}

/// A FIFO at `dir/held` and the test's own end of it. A planted process
/// that must outlive the probe reads the FIFO until its end, which comes
/// when the test drops the holder, on success or on a panic's unwind. So
/// no test signals an id it does not hold: a process that is not this
/// test's unreaped child may have ended and its id gone to a stranger
/// (process/tree.rs, lines 8 to 13). The holder is close-on-exec, so no
/// probed tool inherits it.
fn held(dir: &Path) -> (PathBuf, std::fs::File) {
    use std::os::unix::ffi::OsStrExt;
    let path = dir.join("held");
    let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    // SAFETY: `name` is a NUL-terminated path that outlives the call.
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let mut options = std::fs::OpenOptions::new();
    let holder = options.read(true).write(true).open(&path).unwrap();
    (path, holder)
}

/// Perl that opens the FIFO named by its second argument, writes its pid
/// to its first, and reads the FIFO until the test lets it go ([`held`]).
/// The FIFO is opened before the pid is published, and without blocking,
/// so a holder the test has already dropped, as an early failure's unwind
/// does, ends the read at once instead of leaving the open waiting for a
/// writer that never comes; the read itself then blocks.
const HOLDS: &str = "use Fcntl; sysopen(my $h, $ARGV[1], O_RDONLY | O_NONBLOCK) or die; \
    fcntl($h, F_SETFL, 0) or die; \
    open(my $f, \">\", $ARGV[0]) or die; print $f \"$$\\n\"; close $f; my @end = <$h>";

/// Has `pid` stopped running within PATIENCE? Its end is waited for,
/// never assumed, since a kill is delivered asynchronously.
#[cfg(target_os = "linux")]
fn stopped(pid: Pid) -> bool {
    let started = Instant::now();
    while started.elapsed() < PATIENCE {
        if !running(pid) {
            return true;
        }
        std::thread::sleep(POLL);
    }
    false
}

/// Is `pid` a process that still runs? A zombie does not: a killed orphan
/// is reparented to the namespace's init, which may reap it late, and
/// until then `kill(pid, 0)` still finds it.
#[cfg(target_os = "linux")]
fn running(pid: Pid) -> bool {
    let stat = std::fs::read_to_string(format!("/proc/{}/stat", pid.as_raw_nonzero()));
    let stat = stat.unwrap_or_default();
    let state = stat.rsplit_once(") ").map(|(_, rest)| rest.chars().next());
    state
        .flatten()
        .is_some_and(|state| !matches!(state, 'Z' | 'X'))
}

/// The one home of a never-answering tool's setup (#1137f review, the
/// LOW): a tool planted in `dir` as `name` that `exec`s [`HOLDS`], the
/// only member of its group, writing its pid to `dir/leader`, and the
/// holder of its FIFO, which keeps it from ever answering.
fn never_answering(dir: &Path, name: &str) -> (PathBuf, std::fs::File) {
    let (fifo, holder) = held(dir);
    let body = format!(
        "exec /usr/bin/perl -e '{HOLDS}' '{}' '{}'",
        dir.join("leader").display(),
        fifo.display()
    );
    (plant(dir, name, &body), holder)
}

/// A [`never_answering`] tool asked `--version` through the seam: it is
/// no version, the probe waited the bound, both kills came while its
/// leader was unreaped, and its group was asked once it was reaped, so no
/// process of it remains. The seam is inert: a kill lets the tool go by
/// dropping its FIFO's holder, and the question is answered gone, so the
/// test signals nothing and asks no id after the reap (#1137e, F4).
fn never_answers(dir: &Path) {
    let (tool, holder) = never_answering(dir, "never");
    let mut command = Command::new(tool);
    command.arg("--version");
    let mut holder = Some(holder);
    let on = move |record: &mut Record, call, leader| {
        if call != Call::AskGroup {
            holder.take();
        }
        record.answered(call, leader, || Err(Errno::SRCH))
    };
    let (version, took, record) = played_as(command, disposed, Record::default(), on);
    assert_eq!(version, None);
    at_the_bound(took);
    killed_then_asked_once(record);
}

/// Both kills, then the reap, then one question: the calls of a probe
/// whose group the kernel answered gone at once.
fn killed_then_asked_once(record: Record) {
    let expected = [Call::KillGroup, Call::KillLeader, Call::AskGroup];
    assert_eq!(
        (record.calls, record.reaped_at),
        (expected.to_vec(), Some(2))
    );
}

/// What a probe may take past the bound: the settle's reap and the
/// kernel's answer for the group (one [`SETTLE`]), and 4 seconds for a
/// loaded host.
const MARGIN: Duration = Duration::from_secs(5);

/// A probe that waited the bound and no longer than [`MARGIN`] past it.
fn at_the_bound(took: Duration) {
    assert!(took >= VERSION_BOUND, "the probe waited {took:?}");
    assert!(took < VERSION_BOUND + MARGIN, "the probe waited {took:?}");
}

#[test]
fn a_tool_that_never_answers_gives_no_version_at_the_bound_and_leaves_no_process() {
    let dir = tempfile::tempdir().unwrap();
    never_answers(dir.path());
}

/// The dsh probe keeps the same bound: a selected `dsh` that never
/// answers, asked through `invocation_version` itself (#1137f review, the
/// MEDIUM), is no version at the bound, and the probe's own kill ended
/// it while the test still held its FIFO.
#[cfg(target_os = "linux")]
#[test]
fn an_invocation_that_never_answers_gives_no_version_at_the_bound() {
    if rerun_alone(&[]) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let (tool, _holder) = never_answering(dir.path(), "dsh");
    let (version, took) = timed(move || invocation_version(&DshInvocation::of(tool)));
    assert_eq!(version, None);
    at_the_bound(took);
    let leader = pid_in(&dir.path().join("leader"));
    assert!(
        stopped(leader),
        "the never-answering dsh outlived the probe"
    );
}

/// A leader that moves itself into another process group of the session,
/// out of reach of the group's kill, is killed by its pid and reaped.
/// The group it joins is the test's, made inside the namespace first
/// (#1137h, I5): the test's inherited group is the host's and has no id
/// there, so a leader that asked for it would stay in its own group and
/// die of the group's kill alone. The tool refuses, and so never writes
/// its pid, unless it joined a group the namespace can see, not its own.
#[cfg(target_os = "linux")]
#[test]
fn a_leader_that_left_its_group_is_killed_and_reaped() {
    if rerun_alone(&[]) {
        return;
    }
    rustix::process::setpgid(None, None).expect("the test leads a group of its own");
    let dir = tempfile::tempdir().unwrap();
    let leader = dir.path().join("leader");
    let (fifo, _holder) = held(dir.path());
    let script = format!(
        "my $g = getpgrp(getppid()); $g > 0 && $g != $$ or die; \
         setpgrp(0, $g) or die; getpgrp() == $g or die; {HOLDS}"
    );
    let body = format!(
        "exec /usr/bin/perl -e '{script}' '{}' '{}'",
        leader.display(),
        fifo.display()
    );
    let (version, _) = probed(&plant(dir.path(), "escaping", &body));
    assert_eq!(version, None);
    assert!(stopped(pid_in(&leader)), "the leader outlived the probe");
}

/// A tool that writes without end is read, and what passes the cap is
/// discarded, until the bound, where it is killed: no version.
#[cfg(target_os = "linux")]
#[test]
fn a_tool_that_writes_without_end_is_killed_at_the_bound() {
    if rerun_alone(&[]) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let body = "while :; do echo 'tool 1.0'; done";
    let (version, took) = probed(&plant(dir.path(), "chatty", body));
    assert_eq!(version, None);
    at_the_bound(took);
}

/// M4 of #1137's review: a long but finite answer is still a version. The
/// tool writes its line and then 1 MiB, past the cap, and is never ended
/// by its pipe. The shell writes it with its own `printf`, so a closed
/// pipe is met by the tool itself, whatever the host's coreutils do.
#[cfg(target_os = "linux")]
#[test]
fn a_tool_whose_version_line_is_followed_by_a_mebibyte_reports_its_line() {
    if rerun_alone(&[]) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let body =
        "echo 'tool 1.0'\ni=0\nwhile [ $i -lt 1024 ]; do printf '%01024d' 0; i=$((i+1)); done";
    let (version, took) = probed(&plant(dir.path(), "long", body));
    assert_eq!(version.as_deref(), Some("tool 1.0"));
    assert!(took < VERSION_BOUND, "the probe waited {took:?}");
}

/// M5 of #1137's review: a descendant that leaves the group is out of the
/// group kill's reach and may hold the tool's output open. The probe
/// still ends at the bound, and the tool, whose output never closed, gave
/// no version; the descendant still runs, which is what a group cannot do.
#[cfg(target_os = "linux")]
#[test]
fn a_descendant_that_left_the_group_holding_the_pipe_holds_the_probe_only_to_the_bound() {
    if rerun_alone(&[]) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let escaped = dir.path().join("escaped");
    let (fifo, holder) = held(dir.path());
    let script = format!("setpgrp(0, 0) or die; {HOLDS}");
    let body = format!(
        "echo 'tool 1.0'\n/usr/bin/perl -e '{script}' '{}' '{}' &",
        escaped.display(),
        fifo.display()
    );
    let (version, took) = probed(&plant(dir.path(), "escaping", &body));
    assert_eq!(version, None);
    at_the_bound(took);
    let pid = pid_in(&escaped);
    assert!(running(pid), "the escaped descendant was out of reach");
    drop(holder);
    assert!(stopped(pid), "the escaped descendant ended when let go");
}

/// A probe of a tool planted to run `body`, with every call it makes put
/// to `on` beside `state`: what it answered, how long it took, and the
/// state `on` left.
#[cfg(target_os = "linux")]
fn played_with<S: Send + 'static>(
    body: &str,
    state: S,
    on: impl FnMut(&mut S, Call, Pid) -> rustix::io::Result<()> + Send + 'static,
) -> (Option<Vec<u8>>, Duration, S) {
    let dir = tempfile::tempdir().unwrap();
    let tool = plant(dir.path(), "prompt", body);
    played_as(Command::new(tool), disposed, state, on)
}

/// [`played_with`], probing `command` with SIGCHLD set through `set`.
fn played_as<S: Send + 'static>(
    command: Command,
    set: Sigaction,
    state: S,
    mut on: impl FnMut(&mut S, Call, Pid) -> rustix::io::Result<()> + Send + 'static,
) -> (Option<Vec<u8>>, Duration, S) {
    let ((answer, state), took) = timed(move || {
        let mut state = state;
        let call = &mut |call, leader| on(&mut state, call, leader);
        let answer = answered_with(command, VERSION_BOUND, call, set);
        (answer, state)
    });
    (answer, took, state)
}

/// The calls a probe made, in order, and where its leader's reap fell
/// among them: the index of the first call made once it was reaped.
#[derive(Default)]
struct Record {
    calls: Vec<Call>,
    reaped_at: Option<usize>,
}

impl Record {
    /// Note `call`, made for the group `leader` leads, and whether the
    /// leader was reaped by then: no status of it is left to wait for
    /// (ECHILD). The first ECHILD is a permanent veto, answered here: the
    /// leader is never looked up again, so a child that takes its pid
    /// after it is never mistaken for it.
    fn noted(&mut self, call: Call, leader: Pid) -> bool {
        if self.reaped_at.is_none() && seen(leader) == Some(Leader::Reaped) {
            self.reaped_at = Some(self.calls.len());
        }
        self.calls.push(call);
        self.reaped_at.is_some()
    }

    /// `call` noted and answered inertly: a kill is answered and never
    /// sent, and a question about the group is answered by `asked`, never
    /// asked of the kernel, since it follows the reap. A probe, or a
    /// mutant of it, that kills after the reap is caught by the call the
    /// seam records.
    fn answered(
        &mut self,
        call: Call,
        leader: Pid,
        asked: impl FnOnce() -> rustix::io::Result<()>,
    ) -> rustix::io::Result<()> {
        self.noted(call, leader);
        match call {
            Call::AskGroup => asked(),
            Call::KillGroup | Call::KillLeader => Ok(()),
        }
    }

    /// `call` noted and, when it is a kill, delivered by [`sent`] until
    /// the seam's first ECHILD of the leader, the veto, and never after
    /// it; a question is answered by `asked`. Only a test alone in its
    /// PID namespace delivers ([`rerun_alone`]), so a kill can reach no
    /// process outside the test whatever id it names.
    #[cfg(target_os = "linux")]
    fn delivered(
        &mut self,
        call: Call,
        leader: Pid,
        asked: impl FnOnce() -> rustix::io::Result<()>,
    ) -> rustix::io::Result<()> {
        let vetoed = self.noted(call, leader);
        match call {
            Call::AskGroup => asked(),
            Call::KillGroup | Call::KillLeader if !vetoed => sent(call, leader),
            Call::KillGroup | Call::KillLeader => Ok(()),
        }
    }
}

/// A probe of a tool that writes `tool 1.0` and exits 0 at once; see
/// [`played_by`].
fn played(
    asked: impl FnMut() -> rustix::io::Result<()> + Send + 'static,
) -> (Option<Vec<u8>>, Duration, Record) {
    played_by("echo 'tool 1.0'", asked)
}

/// A probe of a tool planted to run `body`; see [`inertly`].
fn played_by(
    body: &str,
    asked: impl FnMut() -> rustix::io::Result<()> + Send + 'static,
) -> (Option<Vec<u8>>, Duration, Record) {
    let dir = tempfile::tempdir().unwrap();
    let tool = plant(dir.path(), "prompt", body);
    inertly(Command::new(tool), disposed, asked)
}

/// A probe of `command` with SIGCHLD set through `set` and every call
/// recorded through an inert seam: no kill is sent, and each question
/// about the group is answered by `asked`.
fn inertly(
    command: Command,
    set: Sigaction,
    mut asked: impl FnMut() -> rustix::io::Result<()> + Send + 'static,
) -> (Option<Vec<u8>>, Duration, Record) {
    played_as(
        command,
        set,
        Record::default(),
        move |record, call, leader| record.answered(call, leader, &mut asked),
    )
}

/// Both kills, then the question alone, again and again to the settle:
/// the calls of a probe whose group the kernel never answered gone. It
/// took the settle and no more.
fn kept_asking(calls: &[Call], took: Duration) {
    let (killed, asked) = calls.split_at(2);
    assert_eq!(killed, [Call::KillGroup, Call::KillLeader]);
    assert!(asked.len() > 2, "{calls:?}");
    assert!(
        asked.iter().all(|call| *call == Call::AskGroup),
        "{calls:?}"
    );
    assert!(took >= SETTLE, "the probe waited {took:?}");
    assert!(took < VERSION_BOUND, "the probe waited {took:?}");
}

/// H1 of #1137's hold: a member no table can see, such as one under
/// another uid behind `hidepid`, still holds its group, and the kernel
/// refuses the question with EPERM. A tool that answered at once and
/// exited 0 is then no version, and the probe kept asking to the settle.
#[test]
fn a_member_the_kernel_answers_eperm_for_is_no_version_and_is_asked_to_the_settle() {
    let (answer, took, record) = played(|| Err(Errno::PERM));
    assert_eq!(answer, None);
    kept_asking(&record.calls, took);
}

/// K3 of #1137: through the probe, a valid banner and a successful
/// status are no version while the kernel reports a member remaining.
#[test]
fn a_banner_and_a_success_are_no_version_while_a_member_remains() {
    let (answer, took, record) = played(|| Ok(()));
    assert_eq!(answer, None);
    kept_asking(&record.calls, took);
}

/// J1 of #1137d: once the leader is reaped, a member remaining pins the
/// group's id only at the moment the kernel answers, so the probe sends
/// no kill after the reap, only the question, and gives no version.
#[test]
fn a_probe_with_a_member_remaining_sends_no_kill_after_the_reap_and_is_no_version() {
    let (answer, _, record) = played(|| Ok(()));
    assert_eq!(answer, None);
    let calls = &record.calls;
    assert_eq!(record.reaped_at, Some(2), "{calls:?}");
    let after = &calls[2..];
    assert!(!after.is_empty(), "{calls:?}");
    assert!(
        after.iter().all(|call| *call == Call::AskGroup),
        "{calls:?}"
    );
}

/// J1 of #1137d: the kill stays. A member killable before the reap, here
/// one that joins the group as the probe's first call is made, is killed
/// by the group kill sent while the leader is unreaped; the group then
/// answers ESRCH, and the tool that answered gives its version. The one
/// test whose seam delivers the probe's kills, alone in its PID namespace
/// (#1137g), until the seam's first ECHILD. The group's question follows
/// the reap, when its member is reaped by the test and its leader by the
/// probe, so it is answered gone and never asked of the kernel.
#[cfg(target_os = "linux")]
#[test]
fn a_member_killable_before_the_reap_is_killed_and_the_group_gone_gives_the_version() {
    if rerun_alone(&[]) {
        return;
    }
    let joined: Option<(Child, Option<ExitStatus>)> = None;
    let state = (Record::default(), joined);
    let body = "echo 'tool 1.0'";
    let (answer, _, (record, joined)) =
        played_with(body, state, |(record, joined), call, leader| {
            let member = joined.is_none().then(|| {
                let mut member = Command::new("/bin/sleep");
                member.arg("600").stdin(Stdio::null()).stdout(Stdio::null());
                member.process_group(leader.as_raw_nonzero().get());
                member.spawn().unwrap()
            });
            let answer = record.delivered(call, leader, || Err(Errno::SRCH));
            if let Some(mut member) = member {
                let status = reaped(&mut member, Instant::now() + SETTLE);
                *joined = Some((member, status));
            }
            answer
        });
    let (mut member, status) = joined.expect("the probe made a call");
    if status.is_none() {
        member.kill().ok();
        reaped(&mut member, Instant::now() + PATIENCE);
    }
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(
        status.and_then(|status| status.signal()),
        Some(libc::SIGKILL)
    );
    assert_eq!(answer.as_deref(), Some(&b"tool 1.0\n"[..]));
    killed_then_asked_once(record);
}

/// The review of #1137e, its first finding: the seam's first ECHILD of
/// the leader is a permanent veto. A mutant that kills after it is
/// recorded and delivers nothing, even to a child that has since taken
/// the leader's pid, here one the test holds in a group of its own, which
/// then ends by the test's own SIGTERM. Alone in its PID namespace
/// (#1137g), so no sibling test's child can take the leader's pid first.
#[cfg(target_os = "linux")]
#[test]
fn a_kill_after_the_seams_first_echild_is_recorded_and_reaches_no_child_that_took_the_pid() {
    if rerun_alone(&[]) {
        return;
    }
    let mut leader = Command::new("/bin/sh").args(["-c", ":"]).spawn().unwrap();
    assert!(reaped(&mut leader, Instant::now() + PATIENCE).is_some());
    let mut record = Record::default();
    let ask = record.delivered(Call::AskGroup, Pid::from_child(&leader), || {
        Err(Errno::SRCH)
    });
    let mut taker = Command::new("/bin/sleep");
    let mut taker = taker.arg("600").process_group(0).spawn().unwrap();
    let took = Pid::from_child(&taker);
    let kills =
        [Call::KillGroup, Call::KillLeader].map(|call| record.delivered(call, took, || Ok(())));
    // The taker is this test's own child, held and unreaped.
    kill_process(took, Signal::TERM).unwrap();
    let status = reaped(&mut taker, Instant::now() + PATIENCE);
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(
        status.and_then(|status| status.signal()),
        Some(libc::SIGTERM)
    );
    assert_eq!((ask, kills), (Err(Errno::SRCH), [Ok(()), Ok(())]));
    let expected = [Call::AskGroup, Call::KillGroup, Call::KillLeader];
    assert_eq!(
        (record.calls, record.reaped_at),
        (expected.to_vec(), Some(0))
    );
}

/// A group the kernel answers gone after two refusals is the tool's
/// version, and the probe sent no kill once its leader was reaped.
#[test]
fn a_group_answered_gone_after_refusals_is_a_version_and_is_killed_no_more() {
    let mut refused = 2;
    let (answer, _, record) = played(move || {
        refused -= 1;
        Err([Errno::SRCH, Errno::PERM][usize::from(refused >= 0)])
    });
    assert_eq!(answer.as_deref(), Some(&b"tool 1.0\n"[..]));
    let ask = Call::AskGroup;
    let expected = [Call::KillGroup, Call::KillLeader, ask, ask, ask];
    assert_eq!(record.calls, expected);
}

/// The review of #1137d, its third finding: a leader that failed, as a
/// timed-out one killed at the bound does, is no version, and its group
/// is still asked after the reap, so its end is observed within the
/// settle. Only the question follows the reap.
#[test]
fn a_failed_leader_is_no_version_and_its_group_is_still_asked_after_the_reap() {
    let (answer, _, record) = played_by("echo 'tool 1.0'\nexit 3", || Err(Errno::SRCH));
    assert_eq!(answer, None);
    killed_then_asked_once(record);
}

/// J2 of #1137d, in a process of its own since the disposition is the
/// process's: a host that refuses SIGCHLD's default gets no probe. The
/// tool is never spawned, no call is made, and there is no version.
#[test]
fn a_refused_sigchld_default_spawns_nothing_and_is_no_version() {
    if rerun(&[]) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let ran = dir.path().join("ran");
    let body = format!(": > '{}'\necho 'tool 1.0'", ran.display());
    let tool = plant(dir.path(), "prompt", &body);
    let mut calls = Vec::new();
    let call = &mut |call, _| {
        calls.push(call);
        Ok(())
    };
    let refused = |_: &Disposition| Err(std::io::Error::from_raw_os_error(libc::EPERM));
    let answer = answered_with(Command::new(tool), VERSION_BOUND, call, refused);
    assert_eq!((answer, calls), (None, vec![]));
    assert!(!ran.exists(), "the probe spawned the tool");
}

/// How many times [`refused_restore`] was asked.
static ASKED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// SIGCHLD's disposition set on the first call, the default, and refused
/// on every later one, the restore.
fn refused_restore(to: &Disposition) -> std::io::Result<Disposition> {
    match ASKED.fetch_add(1, std::sync::atomic::Ordering::SeqCst) {
        0 => disposed(to),
        _ => Err(std::io::Error::from_raw_os_error(libc::EPERM)),
    }
}

/// J2 of #1137d, in a process of its own: a disposition the probe could
/// not put back is surfaced as one warning of doctor's report, once. The
/// tool's answer stands, and the report stays healthy.
#[test]
fn a_disposition_not_put_back_is_one_warning_of_the_report() {
    if rerun(&[]) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let tool = plant(dir.path(), "prompt", "echo 'tool 4.2.0'");
    let (answer, _, _) = inertly(Command::new(tool), refused_restore, || Err(Errno::SRCH));
    assert_eq!(answer.as_deref(), Some(&b"tool 4.2.0\n"[..]));
    let empty = || Report {
        healthy: true,
        lines: Vec::new(),
    };
    let report = restored(empty());
    assert_eq!(
        (report.healthy, report.lines),
        (true, vec![UNRESTORED.to_owned()])
    );
    assert_eq!(restored(empty()).lines, Vec::<String>::new());
}

/// The warning a disposition the probe could not put back is reported by.
const UNRESTORED: &str = "warn     SIGCHLD: a version probe could not put back its \
    disposition: Operation not permitted (os error 1)";

/// The review of #1137d, F2, in a process of its own: the warning reaches
/// the report `doctor()` itself answers, once, after a probe whose restore
/// was refused. The probe's seam is inert, and its group is first refused
/// and then answered gone, so a probe that killed after the reap is
/// caught by the call recorded, with no signal sent. `doctor()` itself
/// reaches the real probe, so the re-run is alone in its PID namespace
/// (#1137h, I4).
#[cfg(target_os = "linux")]
#[test]
fn a_disposition_not_put_back_is_one_warning_of_doctors_own_report() {
    if rerun_alone(&[]) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let tool = plant(dir.path(), "prompt", "echo 'tool 4.2.0'");
    let mut answers = [Errno::PERM].into_iter();
    let asked = move || Err(answers.next().unwrap_or(Errno::SRCH));
    let (_, _, record) = inertly(Command::new(tool), refused_restore, asked);
    let ask = Call::AskGroup;
    assert_eq!(record.calls, [Call::KillGroup, Call::KillLeader, ask, ask]);
    // The re-run's HOME and XDG_STATE_HOME are its own temporary ones.
    let report = doctored_in(dir.path());
    let warned: Vec<_> = (report.lines.iter())
        .filter(|line| line.contains("SIGCHLD"))
        .collect();
    assert_eq!(warned, [UNRESTORED]);
}

/// J3 of #1137d: the leader's wait and its reap accept nothing first
/// observed past their deadline. An exited, unreaped leader asked at its
/// deadline is still running to the probe, and is not reaped.
#[test]
fn a_leader_first_seen_exited_past_its_deadline_is_not_accepted() {
    let mut child = Command::new("/bin/sh")
        .args(["-c", "exit 0"])
        .spawn()
        .unwrap();
    assert_eq!(exited_by(&child, Instant::now() + PATIENCE), Leader::Exited);
    assert_eq!(exited_by(&child, Instant::now()), Leader::Running);
    assert_eq!(reaped(&mut child, Instant::now()), None);
    let leader = Pid::from_child(&child);
    assert_eq!(seen(leader), Some(Leader::Exited), "a late poll reaped it");
    let status = reaped(&mut child, Instant::now() + PATIENCE);
    assert!(status.is_some_and(|status| status.success()));
}

/// A pipe whose only writer has exited and been reaped: closed, empty.
fn closed_pipe() -> ChildStdout {
    let mut closer = Command::new("/bin/sh")
        .args(["-c", ":"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let pipe = closer.stdout.take().unwrap();
    assert!(reaped(&mut closer, Instant::now() + PATIENCE).is_some());
    pipe
}

/// A pipe that counts how often it was polled or read.
struct Counted {
    pipe: ChildStdout,
    looked: std::cell::Cell<usize>,
}

impl Read for Counted {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.looked.set(self.looked.get() + 1);
        self.pipe.read(buf)
    }
}

impl AsFd for Counted {
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        self.looked.set(self.looked.get() + 1);
        self.pipe.as_fd()
    }
}

/// J3 of #1137d, and the review of 1137d's second round, its first
/// finding: a pipe that has closed is neither polled nor read once its
/// deadline has passed, so it is expired. It carries nothing, so the
/// close is the first thing a read would observe.
#[test]
fn a_pipe_first_seen_closed_past_its_deadline_is_expired() {
    let pipe = closed_pipe();
    let looked = std::cell::Cell::new(0);
    let mut counted = Counted { pipe, looked };
    assert_eq!(kept(&mut counted, Instant::now()), Answer::Expired);
    assert_eq!(counted.looked.get(), 0);
}

/// The review of 1137d's second round, its second finding: whatever a
/// poll begun in time first answers past its deadline is not accepted,
/// and no poll begins once the deadline has passed. Every wait of the
/// probe, the leader's, its reap, the pipe's and the group's, polls so.
#[test]
fn an_observation_a_timely_poll_first_makes_past_its_deadline_is_not_accepted() {
    let until = Instant::now() + Duration::from_millis(100);
    let late = before(until, |_| {
        std::thread::sleep(until.saturating_duration_since(Instant::now()));
        Some(())
    });
    let mut polled = 0;
    let past = before(Instant::now(), |_| {
        polled += 1;
        Some(())
    });
    assert_eq!((late, past, polled), (None, None, 0));
}

/// The review of 1137d's second round, its second finding: an ESRCH the
/// group first answers past the settle, to a question asked within it, is
/// not gone.
#[test]
fn an_esrch_a_timely_question_first_answers_past_the_settle_is_not_gone() {
    let until = Instant::now() + Duration::from_millis(100);
    let mut asked = 0;
    let call = &mut |_, _| {
        asked += 1;
        std::thread::sleep(until.saturating_duration_since(Instant::now()));
        Err(Errno::SRCH)
    };
    let gone = group_gone(rustix::process::getpid(), until, call);
    assert_eq!((gone, asked), (false, 1));
}

/// A closed pipe whose read answers only once `until` has passed: a close
/// a poll begun in time first observes past the deadline. It counts its
/// reads.
struct Late {
    pipe: ChildStdout,
    until: Instant,
    read: usize,
}

impl Read for Late {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.read += 1;
        std::thread::sleep(self.until.saturating_duration_since(Instant::now()));
        self.pipe.read(buf)
    }
}

impl AsFd for Late {
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        self.pipe.as_fd()
    }
}

/// The review of #1137d, its second finding: the pipe's poll is begun in
/// time, but the close it reports is read past the deadline. It is
/// expired, not a version. The deadline starts on the reading thread, and
/// the one read is pinned, so the expiry is the late close's and not the
/// check before the poll's.
#[test]
fn a_close_a_timely_poll_first_sees_past_its_deadline_is_expired() {
    let pipe = closed_pipe();
    let ((answer, read), _) = timed(move || {
        let deadline = Instant::now() + Duration::from_millis(100);
        let until = deadline + Duration::from_millis(100);
        let mut late = Late {
            pipe,
            until,
            read: 0,
        };
        (kept(&mut late, deadline), late.read)
    });
    assert_eq!((answer, read), (Answer::Expired, 1));
}

/// J3 of #1137d: an ESRCH the group would answer is not asked for once the
/// settle has passed, so the group is not gone.
#[test]
fn an_esrch_first_asked_past_the_settle_is_not_gone() {
    let mut asked = 0;
    let call = &mut |_, _| {
        asked += 1;
        Err(Errno::SRCH)
    };
    let gone = group_gone(rustix::process::getpid(), Instant::now(), call);
    assert_eq!((gone, asked), (false, 0));
}

/// K2 of #1137: a leader the probe cannot wait for (ECHILD) left no
/// status, and its id may be another process's: no version, at once, and
/// neither kill nor any question is sent to its id.
#[test]
fn a_leader_the_probe_cannot_wait_for_is_sent_neither_kill_and_is_no_version() {
    let mut child = Command::new("/bin/sh")
        .args(["-c", "echo 'tool 1.0'"])
        .stdout(Stdio::piped())
        .process_group(0)
        .spawn()
        .unwrap();
    assert!(reaped(&mut child, Instant::now() + PATIENCE).is_some());
    let ((answer, calls), took) = timed(move || {
        let mut calls = Vec::new();
        let deadline = Instant::now() + PATIENCE;
        let answer = settled(child, deadline, &mut |call, _| {
            calls.push(call);
            Ok(())
        });
        (answer, calls)
    });
    assert_eq!((answer, calls), (None, vec![]));
    assert!(took < VERSION_BOUND, "the probe waited {took:?}");
}

/// The review of 1137d's second round, its third finding: a leader whose
/// ECHILD the wait never saw in time, here because its deadline had
/// passed, is still looked at once more just before the kills, and is
/// sent neither.
#[test]
fn a_leader_first_seen_unwaitable_past_its_deadline_is_sent_neither_kill() {
    let mut child = Command::new("/bin/sh")
        .args(["-c", ":"])
        .process_group(0)
        .spawn()
        .unwrap();
    assert!(reaped(&mut child, Instant::now() + PATIENCE).is_some());
    let mut calls = Vec::new();
    let answer = settled(child, Instant::now(), &mut |call, _| {
        calls.push(call);
        Ok(())
    });
    assert_eq!((answer, calls), (None, vec![]));
}

/// SIGCHLD's disposition in this process.
#[cfg(target_os = "linux")]
fn sigchld() -> libc::sighandler_t {
    let mut current = std::mem::MaybeUninit::<libc::sigaction>::zeroed();
    // SAFETY: a null new action only reads the current one, into memory
    // sized for it and zeroed, so it is initialised whatever the call does.
    unsafe {
        libc::sigaction(libc::SIGCHLD, std::ptr::null(), current.as_mut_ptr());
        current.assume_init().sa_sigaction
    }
}

/// K2 of #1137, in a process of its own since it sets a process-wide
/// disposition: with SIGCHLD ignored before the probe, a tool that
/// answers at once is still its version, and the ignored disposition is
/// back afterwards. A probe inside another leaves the outer one's default
/// in place, and the last to end puts back the one the first replaced.
#[cfg(target_os = "linux")]
#[test]
fn a_tool_that_answers_at_once_under_sigchld_ignored_is_its_version_and_the_disposition_returns() {
    let dir = tempfile::tempdir().unwrap();
    // Under SIGCHLD ignored the re-run could not reap a planting writer,
    // so only the parent plants; the re-run's `PATH` already holds it.
    let tool = match rerunning() {
        true => PathBuf::new(),
        false => plant(dir.path(), "prompt", "echo 'tool 4.2.0'"),
    };
    let ignoring = [
        "/usr/bin/perl",
        "-e",
        "$SIG{CHLD} = 'IGNORE'; exec @ARGV or die",
    ];
    if rerun_alone_through(&ignoring, &[("prompt", &tool)]) {
        return;
    }
    assert_eq!(
        sigchld(),
        libc::SIG_IGN,
        "the re-run starts ignoring SIGCHLD"
    );
    let (version, took) = timed(|| tool_version("prompt"));
    assert_eq!(version.as_deref(), Some("tool 4.2.0"));
    assert!(took < VERSION_BOUND, "the probe waited {took:?}");
    assert_eq!(sigchld(), libc::SIG_IGN, "the probe put SIGCHLD back");
    let outer = ChildrenWaited::held(disposed);
    assert_eq!(sigchld(), libc::SIG_DFL);
    let (version, _) = timed(|| tool_version("prompt"));
    assert_eq!(version.as_deref(), Some("tool 4.2.0"));
    assert_eq!(sigchld(), libc::SIG_DFL, "the inner probe kept the default");
    drop(outer);
    assert_eq!(sigchld(), libc::SIG_IGN, "the last probe put SIGCHLD back");
}

/// K4 of #1137: a read answers how its pipe ended. One still held open at
/// the deadline is expired, one every holder closed is its bytes, and one
/// whose read fails, a directory's, is expired too. The pipe's holder is
/// let go through its FIFO ([`held`]), never killed (#1137h, I4).
#[test]
fn a_pipe_held_at_the_deadline_is_expired_and_a_closed_one_is_its_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let (fifo, let_go) = held(dir.path());
    let mut holder = Command::new("/usr/bin/perl")
        .args(["-e", HOLDS])
        .arg(dir.path().join("holder"))
        .arg(fifo)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let held = holder.stdout.take().unwrap();
    let briefly = Duration::from_millis(100);
    let (answer, _) = timed(move || kept(held, Instant::now() + briefly));
    drop(let_go);
    assert!(reaped(&mut holder, Instant::now() + PATIENCE).is_some());
    assert_eq!(answer, Answer::Expired);

    let mut closer = Command::new("/bin/sh")
        .args(["-c", "echo 'tool 1.0'"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let closed = closer.stdout.take().unwrap();
    let (answer, _) = timed(move || kept(closed, Instant::now() + PATIENCE));
    assert!(reaped(&mut closer, Instant::now() + PATIENCE).is_some());
    assert_eq!(answer, Answer::Closed(b"tool 1.0\n".to_vec()));

    let dir = tempfile::tempdir().unwrap();
    let unreadable = std::fs::File::open(dir.path()).unwrap();
    let (answer, _) = timed(move || kept(unreadable, Instant::now() + PATIENCE));
    assert_eq!(answer, Answer::Expired);
}

/// A leader still running at the settle is not waited for past it. Its
/// kill names a pid the worker may already have reaped, so the test is
/// alone in its PID namespace (#1137g), where that pid names nothing of
/// another's.
#[cfg(target_os = "linux")]
#[test]
fn a_leader_not_reaped_by_the_settle_is_no_status() {
    if rerun_alone(&[]) {
        return;
    }
    let child = Command::new("/bin/sleep").arg("600").spawn().unwrap();
    let pid = Pid::from_child(&child);
    let (sender, received) = channel();
    std::thread::spawn(move || {
        let mut child = child;
        let status = reaped(&mut child, Instant::now());
        sender.send((status, child)).ok();
    });
    let answer = received.recv_timeout(PATIENCE);
    kill_process(pid, Signal::KILL).unwrap();
    let (status, mut child) = answer.expect("the reap answered within PATIENCE");
    assert_eq!(status, None);
    assert!(reaped(&mut child, Instant::now() + PATIENCE).is_some());
}

/// The probe's real group kill reaches the grandchild the tool forked,
/// inside the test's own PID namespace (#1137g), and nothing outside it.
#[cfg(target_os = "linux")]
#[test]
fn a_child_the_tool_forked_dies_with_its_group() {
    if rerun_alone(&[]) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let forked = dir.path().join("forked");
    let (fifo, _holder) = held(dir.path());
    let body = format!(
        "/usr/bin/perl -e '{HOLDS}' '{}' '{}' &\nwait",
        forked.display(),
        fifo.display()
    );
    let (version, _) = probed(&plant(dir.path(), "forking", &body));
    assert_eq!(version, None);
    assert!(
        stopped(pid_in(&forked)),
        "the forked child outlived the probe"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn a_tool_that_answers_at_once_reports_its_version_line() {
    if rerun_alone(&[]) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let body = "echo 'tool 4.2.0'\necho 'second line'";
    let (version, took) = probed(&plant(dir.path(), "prompt", body));
    assert_eq!(version.as_deref(), Some("tool 4.2.0"));
    assert!(took < VERSION_BOUND, "the probe waited {took:?}");
}

/// R2 of #1137: a doctor test that probes reaches no tool of the host's.
/// Under the re-run its `PATH` holds the planted `node` alone, which is
/// what answers, and no other tool is found.
#[cfg(target_os = "linux")]
#[test]
fn a_hermetic_doctor_test_probes_only_the_tools_it_planted() {
    let dir = tempfile::tempdir().unwrap();
    // Only the parent plants: the re-run's `PATH` already holds it, and
    // nothing runs in the re-run before its isolation is proved.
    let node = match rerunning() {
        true => PathBuf::new(),
        false => plant(dir.path(), "node", "echo v0.0.0-planted"),
    };
    if rerun_alone(&[("node", &node)]) {
        return;
    }
    let path = std::env::var_os("PATH").unwrap();
    let held: Vec<_> = (std::fs::read_dir(&path).unwrap())
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(held, ["node"]);
    assert_eq!(tool_version("node").as_deref(), Some("v0.0.0-planted"));
    assert_eq!(tool_version("git"), None);
}

#[cfg(target_os = "linux")]
#[test]
fn the_posix_shell_probe_executes_a_portable_shell_operation() {
    if rerun_alone(&[("sh", Path::new("/bin/sh"))]) {
        return;
    }
    assert_eq!(tool_version("sh").as_deref(), Some("POSIX shell"));
}
