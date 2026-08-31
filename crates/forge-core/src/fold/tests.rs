use super::*;
use serde_json::json;

fn event(event_type: EventType, payload: Value) -> EventEnvelope {
    EventEnvelope {
        run_id: "r1".into(),
        seq: 2,
        event_id: "e2".into(),
        event_schema_version: 1,
        event_type,
        payload,
        causation_id: None,
        correlation_id: "r1".into(),
        attempt_id: None,
        recorded_at: "2026-08-23T00:00:00Z".into(),
        previous_hash: String::new(),
        event_hash: "hash".into(),
    }
}

fn state(cursor: Cursor) -> RunState {
    RunState {
        run_id: "r1".into(),
        seq: 1,
        last_hash: "hash".into(),
        status: Status::Running,
        phase: Some("work".into()),
        cursor,
        consecutive_failures: BTreeMap::new(),
        visits: BTreeMap::new(),
        last_result: None,
        reviewed_heads: None,
        last_decision: None,
        park_reason: None,
        feature: Some("feature".into()),
        pending_command: None,
        riding_stop: false,
    }
}

fn assert_out_of_place(cursor: Cursor, event_type: EventType, payload: Value) {
    let mut current = state(cursor);
    assert!(matches!(
        apply(&mut current, &event(event_type, payload)),
        Err(FoldError::OutOfPlace { .. })
    ));
}

#[test]
fn fold_and_apply_refuse_malformed_or_out_of_place_events() {
    assert!(matches!(
        fold(&[event(EventType::PhaseEntered, json!({"phase": "work"}))]),
        Err(FoldError::FirstEventNotRunStarted { seq: 2 })
    ));
    let mut current = state(Cursor::RequestEffect);
    assert!(matches!(
        apply(
            &mut current,
            &event(EventType::EffectRequested, json!({"seat": "work"}))
        ),
        Err(FoldError::BadPayload { .. })
    ));

    assert_out_of_place(Cursor::RequestEffect, EventType::RunStarted, json!({}));
    assert_out_of_place(
        Cursor::EnterPhase {
            phase: "expected".into(),
        },
        EventType::PhaseEntered,
        json!({"phase": "other"}),
    );
    assert_out_of_place(
        Cursor::RequestEffect,
        EventType::EffectStarted,
        json!({"effect_id": "e", "attempt_id": "a"}),
    );
    assert_out_of_place(
        Cursor::RequestEffect,
        EventType::EffectCheckpointed,
        json!({"effect_id": "e"}),
    );
    assert_out_of_place(
        Cursor::RequestEffect,
        EventType::EffectSucceeded,
        json!({"effect_id": "e", "result": {}}),
    );
    assert_out_of_place(
        Cursor::RequestEffect,
        EventType::EffectFailed,
        json!({"effect_id": "e"}),
    );
    assert_out_of_place(
        Cursor::RequestEffect,
        EventType::EffectIndeterminate,
        json!({"effect_id": "e"}),
    );
    assert_out_of_place(
        Cursor::RequestEffect,
        EventType::TransitionDecided,
        json!({}),
    );
    assert_out_of_place(Cursor::RequestEffect, EventType::RunParked, json!({}));
}

#[test]
fn operator_and_terminal_refusals_are_explicit() {
    let accepted = |id| event(EventType::OperatorAccepted, json!({"command_id": id}));

    let mut current = state(Cursor::Idle);
    current.status = Status::AwaitingOperator;
    assert_eq!(
        apply(&mut current, &accepted("c1")),
        Err(FoldError::NoMatchingCommand { seq: 2 })
    );

    for (pending, command, status, phase, expected) in [
        (
            Some(("other".into(), "retry".into())),
            "c1",
            Status::AwaitingOperator,
            Some("work".into()),
            "matching",
        ),
        (
            Some(("c1".into(), "retry".into())),
            "c1",
            Status::Running,
            Some("work".into()),
            "impossible",
        ),
        (
            Some(("c1".into(), "retry".into())),
            "c1",
            Status::AwaitingOperator,
            None,
            "impossible",
        ),
    ] {
        let mut current = state(Cursor::Idle);
        current.pending_command = pending;
        current.status = status;
        current.phase = phase;
        let error = apply(&mut current, &accepted(command)).unwrap_err();
        assert!(format!("{error}").contains(expected));
    }

    let mut current = state(Cursor::Idle);
    current.status = Status::AwaitingOperator;
    current.pending_command = Some(("c1".into(), "invented".into()));
    assert_eq!(
        apply(&mut current, &accepted("c1")),
        Err(FoldError::UnknownCommand {
            seq: 2,
            command: "invented".into(),
        })
    );

    for status in [Status::Completed, Status::Stopped] {
        let mut current = state(Cursor::Idle);
        current.status = status;
        assert_eq!(
            apply(&mut current, &event(EventType::OperatorRejected, json!({}))),
            Ok(())
        );
        assert_eq!(
            apply(&mut current, &event(EventType::RunCompleted, json!({}))),
            Err(FoldError::AfterTerminal { seq: 2 })
        );
    }

    for event_type in [EventType::RunCompleted, EventType::RunStopped] {
        let mut current = state(Cursor::Idle);
        current.status = Status::AwaitingOperator;
        assert!(matches!(
            apply(&mut current, &event(event_type, json!({}))),
            Err(FoldError::OutOfPlace { .. })
        ));
    }
}

