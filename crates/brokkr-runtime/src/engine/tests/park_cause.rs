//! The causes the cursor's parks carry (decision 0106 ruling 1): a rule's
//! park its rule id; a moved head `ruled` with no rule and the raw head
//! evidence its close recorded; an indeterminate close `infrastructure`
//! past any operator event that lands before the park; and the cap
//! `attempt-cap` with the failure class the journal's typed evidence gives,
//! or `infrastructure` where its last terminal was indeterminate. The one
//! `run/parked` writer takes a `Park`, and a raw `run/parked` is refused.

use super::*;
use brokkr_core::gate_moved_head::{self, MovedHead};

#[test]
fn compiled_design_upstream_reenters_specify_then_exhausts() {
    for (visits, expected_rule, expected_next) in [
        (2, "DESIGN-UPSTREAM", Some("specify")),
        (3, "DESIGN-UPSTREAM-EXHAUSTED", None),
    ] {
        let (dir, mut engine) = compiled_triage_engine();
        // What a failure below prints, so it names its cause (#1132).
        let seen = |engine: &Engine| crate::witness::witness(engine, dir.path());
        let SeatBody::Sequence { mut steps } = engine.bundle.seats["design"].body.clone() else {
            panic!("compiled triage design is a sequence");
        };
        assert_eq!(steps.last().unwrap().results, ["drafted", "fail"]);
        for step in &mut steps {
            match (&step.name[..], &mut step.body) {
                ("positions", StepBody::Invocation(Invocation::Panel { members, .. })) => {
                    for member in members {
                        member.command = driver_command(
                            "upstream-effect",
                            "upstream-attempt",
                            AttemptOutcome::Succeeded {
                                result: json!({"result":"pass"}),
                            },
                        );
                        member.candidates.clear();
                    }
                }
                (
                    "chief",
                    StepBody::Invocation(Invocation::Single {
                        command,
                        candidates,
                        ..
                    }),
                ) => {
                    *command = driver_command(
                        "upstream-effect",
                        "upstream-attempt",
                        AttemptOutcome::Succeeded {
                            result: json!({"result":"upstream", "notes":"the specification is at fault"}),
                        },
                    );
                    candidates.clear();
                }
                _ => {}
            }
        }
        let mut current = state(Some("design"), Cursor::RequestEffect);
        current.visits.insert("specify".into(), visits);
        let input = engine
            .seat_input(&current, "design", "upstream-effect")
            .unwrap();
        run_sequence(
            &mut engine,
            &SequenceAt::new("upstream-effect", "upstream-attempt", "design"),
            &steps,
            &input,
            10,
            &Selection::new(),
        )
        .unwrap_or_else(|error| panic!("the sequence ran: {error:?}{}", seen(&engine)));
        let events = engine.store.load(&engine.run_id).unwrap();
        let result = events
            .iter()
            .find(|event| event.event_type == EventType::EffectSucceeded)
            .unwrap_or_else(|| panic!("no effect/succeeded{}", seen(&engine)))
            .payload["result"]
            .clone();
        assert_eq!(result["result"], "upstream", "{result}{}", seen(&engine));
        assert!(
            !events
                .iter()
                .any(|event| event.payload["checkpoint"]["step_name"] == "validate"),
            "the validate step ran{}",
            seen(&engine)
        );

        engine
            .decide(&current, result)
            .unwrap_or_else(|error| panic!("the ruling: {error:?}{}", seen(&engine)));
        let decided = engine
            .store
            .load(&engine.run_id)
            .unwrap()
            .pop()
            .unwrap_or_else(|| panic!("the journal is empty{}", seen(&engine)));
        let ruled = format!("{}{}", decided.payload, seen(&engine));
        assert_eq!(decided.payload["rule_id"], expected_rule, "{ruled}");
        assert_eq!(decided.payload["next"].as_str(), expected_next, "{ruled}");
        if expected_next.is_none() {
            park_on_the_ruling(&mut engine, current, expected_rule, seen);
        }
    }
}

