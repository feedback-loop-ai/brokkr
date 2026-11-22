//! Decision 0106 ruling 2 and its amendment of 0103 ruling 4's
//! probe-refusal loop (fix round 0106b1r2): a probe whose provider states a
//! reset past the declared maximum writes another held round, recorded and
//! held by admission, never a park that leaves the route unheld. Every case
//! is planted through the real refusal: the fake driver's probe reports a
//! typed rate limit and the engine's own probe path decides.

use brokkr_core::{ParkCause, ProviderLimitRefusal};
use time::Duration as TimeDuration;

use super::held_wait::{
    effect_of, held_until, parked_at_the_cap, retried_without_a_park, standing, stated_seat,
};
use super::*;

/// A single seat whose attempt and first probe are each refused with a
/// stated reset of `retry` seconds, past the host's two-second maximum;
/// every later probe answers.
fn probed_seat(
    verdicts: &[&str],
    retry: u64,
    attempts: u64,
) -> (tempfile::TempDir, Engine, Arc<Mutex<OffsetDateTime>>) {
    stated_seat(verdicts, Probed::RefuseThenAnswer, retry, attempts)
}

/// The aggregate admission reads for claude/first at `at`.
fn aggregate_at(engine: &Engine, at: OffsetDateTime) -> Option<OffsetDateTime> {
    engine
        .store
        .cooldown("claude", "first", at)
        .unwrap()
        .map(|aggregate| aggregate.until)
}

/// Write the held terminal, resume at its end, and let the probe state
/// another five-second reset: the instant the probe ran.
fn held_twice(engine: &mut Engine, now: &Arc<Mutex<OffsetDateTime>>) -> OffsetDateTime {
    engine.drive().unwrap();
    assert_eq!(standing(engine), (Status::Running, 0));
    let probe_at = held_until(engine);
    *now.lock().unwrap() = probe_at;
    engine.drive().unwrap();
    probe_at
}

#[test]
fn a_probes_provider_stated_reset_past_the_maximum_is_another_held_round() {
    let (dir, mut engine, now) = probed_seat(&["refuse", "succeed"], 5, 2);
    let probe_at = held_twice(&mut engine, &now);
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse", "probe"]);
    // Running, no park, no attempt spent and no `refused` written.
    assert_eq!(standing(&engine), (Status::Running, 0));
    let terminals = failed(&engine);
    assert_eq!(terminals.len(), 1);
    assert_eq!(terminals[0].payload.get("refused"), None);
    // The probe's round 2 is written held beside the terminal's own held
    // round 1, with the maximum it passed.
    let kinds = engine
        .store
        .cooldown_held_rounds(&engine.run_id, terminals[0].seq, "")
        .unwrap();
    assert_eq!(
        kinds.into_iter().collect::<Vec<_>>(),
        [
            (1, brokkr_core::Held { max_seconds: 2 }),
            (2, brokkr_core::Held { max_seconds: 2 })
        ]
    );
    // Admission holds the route through the round's end, the second its
    // row is rounded up to, and not past it.
    let end = probe_at + TimeDuration::seconds(5);
    let held = aggregate_at(&engine, probe_at).expect("the held round holds the route");
    assert!(
        held >= end && held < end + TimeDuration::seconds(1),
        "{held}"
    );
    assert_eq!(
        aggregate_at(&engine, end - TimeDuration::seconds(1)),
        Some(held)
    );
    assert_eq!(aggregate_at(&engine, held), None);
}

