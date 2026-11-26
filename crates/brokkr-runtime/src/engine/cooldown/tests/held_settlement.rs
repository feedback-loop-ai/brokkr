//! Fix round 0106b1r10, H1 to H3: a probe's hold ends on evidence, never
//! because a clock passed. Its stored end is a readout (fix round
//! 0106b1r12, S1), so the route stays held through the kill, the grace and
//! the reap, until the engine's settlement proves the probe's tree ended.
//!
//! Each case moves the engine's clock, which the hold and both production
//! gates read, past where round nine's hold ended (the first whole second
//! after the deadline), while something of the probe's tree still runs,
//! and reads the real pass and the real claim there: through a driver that
//! answers and lingers through its grace, a kill target already past at
//! the fork, a forward step of the wall clock, and a descendant outside
//! the driver's group. A watchdog paused past its target is the protocol's
//! own exact test (fix round 0106b1r13, M1). Each reads
//! the route cooling down to the stored end while the tree lives, and open
//! once the settlement has ended the hold.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::held_deadline::SAMPLE;
use super::held_route::{
    deadline_of, held_to, hold_end_of, legs, probing, slicing, unended_holds, Legs, Shared,
};
use super::*;

/// Whether process `pid` is still unreaped on this host: a zombie is, for
/// its group and its pid are still its own.
fn alive(pid: i32) -> bool {
    rustix::process::Pid::from_raw(pid)
        .is_some_and(|pid| rustix::process::test_kill_process(pid).is_ok())
}