/// Parks `current` on the ruling just journalled, and asserts the park's
/// cause is that rule; every journal read prints `seen`'s witness on failure.
fn park_on_the_ruling(
    engine: &mut Engine,
    mut current: RunState,
    rule: &str,
    seen: impl Fn(&Engine) -> String,
) {
    let events = engine.store.load(&engine.run_id).unwrap();
    let decided = events
        .last()
        .unwrap_or_else(|| panic!("the ruling was not appended{}", seen(engine)))
        .payload
        .clone();
    current.last_decision =
        Some(serde_json::from_value(decided).unwrap_or_else(|error| {
            panic!("the ruling does not read back: {error}{}", seen(engine))
        }));
    let reason = brokkr_core::fold::ParkReason::read(format!("{rule} exhausted"));
    engine
        .advance_running(&events, brokkr_core::fold::Cursor::Park { reason }, current)
        .unwrap_or_else(|error| panic!("the park: {error:?}{}", seen(engine)));
    let parked =
        newest_park(engine).unwrap_or_else(|| panic!("the run did not park{}", seen(engine)));
    let cause = &parked.payload["cause"];
    assert_eq!(cause["kind"], "ruled", "{cause}{}", seen(engine));
    assert_eq!(cause["rule_id"], rule, "{cause}{}", seen(engine));
}

/// Appends an event to the engine's journal, under `attempt`.
fn planter(engine: &mut Engine) -> impl FnMut(EventType, Value, Option<&str>) + '_ {
    let run_id = engine.run_id.clone();
    move |event_type, payload, attempt| {
        engine
            .store
            .append_next(
                &run_id,
                event_type,
                payload,
                None,
                attempt.map(str::to_string),
            )
            .unwrap();
    }
}

/// The phase entered and the effect requested for seat `work`.
fn request_work(plant: &mut impl FnMut(EventType, Value, Option<&str>)) {
    plant(EventType::PhaseEntered, json!({"phase": "work"}), None);
    let requested = json!({"effect_id": "effect-1", "seat": "work"});
    plant(EventType::EffectRequested, requested, None);
}

/// A run whose last ruling matched `IMPL-COMPLETE` and whose effect then
/// closed indeterminate with `reason`, followed by `after`: the journal, and
/// the state the engine parks from, its last decision that rule's.
fn closed_indeterminate(
    reason: &str,
    after: &[EventType],
) -> (tempfile::TempDir, Engine, Vec<EventEnvelope>, RunState) {
    let (dir, mut engine) = engine(single_body(vec!["x".into()]));
    let mut plant = planter(&mut engine);
    request_work(&mut plant);
    let ids = json!({"effect_id": "effect-1", "attempt_id": "a1"});
    plant(EventType::EffectStarted, ids.clone(), Some("a1"));
    let mut closed = ids;
    closed["reason"] = json!(reason);
    plant(EventType::EffectIndeterminate, closed, Some("a1"));
    for event_type in after {
        plant(*event_type, json!({"command": "retry"}), None);
    }
    drop(plant);
    let events = engine.store.load(&engine.run_id).unwrap();
    let mut state = state(Some("work"), Cursor::RequestEffect);
    state.last_decision = Some(
        serde_json::from_value(json!({
            "from": "work", "result": "complete", "next": "verify",
            "rule_id": "IMPL-COMPLETE"
        }))
        .unwrap(),
    );
    (dir, engine, events, state)
}

/// The moved-head park arrives through `effect/indeterminate`, so the last
/// ruling is the one that routed INTO the gate: the park names no rule, and
/// carries the head evidence its close recorded.
#[test]
fn a_moved_head_park_carries_no_rule_and_the_head_evidence() {
    let marker = gate_moved_head::encode(&MovedHead {
        head_at_start: Some("aaa".into()),
        head_at_end: Some("bbb".into()),
    });
    let (_dir, mut engine, events, state) = closed_indeterminate(&marker, &[]);
    let cursor = Cursor::Park {
        reason: brokkr_core::fold::ParkReason::GateMovedHead,
    };
    engine.advance_running(&events, cursor, state).unwrap();
    let parked = last_park(&engine);
    assert_eq!(parked.payload["cause"]["kind"], "ruled");
    assert!(
        parked.payload["cause"].get("rule_id").is_none(),
        "{}",
        parked.payload["cause"]
    );
    assert_eq!(
        parked.payload["evidence"],
        json!({"head_at_end": "bbb", "head_at_start": "aaa"})
    );
}

