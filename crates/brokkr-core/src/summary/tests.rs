use super::*;
use crate::envelope::EventType;
use crate::envelope_builder::EnvelopeBuilder;
use crate::residual::residuals;
use serde_json::{json, Value};

const LISTING: Listing<'static> = Listing {
    feature: "\n   \n  Fix the flaky gate  \nwith the detail below",
    created_at: "2026-01-01T00:00:00Z",
};

/// A journal of `steps`, seq 1 onward, each stamped a second after the
/// last and sealed.
pub(super) fn journal(steps: &[(EventType, Value)]) -> Vec<EventEnvelope> {
    steps
        .iter()
        .enumerate()
        .map(|(at, (event_type, payload))| {
            let seq = at as u64 + 1;
            EnvelopeBuilder::new(*event_type, payload.clone())
                .seq(seq)
                .event_id(format!("e{seq}"))
                .at(format!("2026-01-01T00:00:{seq:02}Z"))
                .sealed()
        })
        .collect()
}

fn opening() -> Vec<(EventType, Value)> {
    vec![
        (
            EventType::RunStarted,
            json!({"feature": "Fix the flaky gate", "manifest": {}}),
        ),
        (EventType::PhaseEntered, json!({"phase": "verify"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "e1", "seat": "verifier"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "e1", "attempt_id": "a1"}),
        ),
    ]
}

/// A run verified with two residuals, reviewed into a park by a ruling
/// that carried a third, and its first ruling's findings superseded.
pub(super) fn parked() -> Vec<EventEnvelope> {
    let mut steps = opening();
    steps.extend([
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "e1", "result": {"result": "pass"}}),
        ),
        (
            EventType::TransitionDecided,
            json!({"from": "verify", "result": "pass", "rule_id": "VERIFY-PASS", "next": "review",
                   "inputs": {"max_residual_severity": "high", "has_security_residual": true}}),
        ),
        (EventType::PhaseEntered, json!({"phase": "review"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "e2", "seat": "reviewer"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "e2", "attempt_id": "a2"}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "e2", "result": {"result": "hold"}}),
        ),
        (
            EventType::TransitionDecided,
            json!({"from": "review", "result": "hold", "rule_id": "REVIEW-HOLD",
                   "problem": "held", "inputs": {"max_residual_severity": "low"}}),
        ),
        (
            EventType::RunParked,
            json!({"reason": "REVIEW-HOLD for (review, hold): held"}),
        ),
        (
            EventType::OperatorCommanded,
            json!({"command_id": "c1", "command": "supersede", "operator": "op",
                   "args": {"by": {"run_id": "other", "seq": 3}, "findings": [6],
                            "reason": "fixed there"}}),
        ),
    ]);
    journal(&steps)
}

/// The aggregates of a run started at `started_ms` that names no realm or
/// recipe, has not ended, and has no review that routed it.
fn unread(started_ms: Option<i64>) -> Aggregate {
    Aggregate {
        realm: None,
        recipe: None,
        started_ms,
        finished: None,
        implement_visits: 0,
        review_first: false,
        review_second: false,
        review_back: false,
    }
}

fn bounded(text: &str) -> Bounded {
    Bounded {
        cut: text.to_string(),
        whole: None,
    }
}

#[test]
fn a_cut_ends_at_a_character_boundary_within_its_bound() {
    assert_eq!(cut("abc", 3), "abc");
    assert_eq!(cut("abcd", 3), "abc");
    assert_eq!(cut("aé", 3), "aé");
    assert_eq!(
        cut("aé", 2),
        "a",
        "é is two bytes, and a cut inside it steps back"
    );
    assert_eq!(cut("a€", 3), "a", "€ is three bytes");
    assert_eq!(cut("", 0), "");
}

