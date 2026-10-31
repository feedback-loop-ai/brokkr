//! The box admission launches on its waiting bootstrap (decision 0065
//! slice two U6c6b, U6c8; MB3, MB5, SD3), driven through the real binary: a
//! real owned box reports ready, its store is read once, and the fixture's
//! retained plan refuses, with nothing of the box left; a box held at its
//! entry by a
//! tracer, so that its bootstrap never reports, refuses at the startup's
//! absolute deadline, or at once when the attempt is cancelled, with
//! nothing of the box left running and nothing looked up; and a host with
//! no launcher refuses the box as unavailable.

use std::mem::MaybeUninit;
use std::os::fd::OwnedFd;
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};

use rustix::fs::inotify::{self, CreateFlags, WatchFlags};
use serde_json::json;

use super::ledger::{failed_with, sealed_ledger, Proved};
use super::observer::{awaited, cancelled, exited, gone, processes, started_to, status, traced};
use super::observer::{tracer, within, Starter, NO_TRACER};
use super::{admitted, refused, skip, unservable, Refusal, Sealed};

/// Every read of a file: of a fixture's store, its one binding's value now
/// in it, where a lookup reads the store's bytes, which no path handle,
/// identity check or mount does; or of a server, which its interpreter
/// reads once it has started. The FIFO store of the admission tests is
/// refused at its identity, before any box, so a store the broker admits
/// is watched here.
pub(crate) struct Reads {
    path: PathBuf,
    watch: OwnedFd,
}

impl Reads {
    pub(super) fn of(sealed: &Sealed) -> Reads {
        let store = sealed.path("store/secrets.env");
        std::fs::write(&store, "DOCS_TOKEN=value\n").unwrap();
        Reads::on(store)
    }

    /// Every read of the file at `path` from now on.
    pub(crate) fn on(path: PathBuf) -> Reads {
        let watch = inotify::init(CreateFlags::NONBLOCK | CreateFlags::CLOEXEC).unwrap();
        inotify::add_watch(&watch, &path, WatchFlags::ACCESS).unwrap();
        Reads { path, watch }
    }

    /// The reads seen since last asked; identical reads unseen between
    /// them are seen as one.
    pub(super) fn seen(&self) -> usize {
        let mut buffer = [MaybeUninit::uninit(); 4096];
        let mut events = inotify::Reader::new(&self.watch, &mut buffer);
        std::iter::from_fn(|| events.next().ok().map(drop)).count()
    }

    /// Whether any lookup read the file; the watch shown to see one read
    /// here, so that none seen is none made.
    pub(crate) fn lookups(self) -> usize {
        let made = self.seen();
        std::fs::read(&self.path).unwrap();
        assert_eq!(self.seen(), 1, "the watch saw no read");
        made
    }
}

/// Every process of a box this fixture's package is mounted in, started to
/// run the bootstrap: bubblewrap's within the box, and the bootstrap.
pub(super) fn bootstraps(sealed: &Sealed) -> Vec<i32> {
    let bootstrap = |pid: &i32| {
        let line = std::fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
        line.split(|byte| *byte == 0).any(|arg| arg == b"bootstrap")
    };
    boxed(sealed).into_iter().filter(bootstrap).collect()
}

/// Every process of a box this fixture's package is mounted in, whatever
/// it runs: the bootstrap, or the server it became.
pub(super) fn boxed(sealed: &Sealed) -> Vec<i32> {
    let package = format!(" {} ", sealed.path("opt/docs").display());
    let mounted = |pid: &i32| {
        let mounts = std::fs::read_to_string(format!("/proc/{pid}/mountinfo"));
        mounts.unwrap_or_default().contains(&package)
    };
    processes().filter(mounted).collect()
}

/// A sealed fixture and its plan's digest; none, declared skipped, where
/// no box stands.
fn sealed() -> Option<(Sealed, String)> {
    if let Some(reason) = unservable() {
        skip(&reason);
        return None;
    }
    let sealed = Sealed::new();
    let digest = sealed.seal_bytes(sealed.plan().to_string().as_bytes());
    Some((sealed, digest))
}

