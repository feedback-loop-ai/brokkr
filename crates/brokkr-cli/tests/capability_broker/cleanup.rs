//! A session's complete cleanup (decision 0065 slice two U6f2; MB5, SD4),
//! observed on real Linux processes. The attempt stands in a process group
//! of its own, as the engine starts one (#403): the broker, bubblewrap, the
//! server its bootstrap became and the server's own child are all of that
//! group and session, and no argument of theirs asks for a new session.
//! The server's HOME and TMPDIR are private tmpfs in a mount namespace of
//! its own. However the session ends, normally, cancelled during a call,
//! by the server's exit mid-call or by the broker's outright kill, every
//! one of those processes is gone, nothing holds that mount namespace or
//! the server's pipes, and the plan and its ledger remain. A killed
//! broker cannot remove the host scratch it held: that is left for the
//! next start, which reaps it. A process-group observation is no cgroup
//! guarantee: a descendant that leaves the group is #403's residual.

use std::collections::BTreeMap;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use brokkr_protocol::broker::ledger::{Disposition, Failure, Invalid, Outcome};

use super::ledger::{sealed_ledger, Proved};
use super::observer::{awaited, processes, within};
use super::protocol::{after, greeting, harnessing, unretained, Ran, Talk};
use super::{ended, Sealed, TALK_MAX};

/// A process's state, parent, process group and session, from
/// `/proc/<pid>/stat`; none once it is gone.
fn stat(pid: i32) -> Option<(char, i32, i32, i32)> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let mut fields = stat.rsplit_once(')')?.1.split_whitespace();
    let state = fields.next()?.chars().next()?;
    let mut number = || fields.next()?.parse().ok();
    Some((state, number()?, number()?, number()?))
}

/// Every live descendant of `root`, found by its parent links.
fn descendants(root: i32) -> Vec<i32> {
    let parents: BTreeMap<i32, i32> = processes()
        .filter_map(|pid| {
            stat(pid)
                .filter(|(state, ..)| *state != 'Z')
                .map(|(_, of, ..)| (pid, of))
        })
        .collect();
    let mut found = vec![root];
    let mut at = 0;
    while at < found.len() {
        let parent = found[at];
        let children = parents.iter().filter(|(_, of)| **of == parent);
        found.extend(children.map(|(pid, _)| *pid));
        at += 1;
    }
    found.split_off(1)
}

/// What `pid` was started with, each argument apart.
fn arguments(pid: i32) -> Vec<String> {
    let line = std::fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
    let args = line.split(|byte| *byte == 0).filter(|arg| !arg.is_empty());
    args.map(|arg| String::from_utf8_lossy(arg).into_owned())
        .collect()
}

/// The link `/proc/<pid>/<name>` reads as, such as a namespace's or an
/// open pipe's.
fn link(pid: i32, name: &str) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/{pid}/{name}")).ok()
}

/// Whether any process of the host is in the mount namespace `namespace`
/// or holds `pipes` open.
fn holding(namespace: &Path, pipes: &[PathBuf]) -> bool {
    processes().any(|pid| {
        let inside = link(pid, "ns/mnt").is_some_and(|held| held == namespace);
        let open = std::fs::read_dir(format!("/proc/{pid}/fd"))
            .into_iter()
            .flatten();
        let opened = open
            .flatten()
            .filter_map(|fd| std::fs::read_link(fd.path()).ok())
            .any(|held| pipes.contains(&held));
        inside | opened
    })
}

/// The mount points of `pid`'s namespace that are tmpfs, among `wanted`.
fn tmpfs(pid: i32, wanted: &[&str]) -> Vec<String> {
    let mounts = std::fs::read_to_string(format!("/proc/{pid}/mountinfo")).unwrap_or_default();
    let mut found: Vec<String> = mounts
        .lines()
        .filter_map(|line| {
            let (mount, rest) = line.split_once(" - ")?;
            let point = mount.split_whitespace().nth(4)?;
            let tmpfs = rest.split_whitespace().next()? == "tmpfs";
            (tmpfs & wanted.contains(&point)).then(|| point.to_string())
        })
        .collect();
    found.sort();
    found
}

/// What a live session's attempt was seen to hold: every process below its
/// harness, the distinct process groups and sessions they are in, those
/// whose arguments ask for a new session, the server's private tmpfs, its
/// mount namespace and its standard input and output.
struct Held {
    members: Vec<i32>,
    owners: Vec<(i32, i32)>,
    detached: Vec<i32>,
    private: Vec<String>,
    namespace: PathBuf,
    pipes: Vec<PathBuf>,
}