#[test]
fn the_feature_line_is_the_first_that_says_anything_cut_to_its_bound() {
    assert_eq!(feature_line(LISTING.feature), "Fix the flaky gate");
    assert_eq!(feature_line("\n \n"), "");
    let long = "x".repeat(FEATURE_LINE_BYTES + 7);
    assert_eq!(feature_line(&long), &long[..FEATURE_LINE_BYTES]);
}

#[test]
fn a_bounded_value_keeps_the_whole_only_when_its_bound_cut_it() {
    assert_eq!(Bounded::within("review", NAME_BYTES), bounded("review"));
    let exact = "n".repeat(NAME_BYTES);
    assert_eq!(Bounded::within(&exact, NAME_BYTES), bounded(&exact));
    let long = "n".repeat(NAME_BYTES + 1);
    assert_eq!(
        Bounded::within(&long, NAME_BYTES),
        Bounded {
            cut: exact,
            whole: Some(long),
        }
    );
}

#[test]
fn a_run_with_no_event_reads_as_an_empty_journal_at_the_zero_head() {
    let empty = Account::empty("r1");
    assert_eq!(empty.run_id(), "r1");
    assert_eq!(empty.head(), (0, ZERO_HASH));
    assert_eq!(empty.state().unwrap_err(), &FoldError::Empty);
    assert_eq!(
        Summary::of(&empty, LISTING),
        Summary {
            feature_line: "Fix the flaky gate".into(),
            created_at: bounded("2026-01-01T00:00:00Z"),
            feature: LISTING.feature.into(),
            status: None,
            stop_accepted_seq: None,
            phase: None,
            park_cause: None,
            park_reason: None,
            ruled: None,
            hired: None,
            residual_open: None,
            residual_count: 0,
            residuals: "[]".into(),
            refused: Some(Refused {
                seq: 0,
                words: bounded("journal is empty"),
            }),
            folded_seq: 0,
            folded_hash: ZERO_HASH.into(),
            last_at_ms: Some(1_767_225_600_000),
            head_seq: 0,
            head_hash: ZERO_HASH.into(),
            last_recorded_at: None,
            aggregate: unread(None),
            closing: None,
            late_tail: None,
        }
    );
}

#[test]
fn a_parked_run_reads_its_ruling_its_reason_and_its_open_residuals() {
    let events = parked();
    let account = Account::whole("run", &events);
    let head = &events[12];
    assert_eq!(account.head(), (13, head.event_hash.as_str()));
    assert_eq!(account.state().unwrap().seq, 13);
    let mark = json!({"seq": 13, "by": {"realm": null, "run_id": "other", "seq": 3},
        "reason": "fixed there", "operator": "op", "recorded_at": "2026-01-01T00:00:13Z"});
    let residuals = json!([
        {"seq": 6, "phase": "verify", "rule_id": "VERIFY-PASS",
         "input": "has_security_residual", "value": "true", "superseded": mark},
        {"seq": 6, "phase": "verify", "rule_id": "VERIFY-PASS",
         "input": "max_residual_severity", "value": "high", "superseded": mark},
        {"seq": 11, "phase": "review", "rule_id": "REVIEW-HOLD",
         "input": "max_residual_severity", "value": "low", "superseded": null},
    ]);
    assert_eq!(
        Summary::of(&account, LISTING),
        Summary {
            feature_line: "Fix the flaky gate".into(),
            created_at: bounded("2026-01-01T00:00:00Z"),
            feature: LISTING.feature.into(),
            status: Some(Status::AwaitingOperator),
            stop_accepted_seq: None,
            phase: Some(bounded("review")),
            park_cause: None,
            park_reason: Some(bounded("REVIEW-HOLD for (review, hold): held")),
            ruled: Some(Ruled {
                rule_id: Some(bounded("REVIEW-HOLD")),
                from: bounded("review"),
                routes: false,
            }),
            hired: None,
            residual_open: Some(Severity::Low),
            residual_count: 1,
            residuals: residuals.to_string(),
            refused: None,
            folded_seq: 13,
            folded_hash: head.event_hash.clone(),
            last_at_ms: Some(1_767_225_613_000),
            head_seq: 13,
            head_hash: head.event_hash.clone(),
            last_recorded_at: Some(bounded("2026-01-01T00:00:13Z")),
            aggregate: unread(Some(1_767_225_601_000)),
            closing: None,
            late_tail: None,
        }
    );
}

