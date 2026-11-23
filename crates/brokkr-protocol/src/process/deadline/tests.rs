//! An absolute deadline is met at its instant (fix rounds 0106b1r6, D1,
//! and 0106b1r9, E1), and one whose monotonic instant has passed is never
//! spawned (the return of fix round 0106b1r12), including one that passes
//! while the caller's clock is read; a watchdog paused past its target
//! kills at once, and one paused short of it waits exactly what remains
//! (fix round 0106b1r13, M1).

use std::cell::Cell;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use super::{Arm, By, Deadline};
use crate::process::tests::command;
use crate::process::{DriverProcess, SpawnEnv, SpawnError};

/// A driver whose monotonic instant had already passed at the fork is not
/// spawned, though the caller's clock still finds thirty seconds left: the
/// fork reads the instant its kill is carried on as well as the caller's
/// clock, and refuses once either has passed.
#[test]
fn a_driver_whose_instant_passed_before_the_fork_is_never_spawned() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("ran");
    let left = || Some(Duration::from_secs(30));
    let by = By {
        kill: Instant::now(),
        left: &left,
    };
    let refused = DriverProcess::spawn_by(
        &command(&format!("touch {}", marker.display())),
        dir.path(),
        by,
        &SpawnEnv::Inherit,
    )
    .err()
    .expect("a driver past its instant was spawned");
    assert!(matches!(refused, SpawnError::Late { .. }), "{refused:?}");
    std::thread::sleep(Duration::from_millis(200));
    assert!(!marker.exists(), "the driver ran");
}

/// The monotonic instant passes while the fork reads the caller's clock,
/// which still finds thirty seconds left: the callback returns only once
/// the instant is behind it, so the fork's monotonic read, taken after the
/// callback, finds it passed and arms nothing.
#[test]
fn an_instant_that_passes_while_the_callers_clock_is_read_arms_nothing() {
    let kill = Instant::now() + Duration::from_millis(50);
    let left = || {
        while Instant::now() < kill {
            std::thread::sleep(Duration::from_millis(5));
        }
        Some(Duration::from_secs(30))
    };
    let by = By { kill, left: &left };
    assert_eq!(Arm::By(by).at_fork(), None);
}

/// A watchdog for a target one second after `base` on an injected
/// monotonic clock, paused to `paused` after it arms, whose every wait on
/// its disarm channel times out and moves the clock by the wait: whether
/// it killed, and each wait it asked for.
fn watched_through_a_pause(paused: Duration) -> (bool, Vec<Duration>) {
    let base = Instant::now();
    let deadline = Deadline::At {
        kill: base + Duration::from_secs(1),
        armed: Duration::from_secs(1),
    };
    let clock = Cell::new(Duration::ZERO);
    let mut waits = Vec::new();
    let killed = deadline.waited_with(
        || clock.set(paused),
        || base + clock.get(),
        |wait| {
            waits.push(wait);
            clock.set(clock.get() + wait);
            Err(RecvTimeoutError::Timeout)
        },
    );
    (killed, waits)
}

/// A pause past the target kills at once: one look at the disarm channel
/// that waits nothing, never a wait sampled before the pause.
#[test]
fn a_watchdog_paused_past_its_target_kills_at_once() {
    let (killed, waits) = watched_through_a_pause(Duration::from_secs(2));
    assert_eq!((killed, waits), (true, vec![Duration::ZERO]));
}

/// A pause short of the target waits exactly what remains, in slices of at
/// most a tenth of a second, and kills at the target.
#[test]
fn a_watchdog_paused_short_of_its_target_waits_exactly_what_remains() {
    let ms = Duration::from_millis;
    let (killed, waits) = watched_through_a_pause(ms(950));
    assert_eq!((killed, waits), (true, vec![ms(50)]));
    let (killed, waits) = watched_through_a_pause(ms(750));
    assert_eq!((killed, waits), (true, vec![ms(100), ms(100), ms(50)]));
}

/// A disarm the channel holds when it is looked at ends the watch without
/// a kill, even once the target has passed.
#[test]
fn a_disarmed_watchdog_never_kills() {
    let base = Instant::now();
    let deadline = Deadline::At {
        kill: base,
        armed: Duration::ZERO,
    };
    let disarmed = deadline.waited_with(|| {}, || base, |_| Ok(()));
    assert!(!disarmed);
}