impl Held {
    /// The attempt below `harness` once its server has a child of its own
    /// running `child`, looked at until the suite's wait for a broker
    /// ([`TALK_MAX`]) has passed since the call, made as the broker starts
    /// where no answer is awaited first: past the startup's deadline that
    /// bounds the box's start, and so the server's, however loaded the host.
    fn seen(harness: i32, child: &str) -> Held {
        let until = Instant::now() + TALK_MAX;
        let named = |pid: &i32| arguments(*pid).first().is_some_and(|arg| arg == child);
        let running = loop {
            let members = descendants(harness);
            if let Some(running) = members.iter().copied().find(named) {
                break running;
            }
            let seen: Vec<_> = processes()
                .filter(|pid| stat(*pid).is_some_and(|s| s.1 == harness || s.2 == harness))
                .map(|pid| (pid, stat(pid), arguments(pid)))
                .collect();
            let seen = (stat(harness), seen);
            assert!(
                Instant::now() < until,
                "no {child} below {harness}: {seen:?}"
            );
            std::thread::sleep(Duration::from_millis(20));
        };
        let server = stat(running).unwrap().1;
        let members = descendants(harness);
        let mut owners: Vec<(i32, i32)> = members
            .iter()
            .filter_map(|pid| stat(*pid).map(|(_, _, group, session)| (group, session)))
            .collect();
        owners.sort();
        owners.dedup();
        let new_session = |pid: &&i32| arguments(**pid).iter().any(|arg| arg == "--new-session");
        let detached = members.iter().filter(new_session).copied().collect();
        let pipes = ["fd/0", "fd/1"].map(|fd| link(server, fd).unwrap());
        Held {
            private: tmpfs(server, &["/runtime/home", "/tmp"]),
            namespace: link(server, "ns/mnt").unwrap(),
            pipes: pipes.to_vec(),
            members,
            owners,
            detached,
        }
    }

    /// Whether, within a while, every process seen is gone, and nothing
    /// holds the server's mount namespace, and so its tmpfs, or its pipes.
    fn settled(&self) -> bool {
        let gone = |pid: &i32| gone(*pid);
        within(|| self.members.iter().all(gone) && !holding(&self.namespace, &self.pipes))
    }
}

/// One session of a fixture's server, started as an attempt starts its
/// harness, in a process group of its own, with a host scratch apart from
/// the fixture as its TMPDIR, as a host's temporary directory is: the
/// fixture, that scratch, the harness, its talk, and what the attempt was
/// seen to hold once the server's child ran.
struct Live {
    sealed: Sealed,
    scratch: PathBuf,
    harness: i32,
    talk: Talk,
    held: Held,
}

/// What a session left: whether everything it held settled ([`Held::settled`]),
/// what its ledger proves, whether the plan, its inventory and its ledger
/// remain, and what the host scratch holds.
type Left = (bool, Proved, bool, Vec<String>);

impl Live {
    /// The session whose server runs `said` in the box's own shell, its
    /// plan retaining nothing and its TMPDIR `dir`, once the harness has
    /// read `answers` frames and the server's child `sleep` runs, checked
    /// owned ([`Live::owned`]); none, declared skipped, where no box stands.
    fn started(dir: &tempfile::TempDir, said: &str, answers: usize) -> Option<Live> {
        let (sealed, digest) = unretained(said)?;
        let scratch = dir.path().canonicalize().unwrap();
        let mut command = harnessing(&sealed, &digest);
        command.process_group(0).env("TMPDIR", &scratch);
        command.stdin(Stdio::piped()).stdout(Stdio::piped());
        let child = command.stderr(Stdio::piped()).spawn().unwrap();
        let talk = Talk::over(child, (&greeting(), answers), TALK_MAX);
        let harness = talk.id() as i32;
        let held = Held::seen(harness, "sleep");
        let live = Live {
            sealed,
            scratch,
            harness,
            talk,
            held,
        };
        live.owned();
        Some(live)
    }

    /// Check that the attempt is owned as MB5 says: every process below
    /// the harness in one process group, the harness's, and its session;
    /// none asking for a new session; the server's HOME and TMPDIR private
    /// tmpfs.
    fn owned(&self) {
        let (_, _, group, session) = stat(self.harness).unwrap();
        let held = &self.held;
        let seen = (group, &held.owners, &held.detached, &held.private);
        let private = ["/runtime/home", "/tmp"].map(String::from).to_vec();
        assert_eq!(
            seen,
            (self.harness, &vec![(group, session)], &vec![], &private)
        );
    }

    /// The broker: the one process seen whose parent is the harness.
    fn broker(&self) -> i32 {
        let of = |pid: &&i32| stat(**pid).is_some_and(|(_, parent, ..)| parent == self.harness);
        let brokers: Vec<i32> = self.held.members.iter().filter(of).copied().collect();
        assert_eq!(brokers.len(), 1, "{brokers:?}");
        brokers[0]
    }

