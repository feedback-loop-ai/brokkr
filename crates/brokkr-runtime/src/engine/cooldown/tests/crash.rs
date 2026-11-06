//! A crashed cool-down probe is found by its reservation (decision 0103
//! ruling 4, fix round 430s4c6): a run's own standing attempt that no
//! journal event names is a probe's, released only where every process it
//! witnessed is proven settled, and otherwise parked with the cleanup
//! refusal before any second probe, retry or driver invocation.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration as StdDuration, Instant};

use serde_json::Value;
use time::OffsetDateTime;

use super::*;
use crate::engine::broker::admission::tests::{protecting_refused, reservation_path, Coordinated};
use crate::engine::tests::in_its_own_engine;

/// The first engine's probe answers and its cleanup is witnessed, then the
/// test's clock panics at the next `now`, which is `probe_end`'s answer
/// time: the crash lands after `settlement::ended` sealed the driver
/// Settled and before `settle_probe` released the attempt. The crashed
/// reservation's one standing attempt is returned.
/// A run whose failed attempt recorded an expired cool-down: the journal a
/// resume finds, with the bundle it was compiled under.
fn waited_run(dir: &Path, compiled: crate::bundle::Bundle) -> (String, crate::bundle::Bundle) {
    let body = compiled.seats["work"].body.clone();
    let engine = journaled(
        dir,
        body,
        wait_value(base() + time::Duration::seconds(1), 1),
    );
    let run = engine.run_id.clone();
    drop(engine);
    (run, compiled)
}

/// The run resumed under `compiled` and driven with a clock at `at`.
fn driven(dir: &Path, compiled: crate::bundle::Bundle, run: &str, at: OffsetDateTime) -> Engine {
    let mut engine = resumed_run(dir, compiled, run);
    engine.cooldown.clock = Clock::advancing(at).0;
    engine.drive().unwrap();
    engine
}

/// The run resumed under `compiled`, its cool-down host file set; the
/// caller sets its clock.
fn resumed_run(dir: &Path, compiled: crate::bundle::Bundle, run: &str) -> Engine {
    let mut engine = Engine::resume(
        brokkr_store::Store::open(&dir.join("forge.db")).unwrap(),
        compiled,
        run,
        Some(dir.join("work")),
    )
    .unwrap();
    engine.cooldown.host = Some(host(dir, 1, 10));
    engine
}

fn crashed_after_probe_cleanup(dir: &Path, home: &Path) -> (crate::bundle::Bundle, String, String) {
    let command = driver(
        dir,
        "work",
        &["succeed"],
        Probed::Answer,
        None,
        "claude",
        "first",
    );
    let mut compiled = compiled(dir, single_body(command));
    compiled.seats.get_mut("work").unwrap().limits = work(2);
    let (run, compiled) = waited_run(dir, compiled);
    let mut crashed = resumed_run(dir, compiled.clone(), &run);
    let done = dir.join("work.done");
    let now = Arc::new(Mutex::new(base()));
    crashed.cooldown.clock = crashing_clock(&done, &now);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crashed.drive()));
    assert!(
        panic.is_err(),
        "the clock did not crash after the probe's cleanup"
    );
    drop(crashed);
    let at = reservation_path(home, &dir.join("work"), &run);
    let sealed: Value = serde_json::from_slice(&std::fs::read(&at).unwrap()).unwrap();
    let first = sealed["attempts"]
        .as_object()
        .expect("the crashed probe's attempt stands")
        .keys()
        .next()
        .expect("the crashed probe's attempt")
        .clone();
    assert_eq!(
        sealed["attempts"][&first]["cleanup"], "settling",
        "the crash landed before the attempt's own settlement: {sealed}"
    );
    (compiled, run, first)
}

