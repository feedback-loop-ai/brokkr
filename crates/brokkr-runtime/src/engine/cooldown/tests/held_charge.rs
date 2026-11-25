//! Fix round 0106b1r15, M1: a held wait is charged the time it observes
//! passing, sampled after each sleep, never the slice it planned: a sleep
//! that overruns its slice, a probe hold's contention between two polls,
//! and a start a hold refused are each charged, so a run whose hold never
//! ends parks at its maximum, within one slice, across all three.

use brokkr_store::{ProbeHold, ProbeHoldId, Store};
use time::Duration as TimeDuration;

use super::held_budget::held_forever;
use super::*;

/// The park every case here comes to: the next slice would pass the
/// ten-second maximum the test declares.
const PARKED: &str = "cool-down for claude/first would wait 11s in round 1 (episode 11s), past \
                      the declared 10s maximum: rate_limit: slow down";

/// `held_forever`'s fixture under a ten-second maximum: its hold standing,
/// and the clock at that instant.
fn held_ten() -> (
    tempfile::TempDir,
    Engine,
    Arc<Mutex<OffsetDateTime>>,
    ProbeHoldId,
) {
    let (dir, mut engine, at, hold) = held_forever();
    engine.cooldown.host = Some(host(dir.path(), 1, 10));
    (dir, engine, Arc::new(Mutex::new(at)), hold)
}

/// A clock on `now` whose reads stand still and whose every sleep takes
/// `taken` for its slice, then runs `slept` with the slices slept so far
/// and the instant reached; it refuses a fiftieth slice, so a wait that
/// never parks ends.
pub(super) fn sleeping(
    now: &Arc<Mutex<OffsetDateTime>>,
    taken: TimeDuration,
    mut slept: impl FnMut(u32, OffsetDateTime) + Send + Sync + 'static,
) -> (Clock, Arc<Mutex<u32>>) {
    let (read, write) = (Arc::clone(now), Arc::clone(now));
    let count = Arc::new(Mutex::new(0_u32));
    let counting = Arc::clone(&count);
    let clock = Clock {
        now: Box::new(move || *read.lock().unwrap()),
        sleep: Box::new(move |_| {
            let mut count = counting.lock().unwrap();
            *count += 1;
            assert!(*count < 50, "the held wait never parked");
            let mut at = write.lock().unwrap();
            *at += taken;
            slept(*count, *at);
        }),
    };
    (clock, count)
}

/// A hold of `attempt`, a drive of `run` never took, on claude/first at
/// `at`, written on a connection of its own.
fn hold_on(journal: &Path, run: &str, attempt: &str, at: OffsetDateTime) -> ProbeHoldId {
    hold_route(journal, (run, attempt), ("claude", "first"), at)
}

/// A hold of `attempt`, a drive of `run` never took, on `provider`/`route`
/// at `at`, written on a connection of its own.
pub(super) fn hold_route(
    journal: &Path,
    (run, attempt): (&str, &str),
    (provider, route): (&str, &str),
    at: OffsetDateTime,
) -> ProbeHoldId {
    let hold = ProbeHold {
        run_id: run,
        attempt_id: attempt,
        site: "",
        provider,
        route,
        deadline: at + TimeDuration::minutes(1),
    };
    Store::open(journal).unwrap().hold_probe(&hold, at).unwrap()
}

/// End `hold` once four slices are slept, on a connection of its own.
fn ending_after_four(
    journal: &Path,
    hold: ProbeHoldId,
) -> impl FnMut(u32, OffsetDateTime) + Send + Sync + 'static {
    let store = Mutex::new(Store::open(journal).unwrap());
    move |slept, at| {
        if slept == 4 {
            store
                .lock()
                .unwrap()
                .end_probe_hold(Some(hold), at)
                .unwrap();
        }
    }
}

/// Where the run parked: the slices it slept and the time it observed.
fn parked_after(
    engine: &Engine,
    slept: &Mutex<u32>,
    now: &Mutex<OffsetDateTime>,
    at: OffsetDateTime,
) -> (u32, TimeDuration) {
    assert_eq!(park_reason(engine), PARKED);
    (*slept.lock().unwrap(), *now.lock().unwrap() - at)
}

