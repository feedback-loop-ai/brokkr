use super::*;
use crate::envelope_builder::EnvelopeBuilder;
use crate::run_state::{run_state, standing};
use serde_json::json;

fn event(event_type: EventType, payload: Value) -> EventEnvelope {
    EnvelopeBuilder::new(event_type, payload)
        .run("r1")
        .seq(2)
        .event_id("e2")
        .at("2026-08-23T00:00:00Z")
        .previous("")
        .hash("hash")
        .build()
}

fn state(cursor: Cursor) -> RunState {
    at(Run::Running(cursor))
}

/// A run standing in `run`, otherwise as [`state`] builds it.
fn at(run: Run) -> RunState {
    let mut state = run_state("r1", 1, run);
    state.phase = Some("work".into());
    state.feature = Some("feature".into());
    state
}

/// A run parked for the operator.
fn parked() -> Run {
    with_status(Status::AwaitingOperator)
}

/// The run each status names, a parked one waiting on the operator.
fn with_status(status: Status) -> Run {
    standing(status, "waiting on the operator")
}

/// A run standing in `run` whose operator stop `c1` is accepted.
fn stop_accepted(run: Run) -> RunState {
    let mut current = at(run);
    current.pending_command = Some(("c1".into(), "stop".into()));
    let accepted = event(EventType::OperatorAccepted, json!({"command_id": "c1"}));
    apply(&mut current, &accepted).unwrap();
    current
}

/// An attempt `a1` of effect `e1` in flight for seat `work`.
fn in_flight(stop_rides: bool) -> Cursor {
    Cursor::EffectInFlight {
        effect_id: "e1".into(),
        attempt_id: "a1".into(),
        seat: "work".into(),
        failed_attempts: 0,
        stop_rides,
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

    let mut current = at(parked());
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
        let mut current = at(with_status(status));
        current.pending_command = pending;
        current.phase = phase;
        let error = apply(&mut current, &accepted(command)).unwrap_err();
        assert!(format!("{error}").contains(expected));
    }

    let mut current = at(parked());
    current.pending_command = Some(("c1".into(), "invented".into()));
    assert_eq!(
        apply(&mut current, &accepted("c1")),
        Err(FoldError::UnknownCommand {
            seq: 2,
            command: "invented".into(),
        })
    );

    for run in [Run::Completed, Run::Stopped] {
        let mut current = at(run);
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
        let mut current = at(parked());
        assert!(matches!(
            apply(&mut current, &event(event_type, json!({}))),
            Err(FoldError::OutOfPlace { .. })
        ));
    }
}

/// `acceptance_refusal` is the one rule the fold and the engine both
/// read: it names each condition an acceptance would fail on, and every
/// word the journal records for a command or a refusal is pinned here.
#[test]
fn the_acceptance_rule_names_each_condition_and_pins_its_word() {
    use OperatorCommand::{Release, Retry, Stop};
    let parked = Status::AwaitingOperator;
    for (status, phase, command, expected) in [
        (
            Status::Stopped,
            Some("work"),
            Release,
            Some(Refusal::AfterTerminal),
        ),
        (parked, None, Release, None),
        (Status::Running, Some("work"), Release, None),
        (
            Status::Completed,
            Some("work"),
            Stop,
            Some(Refusal::AfterTerminal),
        ),
        (
            Status::Stopped,
            Some("work"),
            Retry,
            Some(Refusal::AfterTerminal),
        ),
        (
            Status::Running,
            Some("work"),
            Retry,
            Some(Refusal::RunNotAwaitingOperator),
        ),
        (parked, None, Retry, Some(Refusal::NoPhaseToRetry)),
        (parked, Some("work"), Retry, None),
        (parked, None, Stop, None),
        (Status::Running, None, Stop, None),
    ] {
        let mut current = at(with_status(status));
        current.phase = phase.map(str::to_string);
        let refusal = acceptance_refusal(&current, command);
        assert_eq!(refusal, expected, "{status:?} {phase:?} {command:?}");
    }

    assert_eq!(
        OperatorCommand::ALL.map(OperatorCommand::as_str),
        ["retry", "stop", "release"]
    );
    for command in OperatorCommand::ALL {
        assert_eq!(OperatorCommand::parse(command.as_str()), Some(command));
    }
    assert_eq!(OperatorCommand::parse("supersede"), None);
    let refusals = [
        Refusal::CommandNotAllowed,
        Refusal::AfterTerminal,
        Refusal::RunNotAwaitingOperator,
        Refusal::NoPhaseToRetry,
        Refusal::LostFence,
        Refusal::StaleCursor,
        Refusal::IncompleteCommandReplay,
        Refusal::PreviouslyRejected,
    ];
    assert_eq!(
        refusals.map(Refusal::word),
        [
            "command_not_allowed",
            "after_terminal",
            "run_not_awaiting_operator",
            "no_phase_to_retry",
            "lost_fence",
            "stale_cursor",
            "incomplete_command_replay",
            "previously_rejected",
        ]
    );
}

