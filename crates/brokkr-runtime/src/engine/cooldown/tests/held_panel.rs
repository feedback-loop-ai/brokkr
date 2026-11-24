//! Fix round 0106b1r3, R1 (decision 0106 ruling 2, 0071 ruling 5): in a
//! mixed panel each site's stated reset is judged against its OWN route's
//! maximum, at the probe as at the terminal. A held site adds no seconds to
//! the episode, an ordinary site counts its own, and no site's maximum
//! decides whether another site's reset is held. Every case is planted
//! through the real refusals: a on claude/first and b on codex/second each
//! report a typed rate limit, and the engine's own terminal and probe paths
//! decide.

use brokkr_core::ProviderLimitRefusal;
use time::Duration as TimeDuration;

use super::held_wait::{effect_of, standing};
use super::panel::{panel_engine, route_host, two_candidate_panel, wait_entries};
use super::*;

/// One panel member's stated resets: the attempt's and its first probe's.
struct Stated {
    attempt: u64,
    probe: u64,
    max: u64,
}

/// A two-route panel whose attempts are refused once with the resets
/// `a` and `b` state, a's first probe stating `a.probe` and every later
/// probe answering, under a host whose claude/first and codex/second
/// maxima are `a.max` and `b.max`.
fn mixed_panel(a: &Stated, b: &Stated) -> (tempfile::TempDir, Engine, Arc<Mutex<OffsetDateTime>>) {
    let dir = tempfile::tempdir().unwrap();
    let verdicts = ["refuse", "succeed"];
    let a_argv = probe_states(
        driver(
            dir.path(),
            "a",
            &verdicts,
            Probed::RefuseThenAnswer,
            Some(a.attempt),
            "claude",
            "first",
        ),
        a.attempt,
        a.probe,
    );
    let b_argv = driver(
        dir.path(),
        "b",
        &verdicts,
        Probed::Answer,
        Some(b.attempt),
        "codex",
        "second",
    );
    let host = route_host(dir.path(), (1, a.max), (1, b.max));
    let engine = panel_engine(dir.path(), two_candidate_panel(a_argv, b_argv), 2, host);
    let (engine, now) = with_clock(engine);
    (dir, engine, now)
}

/// `argv` whose probe states `probe` seconds where its attempt states
/// `attempt`: the probe's branch comes first in the fake driver's script.
fn probe_states(mut argv: Vec<String>, attempt: u64, probe: u64) -> Vec<String> {
    let script = argv.pop().expect("a driver script");
    argv.push(script.replacen(
        &format!("\"retry_after_seconds\":{attempt}}}"),
        &format!("\"retry_after_seconds\":{probe}}}"),
        1,
    ));
    argv
}

/// The terminal's `until` for `site`, as its `waits` entry spells it.
fn until_of(engine: &Engine, site: &str) -> OffsetDateTime {
    let entries = wait_entries(&failed(engine)[0]);
    let entry = entries
        .iter()
        .find(|entry| entry["site"] == site)
        .expect("the site's wait");
    OffsetDateTime::parse(entry["until"].as_str().unwrap(), &Rfc3339).unwrap()
}

/// The episode the engine bounds for the run's one effect.
fn episode_seconds(engine: &Engine) -> u64 {
    let events = engine.store.load(&engine.run_id).unwrap();
    super::super::episode_by_site(&engine.store, &engine.run_id, &events, &effect_of(engine))
        .unwrap()
        .seconds
}

/// Write the panel's held terminal, resume at a's held end, and let a's
/// probe state its second reset: the instant the probe ran.
fn probed_at_a_end(engine: &mut Engine, now: &Arc<Mutex<OffsetDateTime>>) -> OffsetDateTime {
    probed_at_held_end(engine, now, "a")
}