/// The `residuals` column reads back into the findings that wrote it, and
/// a key no finding states refuses (decision 0089 ruling 2).
#[test]
fn the_residuals_column_reads_back_as_its_findings() {
    let column = Summary::of(&Account::whole("run", &parked()), LISTING).residuals;
    let read: Vec<Stated> = serde_json::from_str(&column).unwrap();
    assert_eq!(read.len(), 3);
    assert_eq!(serde_json::to_value(&read).unwrap().to_string(), column);
    let mark = read[0].superseded.as_ref().unwrap();
    assert_eq!((mark.seq, mark.by.run_id.as_str()), (13, "other"));
    assert_eq!(read[0].claim, Claim::SecurityResidual);
    assert_eq!(read[1].claim, Claim::MaxSeverity(Severity::High));
    assert_eq!(read[2].phase, ResidualPhase::Review);
    let refused = |from: &str, to: &str| {
        let edited = column.replacen(from, to, 1);
        serde_json::from_str::<Vec<Stated>>(&edited)
            .unwrap_err()
            .to_string()
    };
    let extra = refused("\"input\"", "\"extra\":1,\"input\"");
    assert!(extra.starts_with("unknown field `extra`"), "{extra}");
    let phase = refused("\"phase\":\"verify\"", "\"phase\":\"design\"");
    assert!(
        phase.starts_with("phase 'design' carries no residual finding"),
        "{phase}"
    );
    let value = refused("\"value\":\"high\"", "\"value\":\"none\"");
    let words = "'max_residual_severity: none' is no claim a finding states";
    assert!(value.starts_with(words), "{value}");
    let flag = refused("\"value\":\"true\"", "\"value\":\"yes\"");
    let words = "'has_security_residual: yes' is no claim a finding states";
    assert!(flag.starts_with(words), "{flag}");
}

/// A finding's rule id is read once, from its ruling, as text or not at
/// all, so the summary and every readout of its findings name it alike.
#[test]
fn a_finding_names_its_rule_as_its_ruling_words_it() {
    let mut events = parked();
    let ruled = events
        .iter_mut()
        .rfind(|event| event.event_type == EventType::TransitionDecided)
        .unwrap();
    ruled.payload["rule_id"] = json!(7);
    let held = residuals(&events);
    let stated: Vec<Stated> = held.iter().map(|held| Stated::of(held, None)).collect();
    let rules: Vec<Option<&str>> = stated.iter().map(|it| it.rule_id.as_deref()).collect();
    assert_eq!(rules, [Some("VERIFY-PASS"), Some("VERIFY-PASS"), None]);
}

#[test]
fn an_account_carried_event_by_event_is_the_account_read_whole() {
    let events = parked();
    let whole = Summary::of(&Account::whole("run", &events), LISTING);
    let mut carried = Account::empty("run");
    for event in &events {
        carried = carried.onto(std::slice::from_ref(event)).account();
    }
    assert_eq!(Summary::of(&carried, LISTING), whole);
    let halves = Account::whole("run", &events[..6])
        .onto(&events[6..])
        .account();
    assert_eq!(Summary::of(&halves, LISTING), whole);
    let unmoved = Account::whole("run", &events).onto(&[]).account();
    assert_eq!(Summary::of(&unmoved, LISTING), whole);
}