/// A status reads as one word on every surface, and in every refusal
/// that names it (#351).
#[test]
fn a_status_reads_as_its_one_word() {
    assert_eq!(Status::Running.as_str(), "running");
    assert_eq!(Status::AwaitingOperator.as_str(), "awaiting_operator");
    assert_eq!(Status::Completed.as_str(), "completed");
    assert_eq!(Status::Stopped.as_str(), "stopped");
}

/// A summary row's status word reads back as the status that wrote it,
/// and a word no status takes reads as none (decision 0089 ruling 2).
#[test]
fn a_status_word_reads_back_as_its_status() {
    let all = [
        Status::Running,
        Status::AwaitingOperator,
        Status::Completed,
        Status::Stopped,
    ];
    for status in all {
        assert_eq!(Status::named(status.as_str()), Some(status));
    }
    assert_eq!(Status::named("parked"), None);
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
    let mut current = at(parked());
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

/// The refusal names the cursor it met, as the operator will look for it
/// in the journal (#419).
#[test]
fn an_out_of_place_refusal_names_the_cursor_it_met() {
    let mut current = at(parked());
    assert_eq!(
        apply(
            &mut current,
            &event(EventType::TransitionDecided, json!({}))
        ),
        Err(FoldError::OutOfPlace {
            seq: 2,
            event: "TransitionDecided".into(),
            cursor: "Idle".into(),
        })
    );
}

/// A cursor displays its variant and the ids and count that place it,
/// the bytes `Debug` showed for those, and never a seat's result, a
/// park's free text or a riding stop (#345).
#[test]
fn a_cursor_displays_its_variant_and_ids_never_a_result() {
    let execute = Cursor::ExecuteEffect {
        effect_id: "e1".into(),
        seat: "work".into(),
        failed_attempts: 1,
    };
    let enter = Cursor::EnterPhase {
        phase: "work".into(),
    };
    for (cursor, shown) in [
        (Cursor::Start, "Start"),
        (enter, r#"EnterPhase { phase: "work" }"#),
        (Cursor::RequestEffect, "RequestEffect"),
        (
            execute,
            r#"ExecuteEffect { effect_id: "e1", seat: "work", failed_attempts: 1 }"#,
        ),
        (Cursor::Stop, "Stop"),
    ] {
        assert_eq!(cursor.to_string(), shown);
        assert_eq!(format!("{cursor:?}"), shown);
    }
    for stop_rides in [false, true] {
        assert_eq!(
            in_flight(stop_rides).to_string(),
            r#"EffectInFlight { effect_id: "e1", attempt_id: "a1", seat: "work", failed_attempts: 0 }"#
        );
        // A nonzero count is shown as counted, whether a stop rides or not.
        let mut retried = in_flight(stop_rides);
        if let Cursor::EffectInFlight {
            failed_attempts, ..
        } = &mut retried
        {
            *failed_attempts = 2;
        }
        assert_eq!(
            retried.to_string(),
            r#"EffectInFlight { effect_id: "e1", attempt_id: "a1", seat: "work", failed_attempts: 2 }"#
        );
    }
    let decide = Cursor::Decide {
        effect_id: "e1".into(),
        result: json!({"result": "complete", "notes": "a seat's words"}),
    };
    assert_eq!(decide.to_string(), r#"Decide { effect_id: "e1" }"#);
    let park = Cursor::Park {
        reason: ParkReason::read("effect e1 indeterminate: a seat's words".into()),
    };
    assert_eq!(park.to_string(), "Park");
}

/// A run is one of four states (#345): the status word is the state's,
/// only a running run has a cursor, only a parked one a reason (a running
/// run at `Park` has not parked yet), and every state but running shows
/// as `Idle`, as readouts always printed it.
#[test]
fn a_run_is_one_of_four_states_and_names_its_status_cursor_and_reason() {
    let reason = ParkReason::read("attempts exhausted".into());
    let parking = Cursor::Park {
        reason: reason.clone(),
    };
    for (run, status, cursor, park_reason, shown) in [
        (
            Run::Running(in_flight(true)),
            Status::Running,
            Some(in_flight(true)),
            None,
            r#"EffectInFlight { effect_id: "e1", attempt_id: "a1", seat: "work", failed_attempts: 0 }"#,
        ),
        (
            Run::Running(parking.clone()),
            Status::Running,
            Some(parking),
            None,
            "Park",
        ),
        (
            Run::AwaitingOperator {
                reason: reason.clone(),
            },
            Status::AwaitingOperator,
            None,
            Some(&reason),
            "Idle",
        ),
        (Run::Completed, Status::Completed, None, None, "Idle"),
        (Run::Stopped, Status::Stopped, None, None, "Idle"),
    ] {
        assert_eq!(run.status(), status, "{run:?}");
        assert_eq!(run.cursor(), cursor.as_ref(), "{run:?}");
        assert_eq!(run.park_reason(), park_reason, "{run:?}");
        assert_eq!(run.to_string(), shown, "{run:?}");
    }
}

/// A park reason reads back the words it was read from, and only the
/// marker alone, exactly, is the gate that moved HEAD (#345).
#[test]
fn a_park_reason_reads_back_its_own_words() {
    assert_eq!(
        ParkReason::read("GATE-MOVED-HEAD".into()),
        ParkReason::GateMovedHead
    );
    assert_eq!(ParkReason::GateMovedHead.as_str(), "GATE-MOVED-HEAD");
    for words in [
        "GATE-MOVED-HEAD ",
        "GATE-MOVED-HEAD {}",
        "gate-moved-head",
        "effect e1 indeterminate: engine restarted",
        "",
    ] {
        let reason = ParkReason::read(words.into());
        assert_eq!(reason, ParkReason::Stated(words.into()));
        assert_eq!(reason.as_str(), words);
    }
}

/// An indeterminate whose reason carries the GATE-MOVED-HEAD marker parks
/// on the marker alone, whether or not its evidence reads; any other
/// reason, the marker glued to its evidence included, parks on its
/// detail (#345).
#[test]
fn a_marked_indeterminate_parks_on_the_marker() {
    let park_reason = |reason: &str| {
        let mut current = state(in_flight(false));
        let payload = json!({"effect_id": "e1", "attempt_id": "a1", "reason": reason});
        apply(
            &mut current,
            &event(EventType::EffectIndeterminate, payload),
        )
        .unwrap();
        current.run
    };
    let marked = gate_moved_head::encode(&gate_moved_head::MovedHead {
        head_at_end: Some("bbb".into()),
        head_at_start: Some("aaa".into()),
    });
    for reason in [marked.as_str(), "GATE-MOVED-HEAD not json"] {
        assert_eq!(
            park_reason(reason),
            Run::Running(Cursor::Park {
                reason: ParkReason::GateMovedHead
            })
        );
    }
    assert_eq!(
        park_reason("GATE-MOVED-HEAD{}"),
        Run::Running(Cursor::Park {
            reason: ParkReason::Stated("effect e1 indeterminate: GATE-MOVED-HEAD{}".into())
        })
    );
}

/// A `fail` is a failure only where the table scoped a counter to it
/// (`inputs.consecutive_failures`); any other result under that scope
/// resets the count (#419).
#[test]
fn a_scoped_fail_counts_and_a_scoped_pass_resets() {
    let decide = |result: &str| {
        let mut current = state(Cursor::Decide {
            effect_id: "e".into(),
            result: json!({}),
        });
        current.consecutive_failures.insert("verify".into(), 1);
        let decided = json!({"from": "verify", "result": result, "next": "done",
                             "inputs": {"consecutive_failures": 2}});
        apply(&mut current, &event(EventType::TransitionDecided, decided)).unwrap();
        current.consecutive_failures["verify"]
    };
    assert_eq!(decide("fail"), 2);
    assert_eq!(decide("pass"), 0);
}

/// The consecutive-failure rule has one home (#345): the universal
/// failures always count, and `fail` counts exactly where the counter is
/// read.
#[test]
fn the_failure_rule_counts_universal_failures_and_a_read_fail() {
    for counter_read in [true, false] {
        assert!(counts_as_failure("failed", counter_read));
        assert!(counts_as_failure("broken", counter_read));
        assert!(!counts_as_failure("complete", counter_read));
    }
    assert!(counts_as_failure("fail", true));
    assert!(!counts_as_failure("fail", false));
}

/// A decision outside its closed contract fails the fold closed, naming
/// what does not read (#345).
#[test]
fn a_decision_outside_its_contract_fails_the_fold() {
    let mut current = state(Cursor::Decide {
        effect_id: "e".into(),
        result: json!({}),
    });
    let decided = json!({"from": "verify", "result": "pass", "next": 5});
    assert_eq!(
        apply(&mut current, &event(EventType::TransitionDecided, decided)),
        Err(FoldError::BadDecision {
            seq: 2,
            detail: "invalid type: integer `5`, expected a string".into(),
        })
    );
    assert_eq!(
        FoldError::BadDecision {
            seq: 2,
            detail: "unknown field `x`".into(),
        }
        .to_string(),
        "event 2: transition/decided payload does not read: unknown field `x`"
    );
    assert_eq!(
        FoldError::BadDecision {
            seq: 9,
            detail: String::new(),
        }
        .seq(),
        9
    );
}

/// A severity written as serde's one-key map form of a variant fails
/// the fold at its seq rather than folding as the word it wraps
/// (decision 0084 rulings 1 and 2).
#[test]
fn a_decision_severity_in_map_form_fails_the_fold() {
    for word in crate::policy::RULING_SEVERITIES {
        let mut current = state(Cursor::Decide {
            effect_id: "e".into(),
            result: json!({}),
        });
        let decided = json!({"from": "verify", "result": "pass", "severity": { word: null }});
        assert_eq!(
            apply(&mut current, &event(EventType::TransitionDecided, decided)),
            Err(FoldError::BadDecision {
                seq: 2,
                detail: "invalid type: map, expected a ruling severity string".into(),
            }),
            "{word}"
        );
        assert_eq!(current.last_decision, None, "{word} folded");
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
    assert_eq!(
        current.reviewed_heads,
        Some(ReviewedHeads::read(&json!({"repo": "abc"})))
    );

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
        current.run = Run::Running(Cursor::EnterPhase {
            phase: phase.into(),
        });
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
    current.run = Run::Running(in_flight(false));
    apply(
        &mut current,
        &event(
            EventType::EffectSucceeded,
            json!({"effect_id": "e1", "attempt_id": "a1", "result": finding}),
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
        match current.run {
            Run::Running(Cursor::Park { reason }) => reason.as_str().to_string(),
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
    let riding = || {
        let current = stop_accepted(Run::Running(in_flight(false)));
        // Accepting it changes nothing about the attempt: same attempt,
        // still running, the command spent from pending and now riding.
        assert_eq!(current.run, Run::Running(in_flight(true)));
        assert_eq!(current.pending_command, None);
        assert_eq!(current.stopping_seq(), Some(2), "the accepted stop's seq");
        current
    };

    // The in-flight attempt keeps journaling — checkpoints for the same
    // effect still apply normally after the acceptance, still riding.
    let mut current = riding();
    apply(
        &mut current,
        &event(EventType::EffectCheckpointed, json!({"effect_id": "e1"})),
    )
    .unwrap();
    assert_eq!(current.run, Run::Running(in_flight(true)));
    assert_eq!(current.stopping_seq(), Some(2), "the stop still rides");

    // The ending lands: nothing is stopping any more.
    let mut ended = current.clone();
    apply(&mut ended, &event(EventType::RunStopped, json!({}))).unwrap();
    assert_eq!(ended.run, Run::Stopped);
    assert_eq!(ended.stopping_seq(), None);

    // The evidenced boundary: the effect succeeds, and instead of its
    // normal `Decide` the run concludes per the stop, the riding
    // command spent with the attempt.
    apply(
        &mut current,
        &event(
            EventType::EffectSucceeded,
            json!({"effect_id": "e1", "result": {"result": "complete"}}),
        ),
    )
    .unwrap();
    assert_eq!(current.run, Run::Running(Cursor::Stop));

    // The same redirect at the other two boundaries of the same
    // attempt: a generalization of "the effect's boundary", not
    // separately evidenced. Neither retries nor parks past an accepted
    // stop — the operator's command is the run's conclusion.
    for (event_type, payload) in [
        (EventType::EffectFailed, json!({"effect_id": "e1"})),
        (
            EventType::EffectIndeterminate,
            json!({"effect_id": "e1", "reason": "engine restarted"}),
        ),
    ] {
        let mut current = riding();
        apply(&mut current, &event(event_type, payload)).unwrap();
        assert_eq!(current.run, Run::Running(Cursor::Stop), "{event_type:?}");
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
            reason: ParkReason::read("no ruling".into()),
        },
    ] {
        let current = stop_accepted(Run::Running(cursor.clone()));
        assert_eq!(current.run, Run::Running(Cursor::Stop), "{cursor:?}");
        assert_eq!(current.pending_command, None, "{cursor:?}");
    }

    // The parked case, unchanged: still running until run/stopped is
    // journaled, and the park it answered is spent.
    let parked = stop_accepted(parked());
    assert_eq!(parked.run, Run::Running(Cursor::Stop));
}

/// U8e3: an accepted `release` disposes of its command and moves nothing
/// else, wherever a live run stands: a parked run stays parked, and an
/// attempt in flight stays in flight, its stop not riding.
#[test]
fn an_accepted_release_moves_no_run() {
    for run in [parked(), Run::Running(in_flight(false))] {
        let mut current = at(run.clone());
        current.pending_command = Some(("c1".into(), "release".into()));
        let accepted = event(EventType::OperatorAccepted, json!({"command_id": "c1"}));
        apply(&mut current, &accepted).unwrap();
        assert_eq!((current.run, current.pending_command), (run, None));
    }
}

/// The stop arm is wide; the rest is as narrow as it was. Only a
/// command whose acceptance matches it is read at all, `retry` is still
/// the parked-only command it was, and an unknown one is named —
/// decision 0001 admits no guessed transition.
#[test]
fn the_mid_flight_arm_refuses_everything_it_is_not() {
    let accepted = event(EventType::OperatorAccepted, json!({"command_id": "c1"}));
    let refusal = |cursor: Cursor, pending: (&str, &str)| {
        let mut current = state(cursor.clone());
        current.pending_command = Some((pending.0.into(), pending.1.into()));
        let error = apply(&mut current, &accepted).unwrap_err();
        assert_eq!(
            current.run,
            Run::Running(cursor),
            "a refused stop never rides"
        );
        error
    };

    // An acceptance that names a different command is not this run's
    // stop, wherever the cursor stands.
    assert_eq!(
        refusal(in_flight(false), ("other", "stop")),
        FoldError::NoMatchingCommand { seq: 2 }
    );
    // A command the protocol does not know is named as such.
    assert_eq!(
        refusal(in_flight(false), ("c1", "invented")),
        FoldError::UnknownCommand {
            seq: 2,
            command: "invented".into(),
        }
    );
    // `retry` mid-flight is not evidenced and is not invented here —
    // resuming an attempt that is already running has no meaning, and
    // unlike `stop` there is no conclusion the journal is owed.
    assert!(matches!(
        refusal(in_flight(false), ("c1", "retry")),
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
        stop_rides: false,
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

/// Decision 0047 rulings 1 and 3: `supersede` is an annotation the fold
/// already admits and must never read. A stopped run folded with one
/// appended derives exactly the state it derives without it — same
/// status, same phase, same cursor, same ruling, same park reason, and
/// nothing new anywhere in `RunState`.
///
/// The two fields left out of the comparison are the journal's own
/// position rather than derived facts about the run: `seq` and
/// `last_hash` advance for EVERY appended event, including the
/// `operator/commanded` this arm admitted long before this decision.
/// They are asserted to have moved by exactly the one event, which is
/// the other half of the claim.
#[test]
fn a_supersede_annotation_folds_a_stopped_run_to_the_same_state() {
    let at = |seq: u64, event_type, payload| {
        let mut event = event(event_type, payload);
        event.seq = seq;
        event.event_id = format!("e{seq}");
        event.event_hash = format!("hash-{seq}");
        event
    };
    let mut stopped = vec![
        at(
            1,
            EventType::RunStarted,
            json!({"feature": "close the finding by name"}),
        ),
        at(2, EventType::PhaseEntered, json!({"phase": "review"})),
        at(
            3,
            EventType::EffectRequested,
            json!({"effect_id": "eff", "phase": "review", "seat": "review"}),
        ),
        at(
            4,
            EventType::EffectStarted,
            json!({"effect_id": "eff", "attempt_id": "att"}),
        ),
        at(
            5,
            EventType::EffectSucceeded,
            json!({"effect_id": "eff", "attempt_id": "att",
                   "result": {"result": "residual"}}),
        ),
        at(
            6,
            EventType::TransitionDecided,
            json!({"from": "review", "result": "residual", "next": null,
                   "rule_id": "REVIEW-SECURITY-HOLD", "severity": null,
                   "inputs": {"has_security_residual": true},
                   "problem": "security residual above the shipping bar"}),
        ),
        at(
            7,
            EventType::RunStopped,
            json!({"reason": "security residual above the shipping bar"}),
        ),
    ];
    let before = fold(&stopped).expect("the stopped run folds");
    stopped.push(at(
        8,
        EventType::OperatorCommanded,
        json!({"command_id": "cmd-1", "command": "supersede",
               "operator": "operator",
               "args": {"findings": [6],
                        "by": {"realm": null, "run_id": "later", "seq": 21},
                        "reason": "both residuals fixed and shipped"}}),
    ));
    let after = fold(&stopped).expect("the annotation folds on a terminal run");

    assert_eq!(
        after.seq, 8,
        "the journal advanced by exactly the one event"
    );
    assert_eq!(after.last_hash, "hash-8");
    let derived = |state: &RunState| {
        let mut state = state.clone();
        state.seq = 0;
        state.last_hash = String::new();
        format!("{state:?}")
    };
    assert_eq!(
        derived(&before),
        derived(&after),
        "the fold derives nothing at all from a supersede annotation"
    );
    assert_eq!(after.run, Run::Stopped, "a mark is not a conclusion");
    assert!(
        after.pending_command.is_none(),
        "an annotation on a terminal run is never held as pending"
    );
}

/// Every event type, in declaration order.
const EVENT_TYPES: [EventType; 15] = {
    use EventType::*;
    [
        RunStarted,
        PhaseEntered,
        EffectRequested,
        EffectStarted,
        EffectCheckpointed,
        EffectSucceeded,
        EffectFailed,
        EffectIndeterminate,
        TransitionDecided,
        OperatorCommanded,
        OperatorAccepted,
        OperatorRejected,
        RunParked,
        RunCompleted,
        RunStopped,
    ]
};

/// Every position, in declaration order.
const POSITIONS: [Position; 10] = {
    use Position::*;
    [
        Start,
        EnterPhase,
        RequestEffect,
        ExecuteEffect,
        EffectInFlight,
        Decide,
        Park,
        Stop,
        Idle,
        Terminal,
    ]
};

/// A run standing at `position`, holding effect `e` and phase `work`,
/// with an operator stop pending.
fn standing_at(position: Position) -> RunState {
    let mut current = at(match position {
        Position::Start => Run::Running(Cursor::Start),
        Position::EnterPhase => Run::Running(Cursor::EnterPhase {
            phase: "work".into(),
        }),
        Position::RequestEffect => Run::Running(Cursor::RequestEffect),
        Position::ExecuteEffect => Run::Running(Cursor::ExecuteEffect {
            effect_id: "e".into(),
            seat: "work".into(),
            failed_attempts: 0,
        }),
        Position::EffectInFlight => Run::Running(Cursor::EffectInFlight {
            effect_id: "e".into(),
            attempt_id: "a".into(),
            seat: "work".into(),
            failed_attempts: 0,
            stop_rides: false,
        }),
        Position::Decide => Run::Running(Cursor::Decide {
            effect_id: "e".into(),
            result: json!({"result": "complete"}),
        }),
        Position::Park => Run::Running(Cursor::Park {
            reason: ParkReason::read("no ruling".into()),
        }),
        Position::Stop => Run::Running(Cursor::Stop),
        Position::Idle => parked(),
        Position::Terminal => Run::Completed,
    });
    current.pending_command = Some(("c1".into(), "stop".into()));
    current
}

/// An event of `event_type` that names what [`standing_at`] holds.
fn well_formed(event_type: EventType) -> Value {
    use EventType::*;
    match event_type {
        RunStarted | OperatorRejected | RunCompleted | RunStopped => json!({}),
        PhaseEntered => json!({"phase": "work"}),
        EffectRequested => json!({"effect_id": "e", "seat": "work"}),
        EffectStarted => json!({"effect_id": "e", "attempt_id": "a"}),
        EffectCheckpointed | EffectFailed => json!({"effect_id": "e"}),
        EffectSucceeded => json!({"effect_id": "e", "result": {}}),
        EffectIndeterminate => json!({"effect_id": "e", "reason": "engine restarted"}),
        TransitionDecided => json!({"from": "work", "result": "complete", "next": "done"}),
        OperatorCommanded => json!({"command_id": "c2", "command": "stop"}),
        OperatorAccepted => json!({"command_id": "c1"}),
        RunParked => json!({"reason": "attempts exhausted"}),
    }
}

/// Decision 0050 ruling 6: the relation answers every (event type,
/// position) pair, and the fold answers each pair as the relation says.
/// The expected answers are stated here as the transitions and where
/// each is taken; every other running pair is out of place, and a
/// terminal run admits only the two operator annotations.
#[test]
fn the_relation_answers_every_pair_and_the_fold_answers_by_it() {
    use EventType::*;
    use Position::{
        Decide, EffectInFlight, EnterPhase, ExecuteEffect, Park, RequestEffect, Start, Terminal,
    };
    let running = &POSITIONS[..9];
    let moves: [(EventType, Transition, &[Position]); 14] = [
        (PhaseEntered, Transition::Enter, &[Start, EnterPhase]),
        (EffectRequested, Transition::Request, &[RequestEffect]),
        (EffectStarted, Transition::Start, &[ExecuteEffect]),
        (
            EffectCheckpointed,
            Transition::Checkpoint,
            &[EffectInFlight],
        ),
        (EffectSucceeded, Transition::Succeed, &[EffectInFlight]),
        (EffectFailed, Transition::Fail, &[EffectInFlight]),
        (
            EffectIndeterminate,
            Transition::Close,
            &[ExecuteEffect, EffectInFlight],
        ),
        (TransitionDecided, Transition::Rule, &[Decide]),
        (OperatorCommanded, Transition::Command, running),
        (OperatorAccepted, Transition::Accept, running),
        (OperatorRejected, Transition::Reject, running),
        (
            RunParked,
            Transition::Park,
            &[Start, EnterPhase, ExecuteEffect, Park],
        ),
        (RunCompleted, Transition::Complete, &running[..8]),
        (RunStopped, Transition::Stop, &running[..8]),
    ];
    for (index, event_type) in EVENT_TYPES.into_iter().enumerate() {
        assert_eq!(event_type as usize, index);
    }
    assert_eq!(EVENT_TYPES.len(), RunStopped as usize + 1);
    for (index, position) in POSITIONS.into_iter().enumerate() {
        assert_eq!(position as usize, index);
        assert_eq!(Position::of(&standing_at(position)), position);
    }
    assert_eq!(POSITIONS.len(), Terminal as usize + 1);

    let mut tally = BTreeMap::new();
    for event_type in EVENT_TYPES {
        for position in POSITIONS {
            let moved = moves
                .iter()
                .find(|(moving, _, at)| *moving == event_type && at.contains(&position));
            let expected = match (position, moved) {
                (Terminal, _) if matches!(event_type, OperatorCommanded | OperatorRejected) => {
                    Arm::Annotates
                }
                (Terminal, _) => Arm::AfterTerminal,
                (_, Some((_, transition, _))) => Arm::Moves(*transition),
                (_, None) => Arm::OutOfPlace,
            };
            let pair = format!("{event_type:?} at {position:?}");
            assert_eq!(relation::arm(event_type, position), expected, "{pair}");

            let before = standing_at(position);
            let mut current = before.clone();
            let answer = apply(&mut current, &event(event_type, well_formed(event_type)));
            let class = match expected {
                Arm::Moves(_) => {
                    assert_eq!(answer, Ok(()), "{pair}");
                    "moves"
                }
                Arm::OutOfPlace => {
                    let refusal = FoldError::OutOfPlace {
                        seq: 2,
                        event: format!("{event_type:?}"),
                        cursor: before.run.to_string(),
                    };
                    assert_eq!(answer, Err(refusal), "{pair}");
                    "out of place"
                }
                Arm::Annotates => {
                    assert_eq!(answer, Ok(()), "{pair}");
                    assert_eq!(format!("{current:?}"), format!("{before:?}"), "{pair}");
                    "annotates"
                }
                Arm::AfterTerminal => {
                    assert_eq!(answer, Err(FoldError::AfterTerminal { seq: 2 }), "{pair}");
                    "after terminal"
                }
            };
            *tally.entry(class).or_insert(0) += 1;
        }
    }
    assert_eq!(
        tally,
        BTreeMap::from([
            ("after terminal", 13),
            ("annotates", 2),
            ("moves", 57),
            ("out of place", 78),
        ])
    );

    let mut stopped = standing_at(Terminal);
    stopped.run = Run::Stopped;
    assert_eq!(Position::of(&stopped), Terminal);
}

/// An event that is both malformed and out of place is refused for the
/// field it names its target by, as the fold's arms always read it first;
/// an event that names no target first is refused for its place.
#[test]
fn a_malformed_event_out_of_place_is_refused_for_its_payload_first() {
    use EventType::*;
    let named_first = [
        (PhaseEntered, Some("phase")),
        (EffectStarted, Some("effect_id")),
        (EffectSucceeded, Some("effect_id")),
        (EffectFailed, Some("effect_id")),
        (EffectIndeterminate, Some("effect_id")),
        (RunStarted, None),
        (EffectRequested, None),
        (EffectCheckpointed, None),
        (TransitionDecided, None),
        (RunParked, None),
    ];
    for (event_type, field) in named_first {
        let mut current = state(Cursor::Stop);
        let answer = apply(&mut current, &event(event_type, json!({})));
        let expected = match field {
            Some(field) => FoldError::BadPayload {
                seq: 2,
                field: field.into(),
            },
            None => FoldError::OutOfPlace {
                seq: 2,
                event: format!("{event_type:?}"),
                cursor: "Stop".into(),
            },
        };
        assert_eq!(answer, Err(expected), "{event_type:?}");
    }
    let mut current = state(Cursor::Stop);
    assert_eq!(
        apply(
            &mut current,
            &event(EffectStarted, json!({"effect_id": "e"}))
        ),
        Err(FoldError::BadPayload {
            seq: 2,
            field: "attempt_id".into(),
        })
    );
}

/// `concluded` is the rule `conclude` refuses by and the view reads: a
/// completed or stopped run is concluded, a live or parked one is not,
/// and the refusal names the run and its status in one sentence.
#[test]
fn a_run_is_concluded_only_once_it_completed_or_stopped() {
    for (status, expected) in [
        (Status::Running, None),
        (Status::AwaitingOperator, None),
        (Status::Completed, Some(Status::Completed)),
        (Status::Stopped, Some(Status::Stopped)),
    ] {
        let current = at(with_status(status));
        assert_eq!(concluded(&current), expected, "{status:?}");
    }
    assert_eq!(
        concluded_text("encrypt-the-token-store-41d0a9c2", Status::Stopped),
        "run 'encrypt-the-token-store-41d0a9c2' is already concluded (stopped); conclude appends nothing"
    );
    assert_eq!(
        concluded_text("r", Status::Completed),
        "run 'r' is already concluded (completed); conclude appends nothing"
    );
}

#[test]
fn panel_carry_extension_fields_do_not_change_the_fold() {
    use EventType::*;
    let mut journal: Vec<_> = [
        (RunStarted, json!({"feature":"synthetic panel"})),
        (PhaseEntered, json!({"phase":"work"})),
        (
            EffectRequested,
            json!({"phase":"work","seat":"work","effect_id":"e1"}),
        ),
        (EffectStarted, json!({"effect_id":"e1","attempt_id":"a1"})),
        (
            EffectIndeterminate,
            json!({"effect_id":"e1","attempt_id":"a1","reason":"b vanished"}),
        ),
        (RunParked, json!({"reason":"b vanished"})),
        (
            OperatorCommanded,
            json!({"command":"retry","command_id":"c1","args":{}}),
        ),
        (OperatorAccepted, json!({"command_id":"c1"})),
        (
            EffectRequested,
            json!({"phase":"work","seat":"work","effect_id":"e2"}),
        ),
        (EffectStarted, json!({"effect_id":"e2","attempt_id":"a2"})),
        (
            EffectFailed,
            json!({"effect_id":"e2","attempt_id":"a2","error":"b failed"}),
        ),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (kind, payload))| {
        let mut event = event(kind, payload);
        event.seq = index as u64 + 1;
        event
    })
    .collect();
    let original = format!("{:?}", fold(&journal).unwrap());
    for index in [4, 10] {
        journal[index].payload["tree"] = json!({"at_start":"clean","at_end":"clean"});
        journal[index].payload["head"] = json!({"at_start":"a".repeat(40),"at_end":"a".repeat(40)});
        journal[index].payload["settled_members"] = json!({"a":{"result":"clean"}});
    }
    journal[6].payload["args"] = json!({"members":["b"]});
    journal[9].payload["carried"] = json!([{"member":"a","attempt_id":"a1"}]);
    assert_eq!(format!("{:?}", fold(&journal).unwrap()), original);
}
