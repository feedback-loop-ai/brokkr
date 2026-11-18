//! Fix round 0106b1r5, H2: a probe has one deadline. It is taken once, as
//! the probe begins; the probe's hold ends there, and its driver is armed
//! with what remains of it at spawn, never a fresh timeout, so the driver
//! is killed no later than its hold ends. A probe with nothing left is not
//! spawned, and its hold stands until that deadline.
//!
//! The pause before spawn is scripted on the engine's clock: its first
//! read after the hold is written moves the clock on, as store contention
//! or slow pre-spawn work would, and no test sleeps for it.

use std::path::PathBuf;

use super::held_route::{deadline_of, held_to_its_deadline, probing, unended_holds};
use super::held_wait::standing;
use super::*;

/// A clock over `now` whose first read after the journal at `journal`
/// holds a standing probe hold moves `now` on by `pause` first; its slices
/// advance `now`.
fn pausing_after_the_hold(
    now: &Arc<Mutex<OffsetDateTime>>,
    journal: PathBuf,
    pause: time::Duration,
) -> Clock {
    let (read, advance) = (Arc::clone(now), Arc::clone(now));
    let paused = std::sync::atomic::AtomicBool::new(false);
    Clock {
        now: Box::new(move || {
            let mut at = read.lock().unwrap();
            let ordering = std::sync::atomic::Ordering::SeqCst;
            if !paused.load(ordering) && unended_holds(&journal) == 1 {
                paused.store(true, ordering);
                *at += pause;
            }
            *at
        }),
        sleep: Box::new(move |slice: std::time::Duration| {
            *advance.lock().unwrap() += time::Duration::seconds(slice.as_secs() as i64);
        }),
    }
}

/// The reason the run's one park gives.
fn parked_reason(engine: &Engine) -> String {
    let events = engine.store.load(&engine.run_id).unwrap();
    let parked = (events.iter())
        .find(|event| event.event_type == EventType::RunParked)
        .expect("the run parked");
    parked.payload["reason"].as_str().unwrap().to_string()
}

/// The adversarial position's pause: 3.3 seconds before spawn, against the
/// seat's two-second timeout. The probe is not spawned past its deadline;
/// the run parks naming it, and the real pass and claim read the route
/// cooling down until that deadline, and open at it.
#[test]
fn a_probe_whose_deadline_passes_before_its_spawn_is_not_spawned_and_holds_to_it() {
    let (dir, mut engine, now, probe_at, gates) = probing(Probed::Answer);
    let pause = time::Duration::milliseconds(3_300);
    engine.cooldown.clock = pausing_after_the_hold(&now, engine.store.path().into(), pause);
    engine.drive().unwrap();
    assert_eq!(standing(&engine), (Status::AwaitingOperator, 1));
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
    let deadline = deadline_of(probe_at);
    assert_eq!(
        parked_reason(&engine),
        format!(
            "the cool-down probe for claude/first was not spawned: its deadline {} passed \
             before its driver could start, and its route's hold stands until then",
            crate::admission::spelled(deadline)
        )
    );
    held_to_its_deadline(&engine, &gates, deadline);
}

/// A pause short of the deadline, one second of the seat's two: the driver
/// is armed with the one second that remains, as its watchdog's kill of
/// the hanging probe names, not with a fresh two-second timeout.
#[test]
fn a_probe_paused_before_its_spawn_is_armed_with_the_time_that_remains() {
    let (dir, mut engine, now, probe_at, gates) = probing(Probed::Hang);
    let pause = time::Duration::seconds(1);
    engine.cooldown.clock = pausing_after_the_hold(&now, engine.store.path().into(), pause);
    engine.drive().unwrap();
    assert_eq!(standing(&engine), (Status::AwaitingOperator, 1));
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse", "probe"]);
    let reason = parked_reason(&engine);
    assert!(
        reason.contains("attempt exceeded its 1s deadline and was killed"),
        "{reason}"
    );
    held_to_its_deadline(&engine, &gates, deadline_of(probe_at));
}