/// Write the panel's terminal with `held` its one held member, then resume
/// at that member's held end: the instant the resume ran.
fn probed_at_held_end(
    engine: &mut Engine,
    now: &Arc<Mutex<OffsetDateTime>>,
    held: &str,
) -> OffsetDateTime {
    engine.drive().unwrap();
    assert_eq!(standing(engine), (Status::Running, 0));
    let waits = wait_entries(&failed(engine)[0]);
    for wait in &waits {
        let expected = (wait["site"] == held).then_some(&json!(true));
        assert_eq!(wait.get("held"), expected, "{waits:?}");
    }
    let probe_at = until_of(engine, held);
    *now.lock().unwrap() = probe_at;
    engine.drive().unwrap();
    probe_at
}

/// Fix round 0106b1r4, F2, the correctness position's plant: a states 5s
/// past its 2s maximum and is held, b states an ordinary 3s within its 10s.
/// At a's end the resume judges b's 3s by b's maximum alone, and a's 2s
/// judges none of it, so a's probe runs on the same effect and its renewed
/// 5s is held again. At HEAD the resume judged the 3s episode against a's
/// 2s and parked past the maximum before any probe.
#[test]
fn a_held_members_maximum_never_judges_an_ordinary_members_seconds_at_resume() {
    held_again_at_a_end(&stated(5, 5, 2), &stated(3, 3, 10), 3);
}

/// A member's resets and maximum, in [`Stated`]'s order.
fn stated(attempt: u64, probe: u64, max: u64) -> Stated {
    Stated {
        attempt,
        probe,
        max,
    }
}

/// The panel `a` and `b` state, resumed at a's held end: a's one probe
/// states its reset again and is held by a's own maximum, and the episode
/// stays the `episode` seconds b's ordinary wait counted.
fn held_again_at_a_end(a: &Stated, b: &Stated, episode: u64) {
    let (dir, mut engine, now) = mixed_panel(a, b);
    let probe_at = probed_at_a_end(&mut engine, &now);
    assert_eq!(logs(dir.path(), "a"), ["attempt:refuse", "probe"]);
    assert_a_held_again(&engine, probe_at, a.probe, a.max);
    assert_eq!(episode_seconds(&engine), episode);
}

/// F2, the adversarial position's plant: a states 10s past its 2s maximum,
/// b an ordinary 5s within its 10s. At a's expiry one probe runs on the
/// same effect, a fresh held row stands to probe time + 10s, and the run is
/// still running. At HEAD it was `AwaitingOperator`, a's log held only its
/// refused attempt, and claude/first had no row.
#[test]
fn a_resumed_held_wait_probes_where_an_ordinary_members_seconds_pass_its_maximum() {
    held_again_at_a_end(&stated(10, 10, 2), &stated(5, 5, 10), 5);
}