/// Every refusal names the position it refused at. A fleet read cites
/// that number as the quarantined run's one stated fact, so a reader —
/// or the operator's aide — can go to the journal and check it.
#[test]
fn a_refusal_cites_the_position_it_refused_at() {
    assert_eq!(
        fold(&[]).unwrap_err().seq(),
        0,
        "nothing to cite but the start"
    );
    let mut current = state(Cursor::Idle);
    current.status = Status::AwaitingOperator;
    let error = apply(
        &mut current,
        &event(EventType::OperatorAccepted, json!({"command_id": "c1"})),
    )
    .unwrap_err();
    assert_eq!(error.seq(), 2, "the event the fold stopped on");
    // Every other refusal shape cites its own position too: a reader
    // must never be told to go and look at event 0.
    for refusal in [
        FoldError::FirstEventNotRunStarted { seq: 3 },
        FoldError::OutOfPlace {
            seq: 3,
            event: "OperatorAccepted".into(),
            cursor: "EffectInFlight".into(),
        },
        FoldError::BadPayload {
            seq: 3,
            field: "effect_id".into(),
        },
        FoldError::AfterTerminal { seq: 3 },
        FoldError::NoMatchingCommand { seq: 3 },
        FoldError::UnknownCommand {
            seq: 3,
            command: "invented".into(),
        },
    ] {
        assert_eq!(refusal.seq(), 3, "{refusal}");
    }
}

#[test]
fn decision_captures_reviewed_heads() {
    let mut current = state(Cursor::Decide {
        effect_id: "e".into(),
        result: json!({}),
    });
    apply(
        &mut current,
        &event(
            EventType::TransitionDecided,
            json!({
                "from": "review",
                "result": "clean",
                "next": "done",
                "inputs": {"reviewed_heads": {"repo": "abc"}},
            }),
        ),
    )
    .unwrap();
    assert_eq!(current.reviewed_heads, Some(json!({"repo": "abc"})));

    let mut without_heads = state(Cursor::Decide {
        effect_id: "e".into(),
        result: json!({}),
    });
    apply(
        &mut without_heads,
        &event(
            EventType::TransitionDecided,
            json!({"from":"review", "result":"clean", "next":"done", "inputs":{}}),
        ),
    )
    .unwrap();
    assert_eq!(without_heads.reviewed_heads, None);

    let mut without_inputs = state(Cursor::Decide {
        effect_id: "e".into(),
        result: json!({}),
    });
    apply(
        &mut without_inputs,
        &event(
            EventType::TransitionDecided,
            json!({"from":"review", "result":"clean", "next":"done"}),
        ),
    )
    .unwrap();
}

/// Decision 0022's two derived facts, and the park reason that must not
/// mislabel itself: the fold counts every `phase/entered`, keeps the raw
/// result of the last succeeded effect, and says WHICH rule parked a run
/// when a rule is what parked it.
#[test]
fn the_fold_counts_visits_keeps_the_last_result_and_names_the_rule_that_parked() {
    let mut current = state(Cursor::Start);
    for phase in ["work", "check", "work"] {
        current.cursor = Cursor::EnterPhase {
            phase: phase.into(),
        };
        apply(
            &mut current,
            &event(EventType::PhaseEntered, json!({"phase": phase})),
        )
        .unwrap();
    }
    assert_eq!(current.visits.get("work"), Some(&2));
    assert_eq!(current.visits.get("check"), Some(&1));
    assert_eq!(current.visits.get("ship"), None);

    let finding = json!({"result": "residual", "notes": "the handle came out short"});
    current.cursor = Cursor::EffectInFlight {
        effect_id: "e".into(),
        attempt_id: "a".into(),
        seat: "work".into(),
        failed_attempts: 0,
    };
    apply(
        &mut current,
        &event(
            EventType::EffectSucceeded,
            json!({"effect_id": "e", "attempt_id": "a", "result": finding}),
        ),
    )
    .unwrap();
    assert_eq!(current.last_result, Some(finding));

    // A named rule reached the park position on purpose. Saying "no
    // ruling" there would be exactly the mislabeling 0022 forbids.
    let parked = |payload: Value| {
        let mut current = state(Cursor::Decide {
            effect_id: "e".into(),
            result: json!({}),
        });
        apply(&mut current, &event(EventType::TransitionDecided, payload)).unwrap();
        match current.cursor {
            Cursor::Park { reason } => reason,
            other => panic!("expected a park, got {other:?}"),
        }
    };
    assert_eq!(
        parked(json!({
            "from": "review", "result": "residual", "rule_id": "REVIEW-REFORGE-EXHAUSTED-MEDIUM",
            "next": null, "problem": "a medium security residual survives",
        })),
        "REVIEW-REFORGE-EXHAUSTED-MEDIUM for (review, residual): \
         a medium security residual survives"
    );
    assert_eq!(
        parked(json!({"from": "review", "result": "novel", "rule_id": null, "next": null})),
        "no ruling for (review, novel): no rule matched"
    );
}