/// Past the first row's end and before the probe's round ends, the route
/// is still held: admission's pass and claim both refuse it until the
/// round's end, and a resume refuses naming that end and the lift.
#[test]
fn a_route_a_probe_holds_again_admits_nothing_after_the_first_rows_end() {
    let (dir, mut engine, now) = probed_seat(&["refuse", "succeed"], 5, 2);
    let probe_at = held_twice(&mut engine, &now);
    let between = probe_at + TimeDuration::seconds(2);
    let held = aggregate_at(&engine, between).expect("the route stays held");
    let rows = engine.store.cooldowns_in_force(between).unwrap();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!((rows[0].rounds, rows[0].until_at), (2, held));
    let cooling = Err(crate::admission::Capacity::CoolingDown {
        provider: "claude".into(),
        route: "first".into(),
        until: held,
        words: "rate_limit: slow down".into(),
    });
    // The production pass and claim, their room taken where the round has
    // ended (fix round 0106b1r5, M1).
    let mut gates =
        crate::admission::capacity::tests::gates::Gates::over(engine.store.path(), held);
    assert_eq!(gates.legs(between), [cooling.clone(), cooling.clone()]);
    assert_eq!(
        gates.legs(held - TimeDuration::seconds(1)),
        [cooling.clone(), cooling]
    );
    assert_eq!(gates.legs(held), [Ok(()), Ok(())]);
    *now.lock().unwrap() = between;
    let error = engine.drive().unwrap_err();
    let crate::engine::EngineError::HeldWait(refusal) = &error else {
        panic!("a resume before the probe's held end refuses as held: {error}");
    };
    assert_eq!(
        **refusal,
        crate::engine::HeldWait {
            run_id: engine.run_id.clone(),
            provider: "claude".into(),
            route: "first".into(),
            until: crate::admission::spelled(held),
            max_seconds: 2,
        }
    );
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse", "probe"]);
}

/// The probe's held round adds a round but no remainder to the episode.
#[test]
fn a_probes_repeated_held_interval_is_not_counted_in_the_episode() {
    let (_dir, mut engine, now) = probed_seat(&["refuse", "succeed"], 5, 2);
    held_twice(&mut engine, &now);
    let events = engine.store.load(&engine.run_id).unwrap();
    let effect = effect_of(&engine);
    let episode =
        super::super::episode_by_site(&engine.store, &engine.run_id, &events, &effect).unwrap();
    assert_eq!(episode.seconds, 0);
    assert_eq!(episode.rounds.get(&None), Some(&2));
    assert_eq!(episode.lengths.get(&None), None);
}

/// At the round's end the resume probes once more on the same effect, and
/// the answer retries it with no park. What this asserts is the one probe
/// at expiry on the same effect; the answered probe's clearing of earlier
/// rows is 0106c's, and nothing here asserts it.
#[test]
fn a_probes_held_round_resumed_at_its_end_probes_once_on_the_same_effect() {
    let (dir, mut engine, now) = probed_seat(&["refuse", "succeed"], 5, 2);
    let probe_at = held_twice(&mut engine, &now);
    let end = aggregate_at(&engine, probe_at).unwrap();
    *now.lock().unwrap() = end;
    engine.drive().unwrap();
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "probe", "attempt:succeed"]
    );
    retried_without_a_park(&engine);
}

/// A probe's computed doubling past the maximum still parks as the
/// provider's limit (0106a), and writes no row.
#[test]
fn a_probes_computed_doubling_past_the_maximum_still_parks_as_a_provider_limit() {
    let (dir, mut engine, _now) =
        seated!(&["refuse", "succeed"], Probed::RefuseThenAnswer, None, 3);
    engine.cooldown.host = Some(host(dir.path(), 1, 1));
    engine.drive().unwrap();
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse", "probe"]);
    assert_eq!(standing(&engine), (Status::AwaitingOperator, 1));
    assert!(
        matches!(
            refusal_of(park_cause_of(&engine)),
            ProviderLimitRefusal::PastMaximum {
                seconds: 2,
                max_seconds: 1,
                ..
            }
        ),
        "{:?}",
        park_cause_of(&engine)
    );
    let seq = failed(&engine)[0].seq;
    let rows = engine
        .store
        .cooldown_rounds(&engine.run_id, seq, "")
        .unwrap();
    // The terminal's own one-second row may already have ended when its
    // event landed, and then is not written; no probe round is either way.
    assert!(rows.iter().all(|row| row.rounds == 1), "{rows:?}");
}

/// A probe's held round spends no attempt; the cap is still 0006's, and
/// the retry it reaches parks `attempt-cap` with the provider's limit.
#[test]
fn a_probes_held_round_spends_no_attempt_and_the_cap_parks_attempt_cap() {
    let (dir, mut engine, now) = probed_seat(&["refuse", "refuse"], 5, 2);
    let probe_at = held_twice(&mut engine, &now);
    assert_eq!(failed(&engine).len(), 1);
    *now.lock().unwrap() = aggregate_at(&engine, probe_at).unwrap();
    engine.drive().unwrap();
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "probe", "attempt:refuse"]
    );
    parked_at_the_cap(&engine);
}

