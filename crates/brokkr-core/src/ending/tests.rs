use super::*;
use crate::envelope_builder::EnvelopeBuilder;
use crate::fold::{Cursor, Run};
use crate::run_state::{run_state, standing};
use serde_json::{json, Value};

/// A ruling from `from` on `result`, carrying `severity` when one is
/// given.
fn ruling(from: &str, result: &str, severity: Option<&str>) -> Value {
    let mut ruling = json!({"from": from, "result": result, "rule_id": "R", "next": "done"});
    if let Some(severity) = severity {
        ruling["severity"] = json!(severity);
    }
    ruling
}

/// `rulings` journaled in order, each as a `transition/decided`.
fn decided(rulings: &[Value]) -> Vec<EventEnvelope> {
    let decided =
        |ruling: &Value| EnvelopeBuilder::new(EventType::TransitionDecided, ruling.clone());
    rulings
        .iter()
        .map(|ruling| decided(ruling).build())
        .collect()
}

/// A run folded to `run`, at `phase`, its last ruling `last`.
fn folded(run: Run, phase: &str, last: Option<Value>) -> RunState {
    let mut state = run_state("run", 9, run);
    state.phase = Some(phase.to_string());
    state.last_decision = last.map(|ruling| serde_json::from_value(ruling).unwrap());
    state
}

/// The ending `events` and `state` give, which every reading test reads
/// as a terminal run's.
fn ending(events: &[EventEnvelope], state: &RunState) -> Ending {
    let ended = ended(events, state).expect("a terminal run has an ending");
    assert_eq!(ended.cited, None, "nothing cites before E2 and E3");
    ended.ending
}

#[test]
fn a_policy_stop_reads_each_row_of_the_table() {
    let rows = [
        (("implement", "blocked"), Ending::Blocked),
        (("implement", "broken"), Ending::Failed),
        (("verify", "fail"), Ending::Failed),
        (("architecture", "failed"), Ending::Failed),
        (("design", "fail"), Ending::Failed),
        (("review", "security-hold"), Ending::Refused),
        (("review", "residual"), Ending::Refused),
        (("review", "clean-unverified"), Ending::Refused),
        (("ship", "ready"), Ending::Blocked),
        (("ship", "shipped"), Ending::Blocked),
        (("verify", "stack-conflict"), Ending::Blocked),
        (("verify", "env-degraded"), Ending::Infrastructure),
    ];
    for ((from, result), expected) in rows {
        assert_eq!(Ending::ruled(from, result), expected, "{from} {result}");
    }
}

#[test]
fn the_first_row_that_matches_rules_and_nothing_else_is_classified() {
    // `blocked` and a failing result outrank review's any-result row.
    assert_eq!(Ending::ruled("review", "blocked"), Ending::Blocked);
    assert_eq!(Ending::ruled("review", "fail"), Ending::Failed);
    assert_eq!(Ending::ruled("ship", "broken"), Ending::Failed);
    // A row's result read off its phase, and a phase off its results,
    // match nothing; neither does a pair no row names.
    for (from, result) in [
        ("implement", "ready"),
        ("implement", "stack-conflict"),
        ("implement", "env-degraded"),
        ("ship", "dirty"),
        ("verify", "pass"),
        ("intake", "complete"),
        ("implement", "exhausted"),
    ] {
        assert_eq!(
            Ending::ruled(from, result),
            Ending::Unclassified,
            "{from} {result}"
        );
    }
}

#[test]
fn a_completed_run_splits_on_its_last_review_severity() {
    assert_eq!(
        Ending::completed(Some(RuleSeverity::Flagged)),
        Ending::ShippedFlagged
    );
    for last in [Some(RuleSeverity::Normal), Some(RuleSeverity::Hard), None] {
        assert_eq!(Ending::completed(last), Ending::Shipped, "{last:?}");
    }
}

