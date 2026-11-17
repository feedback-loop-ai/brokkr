//! Decision 0106 ruling 2, slice 0106b1: a provider-stated reset past the
//! declared maximum is a held wait, not a park. Every case is planted
//! through the real refusal: the driver reports a typed rate limit and the
//! engine's own terminal path decides.

use brokkr_core::{FailureClass, ParkCause, ProviderLimitRefusal};

use super::*;

/// A single seat whose first attempt is refused with a stated reset of five
/// seconds, past the host's two-second maximum.
fn held_seat(
    verdicts: &[&str],
    attempts: u64,
) -> (tempfile::TempDir, Engine, Arc<Mutex<OffsetDateTime>>) {
    let (dir, mut engine, now) = seated!(verdicts, Probed::Answer, Some(5), attempts);
    engine.cooldown.host = Some(host(dir.path(), 1, 2));
    (dir, engine, now)
}

/// The run's status and how many parks its journal holds.
fn standing(engine: &Engine) -> (Status, usize) {
    let events = engine.store.load(&engine.run_id).unwrap();
    let parks = events
        .iter()
        .filter(|event| event.event_type == EventType::RunParked)
        .count();
    (fold(&events).unwrap().run.status(), parks)
}

#[test]
fn a_provider_stated_reset_past_the_maximum_is_held_not_parked() {
    let (dir, mut engine, _now) = held_seat(&["refuse", "succeed"], 2);
    let end = engine.drive().unwrap();
    assert_eq!(end.state.run.status(), Status::Running);
    assert_eq!(standing(&engine), (Status::Running, 0));
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
    let waits = &failed(&engine)[0].payload["waits"];
    assert_eq!(
        (
            &waits[0]["held"],
            &waits[0]["max_seconds"],
            &waits[0]["seconds"]
        ),
        (&json!(true), &json!(2), &json!(5))
    );
}

#[test]
fn a_held_wait_carries_no_refused() {
    let (_dir, mut engine, _now) = held_seat(&["refuse", "succeed"], 2);
    engine.drive().unwrap();
    let payload = &failed(&engine)[0].payload;
    assert_eq!(payload.get("refused"), None, "{payload}");
}

/// The engine's own record of the wait, read back from the terminal event,
/// is the held one: the kind and the maximum it passed.
#[test]
fn the_held_wait_is_read_from_the_terminal_event_into_the_account() {
    let (_dir, mut engine, _now) = held_seat(&["refuse", "succeed"], 2);
    engine.drive().unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let effect = failed(&engine)[0].payload["effect_id"]
        .as_str()
        .unwrap()
        .to_string();
    let wait = engine.recorded_wait(&events, &effect).unwrap().unwrap();
    assert_eq!(wait.held, Some(brokkr_core::Held { max_seconds: 2 }));
    assert_eq!(wait.retry_after_seconds, Some(5));
}

/// A computed wait, the route's default doubled for its second round, past
/// the maximum still parks as the provider's limit (decision 0106a), with
/// the provider's words and the maximum in its reason. This replaces the
/// stated-reset park that ruling 2 turns into a held wait.
#[test]
fn a_computed_wait_past_the_maximum_still_parks_as_a_provider_limit() {
    let (dir, mut engine, _now) = seated!(&["refuse", "refuse"], Probed::Answer, None, 3);
    engine.cooldown.host = Some(host(dir.path(), 2, 3));
    engine.drive().unwrap();
    assert_eq!(standing(&engine), (Status::AwaitingOperator, 1));
    let reason = park_reason(&engine);
    assert!(reason.contains("would wait 4s"), "{reason}");
    assert!(reason.contains("declared 3s maximum"), "{reason}");
    assert!(reason.contains("rate_limit: slow down"), "{reason}");
    let refusal = refusal_of(park_cause_of(&engine));
    assert!(
        matches!(
            refusal,
            ProviderLimitRefusal::PastMaximum {
                seconds: 4,
                max_seconds: 3,
                ..
            }
        ),
        "{refusal:?}"
    );
    assert_eq!(failed(&engine)[1].payload["waits"][0].get("held"), None);
}

/// A stated reset within the maximum is the engine's to bound: two of them
/// accumulate past it, and the second parks as the provider's limit.
#[test]
fn an_accumulated_stated_reset_past_the_maximum_still_parks_as_a_provider_limit() {
    let (dir, mut engine, _now) =
        seated!(&["refuse", "refuse", "succeed"], Probed::Answer, Some(2), 3);
    engine.cooldown.host = Some(host(dir.path(), 1, 3));
    engine.drive().unwrap();
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "attempt:refuse"]
    );
    assert_eq!(standing(&engine), (Status::AwaitingOperator, 1));
    // The terminal itself judged the episode: its refusal is on the event.
    assert_eq!(
        failed(&engine)[1].payload["refused"]["cause"],
        "past-maximum"
    );
    assert!(matches!(
        refusal_of(park_cause_of(&engine)),
        ProviderLimitRefusal::PastMaximum {
            seconds: 2,
            max_seconds: 3,
            ..
        }
    ));
}

/// The cap is untouched: a held wait at 0006's cap parks `attempt-cap`,
/// with the provider's limit as its failure class.
#[test]
fn a_held_wait_at_the_cap_parks_attempt_cap() {
    let (dir, mut engine, _now) = held_seat(&["refuse"], 1);
    engine.drive().unwrap();
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
    assert_eq!(standing(&engine), (Status::AwaitingOperator, 1));
    assert!(matches!(
        park_cause_of(&engine),
        ParkCause::AttemptCap {
            failure_class: FailureClass::ProviderLimit,
            ..
        }
    ));
    assert_eq!(failed(&engine)[0].payload["waits"][0]["held"], json!(true));
}
