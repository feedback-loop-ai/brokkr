//! The standing loop (decision 0110 ruling 5), driven tick by tick through
//! its seams: a stop claims nothing more, a stand's ending ends the loop
//! for the next dispatcher's reconciliation, an exited driver is reaped and
//! its run reported, nothing a pass reports latches, a fence-out ends the
//! loop, and each tick sleeps what is left of its interval.

use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::ExitStatus;
use std::rc::Rc;
use std::time::{Duration, Instant};

use brokkr_core::fold::Status;
use brokkr_store::{Claim, Store};
use serde_json::json;

use super::*;
use crate::admission::capacity::tests::{ceilings, Fixture};
use crate::admission::AdmissionError;
use crate::dispatch::pass::tests::support::{claims, commands, count, host, spawned_one};
use crate::dispatch::take_dispatch;

/// What the loop's seams saw, in order.
#[derive(Debug, PartialEq, Eq)]
enum Seen {
    /// A pass's report: the runs it started and the runs reconciliation
    /// spawned, and whether it reported the host unreadable.
    Passed {
        started: Vec<String>,
        spawned: Vec<String>,
        host: bool,
    },
    /// A reaped driver: its run, pid, exit code and what its run read.
    Reaped {
        run: String,
        pid: u32,
        code: Option<i32>,
        reads: Option<Status>,
    },
    Slept(Duration),
}

impl Seen {
    fn of(tick: Tick<'_>) -> Seen {
        match tick {
            Tick::Passed(passed) => Seen::Passed {
                started: passed.started.iter().map(|s| s.run.clone()).collect(),
                spawned: passed.spawned.iter().map(|s| s.run.clone()).collect(),
                host: matches!(passed.admission_refused, Some(AdmissionError::Host(_))),
            },
            Tick::Reaped(reaped) => Seen::Reaped {
                run: reaped.exited.run.clone(),
                pid: reaped.exited.pid,
                code: reaped
                    .exited
                    .exit
                    .as_ref()
                    .ok()
                    .and_then(|exit| exit.code()),
                reads: reaped.reads.as_ref().ok().copied(),
            },
        }
    }
}

/// A pass's spawn seam, as a script holds it.
type SpawnSeam<'a> = Box<dyn FnMut(&Spawn) -> Result<Spawned, SpawnFailed> + 'a>;

/// A loop's script: its interval, how far its clock moves between two
/// reads, the pass's seams, its reap seam, what happens in each sleep (by
/// its count from 1), and the pass after whose report a stop is asked.
struct Script<'a> {
    every: Duration,
    pace: Duration,
    stop: Rc<Cell<bool>>,
    stands: Box<dyn FnMut() -> Result<(), Ending> + 'a>,
    spawn: SpawnSeam<'a>,
    reap: Box<dyn FnMut() -> Vec<Exited> + 'a>,
    slept: Box<dyn FnMut(usize) + 'a>,
    ticks: usize,
}

impl<'a> Script<'a> {
    /// Three ticks a minute apart, each pass taking a second, every seam
    /// standing and starting pid 1, and nothing reaped.
    fn new() -> Script<'a> {
        Script {
            every: Duration::from_secs(60),
            pace: Duration::from_secs(1),
            stop: Rc::new(Cell::new(false)),
            stands: Box::new(|| Ok(())),
            spawn: Box::new(spawned_one),
            reap: Box::new(Vec::new),
            slept: Box::new(|_| {}),
            ticks: 3,
        }
    }

    /// Stand over `store` with the host `file` names: how the loop ended
    /// and what its seams saw.
    fn run(
        mut self,
        store: &mut Store,
        dispatcher: &Dispatcher,
        file: &Path,
    ) -> (Result<Stopped, Box<Ended>>, Vec<Seen>) {
        let seen = RefCell::new(Vec::new());
        let (base, pace, mut reads) = (Instant::now(), self.pace, 0);
        let mut now = || {
            reads += 1;
            base + pace * reads
        };
        let (stop, ticks, mut passes) = (Rc::clone(&self.stop), self.ticks, 0);
        let mut sleeps = 0;
        let slept = &mut self.slept;
        let mut sleep = |rest: Duration| {
            seen.borrow_mut().push(Seen::Slept(rest));
            sleeps += 1;
            slept(sleeps);
        };
        let mut report = |tick: Tick<'_>| {
            if let Tick::Passed(_) = tick {
                passes += 1;
                if passes == ticks {
                    stop.set(true);
                }
            }
            seen.borrow_mut().push(Seen::of(tick));
        };
        let asked = Rc::clone(&self.stop);
        let stand = Stand {
            every: self.every,
            stands: &mut self.stands,
            spawn: &mut self.spawn,
            stopped: &move || asked.get(),
            now: &mut now,
            sleep: &mut sleep,
            reap: &mut self.reap,
            report: &mut report,
        };
        let ended = super::stand(store, dispatcher, &host(file), &OsString::new(), stand);
        (ended, seen.into_inner())
    }
}