/// Every sleep overruns its one-second slice by half: each is charged the
/// second and a half it took, so the run parks after seven slices, at ten
/// and a half seconds observed, before an eighth would pass the maximum.
/// Removal control: charge the planned slice, as r14 did, and the run
/// sleeps ten slices, fifteen seconds, before it parks.
#[test]
fn a_sleep_that_overruns_its_slice_is_charged_what_it_took() {
    let (dir, mut engine, now, _) = held_ten();
    let at = *now.lock().unwrap();
    let (clock, slept) = sleeping(&now, TimeDuration::milliseconds(1500), |_, _| {});
    engine.cooldown.clock = clock;
    engine.drive().unwrap();
    let parked = parked_after(&engine, &slept, &now, at);
    assert_eq!(parked, (7, TimeDuration::milliseconds(10_500)));
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
}

/// The hold ends after four seconds and the probe answers, but another
/// hold takes the route before the retry's start, which is refused and
/// ends the turn. The next turn's wait keeps the four seconds already
/// waited, so the run parks after six more, at ten seconds, not after ten
/// more. Removal control: start every wait's charge afresh, and the run
/// sleeps fourteen slices before it parks.
#[test]
fn a_start_a_hold_refused_keeps_the_time_already_waited() {
    let (dir, mut engine, now, first) = held_ten();
    let at = *now.lock().unwrap();
    let journal = engine.store.path().to_path_buf();
    let ending = ending_after_four(&journal, first);
    let (clock, slept) = sleeping(&now, TimeDuration::seconds(1), ending);
    engine.cooldown.clock = clock;
    let (run, read) = (engine.run_id.clone(), Arc::clone(&now));
    crate::engine::fence::tests::before_the_start(Box::new(move || {
        hold_on(&journal, &run, "start-refused", *read.lock().unwrap());
    }));
    engine.drive().unwrap();
    let parked = parked_after(&engine, &slept, &now, at);
    assert_eq!(parked, (10, TimeDuration::seconds(10)));
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse", "probe"]);
    assert_eq!(starts(&engine).len(), 1, "the refused start was journaled");
}

/// The hold ends after four seconds; as the probe takes its own hold,
/// another run's hold has taken the route, three seconds later. That
/// contention spawns nothing, and its three seconds are charged at the
/// next poll, so the run parks after three more slices, at ten seconds.
/// Removal control: charge the planned slice, and the contention's three
/// seconds are lost, the run sleeping ten slices before it parks.
#[test]
fn a_probe_holds_contention_between_polls_is_charged() {
    let (dir, mut engine, now, first) = held_ten();
    let at = *now.lock().unwrap();
    let journal = engine.store.path().to_path_buf();
    let (clock, slept) = sleeping(
        &now,
        TimeDuration::seconds(1),
        ending_after_four(&journal, first),
    );
    engine.cooldown.clock = clock;
    let (run, read) = (engine.run_id.clone(), Arc::clone(&now));
    let mut contended = false;
    super::super::probe::before_hold_here(Some(Box::new(move || {
        if !std::mem::replace(&mut contended, true) {
            let mut at = read.lock().unwrap();
            *at += TimeDuration::seconds(3);
            hold_on(&journal, &run, "contending", *at);
        }
    })));
    engine.drive().unwrap();
    super::super::probe::before_hold_here(None);
    let parked = parked_after(&engine, &slept, &now, at);
    assert_eq!(parked, (7, TimeDuration::seconds(10)));
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
}

