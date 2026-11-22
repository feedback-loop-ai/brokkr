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
    stated_seat(verdicts, Probed::Answer, 5, attempts)
}

/// A single seat whose attempts are refused with a stated reset of `retry`
/// seconds, past the host's two-second maximum, and whose probes do what
/// `probed` says.
pub(super) fn stated_seat(
    verdicts: &[&str],
    probed: Probed,
    retry: u64,
    attempts: u64,
) -> (tempfile::TempDir, Engine, Arc<Mutex<OffsetDateTime>>) {
    let (dir, mut engine, now) = seated!(verdicts, probed, Some(retry), attempts);
    engine.cooldown.host = Some(host(dir.path(), 1, 2));
    (dir, engine, now)
}

/// The run's status and how many parks its journal holds.
pub(super) fn standing(engine: &Engine) -> (Status, usize) {
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
/// is the held one: the kind and the maximum it passed. This is the
/// engine's reader only. `Account` carries no waits until 0106b2's
/// readouts give it `waits` (decision 0106, "How the readouts show it"),
/// so `the_held_wait_is_read_from_the_terminal_event_into_the_account` is
/// that slice's test, not this one (fix round 0106b1r3, R4).
#[test]
fn the_engine_reads_the_held_wait_back_from_its_terminal_event() {
    let (_dir, mut engine, _now) = held_seat(&["refuse", "succeed"], 2);
    engine.drive().unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let wait = engine
        .recorded_wait(&events, &effect_of(&engine))
        .unwrap()
        .unwrap();
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

/// The adversarial position's case (fix round 0106b1r6, D3): the first
/// refusal states five seconds against the two-second maximum with
/// attempts left, and the table takes its held row without writing it
/// (every cool-down insert ignored by a trigger). The terminal's append
/// refuses with the store's typed words and rolls back, so no held wait is
/// journaled: the attempt is settled indeterminate naming the refusal, and
/// the run parks on it, never Running with no park.
#[test]
fn a_held_wait_the_table_does_not_write_is_not_journaled_and_parks() {
    let (dir, mut engine, _now) = held_seat(&["refuse", "succeed"], 2);
    super::held_route::driven_raising(&mut engine, "provider_cooldowns", "IGNORE");
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
    assert_eq!(failed(&engine).len(), 0);
    parked_unwritten(&engine);
}

/// The run journaled no wait, settled its attempt indeterminate naming the
/// store's refusal of the first terminal's row, and parked once with it,
/// ruled through the one `park_of`, as H3's unwritten round parks (fix
/// round 0106b1r9, E4).
fn parked_unwritten(engine: &Engine) {
    let events = engine.store.load(&engine.run_id).unwrap();
    assert!(
        events
            .iter()
            .all(|event| event.payload.get("waits").is_none()),
        "a wait was journaled"
    );
    let words = format!(
        "the journal's provider cool-downs wrote no row for run '{}' event 5 site '' round 1, so \
         the event that records its wait was not written either",
        engine.run_id
    );
    let settled = (events.iter())
        .find(|event| event.event_type == EventType::EffectIndeterminate)
        .expect("the attempt was settled");
    assert!(
        settled.payload["reason"].as_str().unwrap().contains(&words),
        "{}",
        settled.payload
    );
    let reason = park_reason(engine);
    assert_eq!(
        reason,
        format!("the journal refused the attempt's outcome: {words}")
    );
    let cause = park_cause_of(engine);
    assert!(
        matches!(&cause, ParkCause::Ruled { rule_id: None, detail } if reason.starts_with(detail)),
        "{cause:?}"
    );
    assert_eq!(standing(engine), (Status::AwaitingOperator, 1));
}

/// Fix round 0106b1r9, E3: the first refusal's held wait, on a queue that
/// a newer binary took over while the attempt ran (its storage version
/// moved past this binary's at the attempt's start), has no table to hold
/// it. The terminal is refused as the table's unwritten row refuses it, so
/// no held wait is journaled, nothing marks it written, and the run parks
/// rather than ending its drive Running as if the wait were held.
#[test]
fn a_held_wait_on_a_queue_this_binary_does_not_write_is_not_journaled_and_parks() {
    let (dir, mut engine, _now) = held_seat(&["refuse", "succeed"], 2);
    rusqlite::Connection::open(engine.store.path())
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER newer AFTER INSERT ON events
             WHEN json_extract(NEW.envelope, '$.type') = 'effect/started'
             BEGIN UPDATE meta SET value = '8' WHERE key = 'queue_schema'; END;",
        )
        .unwrap();
    engine.drive().unwrap();
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
    parked_unwritten(&engine);
}

/// The cap is untouched: a held wait at 0006's cap parks `attempt-cap`,
/// with the provider's limit as its failure class.
#[test]
fn a_held_wait_at_the_cap_parks_attempt_cap() {
    let (dir, mut engine, _now) = held_seat(&["refuse"], 1);
    engine.drive().unwrap();
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
    parked_at_the_cap(&engine);
    assert_eq!(failed(&engine)[0].payload["waits"][0]["held"], json!(true));
}

/// The first terminal's effect id.
pub(super) fn effect_of(engine: &Engine) -> String {
    failed(engine)[0].payload["effect_id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// The run parked once, at 0006's cap, with the provider's limit as the
/// last attempt's failure class.
pub(super) fn parked_at_the_cap(engine: &Engine) {
    assert_eq!(standing(engine), (Status::AwaitingOperator, 1));
    assert!(matches!(
        park_cause_of(engine),
        ParkCause::AttemptCap {
            failure_class: FailureClass::ProviderLimit,
            ..
        }
    ));
}

/// The retry succeeded on the first effect with no park before it, and
/// its start records the probe's answer for claude/first.
pub(super) fn retried_without_a_park(engine: &Engine) {
    let events = engine.store.load(&engine.run_id).unwrap();
    let succeeded = events
        .iter()
        .position(|event| event.event_type == EventType::EffectSucceeded)
        .expect("the retry succeeded");
    assert!(
        !events[..succeeded]
            .iter()
            .any(|event| event.event_type == EventType::RunParked),
        "the resume parked before the retry"
    );
    // `starts` keeps the first effect's own starts: the retry is that effect.
    let started = starts(engine);
    assert_eq!(started.len(), 2, "{started:?}");
    assert_eq!(started[1]["probes"][0]["route"], "first");
}

/// The held wait's recorded end, as its terminal event spells it.
pub(super) fn held_until(engine: &Engine) -> OffsetDateTime {
    let until = failed(engine)[0].payload["waits"][0]["until"]
        .as_str()
        .unwrap()
        .to_string();
    OffsetDateTime::parse(&until, &Rfc3339).unwrap()
}

/// Write the held wait, then resume `after` seconds past its end: the
/// held 5s is the provider's, not the engine's, so the 2s maximum does not
/// bound it, and the resume probes once and retries the same effect.
fn resumed_after_the_held_end(after: i64) {
    let (dir, mut engine, now) = held_seat(&["refuse", "succeed"], 2);
    engine.drive().unwrap();
    assert_eq!(standing(&engine), (Status::Running, 0));
    *now.lock().unwrap() = held_until(&engine) + time::Duration::seconds(after);
    engine.drive().unwrap();
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
    // No park before the retry succeeded: the episode did not count the
    // held 5s against the 2s maximum (what follows the success is the
    // fixture's policy, not the wait's).
    retried_without_a_park(&engine);
}

#[test]
fn a_provider_stated_held_interval_is_not_counted_in_the_episode() {
    resumed_after_the_held_end(0);
}

#[test]
fn a_resume_after_the_held_until_probes() {
    resumed_after_the_held_end(30);
}

/// A resume before the held end writes nothing, runs nothing, and refuses
/// by name, naming the end and the lift.
#[test]
fn a_resume_before_the_held_until_refuses_and_names_the_lift() {
    let (dir, mut engine, _now) = held_seat(&["refuse", "succeed"], 2);
    engine.drive().unwrap();
    let before = engine.store.load(&engine.run_id).unwrap().len();
    let until = brokkr_core::stamp::write_millis(held_until(&engine)).unwrap();
    let error = engine.drive().unwrap_err();
    let crate::engine::EngineError::HeldWait(held) = &error else {
        panic!("a resume before the held end refuses as held: {error}");
    };
    assert_eq!(
        **held,
        crate::engine::HeldWait {
            run_id: engine.run_id.clone(),
            provider: "claude".into(),
            route: "first".into(),
            until: until.clone(),
            max_seconds: 2,
        }
    );
    let text = error.to_string();
    assert!(text.contains(&format!("held until {until}")), "{text}");
    let lift = format!("brokkr operator retry --run {}", engine.run_id);
    assert!(text.contains(&lift), "{text}");
    assert_eq!(engine.store.load(&engine.run_id).unwrap().len(), before);
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
}

/// An ordinary wait resumed at or after its own end probes at once: the
/// resume sleeps nothing.
#[test]
fn an_ordinary_wait_resumed_at_or_after_its_until_probes() {
    let until = base() + time::Duration::seconds(3);
    for at in [until, until + time::Duration::seconds(10)] {
        let (dir, mut engine, now) = resumed_at(&["succeed"], until, 3, at, false);
        engine.cooldown.host = None;
        engine.drive().unwrap();
        assert_eq!(logs(dir.path(), "work"), ["probe", "attempt:succeed"]);
        assert_eq!(*now.lock().unwrap(), at, "the resume slept");
    }
}

/// An ordinary wait resumed at 0006's cap parks naming the cool-down, and
/// probes nothing.
#[test]
fn an_ordinary_resume_at_the_cap_parks() {
    let dir = tempfile::tempdir().unwrap();
    let waits = wait_value(base() + time::Duration::seconds(3), 3);
    let payload = json!({"effect_id": "effect", "attempt_id": "a1", "error": "refused",
        "waits": waits});
    let mut engine = resumed_payload(dir.path(), 1, payload);
    engine.drive().unwrap();
    assert!(logs(dir.path(), "work").is_empty(), "the capped resume ran");
    assert!(park_reason(&engine).contains("failed 1 of 1 attempt(s)"));
    assert!(matches!(
        park_cause_of(&engine),
        ParkCause::AttemptCap {
            failure_class: FailureClass::ProviderLimit,
            ..
        }
    ));
}