/// Decision 0064 addendum choice 5: the account notes its ending's
/// reading event by event, so a summary carried over a run's events
/// states the ending, citation and stood-over fact the one reading gives
/// the run read whole; a live run has none.
#[test]
fn an_account_carries_the_ending_the_run_read_whole_states() {
    let mut steps = opening();
    let cause = json!({"cause": "superseded", "by": {"realm": "next-door", "run_id": "later"}});
    steps.extend([
        (
            EventType::OperatorCommanded,
            json!({"command_id": "s", "command": "stop", "operator": "op", "args": cause}),
        ),
        (EventType::OperatorAccepted, json!({"command_id": "s"})),
        (EventType::RunStopped, json!({})),
    ]);
    let events = journal(&steps);
    let mut carried = Account::empty("run");
    for event in &events {
        carried = carried.onto(std::slice::from_ref(event)).account();
    }
    let superseded = Some(Closing {
        ended: crate::ending::Ended {
            ending: crate::ending::Ending::Superseded,
            cited: Some(crate::ending::Citation::Run {
                realm: Some("next-door".into()),
                run_id: "later".into(),
            }),
        },
        stood_over: false,
    });
    let state = carried.state().unwrap().clone();
    assert_eq!(Summary::of(&carried, LISTING).closing, superseded);
    assert_eq!(crate::ending::closing(&events, &state), superseded);
    let live = Account::whole("run", &events[..4]);
    assert_eq!(Summary::of(&live, LISTING).closing, None);
}

#[test]
fn a_seat_at_work_is_hired_while_its_effect_is_requested_or_in_flight() {
    let events = journal(&opening());
    let requested = Account::whole("run", &events[..3]);
    let hired = Some(Hired {
        seat: bounded("verifier"),
        failed_attempts: 0,
    });
    assert_eq!(Summary::of(&requested, LISTING).hired, hired);
    let in_flight = requested.onto(&events[3..]).account();
    let summary = Summary::of(&in_flight, LISTING);
    assert_eq!(summary.hired, hired);
    assert_eq!(summary.status, Some(Status::Running));
    assert_eq!(summary.ruled, None);
    assert_eq!(summary.park_reason, None);
    let entered = Account::whole("run", &events[..2]);
    assert_eq!(Summary::of(&entered, LISTING).hired, None);
}

#[test]
fn a_ruling_that_routes_on_is_read_as_routing() {
    let events = parked();
    let summary = Summary::of(&Account::whole("run", &events[..7]), LISTING);
    assert_eq!(
        summary.ruled,
        Some(Ruled {
            rule_id: Some(bounded("VERIFY-PASS")),
            from: bounded("verify"),
            routes: true,
        })
    );
    assert_eq!(summary.residual_open, Some(Severity::High));
    assert_eq!(summary.residual_count, 2);
}

#[test]
fn a_refusal_stands_as_the_journal_moves_past_it() {
    let mut steps = opening();
    steps.insert(
        1,
        (
            EventType::EffectStarted,
            json!({"effect_id": "e1", "attempt_id": "a"}),
        ),
    );
    let events = journal(&steps);
    let whole = Account::whole("run", &events);
    let refusal = FoldError::OutOfPlace {
        seq: 2,
        event: "EffectStarted".into(),
        cursor: "Start".into(),
    };
    assert_eq!(whole.state().unwrap_err(), &refusal);
    let summary = Summary::of(&whole, LISTING);
    assert_eq!(
        summary.refused,
        Some(Refused {
            seq: 2,
            words: bounded(&refusal.to_string()),
        })
    );
    assert_eq!((summary.status, &summary.hired), (None, &None));
    assert_eq!(summary.folded_seq, 5);
    let carried = Account::whole("run", &events[..2])
        .onto(&events[2..])
        .account();
    assert_eq!(Summary::of(&carried, LISTING), summary);
}