/// A fixture with `entries` queued under ceilings that admit three runs,
/// its host file and its dispatcher.
fn setup(entries: usize) -> (Fixture, std::path::PathBuf, Dispatcher) {
    let mut fixture = Fixture::new();
    let file = fixture.declare(&ceilings(3, 3, json!({})));
    for _ in 0..entries {
        fixture.queue_bundle("bundle");
    }
    let dispatcher = take_dispatch(&mut fixture.store).unwrap().unwrap();
    (fixture, file, dispatcher)
}

/// The queue's command words, in order.
fn words(store: &Store) -> Vec<String> {
    commands(store)
        .into_iter()
        .map(|(_, word, ..)| word)
        .collect()
}

/// A pass's report with `started` runs and nothing else.
fn passed(started: &[&str]) -> Seen {
    Seen::Passed {
        started: started.iter().map(|run| run.to_string()).collect(),
        spawned: vec![],
        host: false,
    }
}

/// A spawn seam that starts pid 1 and records each claim it is handed.
fn recording(handed: &Rc<RefCell<Vec<Claim>>>) -> SpawnSeam<'static> {
    let handed = Rc::clone(handed);
    Box::new(move |spawn: &Spawn| {
        handed.borrow_mut().push(spawn.claim);
        Ok(Spawned { pid: 1 })
    })
}

/// The one run the journal holds.
fn the_run(store: &Store) -> String {
    let runs = store.list_runs().unwrap();
    let [(run, ..)] = runs.as_slice() else {
        panic!("{runs:?}");
    };
    run.clone()
}

/// Run `script` over `fixture` to the stop it ends on: what its seams saw,
/// and the one run the journal then holds.
fn stopped(
    script: Script<'_>,
    fixture: &mut Fixture,
    dispatcher: &Dispatcher,
    file: &Path,
) -> (Vec<Seen>, String) {
    let (ended, seen) = script.run(&mut fixture.store, dispatcher, file);
    assert_eq!(ended.unwrap(), Stopped);
    (seen, the_run(&fixture.store))
}

/// Two entries queued, and a loop whose stop is asked in the `call`th
/// binary check: the fixture, then what the seams saw and the one run.
fn stopped_in_check(call: usize) -> (Fixture, Vec<Seen>, String) {
    let (mut fixture, file, dispatcher) = setup(2);
    let mut script = Script::new();
    let (stop, mut calls) = (Rc::clone(&script.stop), 0);
    script.stands = Box::new(move || {
        calls += 1;
        if calls == call {
            stop.set(true);
        }
        Ok(())
    });
    let (seen, run) = stopped(script, &mut fixture, &dispatcher, &file);
    (fixture, seen, run)
}

/// A driver reaped whose run still reads `running`.
fn reaped_running(run: &str, pid: u32, code: i32) -> Seen {
    Seen::Reaped {
        run: run.to_string(),
        pid,
        code: Some(code),
        reads: Some(Status::Running),
    }
}

#[test]
fn a_stop_during_one_entry_ends_the_loop_after_its_driver_with_no_further_claim() {
    // The stop arrives as the first entry's `spawn` row is about to land.
    let (fixture, seen, run) = stopped_in_check(2);
    assert_eq!(seen, vec![passed(&[run.as_str()])], "one tick, no sleep");
    assert_eq!(
        words(&fixture.store),
        ["add", "add", "claim", "link", "spawn"]
    );
    assert_eq!(
        claims(&fixture.store).len(),
        1,
        "no claim followed the stop"
    );
}