/// Fix rounds 0106b1r3, R3, and 0106b1r9, E2: a held wait ending at the
/// last second the journal can spell, resumed at that end. Its probe's
/// deadline is past that second, so its hold could end only at or before
/// the deadline, while its driver could still run: the probe is refused
/// before dispatch, in the store's typed words, no driver starts, no hold
/// is written, and the run parks ruled, where HEAD's `expect` panicked
/// and r6's clamp dispatched it under a hold already out of force.
#[test]
fn a_probe_from_a_held_end_at_year_9999_is_refused_before_dispatch() {
    let dir = tempfile::tempdir().unwrap();
    let (mut engine, compiled) = engine_probing(dir.path(), 2, Probed::Refuse, Some(2));
    let last = "9999-12-31T23:59:59Z";
    append_failed_payload(
        &mut engine,
        json!({"effect_id": "effect", "attempt_id": "a1", "error": "refused",
               "waits": [{"provider": "claude", "route": "first",
                          "words": "rate_limit: slow down", "seconds": 5,
                          "retry_after_seconds": 5, "until": last, "rounds": 1,
                          "held": true, "max_seconds": 3}]}),
    );
    let run_id = engine.run_id.clone();
    drop(engine);
    let mut engine = resume_engine(dir.path(), compiled, &run_id);
    engine.cooldown.host = Some(host(dir.path(), 1, 3));
    engine.cooldown.clock = Clock::advancing(OffsetDateTime::parse(last, &Rfc3339).unwrap()).0;
    engine.drive().unwrap();
    assert!(logs(dir.path(), "work").is_empty());
    assert_eq!(standing(&engine), (Status::AwaitingOperator, 1));
    let reason = park_reason(&engine);
    assert_eq!(
        reason,
        format!(
            "the cool-down probe for claude/first was not admitted: its route's hold was not \
             written: the journal's probe holds cannot end run '{run_id}''s hold on \
             claude/first after its crash fallback: the first whole second after it is past \
             9999-12-31T23:59:59Z, the last the journal can spell, so no probe runs on it and \
             nothing was written"
        )
    );
    let cause = park_cause_of(&engine);
    assert!(
        matches!(&cause, ParkCause::Ruled { rule_id: None, detail } if reason.starts_with(detail)),
        "{cause:?}"
    );
    let holds: i64 = rusqlite::Connection::open(engine.store.path())
        .unwrap()
        .query_row("SELECT COUNT(*) FROM probe_holds", [], |row| row.get(0))
        .unwrap();
    assert_eq!(holds, 0);
}

/// A probe's stated reset the journal can spell from the terminal's time
/// but not from the probe's parks `no-end`, writing no row: about five
/// thousand years, so the first end lands near year 7000 and the second
/// would pass 9999.
#[test]
fn a_probes_held_round_whose_end_cannot_be_spelled_parks_no_end() {
    const FIVE_MILLENNIA: u64 = 157_788_000_000;
    let (dir, mut engine, now) = probed_seat(&["refuse", "succeed"], FIVE_MILLENNIA, 2);
    held_twice(&mut engine, &now);
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse", "probe"]);
    let reason = parked_no_end(&engine, FIVE_MILLENNIA, 2);
    assert!(
        reason.contains("whose end the journal cannot spell"),
        "{reason}"
    );
}

/// The run parked once as the provider's `no-end` limit on claude/first,
/// with the round's `seconds` and its site's maximum: the park's reason.
fn parked_no_end(engine: &Engine, seconds: u64, max_seconds: u64) -> String {
    assert_eq!(standing(engine), (Status::AwaitingOperator, 1));
    assert_eq!(
        refusal_of(park_cause_of(engine)),
        ProviderLimitRefusal::NoEnd {
            route: "first".into(),
            seconds,
            max_seconds,
        }
    );
    park_reason(engine)
}