    /// The broker's answer once the harness ends its input ([`Talk::ended`]),
    /// and what the session left ([`Left`]).
    fn ended(self) -> ((Option<i32>, usize, String), Left) {
        let ran: Ran = self.talk.ended();
        let sealed = &self.sealed;
        let ledger = super::ledger::of(&sealed.locator);
        let inventory = sealed.attempt.join("inventory.json");
        let kept = [&sealed.locator, &inventory, &ledger];
        let left = (
            self.held.settled(),
            sealed_ledger(sealed),
            kept.iter().all(|path| path.is_file()),
            left(&self.scratch),
        );
        ((ran.0, ran.1.len(), ran.3), left)
    }
}

/// What `scratch` holds.
fn left(scratch: &Path) -> Vec<String> {
    let entries = std::fs::read_dir(scratch).unwrap().flatten();
    let mut names: Vec<String> = entries
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// What a session that settled with its evidence left, its ledger proving
/// the one call ended `outcome` and the session `ending`, its scratch
/// holding `scratch`.
fn settled(outcome: Option<Outcome>, ending: Option<Disposition>, scratch: Vec<String>) -> Left {
    let call = vec![("resolve".to_string(), outcome)];
    let invalid = ending.is_none().then_some(Invalid::Lifecycle);
    (true, (call, ending, invalid), true, scratch)
}

/// Whether `pid` is gone, or ended and not yet reaped.
fn gone(pid: i32) -> bool {
    stat(pid).is_none_or(|(state, ..)| state == 'Z')
}

/// Kill `pid` outright.
fn kill(pid: i32) {
    let pid = rustix::process::Pid::from_raw(pid).unwrap();
    rustix::process::kill_process(pid, rustix::process::Signal::KILL).unwrap();
}

#[test]
fn a_normal_ending_settles_every_owned_process_and_keeps_the_evidence() {
    // The server, called, starts a child of its own, answers, and waits for
    // its input's end. Seen while the harness still holds its input open:
    // every process below the harness is of its group and session, none
    // asks for a new session, and the server's HOME and TMPDIR are tmpfs.
    // The harness ends its input: the broker exits 0, the session closes
    // Clean, and every process is gone, the child too, its namespace, so
    // its tmpfs, and its pipes with it; the plan, inventory and ledger
    // remain, and the host scratch is left empty.
    let dir = tempfile::tempdir().unwrap();
    let Some(live) = Live::started(&dir, &after("sleep 30 &"), 3) else {
        return;
    };
    let clean = settled(Some(Outcome::Succeeded), Some(Disposition::Clean), vec![]);
    assert_eq!(live.ended(), ((Some(0), 3, String::new()), clean));
}

#[test]
fn an_attempt_cancelled_during_initialization_or_a_call_settles_the_server_and_its_child() {
    // The server waits on a child of its own that never ends: before it
    // answers initialize, or once called. The attempt is cancelled, the
    // harness that started the broker killed outright: the broker ends
    // within seconds with the timeout's cause, the session closed with it,
    // with no call or with the one it accepted failed, and every process
    // below the harness, the broker and the server's child among them, is
    // gone with its namespace and pipes; the evidence remains, and the
    // scratch is left empty (MB5). The harness's input, which this test and
    // not the harness holds, ends only once the broker has: ended first, it
    // could reach the broker before the harness's death does, and end the
    // session with the call still begun.
    let timeout = Some(Disposition::Failed(Failure::Timeout));
    let initializing = (true, (vec![], timeout.clone(), None), true, vec![]);
    let called = settled(Some(Outcome::Failed), timeout, vec![]);
    let cases = [
        ("initialization", "sleep 600".to_string(), 0, initializing),
        ("call", after("sleep 600"), 2, called),
    ];
    for (case, said, answers, left) in cases {
        let dir = tempfile::tempdir().unwrap();
        let Some(live) = Live::started(&dir, &said, answers) else {
            return;
        };
        let broker = live.broker();
        kill(live.harness);
        let cancelled = Instant::now();
        assert!(within(|| gone(broker)), "{case}: the broker did not end");
        let ending = live.ended();
        let taken = cancelled.elapsed();
        assert!(taken < Duration::from_secs(5), "{case}: {taken:?}");
        let cause = ended(Failure::Timeout).1;
        assert_eq!((case, ending), (case, ((None, answers, cause), left)));
    }
}

#[test]
fn a_server_that_exits_mid_call_ends_the_session_with_its_box_settled() {
    // The server, called, waits on a child of its own a moment, then exits
    // without answering: the session ends with the protocol's cause, the
    // call failed, and nothing of the attempt below the broker is left. The
    // harness holds its input open until the broker has ended: an input
    // ended first ends the session with the call still begun.
    let dir = tempfile::tempdir().unwrap();
    let Some(live) = Live::started(&dir, &after("sleep 2\nexit 3"), 2) else {
        return;
    };
    awaited(live.broker());
    let (code, cause) = ended(Failure::Protocol);
    let protocol = Some(Disposition::Failed(Failure::Protocol));
    let failed = settled(Some(Outcome::Failed), protocol, vec![]);
    assert_eq!(live.ended(), ((code, 2, cause), failed));
}

#[test]
fn a_broker_killed_mid_call_takes_its_box_with_it_and_the_next_start_reaps_its_scratch() {
    // The broker itself is killed outright while its server waits on its
    // child: bubblewrap dies with its parent, and the box with it, every
    // process, namespace and pipe. The broker says nothing: its standard
    // error holds only the harness shell's one notice of its child's kill.
    // The ledger keeps the call it accepted, never closed, as its prefix
    // proves; the plan and inventory remain. The broker could not remove
    // the server identity's tree it held in the host scratch: the next
    // start reaps it (#415).
    let dir = tempfile::tempdir().unwrap();
    let Some(live) = Live::started(&dir, &after("sleep 600"), 2) else {
        return;
    };
    kill(live.broker());
    let scratch = live.scratch.clone();
    let ((code, answers, stderr), mut left) = live.ended();
    left.3 = left.3.iter().map(|name| shaped(name)).collect();
    let unclosed = settled(None, None, vec![SERVED.to_string()]);
    let said = apart(&stderr);
    assert_eq!(
        ((code, answers, said), left),
        ((Some(137), 2, (vec![], 1)), unclosed)
    );
    assert_eq!(left_for_the_next_start(&scratch), (identity(1), true));
}

/// What a harness's standard error holds once the shell's broker was
/// killed: the broker's own lines, and how many are the shell's notice of
/// that kill, whose text is the shell's (`Killed` from dash; bash names
/// its line, the pid and the command beside it) and which dash writes
/// where the broker's standard error goes.
fn apart(stderr: &str) -> (Vec<&str>, usize) {
    let (notices, said): (Vec<&str>, Vec<&str>) =
        stderr.lines().partition(|line| line.contains("Killed"));
    (said, notices.len())
}

/// `trees` server identity trees, as [`left_for_the_next_start`] shapes
/// them: the observer's generated `etc` beside the lock its holder held.
pub(super) fn identity(trees: usize) -> Vec<(String, Vec<String>)> {
    let tree = (
        SERVED.to_string(),
        [".owner.lock", "etc"].map(String::from).to_vec(),
    );
    vec![tree; trees]
}

/// What a server identity's tree is named, its owner's pid and its uuid
/// apart: `brokkr-hands-server-<pid>-<uuid>`.
const SERVED: &str = "brokkr-hands-server-<pid>-<uuid>";

/// `name` shaped: [`SERVED`] where it is a server identity tree's name, as
/// the reaper reads one; else as it is.
fn shaped(name: &str) -> String {
    let parts = name.strip_prefix("brokkr-hands-server-").and_then(|rest| {
        let (pid, uuid) = rest.split_once('-')?;
        let hyphens = uuid.char_indices().filter(|(_, c)| *c == '-');
        let at: Vec<usize> = hyphens.map(|(at, _)| at).collect();
        let hex = uuid.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
        let digits = !pid.is_empty() && pid.bytes().all(|b| b.is_ascii_digit());
        (digits & hex & (uuid.len() == 36) & (at == [8, 13, 18, 23])).then_some(())
    });
    match parts {
        Some(()) => SERVED.to_string(),
        None => name.to_string(),
    }
}

/// What `scratch` holds once its owners are dead, each tree's name
/// ([`shaped`]) and its entries; and whether the next start (`brokkr run`,
/// which reaps before it drives, #415) said it reaped each and left
/// `scratch` empty.
pub(super) fn left_for_the_next_start(scratch: &Path) -> (Vec<(String, Vec<String>)>, bool) {
    let names = left(scratch);
    let trees = names
        .iter()
        .map(|name| (shaped(name), left(&scratch.join(name))));
    let trees: Vec<(String, Vec<String>)> = trees.collect();
    let run = tempfile::tempdir().unwrap();
    let dir = run.path();
    let bundle = crate::hands::reap_bundle(dir);
    let mut start = crate::engine::brokkr_in(dir);
    start
        .env("TMPDIR", scratch)
        .args(["run", "--bundle"])
        .arg(&bundle);
    start
        .args(["--feature", "reap", "--db"])
        .arg(dir.join("j.db"));
    let out = start.arg("--repo").arg(dir).output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    let said: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with("hands: "))
        .collect();
    let reaping = |name: &String| {
        let tree = scratch.join(name);
        format!(
            "hands: reaped {}: its owner is dead and holds no lock",
            tree.display()
        )
    };
    let expected: Vec<String> = names.iter().map(reaping).collect();
    let reaped = (said == expected) & left(scratch).is_empty();
    (trees, reaped)
}
