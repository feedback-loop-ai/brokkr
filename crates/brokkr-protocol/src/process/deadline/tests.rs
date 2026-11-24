//! An absolute deadline is met at its instant (fix rounds 0106b1r6, D1,
//! and 0106b1r9, E1), and one whose monotonic instant has passed is never
//! spawned (the return of fix round 0106b1r12), including one that passes
//! while the caller's clock is read; a watchdog suspended between a wait's
//! sample and that wait kills within one 10 ms slice of its resuming, past
//! its target or short of it, for a length and an absolute deadline alike
//! (fix rounds 0106b1r13, M1, and 0106b1r15, M2).

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
/// which still finds thirty seconds left: on an injected monotonic clock,
/// the callback moves it past the instant, so the fork's monotonic read,
/// taken after the callback, finds it passed and arms nothing; the same
/// callback that leaves the clock short of it arms the instant. Removal
/// control: read the monotonic clock before the callback, and the first
/// fork arms. No wall time is waited (fix round 0106b1r14, M2).
#[test]
fn an_instant_that_passes_while_the_callers_clock_is_read_arms_nothing() {
    let base = Instant::now();
    let kill = base + Duration::from_secs(1);
    let short = Some(Deadline::At {
        kill,
        armed: Duration::from_secs(30),
    });
    for (read_takes, armed) in [(2000, None), (999, short)] {
        let clock = Cell::new(Duration::ZERO);
        let left = || {
            clock.set(Duration::from_millis(read_takes));
            Some(Duration::from_secs(30))
        };
        let by = By { kill, left: &left };
        assert_eq!(Arm::By(by).at_fork_on(|| base + clock.get()), armed);
    }
}

/// A watchdog for `deadline`, armed at `base` on an injected monotonic
/// clock, suspended for `paused` once, after its `pausing`th wait's sample
/// and before that wait, whose every wait on its disarm channel times out
/// and moves the clock by the wait: whether it killed, each wait it asked
/// for, and the clock at its end.
fn watched_through_a_pause(
    deadline: Deadline,
    base: Instant,
    (pausing, paused): (usize, Duration),
) -> (bool, Vec<Duration>, Duration) {
    let clock = Cell::new(Duration::ZERO);
    let mut waits = Vec::new();
    let mut pauses = 0;
    let killed = deadline.waited_with(
        || {
            pauses += 1;
            if pauses == pausing {
                clock.set(clock.get() + paused);
            }
        },
        || base + clock.get(),
        |wait| {
            waits.push(wait);
            clock.set(clock.get() + wait);
            Err(RecvTimeoutError::Timeout)
        },
    );
    (killed, waits, clock.get())
}

/// A target one second after `base`.
fn at_one_second(base: Instant) -> Deadline {
    Deadline::At {
        kill: base + Duration::from_secs(1),
        armed: Duration::from_secs(1),
    }
}

/// A thread suspended after a wait's sample and before that wait, until the
/// clock is past the target, waits out only the one slice it sampled once
/// it resumes, and kills within one 10 ms slice of the pause's end: for an
/// absolute deadline on its first or its third wait, and for a length.
/// Removal control: hand the wait the whole remainder, as before, and the
/// resumed thread waits that remainder out past the target, the kill
/// landing a whole second after the pause's end.
#[test]
fn a_watchdog_suspended_after_its_sample_past_its_target_kills_within_one_slice() {
    let base = Instant::now();
    let ms = Duration::from_millis;
    let first = watched_through_a_pause(at_one_second(base), base, (1, ms(2000)));
    assert_eq!(first, (true, vec![ms(10)], ms(2010)));
    let third = watched_through_a_pause(at_one_second(base), base, (3, ms(2000)));
    assert_eq!(third, (true, vec![ms(10), ms(10), ms(10)], ms(2030)));
    let length = Deadline::After(Duration::from_secs(1));
    let after = watched_through_a_pause(length, base, (1, ms(2000)));
    assert_eq!(after, (true, vec![ms(10)], ms(2010)));
}

/// A thread suspended short of the target, or a watchdog never suspended,
/// waits in slices of at most 10 ms, reading the clock on every wake, and
/// kills at the first wake at or past the target: exactly at it, or within
/// one slice of it where a suspension leaves less than a slice.
#[test]
fn a_watchdog_suspended_short_of_its_target_kills_within_one_slice_of_it() {
    let base = Instant::now();
    let ms = Duration::from_millis;
    let paused = watched_through_a_pause(at_one_second(base), base, (1, ms(950)));
    assert_eq!(paused, (true, vec![ms(10); 5], ms(1000)));
    let paused = watched_through_a_pause(at_one_second(base), base, (1, ms(995)));
    assert_eq!(paused, (true, vec![ms(10)], ms(1005)));
    let length = Deadline::After(Duration::from_secs(1));
    let never = watched_through_a_pause(length, base, (0, Duration::ZERO));
    assert_eq!(never, (true, vec![ms(10); 100], ms(1000)));
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
