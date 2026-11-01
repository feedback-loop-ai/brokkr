//! Task 46.1's bounded startup and its cancellation, through real dispatch
//! (U9b1b; MB3, MB5): each window of a broker's startup but `Handoff` held, then
//! ended three ways; `Handoff` only by the deadline, the other two are U9b1c's.
//!
//! The harness's death, the broker's own sign of cancellation, its input kept
//! open past it by a process of another session, so EOF on that input is not
//! what ends it. The broker ends within seconds. What it left is bounded, not
//! exact: its ledger is a prefix of the window's records, closed on the window's
//! disposition only where it closed at all, as the engine's sweep may kill it
//! first; and the run parks on the window's cause or on a driver's lost result,
//! never on success (the evidence's F9).
//!
//! The engine's own timeout of the attempt, which sweeps the whole of it: the
//! run parks on the timeout, and `inspect` keeps a begun call interrupted.
//!
//! The startup's own deadline, with the harness alive: the broker closes on the
//! window's exact cause, past the harness's report of success, no call row
//! invented.
//!
//! Every way, nothing that stood below the broker remains, its private mounts
//! gone with the last process of their namespaces, and pre-secret windows keep
//! the store unread.

use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::time::{Duration, Instant, SystemTime};

use brokkr_protocol::broker::ledger::Failure;
use brokkr_protocol::broker::Refusal;
use rustix::process::{kill_process, Pid, Signal};
use serde_json::{json, Value};

use super::{put, skip, Ended, World, STARTS};
use crate::capability_broker::observer::{processes, status, tracer, NO_TRACER};
use crate::capability_broker::readiness::Reads;

/// How long the tracer holds a store read or a server's exec: past the broker's
/// 30 s startup, so that only its deadline or a cancellation ends the window.
const HELD_MS: u32 = 60_000;

/// The attempt's timeout where the engine ends a window: long enough to reach
/// each window on a loaded host, short of the broker's own 30 s.
const TIMEOUT: u64 = 25;

/// The end of the engine's own words for an attempt it timed out.
const TIMED_OUT: &str =
    "s deadline and was killed; its process tree is gone, and what it did is unknown";

/// How long the engine's end of a timed or expired window is waited for.
const PATIENCE: Duration = Duration::from_secs(90);

/// How long a window's broker, and then everything below it, is given to end
/// once its end is due: seconds, on a hosted runner as on a quiet host.
const TEARDOWN: Duration = Duration::from_secs(30);

/// A window of a broker's startup, held open.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Window {
    /// Its observer's launcher query, stopped at its exec.
    Observation,
    /// The box stopped at its entry, the first exec of the system's `env`.
    Entry,
    /// The sender's read of the store, delayed.
    StoreRead,
    /// The whole frame delivered, the bootstrap's exec of the server delayed
    /// with its status pipe open.
    Handoff,
    /// The server holding its answer to `initialize`.
    Initialization,
    /// The server holding its answer to a call.
    Call,
}

impl Window {
    /// The tracer's words that hold this window, ahead of the broker's own.
    fn tracing(self, world: &World) -> Option<String> {
        let traced = |held: String| {
            let tracer = tracer()?;
            let tracer = tracer.display();
            Some(format!(
                "{tracer} -f -DDD --seccomp-bpf -qqq -o /dev/null {held}"
            ))
        };
        let stopped =
            |path: &str| format!("-e trace=execve -P {path} -e inject=execve:signal=SIGSTOP");
        match self {
            Window::Observation => traced(stopped("/proc/self/fd/0")),
            Window::Entry => traced(stopped("/usr/bin/env")),
            Window::StoreRead => traced(format!(
                "-e trace=read -P {} -e inject=read:delay_exit={HELD_MS}000:when=1",
                world.secrets().display()
            )),
            Window::Handoff => traced(format!(
                "-e trace=execve -P {} -e inject=execve:delay_enter={HELD_MS}000",
                world.program().display()
            )),
            Window::Initialization | Window::Call => Some(String::new()),
        }
    }