/// An operator command and its receipt may land between the indeterminate
/// close and the park; the close is still what opened the park, so it stays
/// `infrastructure` and never reads as the old ruling's `ruled`.
#[test]
fn an_operator_event_between_the_indeterminate_close_and_the_park_keeps_it_infrastructure() {
    let reason =
        "engine restarted while the attempt was in flight; completion cannot be established";
    let (_dir, mut engine, events, state) = closed_indeterminate(
        reason,
        &[EventType::OperatorCommanded, EventType::OperatorRejected],
    );
    let cursor = Cursor::Park {
        reason: brokkr_core::fold::ParkReason::read(reason.to_string()),
    };
    engine.advance_running(&events, cursor, state).unwrap();
    let cause = last_park(&engine).payload["cause"].clone();
    assert_eq!(
        cause,
        json!({"kind": "infrastructure", "retryable": false, "detail": reason})
    );
}

/// The cause of the cap's park for an effect whose first attempt failed
/// plainly and whose last attempt ended as `terminal` carrying `extra`,
/// after `checkpoints` of that attempt's own.
fn cap_cause_after(terminal: EventType, extra: Value, checkpoints: usize) -> Value {
    cap_park_after(terminal, extra, checkpoints).payload["cause"].clone()
}

/// The cap's `run/parked` event for the effect [`cap_cause_after`] plants.
fn cap_park_after(terminal: EventType, extra: Value, checkpoints: usize) -> EventEnvelope {
    let (_dir, mut engine) = engine(single_body(vec!["x".into()]));
    let mut plant = planter(&mut engine);
    request_work(&mut plant);
    for attempt in ["a1", "a2"] {
        let ids = json!({"effect_id": "effect-1", "attempt_id": attempt});
        plant(EventType::EffectStarted, ids.clone(), Some(attempt));
        if attempt == "a1" {
            let failed = json!({"effect_id": "effect-1", "attempt_id": "a1", "error": "x"});
            plant(EventType::EffectFailed, failed, Some("a1"));
            continue;
        }
        for _ in 0..checkpoints {
            let marker = json!({"effect_id": "effect-1", "attempt_id": attempt,
                                "checkpoint": {"step": "sequence-step-finished"}});
            plant(EventType::EffectCheckpointed, marker, Some(attempt));
        }
        let mut payload = ids;
        payload["error"] = json!("x");
        payload["reason"] = json!("x");
        for (field, value) in extra.as_object().into_iter().flatten() {
            payload[field] = value.clone();
        }
        plant(terminal, payload, Some(attempt));
    }
    drop(plant);
    let events = engine.store.load(&engine.run_id).unwrap();
    let state = brokkr_core::fold::fold(&events).unwrap();
    let cursor = brokkr_core::fold::Cursor::ExecuteEffect {
        effect_id: "effect-1".into(),
        seat: "work".into(),
        failed_attempts: 2,
    };
    engine.advance_running(&events, cursor, state).unwrap();
    last_park(&engine)
}

/// The newest `run/parked` the run's journal holds.
fn last_park(engine: &Engine) -> EventEnvelope {
    newest_park(engine).expect("the run parked")
}

/// The newest `run/parked` the run's journal holds, if it holds one.
fn newest_park(engine: &Engine) -> Option<EventEnvelope> {
    engine
        .store
        .load(&engine.run_id)
        .unwrap()
        .into_iter()
        .rfind(|event| event.event_type == EventType::RunParked)
}

fn start_failure() -> Value {
    json!({"start_failure": true, "start_failure_sites": [null]})
}

/// A deadline kill is `Failed` because the kill made non-completion
/// determinate (decision 0053 ruling 5), and writes no `start_failure`: the
/// cap it reaches is a determinate failure, never a start failure a budget
/// could retry.
#[test]
fn a_cap_whose_last_attempt_was_killed_at_its_deadline_reads_determinate() {
    let killed = json!({"error": "the attempt exceeded its 2s deadline and was killed"});
    let cause = cap_cause_after(EventType::EffectFailed, killed, 0);
    assert_eq!(cause["kind"], "attempt-cap", "{cause}");
    assert_eq!(cause["failure_class"], "determinate", "{cause}");
}