#[test]
fn a_value_past_its_bound_is_cut_and_kept_whole() {
    let phase = "p".repeat(NAME_BYTES + 6);
    let mut steps = opening();
    steps[1] = (EventType::PhaseEntered, json!({"phase": phase}));
    let summary = Summary::of(&Account::whole("run", &journal(&steps[..2])), LISTING);
    assert_eq!(
        summary.phase,
        Some(Bounded {
            cut: phase[..NAME_BYTES].to_string(),
            whole: Some(phase.clone()),
        })
    );
    let long_stamp = format!("{}{}", LISTING.created_at, "z".repeat(STAMP_BYTES));
    let listing = Listing {
        created_at: &long_stamp,
        ..LISTING
    };
    let empty = Summary::of(&Account::empty("run"), listing);
    assert_eq!(empty.created_at.cut, &long_stamp[..STAMP_BYTES]);
    assert_eq!(
        empty.last_at_ms, None,
        "a stamp that does not parse orders as none"
    );
}

/// Decision 0108 ruling 4, F7: a late classify and a late supersede
/// inside a repaired tail are kept and shown as the tail, never applied by
/// the scans that derive a figure — the shown ending, the open residual
/// and the findings' marks are exactly the terminal-only read's.
#[test]
fn a_late_classify_and_supersede_change_no_figure_the_summary_states() {
    fn append(events: &mut Vec<EventEnvelope>, event_type: EventType, payload: Value) {
        let seq = events.len() as u64 + 1;
        let previous = events
            .last()
            .map_or_else(String::new, |event| event.event_hash.clone());
        events.push(
            EnvelopeBuilder::new(event_type, payload)
                .run("run")
                .seq(seq)
                .event_id(format!("e{seq}"))
                .at(format!("2026-01-02T00:00:{seq:02}Z"))
                .previous(previous)
                .sealed(),
        );
    }

    let mut base = parked();
    append(
        &mut base,
        EventType::OperatorCommanded,
        json!({"command_id": "c-stop", "command": "stop", "operator": "operator",
               "args": {}}),
    );
    append(
        &mut base,
        EventType::OperatorAccepted,
        json!({"command_id": "c-stop", "operator": "operator", "reason": "concluded"}),
    );
    append(
        &mut base,
        EventType::RunStopped,
        json!({"reason": "concluded"}),
    );
    let terminal = base.len() as u64;
    let mut repaired = base.clone();
    append(
        &mut repaired,
        EventType::OperatorCommanded,
        json!({"command_id": "c-classify", "command": "classify",
               "operator": "operator",
               "args": {"cause": "withdrawn", "reason": "late"}}),
    );
    append(
        &mut repaired,
        EventType::OperatorCommanded,
        json!({"command_id": "c-supersede", "command": "supersede",
               "operator": "operator",
               "args": {"findings": [11],
                        "by": {"realm": null, "run_id": "later", "seq": 1},
                        "reason": "late"}}),
    );
    append(
        &mut repaired,
        EventType::OperatorCommanded,
        json!({"command_id": "c-repair", "command": "repair",
               "operator": "operator",
               "args": {"first_seq": terminal + 1, "last_seq": terminal + 2,
                        "terminal_seq": terminal,
                        "event_types": ["operator/commanded", "operator/commanded"],
                        "reason": "kept"}}),
    );
    let state = crate::fold::fold(&repaired).expect("the repaired run folds");
    assert_eq!(state.run.status(), Status::Stopped);
    assert_eq!(
        state.late_tail.as_ref().map(|tail| tail.last_seq),
        Some(terminal + 2)
    );

    let before = Summary::of(&Account::whole("run", &base), LISTING);
    let after = Summary::of(&Account::whole("run", &repaired), LISTING);
    assert_eq!(after.status, before.status);
    assert_eq!(
        after.closing, before.closing,
        "the late classify moves no ending"
    );
    assert_eq!(
        after.residual_open, before.residual_open,
        "the late supersede closes no finding"
    );
    assert_eq!(after.residual_count, before.residual_count);
    assert_eq!(after.residuals, before.residuals);
    assert_eq!(
        crate::ending::closing(&repaired, &state),
        before.closing,
        "the one closing read skips the late tail too"
    );
    assert!(after.late_tail.is_some(), "the tail is still shown");
}