/// A three-second probe refuses and writes its next round. Its elapsed
/// time is outside either recorded round, so the permanent hold after
/// that round parks at the original ten-second maximum.
#[test]
fn a_probe_that_writes_an_extension_still_charges_its_elapsed_time() {
    let (dir, mut engine, now) = seated!(&["refuse", "succeed"], Probed::RefuseThenAnswer, None, 2);
    engine.bundle.seats.get_mut("work").unwrap().limits = super::support::work(2);
    let at = live_row_clock_start();
    *now.lock().unwrap() = at;
    let (journal, run) = (engine.store.path().to_path_buf(), engine.run_id.clone());
    let (clock, slept) = sleeping(&now, TimeDuration::seconds(1), move |slept, at| {
        if slept == 3 {
            hold_on(&journal, &run, "after-renewal", at);
        }
    });
    engine.cooldown.clock = clock;
    let mut read = Some(Arc::clone(&now));
    super::super::probe::before_hold_here(Some(Box::new(move || {
        if let Some(now) = read.take() {
            *now.lock().unwrap() += TimeDuration::seconds(3);
        }
    })));
    let driven = engine.drive();
    super::super::probe::before_hold_here(None);
    driven.unwrap();
    let terminal = failed(&engine).remove(0);
    let rounds = engine
        .store
        .cooldown_rounds(&engine.run_id, terminal.seq, "")
        .unwrap();
    let reason = park_reason(&engine);
    assert_eq!(
        rounds.iter().map(|row| row.rounds).collect::<Vec<_>>(),
        [1, 2],
        "{reason}"
    );
    assert_eq!(
        (*slept.lock().unwrap(), *now.lock().unwrap() - at),
        (7, TimeDuration::seconds(10))
    );
    assert_eq!(
        reason,
        "cool-down for claude/first would wait 8s in round 2 (episode 11s), past the declared \
         10s maximum: rate_limit: slow down"
    );
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse", "probe"]);
}

/// The sample that finds a wait's end passed charges the wait's last time
/// from the episode's own rows; an episode that stops reading by then
/// parks by name, never as no time waited.
#[test]
fn an_episode_unreadable_at_the_waits_end_parks_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = bare_engine(dir.path(), 2);
    engine.cooldown.host = Some(host(dir.path(), 30, 1000));
    let end = base() + TimeDuration::seconds(3);
    append_waited(&mut engine, wait_value(end, 3));
    // The terminal's own round, filled at its end: the row the episode
    // reads.
    let terminal = failed(&engine).remove(0).seq;
    super::probe_rounds::append_round(&mut engine, terminal, end, 1, 3);
    let now = Arc::new(Mutex::new(base()));
    let journal = engine.store.path().to_path_buf();
    engine.cooldown.clock = clock_acting(&now, Acting::AfterFirstSlice, move || {
        rusqlite::Connection::open(&journal)
            .unwrap()
            .execute_batch(
                "DROP TRIGGER provider_cooldowns_append_only_update;
                 UPDATE provider_cooldowns SET recorded_at = -9223372036854775808;",
            )
            .unwrap();
    });
    engine.drive().unwrap();
    let reason = park_reason(&engine);
    assert!(
        reason.starts_with("a cool-down wait for claude/first cannot read its episode"),
        "{reason}"
    );
    assert_eq!(*now.lock().unwrap(), base() + TimeDuration::seconds(3));
}

/// A retry the provider refuses again records new waits, and the wait on
/// them is charged afresh: the time the first wait was charged on another
/// run's hold is not carried to it. Another hold that never ends then
/// parks the run once the second wait's own charge would pass the maximum.
/// Removal control: carry a charge to a wait on any waits, and the run
/// parks earlier, the first wait's seconds charged twice.
#[test]
fn a_retry_refused_again_charges_its_next_wait_afresh() {
    let verdicts = &["refuse", "refuse", "succeed"];
    let (dir, mut engine, now) = seated!(verdicts, Probed::Answer, None, 3);
    let at = *now.lock().unwrap();
    let (journal, run) = (engine.store.path().to_path_buf(), engine.run_id.clone());
    let first = hold_on(&journal, &run, "first-hold", at);
    let mut ending = ending_after_four(&journal, first);
    let (clock, slept) = sleeping(&now, TimeDuration::seconds(1), move |slept, at| {
        ending(slept, at);
        if slept == 6 {
            hold_on(&journal, &run, "second-hold", at);
        }
    });
    engine.cooldown.clock = clock;
    engine.drive().unwrap();
    // The first wait charged three seconds on the first hold, its own row
    // ending a second in; the retry refused at four records two seconds
    // more. The second hold stands from six, so the episode's three
    // seconds and the eight the second wait would reach pass the maximum
    // at thirteen, not at ten.
    assert_eq!(
        park_reason(&engine),
        "cool-down for claude/first would wait 8s in round 2 (episode 11s), past the declared \
         10s maximum: rate_limit: slow down"
    );
    let parked = (*slept.lock().unwrap(), *now.lock().unwrap() - at);
    assert_eq!(parked, (13, TimeDuration::seconds(13)));
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "attempt:refuse"]
    );
}