/// The first engine's probe answers, writes its done marker, and the test's
/// clock panics at the next `now`, which is `probe_end`'s answer time: the
/// crash lands after `settlement::ended` sealed the driver Settled and
/// before `settle_probe` released the attempt.
#[test]
fn a_crash_after_a_probes_cleanup_leaves_a_proven_reservation_the_resume_releases() {
    let dir = tempfile::tempdir().unwrap();
    let home = super::lease::home(dir.path());
    let _stand = Coordinated::at(&home);
    let (compiled, run, first) = crashed_after_probe_cleanup(dir.path(), &home);
    let at = reservation_path(&home, &dir.path().join("work"), &run);

    let resumed = driven(
        dir.path(),
        compiled,
        &run,
        base() + time::Duration::seconds(1),
    );
    assert_eq!(
        logs(dir.path(), "work"),
        ["probe", "probe", "attempt:succeed"],
        "the proven reservation was not released and probed again"
    );
    let started = starts(&resumed);
    assert_eq!(
        started.len(),
        2,
        "the effect spent one attempt: {started:?}"
    );
    let attempts = failed(&resumed);
    let effect = attempts
        .iter()
        .filter(|event| event.payload["effect_id"] == "effect")
        .count();
    assert_eq!(effect, 1, "the probe spent no effect attempt: {attempts:?}");
    let after: Value = serde_json::from_slice(&std::fs::read(&at).unwrap()).unwrap();
    assert_eq!(
        after["attempts"]
            .as_object()
            .map_or(0, serde_json::Map::len),
        0,
        "a probe's reservation stands: {after}"
    );
    let settled: Vec<&str> = after["settled"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(settled.contains(&first.as_str()), "{after}");
    assert!(
        protecting_refused(&home, &dir.path().join("work")).is_ok(),
        "a later admission is refused"
    );
}

/// A clock that reads `now` and panics once the probe's done marker exists:
/// the one instant between `settlement::ended` and `settle_probe`.
fn crashing_clock(done: &Path, now: &Arc<Mutex<OffsetDateTime>>) -> Clock {
    let read = Arc::clone(now);
    let advance = Arc::clone(now);
    let done = done.to_path_buf();
    Clock {
        now: Box::new(move || {
            if done.exists() {
                panic!("crash after the probe's cleanup and before its settlement");
            }
            *read.lock().unwrap()
        }),
        sleep: Box::new(move |slice| {
            *advance.lock().unwrap() += time::Duration::seconds(slice.as_secs() as i64)
        }),
    }
}

/// The shared directory and run a live-probe crash test leaves behind for
/// its engine child.
const LIVE_DIR: &str = "BROKKR_430_LIVE_DIR";
const LIVE_RUN: &str = "BROKKR_430_LIVE_RUN";

/// The live fixture both the test and its engine child rebuild: a run whose
/// failed attempt recorded an expired cool-down, and the hanging probe the
/// engine child blocks in.
fn live_fixture(dir: &Path) -> (String, crate::bundle::Bundle) {
    let compiled = live_bundle(dir);
    waited_run(dir, compiled)
}

/// The bundle both sides rebuild, whose hanging probe is the same command.
fn live_bundle(dir: &Path) -> crate::bundle::Bundle {
    let command = driver(
        dir,
        "work",
        &["succeed"],
        Probed::HangThenAnswer,
        None,
        "claude",
        "first",
    );
    let mut compiled = compiled(dir, single_body(command));
    compiled.seats.get_mut("work").unwrap().limits = work(2);
    compiled
}

/// The engine child: resume the shared run and block in the probe, which
/// its parent kills before it answers.
fn live_engine_child(dir: &Path, run: &str) -> ! {
    let home = dir.join("home");
    let _stand = Coordinated::at(&home);
    let compiled = live_bundle(dir);
    driven(dir, compiled, run, base());
    panic!("the engine was not killed in its probe");
}

/// F1, the chief's live-probe case under this test's own name: a crash that
/// leaves the probe's driver live. The resume finds the probe's reservation
/// by its unnamed attempt, cannot prove the driver settled, and parks with
/// the cleanup refusal: no second probe, no retry, no driver invocation.
#[test]
fn a_live_probe_crash_parks_the_resume_before_a_second_probe() {
    let test =
        "engine::cooldown::tests::crash::a_live_probe_crash_parks_the_resume_before_a_second_probe";
    if let (Some(dir), Some(run)) = (std::env::var_os(LIVE_DIR), std::env::var_os(LIVE_RUN)) {
        live_engine_child(Path::new(&dir), &run.to_string_lossy());
    }
    if !in_its_own_engine(test) {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let home = super::lease::home(dir.path());
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let _stand = Coordinated::at(&home);
    let (run, compiled) = live_fixture(dir.path());
    let log = dir.path().join("work.log");
    let pid = dir.path().join("work.pid");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test])
        .env(LIVE_DIR, dir.path())
        .env(LIVE_RUN, &run)
        .env_remove("BROKKR_TEST_OWN_ENGINE")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + StdDuration::from_secs(30);
    while !(log.exists() && pid.exists()) {
        assert!(
            child.try_wait().unwrap().is_none(),
            "the engine child ended before its probe started"
        );
        assert!(Instant::now() < deadline, "the probe never started");
        std::thread::sleep(StdDuration::from_millis(20));
    }
    child.kill().unwrap();
    child.wait().unwrap();
    let driver = std::fs::read_to_string(&pid)
        .unwrap()
        .trim()
        .parse::<i32>()
        .expect("the probe's pid");
    let driver = rustix::process::Pid::from_raw(driver).unwrap();
    assert!(
        rustix::process::test_kill_process(driver).is_ok(),
        "the crashed engine's probe was not left live"
    );

    let at = reservation_path(&home, &dir.path().join("work"), &run);
    let sealed: Value = serde_json::from_slice(&std::fs::read(&at).unwrap()).unwrap();
    let attempt = sealed["attempts"]
        .as_object()
        .expect("the crashed probe's attempt stands")
        .keys()
        .next()
        .expect("the crashed probe's attempt")
        .clone();

    let resumed = driven(
        dir.path(),
        compiled,
        &run,
        base() + time::Duration::seconds(1),
    );
    let reason = park_reason(&resumed);
    assert!(
        reason.contains("did not prove its process tree gone"),
        "the resume did not park with the cleanup refusal (log {:?}): {reason}",
        logs(dir.path(), "work")
    );
    assert!(reason.contains(&at.display().to_string()), "{reason}");
    assert!(reason.contains(&attempt), "{reason}");
    assert_eq!(
        logs(dir.path(), "work"),
        ["probe"],
        "the resume probed over the live probe's reservation"
    );
    assert_eq!(starts(&resumed).len(), 1, "the effect attempted again");
    assert_eq!(failed(&resumed).len(), 1, "an effect attempt was spent");
    kill_probe(driver);
}

/// End the live probe's whole group and reap it if this process adopted it.
fn kill_probe(driver: rustix::process::Pid) {
    rustix::process::kill_process_group(driver, rustix::process::Signal::KILL).ok();
    let until = Instant::now() + StdDuration::from_secs(5);
    while Instant::now() < until {
        match rustix::process::waitpid(Some(driver), rustix::process::WaitOptions::NOHANG) {
            Ok(Some(_)) | Err(_) => return,
            Ok(None) => std::thread::sleep(StdDuration::from_millis(10)),
        }
    }
}
