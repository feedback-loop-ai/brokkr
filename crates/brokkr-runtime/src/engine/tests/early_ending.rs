//! Decision 0050 ruling 7's early-ending leg: an ending a middle step
//! reaches runs the same contradiction check as the final step, so no
//! later result can judge a deterministic dialect failure away.

use super::*;

/// The clarify sequence's journal: its requested-time state, carrying the
/// change a dialect check expands, and the steps' events.
fn clarify_events(
    engine: &mut Engine,
    steps: &[SequenceStep],
    selection: &Selection,
) -> Vec<EventEnvelope> {
    let mut current = state(Some("clarify"), Cursor::RequestEffect);
    drafted(&mut current, "proof-change");
    sequence_journal(engine, &current, "check", steps, selection)
}

/// The deterministic `check` step reports `ambiguous`, the middle judge
/// reports `clear`, and the final `unanimous-pass` panel can emit only
/// `pass`/`fail`. The judge's `clear` is therefore a word no later step
/// can emit, compiled into `ends_on`, and it ends the sequence early —
/// but the check's failure is deterministic evidence that ending sooner
/// may not overrule. The early ending must park on it, exactly as the
/// final step would.
#[test]
fn an_early_ending_cannot_judge_away_a_deterministic_check_failure() {
    let test = "engine::tests::early_ending::\
                an_early_ending_cannot_judge_away_a_deterministic_check_failure";
    let (_dir, mut engine, steps, selection) = judged_sequence(
        "clarify",
        "check",
        json!({"result": "clear"}),
        json!({"result": "ambiguous"}),
    );
    let mut steps = steps;
    // A final pass/fail panel makes `clear` an ending no later step can
    // emit; this is the set the compiler derives for that sequence.
    steps[1].ends_on = vec!["ambiguous".into(), "clear".into()];
    let failed = || AttemptOutcome::Succeeded {
        result: json!({"result": "fail"}),
    };
    steps.push(uncompiled_sequence_step! {
        name: "last".into(),
        class: SeatClass::Work,
        results: vec![
            "clear".into(), "ambiguous".into(), "pass".into(), "fail".into(),
        ],
        body: StepBody::Invocation(Invocation::Panel {
            members: vec![
                member(
                    "one",
                    driver_command("check-effect", "check-attempt", failed()),
                ),
                member(
                    "two",
                    driver_command("check-effect", "check-attempt", failed()),
                ),
            ],
            aggregate: Aggregate::UnanimousPass,
        }),
    });
    let seat = engine.bundle.seats.get_mut("clarify").unwrap();
    seat.results = vec![
        "clear".into(),
        "ambiguous".into(),
        "pass".into(),
        "fail".into(),
    ];
    let SeatBody::Sequence { steps: body } = &mut seat.body else {
        panic!("{test}: the clarify seat is a sequence");
    };
    *body = steps.clone();

    let events = clarify_events(&mut engine, &steps, &selection);

    let contradiction = events
        .iter()
        .find(|event| event.event_type == EventType::EffectIndeterminate)
        .unwrap_or_else(|| panic!("{test}: the early ending advanced: {events:#?}"));
    assert_eq!(
        contradiction.payload["reason"],
        "sequence step 'judge': result '\"clear\"' contradicts deterministic step 'check' \
         result 'ambiguous'",
        "{test}"
    );
    assert!(
        events.iter().any(|event| {
            event.event_type == EventType::EffectCheckpointed
                && event.payload["checkpoint"]["step_name"] == "check"
        }),
        "{test}: the deterministic check settles first"
    );
    assert!(
        !events.iter().any(|event| {
            event.event_type == EventType::EffectCheckpointed
                && event.payload["checkpoint"]["step_name"] == "judge"
        }),
        "{test}: the refused early ending finishes no judge step"
    );
    assert!(
        !events
            .iter()
            .any(|event| event.event_type == EventType::EffectSucceeded),
        "{test}: no result escaped the refusal: {events:#?}"
    );
    assert!(
        !events
            .iter()
            .any(|event| event.event_type == EventType::TransitionDecided),
        "{test}: the machine rules on no contradiction: {events:#?}"
    );
}