#[test]
fn tone_gutter_wants_and_ownership_are_functions_of_the_ending_alone() {
    use Ending::*;
    let (ruling, action) = (Some(Wants::Ruling), Some(Wants::Action));
    let table = [
        (Shipped, Tone::Green, ' ', None, false),
        (ShippedFlagged, Tone::Green, '~', None, false),
        (Refused, Tone::Yellow, '!', ruling, false),
        (Failed, Tone::Red, ' ', None, false),
        (Blocked, Tone::Cyan, '>', action, false),
        (Exhausted, Tone::Yellow, '!', ruling, true),
        (Superseded, Tone::Dim, ' ', None, true),
        (Infrastructure, Tone::Cyan, '>', action, true),
        (DeliveredElsewhere, Tone::Dim, ' ', None, true),
        (Withdrawn, Tone::Dim, ' ', None, true),
        (OperatorStopped, Tone::Dim, ' ', None, true),
        (Unclassified, Tone::Dim, '?', None, false),
    ];
    // Every ending once, in ruling 2's order: each mapping below takes
    // the ending and nothing else, so this row is all it can say.
    assert_eq!(table.map(|(ending, ..)| ending), Ending::ALL);
    for (ending, tone, gutter, wants, owned) in table {
        let painted = (ending.tone(), ending.gutter(), ending.wants());
        assert_eq!(painted, (tone, gutter, wants), "{ending:?}");
        assert_eq!(ending.operator_owned(), owned, "{ending:?}");
    }
}

#[test]
fn the_twelve_words_round_trip_and_a_word_outside_them_is_refused() {
    let words = [
        "shipped",
        "shipped-flagged",
        "refused",
        "failed",
        "blocked",
        "exhausted",
        "superseded",
        "infrastructure",
        "delivered-elsewhere",
        "withdrawn",
        "operator-stopped",
        "unclassified",
    ];
    assert_eq!(Ending::ALL.map(Ending::word), words);
    for ending in Ending::ALL {
        assert_eq!(Ending::named(ending.word()), Ok(ending));
    }
    // `running` is a status, never an ending, and a word is read as
    // written.
    for outside in ["running", "Shipped", "shipped_flagged", ""] {
        let refused = UnknownEnding {
            word: outside.into(),
        };
        assert_eq!(Ending::named(outside), Err(refused));
    }
    assert_eq!(
        Ending::named("running").unwrap_err().to_string(),
        "'running' is not an ending; an ending is one of shipped, shipped-flagged, refused, \
         failed, blocked, exhausted, superseded, infrastructure, delivered-elsewhere, \
         withdrawn, operator-stopped, unclassified"
    );
}

#[test]
fn a_live_parked_or_stopping_run_has_no_ending() {
    let events = decided(&[ruling("review", "clean", Some("normal"))]);
    let mut stopping = folded(Run::Running(Cursor::Stop), "implement", None);
    stopping.stop_accepted_seq = Some(8);
    for state in [
        folded(standing(Status::Running, ""), "implement", None),
        folded(
            standing(Status::AwaitingOperator, "parked"),
            "implement",
            None,
        ),
        stopping,
    ] {
        assert_eq!(ended(&events, &state), None, "{:?}", state.run);
    }
}

#[test]
fn a_completed_run_reads_its_last_ruling_from_review_and_no_other() {
    let done = folded(Run::Completed, "done", None);
    let flagged = ruling("review", "residual", Some("flagged"));
    let clean = ruling("review", "clean", Some("normal"));
    let unmarked = ruling("review", "clean", None);
    let ship_drift = ruling("ship", "drift", Some("flagged"));
    let cases = [
        (vec![], Ending::Shipped),
        (vec![flagged.clone()], Ending::ShippedFlagged),
        (vec![clean.clone(), flagged.clone()], Ending::ShippedFlagged),
        (vec![flagged.clone(), clean.clone()], Ending::Shipped),
        (vec![flagged.clone(), unmarked], Ending::Shipped),
        // A flagged route from another phase is not read, either way.
        (vec![clean, ship_drift.clone()], Ending::Shipped),
        (vec![flagged, ship_drift], Ending::ShippedFlagged),
    ];
    for (rulings, expected) in cases {
        assert_eq!(ending(&decided(&rulings), &done), expected, "{rulings:?}");
    }
}