/// The operator's kill switch journals a stop AND its acceptance
/// without reading the run's cursor, so acceptance lands mid-flight.
/// The fold reads what the engine recorded: the command rides, the
/// attempt is untouched and keeps journaling, and the effect's boundary
/// concludes the run per the command.
#[test]
fn an_accepted_stop_rides_the_in_flight_attempt_to_the_effects_boundary() {
    let in_flight = || Cursor::EffectInFlight {
        effect_id: "open".into(),
        attempt_id: "attempt".into(),
        seat: "verify".into(),
        failed_attempts: 0,
    };
    let riding = || {
        let mut current = state(in_flight());
        current.pending_command = Some(("c1".into(), "stop".into()));
        apply(
            &mut current,
            &event(EventType::OperatorAccepted, json!({"command_id": "c1"})),
        )
        .unwrap();
        // Accepting it changes nothing about the attempt: same cursor,
        // still running, the command spent from pending and now riding.
        assert_eq!(current.cursor, in_flight());
        assert_eq!(current.status, Status::Running);
        assert_eq!(current.pending_command, None);
        assert!(current.riding_stop);
        current
    };

    // The in-flight attempt keeps journaling — checkpoints for the same
    // effect still apply normally after the acceptance.
    let mut current = riding();
    apply(
        &mut current,
        &event(EventType::EffectCheckpointed, json!({"effect_id": "open"})),
    )
    .unwrap();
    assert_eq!(current.cursor, in_flight());
    assert!(current.riding_stop, "still riding");

    // The evidenced boundary: the effect succeeds, and instead of its
    // normal `Decide` the run concludes per the stop.
    apply(
        &mut current,
        &event(
            EventType::EffectSucceeded,
            json!({"effect_id": "open", "result": {"result": "complete"}}),
        ),
    )
    .unwrap();
    assert_eq!(current.cursor, Cursor::Stop);
    assert!(!current.riding_stop, "the riding command is spent");

    // The same redirect at the other two boundaries of the same
    // attempt: a generalization of "the effect's boundary", not
    // separately evidenced. Neither retries nor parks past an accepted
    // stop — the operator's command is the run's conclusion.
    for (event_type, payload) in [
        (EventType::EffectFailed, json!({"effect_id": "open"})),
        (
            EventType::EffectIndeterminate,
            json!({"effect_id": "open", "reason": "engine restarted"}),
        ),
    ] {
        let mut current = riding();
        apply(&mut current, &event(event_type, payload)).unwrap();
        assert_eq!(current.cursor, Cursor::Stop, "{event_type:?}");
        assert!(!current.riding_stop);
    }
}

