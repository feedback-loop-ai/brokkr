//! The box admission launches on its waiting bootstrap (decision 0065
//! slice two U6c6b; MB3, MB5, SD3), driven through the real binary: a real
//! owned box reports ready and serving still refuses, with nothing of the
//! box left; a box held at its entry by a tracer, so that its bootstrap
//! never reports, refuses at the startup's absolute deadline, or at once
//! when the attempt is cancelled, with nothing of the box left running; and
//! a host with no launcher refuses the box as unavailable. None looks a
//! value up or starts a dialect server.

use std::mem::MaybeUninit;
use std::os::fd::OwnedFd;
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};

use rustix::fs::inotify::{self, CreateFlags, WatchFlags};
use serde_json::json;

use super::observer::{awaited, cancelled, gone, processes, started, status, traced};
use super::observer::{tracer, within, Starter, NO_TRACER};
use super::{admitted, refused, skip, unservable, Refusal, Sealed};

/// Every read of a fixture's store, its one binding's value now in it: a
/// lookup reads the store's bytes, which no path handle, identity check or
/// mount does. The FIFO store of the admission tests is refused at its
/// identity, before any box, so a store the broker admits is watched here.
struct Reads {
    store: PathBuf,
    watch: OwnedFd,
}

impl Reads {
    fn of(sealed: &Sealed) -> Reads {
        let store = sealed.path("store/secrets.env");
        std::fs::write(&store, "DOCS_TOKEN=value\n").unwrap();
        let watch = inotify::init(CreateFlags::NONBLOCK | CreateFlags::CLOEXEC).unwrap();
        inotify::add_watch(&watch, &store, WatchFlags::ACCESS).unwrap();
        Reads { store, watch }
    }

    /// The reads seen since last asked; identical reads unseen between
    /// them are seen as one.
    fn seen(&self) -> usize {
        let mut buffer = [MaybeUninit::uninit(); 4096];
        let mut events = inotify::Reader::new(&self.watch, &mut buffer);
        std::iter::from_fn(|| events.next().ok().map(drop)).count()
    }

    /// Whether any lookup read the store; the watch shown to see one read
    /// here, so that none seen is none made.
    fn lookups(self) -> usize {
        let made = self.seen();
        std::fs::read(&self.store).unwrap();
        assert_eq!(self.seen(), 1, "the watch saw no read");
        made
    }
}

/// Every process of a box this fixture's package is mounted in, started to
/// run the bootstrap: bubblewrap's within the box, and the bootstrap.
fn bootstraps(sealed: &Sealed) -> Vec<i32> {
    let package = sealed.path("opt/docs");
    let package = format!(" {} ", package.display());
    processes()
        .filter(|pid| {
            let line = std::fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
            let args: Vec<&[u8]> = line.split(|byte| *byte == 0).collect();
            let mounts = std::fs::read_to_string(format!("/proc/{pid}/mountinfo"));
            args.contains(&&b"bootstrap"[..]) & mounts.unwrap_or_default().contains(&package)
        })
        .collect()
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
fn a_real_box_reports_ready_and_serving_still_refuses_with_nothing_of_it_left() {
    let Some((sealed, digest)) = sealed() else {
        return;
    };
    // Admitted only where the bootstrap the box ran reported ready, its
    // digests, child and namespaces checked: anything else is the
    // establishment cause. Nothing is looked up.
    let reads = Reads::of(&sealed);
    assert_eq!(sealed.serve(&digest), admitted());
    assert_eq!(reads.lookups(), 0);
    assert!(within(|| bootstraps(&sealed).is_empty()));
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
/// stopped at its entry.
fn held(sealed: &Sealed) -> i32 {
    let until = Instant::now() + Duration::from_secs(20);
    while Instant::now() < until {
        let stopped = |pid: &i32| status(*pid).is_some_and(|(state, _, _)| "tT".contains(state));
        if let Some(pid) = bootstraps(sealed).into_iter().find(stopped) {
            return pid;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("no box of {} held", sealed.locator.display());
}

/// `stderr` without the notices `tracer` writes itself, such as that the
/// path it holds at is a link on this host.
fn untraced(stderr: Vec<u8>, tracer: &std::path::Path) -> String {
    let notice = format!("{}: ", tracer.display());
    let stderr = String::from_utf8(stderr).unwrap();
    let lines = stderr.lines().filter(|line| !line.starts_with(&notice));
    lines.map(|line| format!("{line}\n")).collect()
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
struct Held {
    sealed: Sealed,
    tracer: PathBuf,
    attempt: Child,
    begun: Instant,
    chain: [i32; 4],
    reads: Reads,
}

impl Held {
    /// The broker started by `starter`, once its box is held; none,
    /// declared skipped, where no box stands or no tracer traces.
    fn started_by(starter: Starter) -> Option<Held> {
        let (sealed, digest, tracer) = holding()?;
        let reads = Reads::of(&sealed);
        let mut command = started(traced(&tracer, HELD), &sealed, &digest, starter);
        let begun = Instant::now();
        let attempt = command.stderr(Stdio::piped()).spawn().unwrap();
        let mut chain = [held(&sealed); 4];
        for at in 1..chain.len() {
            chain[at] = status(chain[at - 1]).unwrap().1;
        }
        Some(Held {
            sealed,
            tracer,
            attempt,
            begun,
            chain,
            reads,
        })
    }

    /// The broker's exit and stderr once it has ended, and whether nothing
    /// of its box is left, nothing was looked up and no dialect server was
    /// started.
    fn ended(self) -> ((Option<i32>, String), bool) {
        let out = self.attempt.wait_with_output().unwrap();
        let [bootstrap, first, launcher, _] = self.chain;
        let started = self.sealed.path("started").exists();
        let left = !(gone(bootstrap) & gone(first) & gone(launcher)) | started;
        let looked = self.reads.lookups() != 0;
        let answer = (out.status.code(), untraced(out.stderr, &self.tracer));
        (answer, !(left | looked))
    }
}

#[test]
fn a_box_held_at_its_entry_refuses_at_the_absolute_deadline() {
    let Some(held) = Held::started_by(Starter::Harness) else {
        return;
    };
    // The one deadline admission started with, observation's and
    // readiness's alike, never extended: the broker ends once it has
    // passed, and no more than a slice after. The box went with it.
    let elapsed = awaited(held.chain[3]) - held.begun;
    let (answer, settled) = held.ended();
    assert_eq!(answer, refused(Refusal::Establishment));
    let bounded = (elapsed >= Duration::from_secs(30)) & (elapsed < Duration::from_secs(32));
    assert!(bounded, "{elapsed:?}");
    assert!(settled);
}

#[test]
fn a_cancelled_attempt_ends_a_box_held_at_its_entry() {
    // The shell that started the broker, standing for the attempt's own
    // start, ends while the box is held.
    let Some(held) = Held::started_by(Starter::Attempt) else {
        return;
    };
    let taken = cancelled(held.chain[3]);
    let ((_, stderr), settled) = held.ended();
    assert_eq!(stderr, refused(Refusal::Establishment).1);
    assert!(taken < Duration::from_secs(5), "{taken:?}");
    assert!(settled);
}
