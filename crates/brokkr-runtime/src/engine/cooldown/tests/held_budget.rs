//! Fix round 0106b1r14, M1: a wait on a standing probe hold is charged its
//! real time against the run's maximum. Each slice is judged from the one
//! clock sample its end was read at and kept to the fraction, so a hold
//! that never ends parks the run at its maximum on a clock that moves
//! between every read. An ordinary row beside the hold keeps its own end,
//! and a slice the clock cannot name refuses by type.

use brokkr_store::ProbeHold;
use time::Duration as TimeDuration;

use super::held_route::held_at_its_end;
use super::probe_rounds::{append_second_row_on, second_run};
use super::*;
use crate::engine::cooldown::CooldownError;

/// The fixture at its held row's end, nothing holding the route, with a
/// hold of an attempt no drive of it took standing on claude/first from
/// that instant: no answer, round or settlement ever ends it.
fn held_forever() -> (tempfile::TempDir, Engine, OffsetDateTime) {
    let (dir, mut engine, now) = held_at_its_end(Probed::Answer, |_, engine| {
        engine.drive().unwrap();
    });
    let at = *now.lock().unwrap();
    let run = engine.run_id.clone();
    let hold = ProbeHold {
        run_id: &run,
        attempt_id: "never-settled",
        site: "",
        provider: "claude",
        route: "first",
        deadline: at + TimeDuration::minutes(1),
    };
    engine.store.hold_probe(&hold, at).unwrap();
    (dir, engine, at)
}

/// A clock from `at` that moves a millisecond at every read and a whole
/// slice at every sleep, so no two reads ever agree; it counts the slices
/// slept and refuses to read past `reads`, so a wait that never parks ends.
fn moving(at: OffsetDateTime, reads: u32) -> (Clock, Arc<Mutex<u32>>) {
    let now = Arc::new(Mutex::new((at, 0_u32)));
    let slept = Arc::new(Mutex::new(0_u32));
    let (read, write, count) = (Arc::clone(&now), now, Arc::clone(&slept));
    let clock = Clock {
        now: Box::new(move || {
            let mut now = read.lock().unwrap();
            now.1 += 1;
            assert!(now.1 < reads, "the held wait never parked");
            now.0 += TimeDuration::milliseconds(1);
            now.0
        }),
        sleep: Box::new(move |slice| {
            write.lock().unwrap().0 += TimeDuration::try_from(slice).unwrap();
            *count.lock().unwrap() += 1;
        }),
    };
    (clock, slept)
}

/// A hold that never ends, read on a clock that moves between every read:
/// each slice on it is charged its whole second, and the run parks once its
/// waits pass the two-second maximum, after two slices and before a third,
/// with no probe. Removal controls: judge the slice at a second reading of
/// the clock, truncated, as before, and the wait never parks; at a second
/// reading kept to the fraction, and it parks a slice late.
#[test]
fn a_hold_that_never_ends_parks_the_wait_at_its_maximum_within_one_slice() {
    let (dir, mut engine, at) = held_forever();
    let (clock, slept) = moving(at, 500);
    engine.cooldown.clock = clock;
    engine.drive().unwrap();
    assert_eq!(
        park_reason(&engine),
        "cool-down for claude/first would wait 3s in round 1 (episode 3s), past the declared 2s \
         maximum: rate_limit: slow down"
    );
    assert_eq!(*slept.lock().unwrap(), 2);
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
}

/// A hold beside another run's ordinary row ending ten seconds later: the
/// row keeps its own end, so the wait is judged to it, nine whole seconds
/// and a fraction from the moving clock's read, and parks at once,
/// before any slice, past the two-second maximum. Removal control: let the
/// hold's slice stand for the route's end, as before, and the wait is
/// judged a second at a time and parks only after two slices.
#[test]
fn a_longer_ordinary_row_beside_a_hold_parks_at_its_own_end() {
    let (dir, mut engine, at) = held_forever();
    let other = second_run(dir.path());
    let journal = dir.path().join("forge.db");
    append_second_row_on(
        &journal,
        &other,
        "claude",
        "first",
        at + TimeDuration::seconds(10),
    );
    let (clock, slept) = moving(at, 500);
    engine.cooldown.clock = clock;
    engine.drive().unwrap();
    assert_eq!(
        park_reason(&engine),
        "cool-down for claude/first would wait 9s in round 1 (episode 9s), past the declared 2s \
         maximum: rate_limit: slow down"
    );
    assert_eq!(*slept.lock().unwrap(), 0);
}

/// A hold read at the clock's last instant: its next slice cannot be named,
/// so the wait's end refuses by type, never wrapping or panicking. Removal
/// control: add the slice unchecked, and the read panics.
#[test]
fn a_hold_read_at_the_clocks_last_instant_refuses_its_next_slice() {
    let (_dir, engine, at) = held_forever();
    let wait = brokkr_core::ProviderWait {
        site: None,
        provider: "claude".into(),
        route: "first".into(),
        words: "rate_limit: slow down".into(),
        retry_after_seconds: None,
        seconds: 5,
        until: at,
        rounds: 1,
        held: None,
    };
    let last = time::PrimitiveDateTime::MAX.assume_utc();
    let refused = engine.cool_down_end(&[wait], last).unwrap_err();
    assert!(
        matches!(&refused, CooldownError::HeldPastTheClock { provider, route }
            if provider == "claude" && route == "first"),
        "{refused:?}"
    );
    assert_eq!(
        refused.to_string(),
        "a cool-down wait for claude/first is held by a probe at an instant whose next slice the \
         clock cannot name"
    );
}
