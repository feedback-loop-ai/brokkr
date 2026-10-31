//! Task 46.1's cancellation at every startup window, through real dispatch
//! (U9b1b; MB3, MB5): the harness dies with its broker held at one window, the
//! broker's input kept open past it by a process of another session, so EOF on
//! that input is not what ends it. The broker ends within seconds and nothing
//! that stood below it remains. What it left is bounded, not exact: its ledger
//! is a prefix of the window's records, closed on the window's disposition only
//! where it closed at all, as the engine's sweep may kill it first; and the run
//! parks on the window's cause or on a driver's lost result, never on success
//! (the evidence's F9). Pre-secret windows keep their store unread.

use std::process::Stdio;
use std::time::{Duration, Instant};

use brokkr_protocol::broker::ledger::Failure;
use brokkr_protocol::broker::Refusal;
use rustix::process::{kill_process, Pid, Signal};
use serde_json::{json, Value};

use super::{put, skip, World, STARTS};
use crate::capability_broker::observer::{processes, status, tracer, NO_TRACER};

/// A window of a broker's startup, held open.
#[derive(Debug, Clone, Copy)]
enum Window {
    /// Its observer's launcher query, stopped at its exec.
    Observation,
    /// The box stopped at its entry, the first exec of the system's `env`.
    Entry,
    /// The sender's read of the store, delayed.
    StoreRead,
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
                "-e trace=read -P {} -e inject=read:delay_exit=20000000:when=1",
                world.secrets().display()
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
        match self {
            Window::Observation => (closed, Some(json!({"failed": {"refused": "identity"}}))),
            Window::Entry => (
                closed,
                Some(json!({"failed": {"refused": "establishment"}})),
            ),
            Window::StoreRead => (&["opened"], None),
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
            Window::Initialization | Window::Call => (Failure::Timeout.to_string(), true),
        }
    }
}

/// `pid`'s number, once its file stands.
fn pid_in(world: &World, name: &str) -> i32 {
    let until = Instant::now() + Duration::from_secs(90);
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

/// Waits until `window` is held below `broker`.
fn held(world: &World, window: Window, broker: i32, reads: &mut dyn FnMut() -> usize) {
    let until = Instant::now() + Duration::from_secs(90);
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
            Window::StoreRead => reads() > 0,
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

/// The one proof of a window, whichever: see the module's text.
fn cancelled_at(window: Window) {
    let Some(world) = World::new() else {
        return;
    };
    let Some(wrap) = window.tracing(&world) else {
        return skip(NO_TRACER);
    };
    if !wrap.is_empty() {
        put(&world.bin("wrap"), wrap);
    }
    if let Some(server) = window.server() {
        world.server(&server);
    }
    put(&world.bin("keep-input"), "");
    let lookups = world.store_reads();
    let (cause, read) = window.expected();
    let args = world.run_args();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let engine = world
        .brokkr(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let harness = pid_in(&world, "self.pid");
    let until = Instant::now() + Duration::from_secs(90);
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
    let standing = below(broker);
    killed(harness);
    let cancelled = Instant::now();
    let keeper = pid_in(&world, "keeper.pid");
    while status(broker).is_some_and(|(state, _, _)| state != 'Z') {
        let taken = cancelled.elapsed();
        assert!(taken < Duration::from_secs(5), "{window:?}: {taken:?}");
        std::thread::sleep(Duration::from_millis(10));
    }
    killed(keeper);
    for pid in standing {
        let settled = Instant::now() + Duration::from_secs(5);
        while status(pid).is_some_and(|(state, _, _)| state != 'Z') {
            assert!(Instant::now() < settled, "{window:?}: {pid} still stands");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    let ended = super::ended(engine.wait_with_output().unwrap());
    // The broker's own record, closed on the window's cause, or a prefix of it
    // where the engine's sweep of the attempt's tree killed it before it closed.
    let ledger = std::fs::read_to_string(world.plan().with_extension("json.ledger")).unwrap();
    let records: Vec<Value> = (ledger.lines())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let kinds: Vec<&str> = (records.iter())
        .map(|record| record["record"].as_str().unwrap())
        .collect();
    let disposition = records.last().unwrap().get("disposition");
    let (want, closed) = window.ledger();
    assert!(want.starts_with(&kinds), "{window:?}: {ledger}");
    if disposition.is_some() {
        assert_eq!(
            (&kinds[..], disposition),
            (want, closed.as_ref()),
            "{ledger}"
        );
    }
    // The park never claims success. It is the broker's cause where the engine
    // proved the attempt settled before it read the ledger, and its own
    // indeterminate otherwise, the harness's death being a driver's lost result.
    let lost = "indeterminate: driver exited after accepting, before a result — \
                attempt cannot be established as complete";
    let parked = ended.cause.clone().unwrap_or_default();
    assert!(
        ended.status == "awaiting_operator" && (parked == cause || parked.ends_with(lost)),
        "{window:?} {ended:?}"
    );
    let reads = seen + lookups.lookups();
    assert_eq!(reads > 0, read, "{window:?}: the store's {reads} reads");
    assert_eq!(world.starts(), 1, "{window:?}: the harness started once");
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