    /// The server that holds this window, or none where the tracer holds it.
    fn server(self) -> Option<String> {
        match self {
            Window::Initialization => Some("#!/bin/sh\nexec sleep 600\n".into()),
            Window::Call => Some(format!("{STARTS}exec sleep 600\n")),
            _ => None,
        }
    }

    /// The records of the broker's ledger, and the disposition it closes on;
    /// at a held store read it knew no bindings to guard a closure by, so it
    /// withheld the closure, and the ledger reads as a lifetime unended.
    fn ledger(self) -> (&'static [&'static str], Option<Value>) {
        let closed: &[&str] = &["opened", "closed"];
        let refused = |cause: &str| Some(json!({"failed": {"refused": cause}}));
        match self {
            Window::Observation => (closed, refused("identity")),
            Window::Entry => (closed, refused("establishment")),
            Window::StoreRead => (&["opened"], None),
            Window::Handoff => (closed, refused("handoff")),
            Window::Initialization => (closed, Some(json!({"failed": "timeout"}))),
            Window::Call => (
                &["opened", "started", "terminal", "closed"],
                Some(json!({"failed": "timeout"})),
            ),
        }
    }

    /// The cause the attempt parks on, and whether the store is read by then.
    fn expected(self) -> (String, bool) {
        match self {
            Window::Observation => (Refusal::Identity.to_string(), false),
            Window::Entry => (Refusal::Establishment.to_string(), false),
            Window::StoreRead => ("broker ledger lifecycle is invalid".to_string(), true),
            Window::Handoff => (Refusal::Handoff.to_string(), true),
            Window::Initialization | Window::Call => (Failure::Timeout.to_string(), true),
        }
    }

    /// Whether a server's box stands in this window.
    fn boxed(self) -> bool {
        self != Window::Observation
    }

    /// Whether the server has started by this window.
    fn serves(self) -> bool {
        matches!(self, Window::Initialization | Window::Call)
    }
}