/// Wait for `path` to exist.
fn written(path: &Path) {
    let begun = Instant::now();
    while !path.exists() {
        assert!(
            begun.elapsed().as_secs() < 30,
            "{} never appeared",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Whether process `pid` runs: on the process table and not a zombie,
/// which is gone but for its reap, an orphan's by the engine's registry.
/// Only the Linux-only descendant case reads it.
#[cfg(target_os = "linux")]
fn running(pid: i32) -> bool {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
    stat.rsplit_once(')')
        .is_some_and(|(_, rest)| !rest.trim_start().starts_with('Z'))
}

/// What a watcher read of both gates while a process of the probe's tree
/// lived, and whether a read after it admitted, taking the queued entry.
struct Watched {
    seen: Vec<Legs>,
    admitted: bool,
}

/// Once `marker` exists, move the engine's clock to `at`, then read both
/// gates at `at` for as long as the process whose pid `pid_file` holds
/// `lives`: only a read the process outlived is kept, so each one kept was
/// taken while it could still run. The first read that admits ends the
/// look, for its claim took the entry.
fn read_while(
    (marker, pid_file): (PathBuf, PathBuf),
    lives: fn(i32) -> bool,
    now: &Arc<Mutex<OffsetDateTime>>,
    (at, gates): (OffsetDateTime, &Shared),
) -> JoinHandle<Watched> {
    let (now, gates) = (Arc::clone(now), Arc::clone(gates));
    std::thread::spawn(move || {
        written(&marker);
        let pid = pid_in(&pid_file);
        *now.lock().unwrap() = at;
        let mut watched = Watched {
            seen: Vec::new(),
            admitted: false,
        };
        while lives(pid) && !watched.admitted {
            let read = legs(&gates, at);
            watched.admitted = read.iter().any(Result::is_ok);
            if lives(pid) {
                watched.seen.push(read);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        watched
    })
}

/// [`read_while`] the process is unreaped, a zombie included.
fn read_while_alive(
    files: (PathBuf, PathBuf),
    now: &Arc<Mutex<OffsetDateTime>>,
    at: OffsetDateTime,
    gates: &Shared,
) -> JoinHandle<Watched> {
    read_while(files, alive, now, (at, gates))
}

/// The pid the driver wrote at `path`, once it has written it whole.
fn pid_in(path: &Path) -> i32 {
    let begun = Instant::now();
    loop {
        let read = std::fs::read_to_string(path).unwrap_or_default();
        if let Ok(pid) = read.trim().parse() {
            return pid;
        }
        assert!(
            begun.elapsed().as_secs() < 30,
            "{} held no pid",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// The reads kept while the tree lived: at least one, and every one holding
/// the route cooling down to the hold's stored end.
fn held_while_it_lived(seen: &[Legs], probe_at: OffsetDateTime) {
    assert!(!seen.is_empty(), "no read was taken while the tree lived");
    let end = hold_end_of(probe_at);
    assert!(seen.iter().all(|legs| held_to(legs, end)), "{seen:?}");
}

/// The settlement ended the hold: none stands, and both gates admit at
/// `at`, unless the watcher's last read, once the tree was gone, already
/// found them admitting and took the entry.
fn ended_by_settlement(engine: &Engine, gates: &Shared, at: OffsetDateTime, admitted: bool) {
    assert_eq!(unended_holds(engine.store.path()), 0);
    if !admitted {
        assert_eq!(legs(gates, at), [Ok(()), Ok(())]);
    }
}

/// What the watcher kept held while the tree lived, then the settlement's
/// end of the hold.
fn held_then_ended(
    engine: &Engine,
    (gates, at): (&Shared, OffsetDateTime),
    watched: Watched,
    probe_at: OffsetDateTime,
) {
    held_while_it_lived(&watched.seen, probe_at);
    ended_by_settlement(engine, gates, at, watched.admitted);
}

/// The shutdown: the probe answers before its deadline and keeps running
/// through the grace its shutdown gives it, until its tree is killed and
/// reaped. With the clock three seconds past the deadline, inside that
/// grace, both gates read the route cooling down while the driver lives;
/// the hold ends once the settlement reaps it, and the effect is retried.
#[test]
fn a_probe_that_answers_and_lingers_through_its_grace_holds_its_route_until_it_is_reaped() {
    own_engine!();
    let (dir, mut engine, now, probe_at, gates) = probing(Probed::AnswerThenLinger);
    let graced = deadline_of(probe_at) + time::Duration::seconds(3);
    let files = (
        dir.path().join("work.answered"),
        dir.path().join("work.pid"),
    );
    let watch = read_while_alive(files, &now, graced, &gates);
    engine.drive().unwrap();
    held_then_ended(&engine, (&gates, graced), watch.join().unwrap(), probe_at);
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
}

/// The arm gap: a pause of 2.2 real seconds at the probe's own sample, the
/// engine's clock unmoved (not at the fork's read, which runs under the
/// process-wide registry lock every other attempt needs), so the fork
/// finds the deadline ahead on the engine's clock while the watchdog's kill
/// target, two monotonic seconds after the probe began, has already
/// passed. The fork reads the monotonic target too (the return of fix round
/// 0106b1r12), so the driver is never spawned: the probe parks late, no
/// process of it ran, and its empty tree ends its hold.
///
/// That the kill target passed before the fork is asserted, not assumed
/// (fix round 0106b1r12, S5): the target is two monotonic seconds after
/// the probe began, which is before the sample's read begins, and the fork
/// follows the sample's read; so a pause at that read longer than the
/// seat's two-second timeout puts the target behind the fork.
#[test]
fn a_probe_whose_kill_target_passed_before_the_fork_is_never_spawned() {
    own_engine!();
    let (dir, mut engine, now, probe_at, gates) = probing(Probed::Hang);
    let (journal, reads) = (engine.store.path().to_path_buf(), AtomicUsize::new(0));
    let (shared, stalled) = (Arc::clone(&now), Arc::new(Mutex::new(None)));
    let stall = Arc::clone(&stalled);
    engine.cooldown.clock = Clock {
        now: Box::new(move || {
            let held = unended_holds(&journal) == 1;
            if held && reads.fetch_add(1, Ordering::SeqCst) + 1 == SAMPLE {
                let begun = Instant::now();
                std::thread::sleep(Duration::from_millis(2_200));
                *stall.lock().unwrap() = Some(begun.elapsed());
            }
            *shared.lock().unwrap()
        }),
        sleep: slicing(&now),
    };
    engine.drive().unwrap();
    let stalled = stalled
        .lock()
        .unwrap()
        .expect("the sample's read was never taken");
    assert!(
        stalled > Duration::from_secs(2),
        "the kill target had not passed by the fork: the sample's read stalled {stalled:?}"
    );
    let reason = park_reason(&engine);
    assert!(
        reason.contains("was not spawned: its deadline")
            && reason.contains("passed before its driver could start"),
        "{reason}"
    );
    assert!(!dir.path().join("work.pid").exists(), "the driver ran");
    let (at, admitted) = (deadline_of(probe_at), false);
    ended_by_settlement(&engine, &gates, at, admitted);
}

/// A hanging probe whose engine clock moves `step` past the instant it was
/// dispatched while its driver runs: both gates read the route cooling
/// down at that instant while the driver lives, the watchdog kills it at
/// its own deadline, and the settlement then ends the hold.
fn a_probe_whose_clock_moves_in_flight_by(step: time::Duration) {
    let (dir, mut engine, now, probe_at, gates) = probing(Probed::Hang);
    let at = probe_at + step;
    let files = (dir.path().join("work.pid"), dir.path().join("work.pid"));
    let watch = read_while_alive(files, &now, at, &gates);
    engine.drive().unwrap();
    held_then_ended(&engine, (&gates, at), watch.join().unwrap(), probe_at);
    let reason = park_reason(&engine);
    assert!(
        reason.contains("attempt exceeded its 2s deadline and was killed"),
        "{reason}"
    );
}

/// A forward step of the engine's wall clock while the probe runs, thirty
/// seconds, against the monotonic clock its watchdog waits on.
#[test]
fn a_forward_wall_clock_step_while_the_probe_runs_finds_the_route_still_held() {
    own_engine!();
    a_probe_whose_clock_moves_in_flight_by(time::Duration::seconds(30));
}

/// Fix rounds 0106b1r11, L1, and 0106b1r12, S1: a forward step of the
/// engine's wall clock past the hold's stored end itself while the probe
/// runs. No gate reads the stored end, so both read the route cooling
/// down while the driver lives; the settlement then ends the hold.
#[test]
fn a_wall_clock_step_past_the_stored_end_while_the_probe_runs_finds_the_route_still_held() {
    own_engine!();
    let epoch = OffsetDateTime::UNIX_EPOCH;
    a_probe_whose_clock_moves_in_flight_by(
        hold_end_of(epoch) - epoch + time::Duration::seconds(10),
    );
}

/// A descendant outside the driver's group (`setsid`), as the registry and
/// the process table find it: the probe answers and lingers, and with the
/// clock inside the grace both gates read the route cooling down while the
/// descendant lives. The settlement ends it with the tree, and only then
/// the hold.
#[cfg(target_os = "linux")]
#[test]
fn a_descendant_outside_the_group_keeps_the_route_held_until_settlement_ends_it() {
    own_engine!();
    let (dir, mut engine, now, probe_at, gates) = probing(Probed::DescendantThenLinger);
    let graced = deadline_of(probe_at) + time::Duration::seconds(3);
    let descendant = dir.path().join("work.descendant");
    let files = (dir.path().join("work.answered"), descendant.clone());
    let watch = read_while(files, running, &now, (graced, &gates));
    engine.drive().unwrap();
    held_then_ended(&engine, (&gates, graced), watch.join().unwrap(), probe_at);
    let pid = pid_in(&descendant);
    assert!(
        !running(pid),
        "the settlement left the descendant {pid} running"
    );
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
}