/// Decision 0106 ruling 1: every `run/parked` states its cause. The one
/// writer takes a `Park`, which cannot be built without one, and the
/// engine's general append refuses a raw `run/parked` that skips it.
#[test]
fn a_raw_run_parked_is_refused_and_the_one_writer_states_its_cause() {
    let (_dir, mut engine) = engine(single_body(vec!["x".into()]));
    let raw = json!({"reason": "held", "evidence": {}});
    let refused = engine
        .append(EventType::RunParked, raw, None)
        .expect_err("a park with no cause is not written");
    assert!(
        matches!(&refused, EngineError::Invariant("append", why) if why.contains("Engine::park")),
        "{refused:?}"
    );
    engine.park(&Park::ruled("held")).unwrap();
    assert_eq!(
        last_park(&engine).payload["cause"],
        json!({"kind": "ruled", "detail": "held"})
    );
}

#[test]
fn a_cap_reached_on_a_start_failure_reads_start_failure() {
    let cause = cap_cause_after(EventType::EffectFailed, start_failure(), 0);
    assert_eq!(cause["kind"], "attempt-cap", "{cause}");
    assert_eq!(cause["failure_class"], "start-failure", "{cause}");
}

#[test]
fn a_cap_whose_last_failure_recorded_a_wait_reads_provider_limit() {
    for extra in [
        json!({"start_failure": true, "start_failure_sites": [null],
               "waits": [{"provider": "claude", "route": "r", "words": "slow",
                          "seconds": 5, "until": "2030-01-01T00:00:00Z", "rounds": 1}]}),
        json!({"start_failure": true, "start_failure_sites": [null],
               "refused": {"provider": "claude", "cause": "route-unresolved",
                           "words": "slow"}}),
        // A record that is present and does not read is still a typed
        // refusal's record, never a start failure.
        json!({"start_failure": true, "start_failure_sites": [null], "waits": "garbled"}),
    ] {
        let cause = cap_cause_after(EventType::EffectFailed, extra.clone(), 0);
        assert_eq!(cause["kind"], "attempt-cap", "{extra}: {cause}");
        assert_eq!(cause["failure_class"], "provider-limit", "{extra}: {cause}");
    }
}

/// Ruling 1: the provider's words stay in the park's reason and the typed
/// `refused` record, and the cause does not copy them. The cap's detail is
/// the engine's own sentence, 0006's cap and the class, built from typed
/// counts: no seat error and no provider words, so a readout that paints the
/// reason and the cause paints the words once.
#[test]
fn a_caps_cause_never_copies_the_providers_words_or_the_seats_error() {
    const MARKER: &str = "MARKER-the-providers-own-words";
    let refused = json!({"start_failure": true, "start_failure_sites": [null],
        "refused": {"provider": "claude", "cause": "route-unresolved", "words": MARKER}});
    let park = cap_park_after(EventType::EffectFailed, refused, 0);
    let reason = park.payload["reason"].as_str().expect("the park's reason");
    assert!(reason.contains(MARKER), "{reason}");
    assert!(reason.contains("last error: x"), "{reason}");
    assert_eq!(
        park.payload["cause"],
        json!({"kind": "attempt-cap", "failure_class": "provider-limit",
               "detail": "effect effect-1 failed 2 of 2 attempt(s); \
                          the last failure class is provider-limit"})
    );
    let line = brokkr_core::CauseRead::of_event(&park).expect("the park states a cause");
    let painted = format!("{reason} / {}", line.line());
    assert_eq!(painted.matches(MARKER).count(), 1, "{painted}");
}

#[test]
fn a_cap_whose_last_attempt_executed_before_its_start_failure_reads_determinate() {
    // A sequence step that failed to start after an earlier step was
    // accepted and checkpointed: the field says start failure, the
    // journal says the attempt executed.
    let cause = cap_cause_after(EventType::EffectFailed, start_failure(), 1);
    assert_eq!(cause["kind"], "attempt-cap", "{cause}");
    assert_eq!(cause["failure_class"], "determinate", "{cause}");
}

#[test]
fn a_cap_whose_last_terminal_is_indeterminate_reads_infrastructure_not_retryable() {
    let cause = cap_cause_after(EventType::EffectIndeterminate, json!({}), 0);
    assert_eq!(cause["kind"], "infrastructure", "{cause}");
    assert_eq!(cause["retryable"], false, "{cause}");
}