#[test]
fn an_event_that_is_no_ruling_leaves_the_reading_as_it_was() {
    let done = folded(Run::Completed, "done", None);
    let mut events = decided(&[ruling("review", "residual", Some("flagged"))]);
    // Another event's payload is never read as a ruling, and a ruling
    // that does not read as one (the fold refuses it) moves nothing.
    let entered = json!({"from": "review", "result": "clean", "severity": "normal"});
    events.push(EnvelopeBuilder::new(EventType::PhaseEntered, entered).build());
    let unreadable = json!({"from": "review", "severity": "normal"});
    events.push(EnvelopeBuilder::new(EventType::TransitionDecided, unreadable).build());
    assert_eq!(ending(&events, &done), Ending::ShippedFlagged);
}

#[test]
fn a_policy_stop_reads_the_ruling_that_stopped_it_and_never_its_rule_id() {
    let stopped = |last| folded(Run::Stopped, "stop", last);
    let held = ruling("review", "security-hold", Some("hard"));
    assert_eq!(ending(&[], &stopped(Some(held))), Ending::Refused);
    let blocked = ruling("implement", "blocked", Some("hard"));
    assert_eq!(ending(&[], &stopped(Some(blocked))), Ending::Blocked);
    // fast's exhausted verify loop is judged work that did not pass.
    let mut exhausted = ruling("verify", "fail", Some("hard"));
    exhausted["rule_id"] = json!("VERIFY-FAIL-EXHAUSTED");
    assert_eq!(ending(&[], &stopped(Some(exhausted))), Ending::Failed);
    let unnamed = ruling("intake", "complete", Some("hard"));
    assert_eq!(ending(&[], &stopped(Some(unnamed))), Ending::Unclassified);
    assert_eq!(ending(&[], &stopped(None)), Ending::Unclassified);
}

#[test]
fn an_operator_closure_with_no_typed_cause_is_unclassified() {
    // Stopped anywhere but the table's stop, whatever ruled last: the
    // cause that would name it is not in any journal yet.
    let last = ruling("review", "security-hold", Some("hard"));
    for state in [
        folded(Run::Stopped, "implement", None),
        folded(Run::Stopped, "review", Some(last)),
    ] {
        assert_eq!(
            ending(&[], &state),
            Ending::Unclassified,
            "{:?}",
            state.phase
        );
    }
}