/// F2's mirror: b is held (5s past its 2s maximum) and a waits an ordinary
/// 3s within its 10s. At b's end the resume and the round's wait judge a's
/// 3s by a's maximum only, and a's probe's ordinary 3s round, 6s of
/// episode, is judged by no maximum of b's: it is written, waited, and the
/// next probes answer, so the panel retries and both members succeed. At
/// HEAD b's 2s maximum parked the resume on a's 3s.
#[test]
fn an_ordinary_members_rounds_are_never_judged_by_a_held_members_maximum() {
    let a = Stated {
        attempt: 3,
        probe: 3,
        max: 10,
    };
    let b = Stated {
        attempt: 5,
        probe: 5,
        max: 2,
    };
    let (dir, mut engine, now) = mixed_panel(&a, &b);
    probed_at_held_end(&mut engine, &now, "b");
    assert_eq!(
        logs(dir.path(), "a"),
        ["attempt:refuse", "probe", "probe", "attempt:succeed"]
    );
    assert_eq!(
        logs(dir.path(), "b"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
    // The retry is the effect's next event after its refused terminal: no
    // park came between. (The fixture panel's result is outside its
    // declared results, so the run parks `ruled` after the retry.)
    let events = engine.store.load(&engine.run_id).unwrap();
    let after: Vec<_> = events
        .iter()
        .skip_while(|event| event.event_type != EventType::EffectFailed)
        .skip(1)
        .map(|event| event.event_type)
        .take(2)
        .collect();
    assert_eq!(
        after,
        [EventType::EffectStarted, EventType::EffectCheckpointed]
    );
    assert!(events
        .iter()
        .any(|event| event.event_type == EventType::EffectSucceeded));
    assert_eq!(episode_seconds(&engine), 6);
}

/// a's probe round, held, with the maximum it passed and its end.
fn assert_a_held_again(engine: &Engine, probe_at: OffsetDateTime, stated: u64, max: u64) {
    assert_eq!(standing(engine), (Status::Running, 0));
    let seq = failed(engine)[0].seq;
    let held = engine
        .store
        .cooldown_held_rounds(&engine.run_id, seq, "a")
        .unwrap();
    assert_eq!(
        held.into_iter().collect::<Vec<_>>(),
        [
            (1, brokkr_core::Held { max_seconds: max }),
            (2, brokkr_core::Held { max_seconds: max })
        ]
    );
    let end = probe_at + TimeDuration::seconds(stated as i64);
    let until = ordinary_end(
        (engine.store.cooldown("claude", "first", probe_at))
            .unwrap()
            .expect("a's route is held again"),
    );
    assert!(
        until >= end && until < end + TimeDuration::seconds(1),
        "{until}"
    );
}

/// The adversarial position's plant: a states 5s past its 2s maximum and
/// is held; b waits an ordinary 2s within its 6s. At a's end its probe
/// states 5s again: a's own 2s maximum holds it, and b's 6s never judges
/// it, so the episode stays b's 2s. At HEAD b's maximum judged the round
/// as 7s of episode and parked, leaving claude/first with no row.
#[test]
fn a_probes_stated_reset_is_held_by_its_own_maximum_not_an_ordinary_members() {
    held_again_at_a_end(&stated(5, 5, 2), &stated(2, 2, 6), 2);
}

/// The correctness position's plant: a's 10s past its 2s maximum is held,
/// b's ordinary 1s is within its 10s. a's probe states 10s again: held by
/// a's own maximum. At HEAD b's 10s maximum judged the 11s it summed and
/// parked before the append.
#[test]
fn a_larger_members_maximum_never_parks_a_held_probe_round() {
    let a = Stated {
        attempt: 10,
        probe: 10,
        max: 2,
    };
    let b = Stated {
        attempt: 1,
        probe: 1,
        max: 10,
    };
    let (_dir, mut engine, now) = mixed_panel(&a, &b);
    let probe_at = probed_at_a_end(&mut engine, &now);
    assert_a_held_again(&engine, probe_at, 10, 2);
    assert_eq!(episode_seconds(&engine), 1);
}

/// The reverse: a's maximum is 10s and b's 2s, both wait an ordinary 1s,
/// and a's probe states 5s. Within a's own maximum the round is ordinary:
/// b's 2s maximum does not make it held. It is then counted, so the
/// episode through it, 7s, passes b's maximum and the run parks as the
/// provider's limit, writing no row. At HEAD b's maximum marked the round
/// held and the run stayed running.
#[test]
fn a_smaller_members_maximum_never_holds_an_ordinary_probe_round() {
    let a = Stated {
        attempt: 1,
        probe: 5,
        max: 10,
    };
    let b = Stated {
        attempt: 1,
        probe: 1,
        max: 2,
    };
    let (dir, mut engine, _now) = mixed_panel(&a, &b);
    engine.drive().unwrap();
    assert_eq!(logs(dir.path(), "a"), ["attempt:refuse", "probe"]);
    assert_eq!(standing(&engine), (Status::AwaitingOperator, 1));
    let refusal = refusal_of(park_cause_of(&engine));
    assert!(
        matches!(
            refusal,
            ProviderLimitRefusal::PastMaximum {
                seconds: 5,
                max_seconds: 2,
                ..
            }
        ),
        "{refusal:?}"
    );
    let seq = failed(&engine)[0].seq;
    for site in ["a", "b"] {
        let held = engine
            .store
            .cooldown_held_rounds(&engine.run_id, seq, site)
            .unwrap();
        assert!(held.is_empty(), "{site}: {held:?}");
        let rounds = engine
            .store
            .cooldown_rounds(&engine.run_id, seq, site)
            .unwrap();
        assert!(rounds.iter().all(|row| row.rounds == 1), "{rounds:?}");
    }
}
