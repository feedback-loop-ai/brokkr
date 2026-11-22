//! Fix rounds 0106b1r5, H2, 0106b1r6, D1, and 0106b1r9, E1: a probe has one
//! deadline. It is taken once, as the probe begins, as an instant on the
//! engine's clock; that instant, never a length sampled from it, travels
//! to the fork, which reads it again under the registry's lock as the last
//! step before the spawn, and to the watchdog, which kills at it. The
//! probe's hold ends at the first whole second strictly after it, so the
//! kill is always earlier than the hold's end. A probe past its deadline is
//! not spawned, and its hold stands.
//!
//! The pause is scripted on the engine's clock: one read after the hold is
//! written moves the clock on, as store contention, slow dispatch work or
//! a registry lock another attempt holds would, and no test sleeps for it.
//! The first such read is the probe's own sample as its spawn begins; the
//! second is the fork's, taken under the registry's lock after the process
//! table's read, immediately before the spawn.

use std::path::PathBuf;

use super::held_route::{deadline_of, held_to_its_deadline, legs, probing, slicing, unended_holds};
use super::held_wait::standing;
use super::*;

/// The probe's sample as its spawn begins, and the fork's check after the
/// door, record, charter and settlement work, under the registry's lock.
const SAMPLE: usize = 1;
const FORK: usize = 2;

/// A clock over `now` whose `read`th read once the journal at `journal`
/// holds a standing probe hold moves `now` on by `pause` first; its slices
/// advance `now`.
fn pausing_at(
    now: &Arc<Mutex<OffsetDateTime>>,
    journal: PathBuf,
    (read, pause): (usize, time::Duration),
) -> Clock {
    let shared = Arc::clone(now);
    let reads = std::sync::atomic::AtomicUsize::new(0);
    Clock {
        now: Box::new(move || {
            let mut at = shared.lock().unwrap();
            let ordering = std::sync::atomic::Ordering::SeqCst;
            if reads.load(ordering) < read
                && unended_holds(&journal) == 1
                && reads.fetch_add(1, ordering) + 1 == read
            {
                *at += pause;
            }
            *at
        }),
        sleep: slicing(now),
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

/// The adversarial position's pause, 3.3 seconds against the seat's
/// two-second timeout, at clock read `read`: the probe is not spawned past
/// its deadline, the run parks naming it, and the real pass and claim read
/// the route cooling down through its hold's end.
fn a_probe_paused_past_its_deadline_at(read: usize) {
    let (dir, mut engine, now, probe_at, gates) = probing(Probed::Answer);
    let pause = (read, time::Duration::milliseconds(3_300));
    engine.cooldown.clock = pausing_at(&now, engine.store.path().into(), pause);
    engine.drive().unwrap();
    assert_eq!(standing(&engine), (Status::AwaitingOperator, 1));
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
    let deadline = deadline_of(probe_at);
    assert_eq!(
        parked_reason(&engine),
        format!(
            "the cool-down probe for claude/first was not spawned: its deadline {} passed \
             before its driver could start, and its route's hold stands until the first whole \
             second after it",
            crate::admission::spelled(deadline)
        )
    );
    held_to_its_deadline(&engine, &gates, deadline);
}

/// Paused before its spawn begins, the probe composes no spawn.
#[test]
fn a_probe_whose_deadline_passes_before_its_spawn_is_not_spawned_and_holds_to_it() {
    a_probe_paused_past_its_deadline_at(SAMPLE);
}

/// Fix round 0106b1r9, E1: paused after the probe's own sample, at the
/// fork itself, between the registry's lock and the spawn, as dispatch
/// work or another attempt's hold on that lock would pause it: the fork
/// finds the deadline passed and spawns nothing.
#[test]
fn a_probe_whose_deadline_passes_under_the_registry_lock_is_not_spawned_and_holds_to_it() {
    a_probe_paused_past_its_deadline_at(FORK);
}

/// A pause after the probe's sample, one second of the seat's two: the
/// watchdog kills the hanging probe at the probe's own deadline, and its
/// kill's words name the one second that remained at the fork, not the two
/// seconds the sample saw. Its hold stands through that kill and ends once
/// the engine's settlement proves the killed tree ended, never at a clock
/// (fix round 0106b1r10, H1): the route then opens at the very instant the
/// probe ran.
#[test]
fn a_probe_paused_after_its_sample_is_killed_at_its_deadline() {
    let (dir, mut engine, now, probe_at, gates) = probing(Probed::Hang);
    let pause = (FORK, time::Duration::seconds(1));
    engine.cooldown.clock = pausing_at(&now, engine.store.path().into(), pause);
    engine.drive().unwrap();
    assert_eq!(standing(&engine), (Status::AwaitingOperator, 1));
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse", "probe"]);
    let reason = parked_reason(&engine);
    assert!(
        reason.contains("attempt exceeded its 1s deadline and was killed"),
        "{reason}"
    );
    assert_eq!(unended_holds(engine.store.path()), 0);
    assert_eq!(legs(&gates, probe_at), [Ok(()), Ok(())]);
}