/// Nothing is in flight, so there is no boundary to ride to: the
/// operator's stop is the run's conclusion at the position it was
/// accepted. The parked case (the operator answering a park with
/// `stop`) was always legal; the running-between-effects cases are the
/// same sentence — an accepted stop that the engine must finish, not a
/// journal it may refuse to read.
#[test]
fn a_stop_accepted_with_nothing_in_flight_concludes_where_it_stands() {
    let concluded = |cursor: Cursor, status: Status| {
        let mut current = state(cursor);
        current.status = status;
        current.park_reason = Some("waiting on the operator".into());
        current.pending_command = Some(("c1".into(), "stop".into()));
        apply(
            &mut current,
            &event(EventType::OperatorAccepted, json!({"command_id": "c1"})),
        )
        .unwrap();
        current
    };

    // Every running position the engine can be standing at when the
    // kill switch fires with no attempt of its own in flight.
    for cursor in [
        Cursor::Start,
        Cursor::EnterPhase {
            phase: "review".into(),
        },
        Cursor::RequestEffect,
        Cursor::ExecuteEffect {
            effect_id: "open".into(),
            seat: "work".into(),
            failed_attempts: 1,
        },
        Cursor::Decide {
            effect_id: "open".into(),
            result: json!({"result": "complete"}),
        },
        Cursor::Park {
            reason: "no ruling".into(),
        },
    ] {
        let current = concluded(cursor.clone(), Status::Running);
        assert_eq!(current.cursor, Cursor::Stop, "{cursor:?}");
        assert_eq!(current.status, Status::Running, "{cursor:?}");
        assert!(
            !current.riding_stop,
            "nothing in flight to ride: {cursor:?}"
        );
        assert_eq!(current.pending_command, None, "{cursor:?}");
        assert_eq!(current.park_reason, None, "{cursor:?}");
    }

    // The parked case, unchanged: still running until run/stopped is
    // journaled, and the park it answered is spent.
    let parked = concluded(Cursor::Idle, Status::AwaitingOperator);
    assert_eq!(parked.cursor, Cursor::Stop);
    assert_eq!(parked.status, Status::Running);
    assert_eq!(parked.park_reason, None);
}

/// The stop arm is wide; the rest is as narrow as it was. Only a
/// command whose acceptance matches it is read at all, `retry` is still
/// the parked-only command it was, and an unknown one is named —
/// decision 0001 admits no guessed transition.
#[test]
fn the_mid_flight_arm_refuses_everything_it_is_not() {
    let in_flight = || Cursor::EffectInFlight {
        effect_id: "open".into(),
        attempt_id: "attempt".into(),
        seat: "verify".into(),
        failed_attempts: 0,
    };
    let accepted = event(EventType::OperatorAccepted, json!({"command_id": "c1"}));
    let refusal = |cursor: Cursor, pending: (&str, &str)| {
        let mut current = state(cursor);
        current.pending_command = Some((pending.0.into(), pending.1.into()));
        let error = apply(&mut current, &accepted).unwrap_err();
        assert!(!current.riding_stop, "a refused stop never rides");
        error
    };

    // An acceptance that names a different command is not this run's
    // stop, wherever the cursor stands.
    assert_eq!(
        refusal(in_flight(), ("other", "stop")),
        FoldError::NoMatchingCommand { seq: 2 }
    );
    // A command the protocol does not know is named as such.
    assert_eq!(
        refusal(in_flight(), ("c1", "invented")),
        FoldError::UnknownCommand {
            seq: 2,
            command: "invented".into(),
        }
    );
    // `retry` mid-flight is not evidenced and is not invented here —
    // resuming an attempt that is already running has no meaning, and
    // unlike `stop` there is no conclusion the journal is owed.
    assert!(matches!(
        refusal(in_flight(), ("c1", "retry")),
        FoldError::OutOfPlace { .. }
    ));
    // `retry` at a running cursor with nothing to retry is refused the
    // same way: only a park can be retried out of.
    assert!(matches!(
        refusal(Cursor::RequestEffect, ("c1", "retry")),
        FoldError::OutOfPlace { .. }
    ));
}

#[test]
fn open_effect_guards_refuse_foreign_effects_in_the_same_cursor_shape() {
    let in_flight = || Cursor::EffectInFlight {
        effect_id: "open".into(),
        attempt_id: "attempt".into(),
        seat: "work".into(),
        failed_attempts: 0,
    };
    let executing = || Cursor::ExecuteEffect {
        effect_id: "open".into(),
        seat: "work".into(),
        failed_attempts: 0,
    };
    assert_out_of_place(
        executing(),
        EventType::EffectStarted,
        json!({"effect_id":"foreign", "attempt_id":"attempt"}),
    );
    assert_out_of_place(
        in_flight(),
        EventType::EffectCheckpointed,
        json!({"effect_id":"foreign"}),
    );
    assert_out_of_place(
        in_flight(),
        EventType::EffectSucceeded,
        json!({"effect_id":"foreign", "result":{}}),
    );
    assert_out_of_place(
        in_flight(),
        EventType::EffectFailed,
        json!({"effect_id":"foreign"}),
    );
    assert_out_of_place(
        in_flight(),
        EventType::EffectIndeterminate,
        json!({"effect_id":"foreign"}),
    );
    assert_out_of_place(
        executing(),
        EventType::EffectIndeterminate,
        json!({"effect_id":"foreign"}),
    );
}