#[test]
fn a_stop_during_the_binary_check_before_a_claim_claims_nothing() {
    // The stop arrives in the binary check before the second entry's claim.
    let (fixture, seen, run) = stopped_in_check(3);
    assert_eq!(seen, vec![passed(&[run.as_str()])]);
    assert_eq!(
        words(&fixture.store),
        ["add", "add", "claim", "link", "spawn"]
    );
}

#[test]
fn a_stand_that_ends_before_the_spawn_row_ends_the_loop_and_reconciliation_spawns_it() {
    let (mut fixture, file, dispatcher) = setup(1);
    let mut script = Script::new();
    let mut calls = 0;
    script.stands = Box::new(move || {
        calls += 1;
        match calls {
            1 => Ok(()),
            _ => Err(Ending::Stood),
        }
    });
    let (ended, seen) = script.run(&mut fixture.store, &dispatcher, &file);
    assert!(matches!(ended.unwrap_err().ending, Ending::Stood));
    assert_eq!(seen, vec![passed(&[])]);
    assert_eq!(words(&fixture.store), ["add", "claim", "link"]);
    drop(dispatcher);

    let next = take_dispatch(&mut fixture.store).unwrap().unwrap();
    let script = Script {
        ticks: 1,
        ..Script::new()
    };
    let (ended, seen) = script.run(&mut fixture.store, &next, &file);
    assert_eq!(ended.unwrap(), Stopped);
    let run = the_run(&fixture.store);
    let spawned = Seen::Passed {
        started: vec![],
        spawned: vec![run],
        host: false,
    };
    assert_eq!(seen, vec![spawned]);
    assert_eq!(count(&fixture.store, "spawn"), 1);
}

#[test]
fn an_exited_driver_is_reaped_and_its_still_running_run_reported() {
    let (mut fixture, file, dispatcher) = setup(1);
    let spawned = Rc::new(RefCell::new(Vec::new()));
    let mut script = Script::new();
    let record = Rc::clone(&spawned);
    script.spawn = Box::new(move |spawn: &Spawn| {
        record.borrow_mut().push((spawn.run.clone(), spawn.claim));
        Ok(Spawned { pid: 7 })
    });
    // Each driver spawned exits at once, with code 1, its run unconcluded.
    let drained = Rc::clone(&spawned);
    script.reap = Box::new(move || {
        let runs = std::mem::take(&mut *drained.borrow_mut());
        let exit = || Ok(ExitStatus::from_raw(1 << 8));
        let exited = |(run, claim)| Exited {
            run,
            claim,
            pid: 7,
            exit: exit(),
        };
        runs.into_iter().map(exited).collect()
    });
    let (seen, run) = stopped(script, &mut fixture, &dispatcher, &file);
    let slept = Seen::Slept(Duration::from_secs(59));
    assert_eq!(
        seen,
        vec![
            passed(&[run.as_str()]),
            reaped_running(&run, 7, 1),
            Seen::Slept(Duration::from_secs(59)),
            passed(&[]),
            slept,
            passed(&[]),
        ]
    );
}

#[test]
fn a_driver_that_exits_during_the_sleep_a_stop_ends_is_reaped_before_the_loop_ends() {
    let (mut fixture, file, dispatcher) = setup(1);
    let exits = Rc::new(RefCell::new(Vec::new()));
    let mut script = Script::new();
    let drained = Rc::clone(&exits);
    script.reap = Box::new(move || std::mem::take(&mut *drained.borrow_mut()));
    let handed = Rc::new(RefCell::new(Vec::new()));
    script.spawn = recording(&handed);
    // In the first sleep, the driver pid 1 started exits, and a stop ends
    // the sleep.
    let (stop, store) = (Rc::clone(&script.stop), fixture.store.path().to_path_buf());
    script.slept = Box::new(move |_| {
        let run = the_run(&Store::open(&store).unwrap());
        let (claim, exit) = (handed.borrow()[0], Ok(ExitStatus::from_raw(0)));
        let pid = 1;
        exits.borrow_mut().push(Exited {
            run,
            claim,
            pid,
            exit,
        });
        stop.set(true);
    });
    let (seen, run) = stopped(script, &mut fixture, &dispatcher, &file);
    let slept = Seen::Slept(Duration::from_secs(59));
    let reaped = reaped_running(&run, 1, 0);
    assert_eq!(seen, vec![passed(&[run.as_str()]), slept, reaped]);
}