/// Decision 0064 ruling 5 and choice 7: the `args` of a `classify`
/// annotation are a closed shape, read as ruling 4's six operator-owned
/// endings; `superseded` cites a run, `delivered-elsewhere` a ref, and a
/// malformed or mis-cited annotation is read as no annotation rather than
/// a nearest guess.
#[test]
fn a_classify_annotation_reads_its_cause_and_its_citation() {
    let ended = |args: Value| operator_ending(&args);
    let run = || {
        Some(Citation::Run {
            realm: None,
            run_id: "later".into(),
        })
    };
    let world = || {
        Some(Citation::Run {
            realm: Some("next-door".into()),
            run_id: "later".into(),
        })
    };
    assert_eq!(
        ended(
            json!({"cause": "superseded", "by": {"realm": null, "run_id": "later"}, "reason": "r"})
        ),
        Some(Ended {
            ending: Ending::Superseded,
            cited: run()
        })
    );
    assert_eq!(
        ended(
            json!({"cause": "superseded", "by": {"realm": "next-door", "run_id": "later"}, "reason": "r"})
        ),
        Some(Ended {
            ending: Ending::Superseded,
            cited: world()
        })
    );
    assert_eq!(
        ended(json!({"cause": "delivered-elsewhere", "ref": "abc123", "reason": "r"})),
        Some(Ended {
            ending: Ending::DeliveredElsewhere,
            cited: Some(Citation::Ref("abc123".into()))
        })
    );
    // `reason` is optional in the contract, and absent is not `null`.
    assert_eq!(
        ended(json!({"cause": "withdrawn"})),
        Some(Ended {
            ending: Ending::Withdrawn,
            cited: None,
        })
    );
    for cause in [
        "infrastructure",
        "withdrawn",
        "exhausted",
        "operator-stopped",
    ] {
        assert_eq!(
            ended(json!({"cause": cause, "reason": "r"})),
            Some(Ended {
                ending: Ending::named(cause).unwrap(),
                cited: None,
            }),
            "{cause}"
        );
    }
    for outside in [
        json!({"cause": "shipped", "reason": "why"}),
        json!({"cause": "unclassified", "reason": "why"}),
        json!({"cause": "not-a-word", "reason": "why"}),
        json!({"cause": "superseded", "reason": "why"}),
        // The contract requires `by.realm` (null for the workspace), and
        // writes `by`, `ref` and `reason` as values, never as `null`.
        json!({"cause": "superseded", "by": {"run_id": "later"}, "reason": "why"}),
        json!({"cause": "superseded", "by": {"realm": null}, "reason": "why"}),
        json!({"cause": "superseded", "by": null, "reason": "why"}),
        json!({"cause": "delivered-elsewhere", "ref": null, "reason": "why"}),
        json!({"cause": "withdrawn", "reason": null}),
        json!({"cause": "withdrawn", "by": null, "reason": "why"}),
        json!({"cause": "superseded", "by": {"realm": null, "run_id": "later"}, "ref": "abc", "reason": "why"}),
        json!({"cause": "delivered-elsewhere", "reason": "why"}),
        json!({"cause": "delivered-elsewhere", "ref": "abc", "by": {"realm": null, "run_id": "later"}, "reason": "why"}),
        json!({"cause": "withdrawn", "by": {"realm": null, "run_id": "later"}, "reason": "why"}),
        json!({"cause": "withdrawn", "ref": "abc", "reason": "why"}),
        json!({"cause": "withdrawn", "reason": "why", "extra": true}),
        json!({"reason": "why"}),
    ] {
        assert_eq!(ended(outside.clone()), None, "{outside}");
    }
}

/// Ruling 5: a later `classify` stands over an earlier one, and the event
/// reading returns the latest. An annotation this build does not read
/// leaves the earlier one standing.
#[test]
fn the_latest_classify_annotation_stands_over_the_earlier_one() {
    let classified = |seq: u64, cause: &str, ref_: Option<&str>| {
        let mut args = json!({"cause": cause, "reason": "the operator named it"});
        if let Some(reference) = ref_ {
            args["ref"] = json!(reference);
        }
        EnvelopeBuilder::new(
            EventType::OperatorCommanded,
            json!({"command_id": format!("c{seq}"), "command": "classify", "operator": "operator", "args": args}),
        )
        .seq(seq)
        .build()
    };
    let stop = EnvelopeBuilder::new(
        EventType::OperatorCommanded,
        json!({"command_id": "s", "command": "stop", "operator": "operator"}),
    )
    .seq(1)
    .build();
    let no_args = EnvelopeBuilder::new(
        EventType::OperatorCommanded,
        json!({"command_id": "c0", "command": "classify", "operator": "operator"}),
    )
    .seq(2)
    .build();
    let events = vec![
        stop,
        no_args,
        classified(3, "withdrawn", None),
        classified(4, "delivered-elsewhere", Some("abc123")),
        classified(5, "shipped", None),
    ];
    let state = folded(Run::Stopped, "implement", None);
    assert_eq!(
        ended(&events, &state),
        Some(Ended {
            ending: Ending::DeliveredElsewhere,
            cited: Some(Citation::Ref("abc123".into())),
        }),
        "the latest readable classify wins; an unreadable one is not a guess"
    );
}