#[test]
fn a_real_box_reports_ready_and_its_retained_plan_refuses_with_nothing_of_it_left() {
    let Some((sealed, digest)) = sealed() else {
        return;
    };
    // Admitted only where the bootstrap the box ran reported ready, its
    // digests, child and namespaces checked: anything else is the
    // establishment cause. Only then is the store read, once, and the
    // server started in the box, which is ended before any MCP message.
    let reads = Reads::of(&sealed);
    assert_eq!(sealed.serve(&digest), admitted());
    assert_eq!(reads.lookups(), 1);
    assert!(within(|| boxed(&sealed).is_empty()));
    assert!(!sealed.path("started").exists());
    // The generated identity's tree the observer handed over goes with the
    // box, admitted or refused at the comparison, as drifted sources are.
    let scratch = tempfile::tempdir().unwrap();
    let scratch = std::fs::canonicalize(scratch.path()).unwrap();
    let served = |digest: &str| {
        let answer = sealed.serve_in(&sealed.locator, digest, |command| {
            command.env("HOME", &sealed.home).env("TMPDIR", &scratch);
        });
        (answer, std::fs::read_dir(&scratch).unwrap().count())
    };
    assert_eq!(served(&digest), (admitted(), 0));
    let mut drifted = sealed.plan();
    drifted["box"]["sources"]["digest"] = json!("3".repeat(64));
    let drifted = sealed.seal_bytes(drifted.to_string().as_bytes());
    assert_eq!(served(&drifted), (refused(Refusal::Identity), 0));
}

#[test]
fn unavailable_support_refuses_before_any_lookup_or_start() {
    let Some((sealed, digest)) = sealed() else {
        return;
    };
    // No launcher that mounts a checked descriptor and names the box's
    // first process.
    let reads = Reads::of(&sealed);
    assert_eq!(sealed.serve_unboxed(&digest), refused(Refusal::Unavailable));
    assert_eq!(reads.lookups(), 0);
    assert!(!sealed.path("started").exists());
}

/// The tracer's arguments: every process of the run followed, stopped by a
/// seccomp filter at `execve` alone, nothing written, and each `execve` of
/// `/usr/bin/env` (the box's entry, which execs the bootstrap without the
/// `PWD` bubblewrap sets, and which no other process of the run execs)
/// stopped at its entry by `SIGSTOP`, which only a kill ends: the box
/// stands, mounted, and its bootstrap never runs.
const HELD: [&str; 11] = [
    "-f",
    "--seccomp-bpf",
    "-qqq",
    "-o",
    "/dev/null",
    "-e",
    "trace=execve",
    "-P",
    "/usr/bin/env",
    "-e",
    "inject=execve:signal=SIGSTOP",
];

/// The process entering this fixture's box, once the tracer holds it
/// stopped at its entry: one with no child of its own, still stopped a
/// moment later. The launcher's own processes carry the bootstrap's words
/// on their command lines and pass through a tracer's stop at each fork,
/// so a process merely stopped once could be the box's first process and
/// put the chain one process short (CI's empty broker channel, #639).
/// Looked for until the suite's wait for a broker ([`super::TALK_MAX`])
/// has passed since `begun`, the broker's start: past the startup's
/// deadline that bounds the box's start, however loaded the host.
fn held(sealed: &Sealed, begun: Instant) -> i32 {
    let until = begun + super::TALK_MAX;
    let stopped = |pid: i32| status(pid).is_some_and(|(state, _, _)| "tT".contains(state));
    let parent =
        |pid: i32| processes().any(|child| status(child).is_some_and(|(_, of, _)| of == pid));
    while Instant::now() < until {
        let candidates = bootstraps(sealed)
            .into_iter()
            .filter(|pid| stopped(*pid) & !parent(*pid));
        let candidates: Vec<i32> = candidates.collect();
        std::thread::sleep(Duration::from_millis(200));
        if let Some(pid) = candidates
            .into_iter()
            .find(|pid| stopped(*pid) & !parent(*pid))
        {
            return pid;
        }
    }
    panic!("no box of {} held", sealed.locator.display());
}

/// A sealed fixture, its digest and the tracer that holds its box at its
/// entry; none, declared skipped, where no box stands or no tracer traces.
fn holding() -> Option<(Sealed, String, PathBuf)> {
    let (sealed, digest) = sealed()?;
    let Some(tracer) = tracer() else {
        skip(NO_TRACER);
        return None;
    };
    Some((sealed, digest, tracer))
}

