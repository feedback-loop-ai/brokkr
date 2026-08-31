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
        reviewed_heads: None,
        last_decision: None,
        park_reason: None,
        feature: Some("feature".into()),
        pending_command: None,
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
