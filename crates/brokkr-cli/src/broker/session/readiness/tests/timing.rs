//! Deadline and backoff policy without scheduler-dependent elapsed times.
//! The production poller supplies the real clock and sleeper; these tests
//! advance one monotonic clock explicitly and record every requested wait.

use std::cell::Cell;
use std::io::{Error, ErrorKind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use super::{polled_with, timely_at, Pace, BRIEF, SLICE};

#[test]
fn only_idle_turns_wait_and_progress_resets_the_capped_backoff() {
    let mut pace = Pace::new();
    let mut waits = Vec::new();
    pace.after(true, |wait| waits.push(wait));
    assert!(waits.is_empty());
    for _ in 0..12 {
        pace.after(false, |wait| waits.push(wait));
    }
    let expected: Vec<_> = [
        50, 100, 200, 400, 800, 1600, 3200, 6400, 10000, 10000, 10000, 10000,
    ]
    .into_iter()
    .map(Duration::from_micros)
    .collect();
    assert_eq!(waits, expected);
    pace.after(true, |_| panic!("a progressing turn must not wait"));
    pace.after(false, |wait| assert_eq!(wait, BRIEF));
}

#[test]
fn the_poller_waits_only_for_would_block_and_resets_after_partial_progress() {
    let now = Cell::new(Instant::now());
    let startup: super::Startup<'_> = (now.get() + Duration::from_secs(1), &|| false);
    let mut waits = Vec::new();
    let mut steps = [
        Err(ErrorKind::WouldBlock),
        Err(ErrorKind::WouldBlock),
        Ok(false),
        Err(ErrorKind::WouldBlock),
        Ok(true),
    ]
    .into_iter();
    assert!(polled_with(
        startup,
        || now.get(),
        |wait| {
            waits.push(wait);
            now.set(now.get() + wait);
        },
        || steps
            .next()
            .expect("polled after completion")
            .map_err(Error::from),
    ));
    assert_eq!(waits, [BRIEF, BRIEF * 2, BRIEF]);

    assert!(!polled_with(
        startup,
        || now.get(),
        |_| panic!("wait after failure"),
        || Err(ErrorKind::PermissionDenied.into())
    ));
}

#[test]
fn startup_stages_share_one_budget_even_while_every_step_makes_progress() {
    let begun = Instant::now();
    let now = Cell::new(begun);
    let deadline = begun + Duration::from_secs(30);
    let startup: super::Startup<'_> = (deadline, &|| false);
    // Observation and readiness consume 20 seconds of the same budget.
    // Initialization gets only what remains, even when it keeps receiving
    // bytes: neither a new stage nor progress grants another 30 seconds.
    for seconds in [8, 12] {
        assert!(polled_with(
            startup,
            || now.get(),
            |_| panic!("wait on progress"),
            || {
                now.set(now.get() + Duration::from_secs(seconds));
                Ok(true)
            }
        ));
    }
    let mut steps = 0;
    assert!(!polled_with(
        startup,
        || now.get(),
        |_| panic!("wait on progress"),
        || {
            steps += 1;
            assert!(steps <= 2, "a stage extended the startup deadline");
            now.set(now.get() + Duration::from_secs(5));
            Ok(false)
        }
    ));
    assert_eq!((steps, now.get()), (2, deadline));
    assert!(!polled_with(
        startup,
        || now.get(),
        |_| panic!("wait after deadline"),
        || panic!("another stage started after the budget expired")
    ));
}

#[test]
fn an_idle_poller_expires_without_restarting_its_deadline() {
    let now = Cell::new(Instant::now());
    let deadline = now.get() + Duration::from_millis(40);
    let mut steps = 0;
    assert!(!polled_with(
        (deadline, &|| false),
        || now.get(),
        |wait| now.set(now.get() + wait),
        || {
            steps += 1;
            assert!(steps < 32, "an idle poller extended its deadline");
            Err(ErrorKind::WouldBlock.into())
        }
    ));
    assert!(now.get() >= deadline);
    assert!(now.get() < deadline + SLICE);
}

#[test]
fn the_deadline_is_exclusive_and_cancellation_stops_the_next_step() {
    let now = Instant::now();
    let cancelled = AtomicBool::new(false);
    let check = || cancelled.load(Ordering::SeqCst);
    assert!(timely_at((now, &check), now - Duration::from_nanos(1)));
    assert!(!timely_at((now, &check), now));
    assert!(!timely_at((now, &check), now + Duration::from_nanos(1)));
    let mut steps = 0;
    assert!(!polled_with(
        (now + Duration::from_secs(30), &check),
        || now,
        |_| cancelled.store(true, Ordering::SeqCst),
        || {
            steps += 1;
            assert_eq!(steps, 1, "a cancelled poller took another step");
            Err(ErrorKind::WouldBlock.into())
        }
    ));
    assert_eq!(steps, 1);
    assert!(!timely_at((now + Duration::from_secs(30), &check), now));
}