/// A broker serving a sealed fixture's plan, started by a starter, its box
/// held at its entry by the tracer: when it was started, and the held
/// process followed by its parent, the box's first process, that one's, the
/// launcher, and the launcher's, the broker; and every read of its store.
/// The broker's stderr is its own file, which the tracer's notices never
/// share: strace writes a notice's prefix and its text apart, so a line of
/// the broker's could land between them on one stream.
struct Held {
    sealed: Sealed,
    attempt: Child,
    begun: Instant,
    chain: [i32; 4],
    reads: Reads,
}

impl Held {
    /// The broker started by `starter` under the tracer given `tracing`,
    /// once its box is held; none, declared skipped, where no box stands
    /// or no tracer traces.
    fn started_by(starter: Starter, tracing: &[&str]) -> Option<Held> {
        let (sealed, digest, tracer) = holding()?;
        let reads = Reads::of(&sealed);
        let apart = sealed.path("broker.err");
        let command = traced(&tracer, tracing);
        let mut command = started_to(command, &sealed, &digest, (starter, Some(&apart)));
        let begun = Instant::now();
        let attempt = command.stderr(Stdio::piped()).spawn().unwrap();
        let mut chain = [held(&sealed, begun); 4];
        for at in 1..chain.len() {
            chain[at] = status(chain[at - 1]).unwrap().1;
        }
        Some(Held {
            sealed,
            attempt,
            begun,
            chain,
            reads,
        })
    }

    /// The broker's exit and its own stderr once it has ended, and whether
    /// nothing of its box is left, nothing was looked up and no dialect
    /// server was started; what the tracer wrote on its stderr; and what
    /// the plan's ledger proves.
    fn ended(self) -> ((Option<i32>, String), bool, String, Proved) {
        let apart = self.sealed.path("broker.err");
        let out = super::bounded_apart(self.attempt, b"", &apart);
        let ledger = sealed_ledger(&self.sealed);
        let [bootstrap, first, launcher, _] = self.chain;
        let started = self.sealed.path("started").exists();
        let left = !(gone(bootstrap) & gone(first) & gone(launcher)) | started;
        let looked = self.reads.lookups() != 0;
        let broker = std::fs::read_to_string(apart).unwrap();
        let tracer = String::from_utf8(out.stderr).unwrap();
        (
            (out.status.code(), broker),
            !(left | looked),
            tracer,
            ledger,
        )
    }
}

#[test]
fn a_box_held_at_its_entry_refuses_at_the_absolute_deadline() {
    let Some(held) = Held::started_by(Starter::Harness, &HELD) else {
        return;
    };
    // Real-process proof of deadline refusal and cleanup. The shared
    // poller's controlled-clock tests prove the exact budget; this parent
    // includes process startup and its own scheduling in elapsed time.
    let elapsed = awaited(held.chain[3]) - held.begun;
    let finished = exited(held.chain[3]);
    let (answer, settled, _, ledger) = held.ended();
    assert_eq!(answer, refused(Refusal::Establishment));
    assert!(
        finished,
        "broker still running at the process wait's hang guard"
    );
    assert!(
        elapsed >= Duration::from_secs(30),
        "early refusal: {elapsed:?}"
    );
    assert!(settled);
    // Its ledger closed Failed with that cause, and no call.
    assert_eq!(ledger, failed_with(Refusal::Establishment));
}

#[test]
fn a_cancelled_attempt_ends_a_box_held_at_its_entry() {
    // The shell that started the broker, standing for the attempt's own
    // start, ends while the box is held.
    let Some(held) = Held::started_by(Starter::Attempt, &HELD) else {
        return;
    };
    let taken = cancelled(held.chain[3]);
    let ((_, stderr), settled, _, _) = held.ended();
    assert_eq!(stderr, refused(Refusal::Establishment).1);
    assert!(taken < Duration::from_secs(5), "{taken:?}");
    assert!(settled);
}

#[test]
fn the_brokers_own_line_is_read_whole_while_its_tracer_writes_notices() {
    // The tracer told to write its trace on its own stderr writes there
    // all through the run, the broker's cause among it in time; read from
    // the broker's own channel, that cause is whole and alone.
    let loud = HELD
        .into_iter()
        .filter(|arg| !["-o", "/dev/null"].contains(arg));
    let Some(held) = Held::started_by(Starter::Attempt, &loud.collect::<Vec<_>>()) else {
        return;
    };
    cancelled(held.chain[3]);
    let ((_, stderr), settled, notices, _) = held.ended();
    assert_eq!(stderr, refused(Refusal::Establishment).1);
    assert!(notices.contains("execve(\"/usr/bin/env\""), "{notices}");
    assert!(settled);
}