/// The test helper's guard: a cut these tests designed cannot meet a
/// repair naming a carried event, so its `Refold` arm panics rather than
/// handing back an account that is not the carried one. Production
/// callers match [`Carried`] and refold whole; `Account::whole` reads the
/// same body and never meets the signal.
#[test]
#[should_panic(expected = "the caller must refold the whole journal")]
fn the_test_account_helper_refuses_a_refold() {
    let _ = Carried::Refold.account();
}

/// The summary of a parked run whose park states `cause`.
fn parked_with(cause: Value) -> Summary {
    let mut events = parked();
    let park = events
        .iter_mut()
        .find(|event| event.event_type == EventType::RunParked)
        .unwrap();
    park.payload["cause"] = cause;
    Summary::of(&Account::whole("run", &events), LISTING)
}

/// The summary row carries the latest park's typed cause from the event,
/// while the fold itself still reads only the reason (decision 0106
/// ruling 1).
#[test]
fn the_summary_carries_the_parks_typed_cause() {
    let summary = parked_with(
        json!({"kind": "attempt-cap", "failure_class": "start-failure",
        "detail": "two attempts failed"}),
    );
    let cause = summary.park_cause.expect("the row carries the cause");
    assert!(matches!(
        *cause,
        crate::CauseRead::Stated(crate::ParkCause::AttemptCap { .. })
    ));
    assert_eq!(
        cause.line(),
        "attempt-cap (start-failure): two attempts failed"
    );
    assert!(summary
        .park_reason
        .as_ref()
        .is_some_and(|reason| !reason.cut.is_empty()));
}

/// The row keeps a cause's sentence within the bound the reason beside it
/// takes (PARK_REASON_BYTES, 256), because the reader bounds it as a
/// writer does (`ParkCause::read`), so the row cuts nothing itself and every
/// readout holds the same bytes. The largest detail written here is
/// 10,000 bytes of two-byte characters, which cuts at a character boundary
/// at 256 bytes; a detail within the bound is stored as written.
#[test]
fn the_summary_bounds_a_causes_sentence_as_it_bounds_the_reason() {
    let long = "é".repeat(5_000);
    let summary = parked_with(
        json!({"kind": "attempt-cap", "failure_class": "determinate",
                                     "detail": long}),
    );
    let cause = summary.park_cause.expect("the row carries the cause");
    let crate::CauseRead::Stated(crate::ParkCause::AttemptCap { detail, .. }) = *cause else {
        panic!("the cap's cause is stated");
    };
    assert_eq!(detail.len(), PARK_REASON_BYTES);
    assert!(long.starts_with(&detail));
    let short = "e".repeat(PARK_REASON_BYTES);
    let kept = parked_with(json!({"kind": "ruled", "detail": short}));
    assert_eq!(
        kept.park_cause.map(|cause| cause.line()),
        Some(format!("ruled: {short}"))
    );
    let refused = parked_with(json!({"kind": "x".repeat(10_000)}));
    let Some(crate::CauseRead::Refused(why)) = refused.park_cause.map(|cause| *cause) else {
        panic!("a cause that does not read is refused by name");
    };
    assert_eq!(why.len(), PARK_REASON_BYTES);
}

/// A park whose present `cause` does not read is carried by the row as its
/// named refusal, never as a park that stated none.
#[test]
fn the_summary_carries_a_cause_that_does_not_read_as_its_named_refusal() {
    let summary = parked_with(json!({"kind": "not-a-cause"}));
    let cause = summary.park_cause.expect("the row carries the refusal");
    let crate::CauseRead::Refused(why) = *cause else {
        panic!("a malformed cause is a refusal, not a cause or an absence");
    };
    assert!(why.contains("not-a-cause"), "{why}");
}