#[test]
fn a_reaped_driver_whose_run_does_not_read_is_reported_unread() {
    let (mut fixture, file, dispatcher) = setup(1);
    let mut script = Script::new();
    let handed = Rc::new(RefCell::new(Vec::new()));
    script.spawn = recording(&handed);
    let mut once = true;
    // Its handle could not say how it exited, either.
    script.reap = Box::new(move || {
        let exit = Err(std::io::Error::from_raw_os_error(10));
        let (run, claim, pid) = ("gone".to_string(), handed.borrow()[0], 5);
        let exited = Exited {
            run,
            claim,
            pid,
            exit,
        };
        let exited = std::mem::take(&mut once).then_some(exited);
        exited.into_iter().collect()
    });
    script.ticks = 1;
    let (seen, run) = stopped(script, &mut fixture, &dispatcher, &file);
    let unread = Seen::Reaped {
        run: "gone".into(),
        pid: 5,
        code: None,
        reads: None,
    };
    assert_eq!(seen, vec![passed(&[run.as_str()]), unread]);
}

#[test]
fn an_unreadable_host_is_reported_and_the_next_tick_judges_again() {
    let (mut fixture, file, dispatcher) = setup(1);
    std::fs::write(&file, "{not json").unwrap();
    let mut script = Script::new();
    let declared = file.clone();
    script.slept = Box::new(move |sleep| {
        if sleep == 1 {
            std::fs::write(&declared, ceilings(3, 3, json!({})).to_string()).unwrap();
        }
    });
    let (ended, seen) = script.run(&mut fixture.store, &dispatcher, &file);
    assert_eq!(ended.unwrap(), Stopped);
    let run = the_run(&fixture.store);
    let unreadable = Seen::Passed {
        started: vec![],
        spawned: vec![],
        host: true,
    };
    let slept = || Seen::Slept(Duration::from_secs(59));
    assert_eq!(
        seen,
        vec![
            unreadable,
            slept(),
            passed(&[run.as_str()]),
            slept(),
            passed(&[]),
        ]
    );
}

#[test]
fn a_fence_out_ends_the_loop_with_its_ending() {
    let (mut fixture, file, dispatcher) = setup(0);
    let generation = dispatcher.generation;
    let path = fixture.store.path().to_path_buf();
    let mut script = Script::new();
    script.slept = Box::new(move |_| {
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(
                "INSERT INTO dispatch_holders (generation, host, pid, started_at) \
                 VALUES (99, NULL, 1, 'then')",
            )
            .unwrap();
    });
    let (ended, seen) = script.run(&mut fixture.store, &dispatcher, &file);
    let ended = ended.unwrap_err();
    assert!(
        matches!(ended.ending, Ending::Fenced(g) if g == generation),
        "{ended:?}"
    );
    let slept = Seen::Slept(Duration::from_secs(59));
    assert_eq!(seen, vec![passed(&[]), slept, passed(&[])]);
}

#[test]
fn a_tick_sleeps_its_interval_less_its_pass_and_a_slow_pass_not_at_all() {
    let (mut fixture, file, dispatcher) = setup(0);
    // Two ticks ten seconds apart, each pass taking `pace` seconds.
    let mut two_ticks = |pace| {
        let script = Script {
            every: Duration::from_secs(10),
            pace: Duration::from_secs(pace),
            ticks: 2,
            ..Script::new()
        };
        let (ended, seen) = script.run(&mut fixture.store, &dispatcher, &file);
        assert_eq!(ended.unwrap(), Stopped);
        seen
    };
    let slept = Seen::Slept(Duration::from_secs(7));
    assert_eq!(two_ticks(3), vec![passed(&[]), slept, passed(&[])]);
    assert_eq!(two_ticks(10), vec![passed(&[]), passed(&[])]);
}

#[test]
fn a_stop_asked_before_the_first_tick_runs_no_pass() {
    let (mut fixture, file, dispatcher) = setup(1);
    let script = Script::new();
    script.stop.set(true);
    let (ended, seen) = script.run(&mut fixture.store, &dispatcher, &file);
    assert_eq!(ended.unwrap(), Stopped);
    assert_eq!(seen, vec![]);
    assert_eq!(words(&fixture.store), ["add"]);
}