/// `pid`'s number, once its file stands.
fn pid_in(world: &World, name: &str) -> i32 {
    let until = Instant::now() + PATIENCE;
    loop {
        let held = std::fs::read_to_string(world.bin(name)).unwrap_or_default();
        if let Ok(pid) = held.trim().parse() {
            return pid;
        }
        assert!(Instant::now() < until, "no {name}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Every live process below `root`.
fn below(root: i32) -> Vec<i32> {
    let mut found = vec![root];
    let mut at = 0;
    while at < found.len() {
        let parent = found[at];
        let children = processes()
            .filter(|pid| status(*pid).is_some_and(|(state, of, _)| of == parent && state != 'Z'));
        found.extend(children.collect::<Vec<_>>());
        at += 1;
    }
    found.remove(0);
    found
}

/// The broker this world's harness started: the child of `harness` whose
/// executable is the engine's own.
fn broker_below(harness: i32) -> Option<i32> {
    let engine = crate::capability_broker::brokkr();
    below(harness).into_iter().find(|pid| {
        let exe = std::fs::read_link(format!("/proc/{pid}/exe"));
        exe.is_ok_and(|exe| exe == engine)
    })
}

/// The mount namespace `pid` stands in; none once it is gone.
fn mount_space(pid: i32) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/{pid}/ns/mnt")).ok()
}

/// Waits until `window` is held below `broker`.
fn held(world: &World, window: Window, broker: i32, reads: &mut dyn FnMut() -> usize) {
    let until = Instant::now() + PATIENCE;
    let stopped = || {
        below(broker).into_iter().any(|pid| {
            status(pid).is_some_and(|(state, _, _)| "tT".contains(state)) && below(pid).is_empty()
        })
    };
    let holding = || {
        below(broker).into_iter().any(|pid| {
            std::fs::read_to_string(format!("/proc/{pid}/comm")).is_ok_and(|comm| comm == "sleep\n")
        })
    };
    loop {
        let there = match window {
            Window::Observation | Window::Entry => {
                let first = stopped();
                std::thread::sleep(Duration::from_millis(200));
                first && stopped()
            }
            Window::StoreRead | Window::Handoff => reads() > 0,
            Window::Initialization => holding(),
            Window::Call => {
                let config = world.bin("config.json");
                config.exists()
                    && std::fs::read(world.plan().with_extension("json.ledger"))
                        .is_ok_and(|ledger| ledger.windows(7).any(|said| said == b"resolve"))
            }
        };
        if there {
            return;
        }
        assert!(Instant::now() < until, "{window:?} never held");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn killed(pid: i32) {
    let pid = Pid::from_raw(pid).unwrap();
    kill_process(pid, Signal::KILL).ok();
}

/// A broker held at its window through real dispatch, and what stood below it
/// then.
struct Standing {
    world: World,
    window: Window,
    engine: Child,
    harness: i32,
    broker: i32,
    lookups: Reads,
    seen: usize,
    started: Reads,
    below: Vec<i32>,
    spaces: Vec<PathBuf>,
}

impl Standing {
    /// `window` held in a fresh world readied by `ready`; none where this host
    /// holds no such window, skipped.
    fn at(window: Window, ready: impl FnOnce(&World)) -> Option<Standing> {
        let world = World::new()?;
        let Some(wrap) = window.tracing(&world) else {
            skip(NO_TRACER);
            return None;
        };
        if !wrap.is_empty() {
            put(&world.bin("wrap"), wrap);
        }
        if let Some(server) = window.server() {
            world.server(&server);
        }
        ready(&world);
        let (lookups, started) = (world.store_reads(), world.server_reads());
        let args = world.run_args();
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let engine = world
            .brokkr(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let harness = pid_in(&world, "self.pid");
        let until = Instant::now() + PATIENCE;
        let broker = loop {
            if let Some(broker) = broker_below(harness) {
                break broker;
            }
            assert!(Instant::now() < until, "no broker");
            std::thread::sleep(Duration::from_millis(20));
        };
        let mut seen = 0;
        held(&world, window, broker, &mut || {
            seen += lookups.seen();
            seen
        });
        let below = below(broker);
        let ours = mount_space(std::process::id().cast_signed());
        let mut spaces: Vec<PathBuf> = below.iter().filter_map(|pid| mount_space(*pid)).collect();
        spaces.retain(|space| Some(space) != ours.as_ref());
        spaces.dedup();
        assert_eq!(!spaces.is_empty(), window.boxed(), "{window:?}: {spaces:?}");
        Some(Standing {
            world,
            window,
            engine,
            harness,
            broker,
            lookups,
            seen,
            started,
            below,
            spaces,
        })
    }

    /// Waits for the broker's end, within `bound`.
    fn broker_ended(&self, bound: Duration) {
        let begun = Instant::now();
        while status(self.broker).is_some_and(|(state, _, _)| state != 'Z') {
            let taken = begun.elapsed();
            assert!(taken < bound, "{:?}: {taken:?}", self.window);
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Nothing of the window remains within `bound`: its private mounts, gone
    /// with the last process of their namespaces, and every process that stood
    /// below the broker. Asked first of the namespaces, as a process of the box
    /// left running holds its mounts up by itself.
    fn settled(&self, bound: Duration) {
        let window = self.window;
        let until = Instant::now() + bound;
        let holding = || {
            processes().find(|pid| mount_space(*pid).is_some_and(|at| self.spaces.contains(&at)))
        };
        while let Some(pid) = holding() {
            assert!(
                Instant::now() < until,
                "{window:?}: {pid} holds the box's mounts"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        for pid in &self.below {
            while status(*pid).is_some_and(|(state, _, _)| state != 'Z') {
                assert!(Instant::now() < until, "{window:?}: {pid} still stands");
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }

    /// Whether the store was read, and the server's program, however it ended;
    /// each window's own, held to its table.
    fn reads_held(self) {
        let window = self.window;
        let (_, read) = window.expected();
        let reads = self.seen + self.lookups.lookups();
        assert_eq!(reads > 0, read, "{window:?}: the store's {reads} reads");
        let served = self.started.lookups();
        assert_eq!(
            served > 0,
            window.serves(),
            "{window:?}: {served} reads of the server"
        );
        assert_eq!(
            self.world.starts(),
            1,
            "{window:?}: the harness started once"
        );
    }

    /// The broker's ledger, its records' kinds and its last disposition.
    fn ledger(&self) -> (Vec<String>, Option<Value>) {
        let records = self.world.ledger();
        let kinds = records
            .iter()
            .map(|record| record["record"].as_str().unwrap());
        let kinds = kinds.map(String::from).collect();
        (kinds, records.last().unwrap().get("disposition").cloned())
    }

    /// The engine's end, within [`PATIENCE`], whatever it is waiting for.
    fn engine_ended(&mut self) -> Ended {
        let until = Instant::now() + PATIENCE;
        while self.engine.try_wait().unwrap().is_none() {
            if Instant::now() >= until {
                self.engine.kill().ok();
                self.engine.wait().ok();
                killed(self.broker);
                panic!("{:?}: the engine did not end the window", self.window);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let mut out = String::new();
        let stdout = self.engine.stdout.take().unwrap();
        std::io::Read::read_to_string(&mut { stdout }, &mut out).unwrap();
        super::ended_from(&out)
    }
}

/// The proof of a harness's death at `window`: see the module's text.
fn cancelled_at(window: Window) {
    let Some(mut standing) = Standing::at(window, |world| {
        put(&world.bin("keep-input"), "");
    }) else {
        return;
    };
    killed(standing.harness);
    let keeper = pid_in(&standing.world, "keeper.pid");
    standing.broker_ended(TEARDOWN);
    killed(keeper);
    standing.settled(TEARDOWN);
    let ended = standing.engine_ended();
    // The broker's own record, closed on the window's cause, or a prefix of it
    // where the engine's sweep of the attempt's tree killed it before it closed.
    let (kinds, disposition) = standing.ledger();
    let (want, closed) = window.ledger();
    let kept: Vec<&str> = kinds.iter().map(String::as_str).collect();
    assert!(want.starts_with(&kept), "{window:?}: {kinds:?}");
    if disposition.is_some() {
        assert_eq!((&kept[..], disposition), (want, closed), "{window:?}");
    }
    // The park never claims success. It is the broker's cause where the engine
    // proved the attempt settled before it read the ledger, and its own
    // indeterminate otherwise, the harness's death being a driver's lost result.
    let (cause, _) = window.expected();
    let lost = "indeterminate: driver exited after accepting, before a result — \
                attempt cannot be established as complete";
    let parked = ended.cause.clone().unwrap_or_default();
    assert!(
        ended.status == "awaiting_operator" && (parked == cause || parked.ends_with(lost)),
        "{window:?} {ended:?}"
    );
    standing.reads_held();
}

#[test]
fn a_harness_dying_at_its_brokers_observation_ends_it_unread() {
    cancelled_at(Window::Observation);
}

#[test]
fn a_harness_dying_at_a_standing_box_ends_its_broker_unread() {
    cancelled_at(Window::Entry);
}

#[test]
fn a_harness_dying_at_a_held_store_read_ends_its_broker() {
    cancelled_at(Window::StoreRead);
}

#[test]
fn a_harness_dying_during_initialization_ends_its_broker() {
    cancelled_at(Window::Initialization);
}

#[test]
fn a_harness_dying_during_a_call_ends_its_broker() {
    cancelled_at(Window::Call);
}

/// The proof of the engine's own timeout at `window`, whose attempt it sweeps:
/// the run parks on the timeout, nothing is left, and `calls` is what `inspect`
/// keeps of the attempt.
fn timed_out_at(window: Window, calls: &[(String, String, Option<String>)]) {
    let limits = json!({"max_attempts": 1, "timeout_seconds": TIMEOUT});
    let Some(mut standing) = Standing::at(window, |world| world.chain(&["opus"], Some(limits)))
    else {
        return;
    };
    // Held, this waits on the timeout's clock and no survey of its own.
    standing.world.unserialized();
    let ended = standing.engine_ended();
    standing.broker_ended(TEARDOWN);
    standing.settled(TEARDOWN);
    // The deadline it names is what admission and provisioning left of the
    // seat's timeout.
    let parked = ended.cause.clone().unwrap_or_default();
    let (_, why) = parked
        .split_once(" indeterminate: ")
        .unwrap_or(("", &parked));
    let left = why
        .strip_prefix("attempt exceeded its ")
        .and_then(|rest| rest.strip_suffix(TIMED_OUT))
        .and_then(|seconds| seconds.parse::<u64>().ok());
    assert!(
        ended.status == "awaiting_operator" && left.is_some_and(|left| left <= TIMEOUT),
        "{window:?} {ended:?}"
    );
    assert_eq!(standing.world.calls(&ended.run), calls, "{window:?}");
    standing.reads_held();
}

#[test]
fn the_engines_timeout_at_a_brokers_observation_ends_it_unread() {
    timed_out_at(Window::Observation, &[]);
}

#[test]
fn the_engines_timeout_at_a_standing_box_ends_it_unread() {
    timed_out_at(Window::Entry, &[]);
}

#[test]
fn the_engines_timeout_at_a_held_store_read_ends_it() {
    timed_out_at(Window::StoreRead, &[]);
}

#[test]
fn the_engines_timeout_during_initialization_ends_it() {
    timed_out_at(Window::Initialization, &[]);
}

/// The call began and the sweep took the broker before its terminal: `inspect`
/// keeps it interrupted, with no digest.
#[test]
fn the_engines_timeout_during_a_call_leaves_it_interrupted_through_inspect() {
    let row = ("resolve".to_string(), "interrupted".to_string(), None);
    timed_out_at(Window::Call, &[row]);
}

/// The proof of the startup's own deadline at `window`, the harness alive: the
/// broker ends at it and no sooner, closes on the window's exact cause, past the
/// harness's report of success, with no call row.
fn expired_at(window: Window) {
    let Some(mut standing) = Standing::at(window, |_| {}) else {
        return;
    };
    // Held, this waits on the deadline's clock and no survey of its own.
    standing.world.unserialized();
    // The broker's end by its own state: a process held in the tracer's delay is
    // reaped only with the delay's end, which is the hold's and not the deadline's.
    standing.broker_ended(PATIENCE);
    let at = SystemTime::now();
    let ended = standing.engine_ended();
    let (cause, _) = window.expected();
    assert_eq!(
        (ended.status.as_str(), ended.cause.as_deref()),
        ("awaiting_operator", Some(cause.as_str())),
        "{window:?}"
    );
    let world = &standing.world;
    assert_eq!(world.broker_code(), Some(1), "{window:?}");
    let times = std::fs::read_to_string(world.bin("times")).unwrap();
    let begun: u64 = times.lines().next().unwrap().parse().unwrap();
    let harness = SystemTime::UNIX_EPOCH + Duration::from_nanos(begun);
    let lasted = at.duration_since(harness).unwrap();
    let released = Duration::from_millis(u64::from(HELD_MS));
    assert!(
        lasted >= Duration::from_secs(30) && lasted < released,
        "{window:?}: {lasted:?}"
    );
    assert_eq!(world.calls(&ended.run), [], "{window:?}");
    let (kinds, disposition) = standing.ledger();
    let (want, closed) = window.ledger();
    let kept: Vec<&str> = kinds.iter().map(String::as_str).collect();
    assert_eq!((&kept[..], disposition), (want, closed), "{window:?}");
    // A process killed in the tracer's delay ends with the delay's end.
    standing.settled(Duration::from_secs(40));
    standing.reads_held();
}

#[test]
fn an_observation_held_to_the_startup_deadline_refuses_by_its_identity_cause() {
    expired_at(Window::Observation);
}

#[test]
fn a_box_held_at_its_entry_to_the_startup_deadline_refuses_by_its_establishment_cause() {
    expired_at(Window::Entry);
}

#[test]
fn a_store_read_held_to_the_startup_deadline_withholds_its_closure() {
    expired_at(Window::StoreRead);
}

#[test]
fn a_status_pipe_open_to_the_startup_deadline_refuses_by_the_handoffs_cause() {
    expired_at(Window::Handoff);
}

#[test]
fn a_server_silent_to_the_startup_deadline_fails_by_its_timeout_cause() {
    expired_at(Window::Initialization);
}
