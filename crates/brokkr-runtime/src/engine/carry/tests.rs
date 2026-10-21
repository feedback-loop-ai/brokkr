use super::record::{PanelRecord, RetryArgs};
use super::*;
use crate::engine::tests::{engine, git_commit, member, report, state};
use crate::engine::{Cursor, SeatBody};
use crate::envelope_builder::EnvelopeBuilder;
use serde_json::json;

fn event(seq: u64, kind: brokkr_core::EventType, payload: Value) -> EventEnvelope {
    EnvelopeBuilder::new(kind, payload).seq(seq).build()
}

fn subject(runtime: &Engine, state: &RunState, result: Value) -> Vec<EventEnvelope> {
    let input = runtime.seat_input(state, "work", "source").unwrap();
    let span = PanelSpan::open(&runtime.repo);
    let record = PanelRecord {
        span,
        settled_members: [(Tag("a".into()), result)].into_iter().collect(),
    };
    let mut terminal = json!({"effect_id":"source", "attempt_id":"settled", "reason":"b vanished"});
    record.stamp(&mut terminal);
    vec![
        event(
            1,
            brokkr_core::EventType::PhaseEntered,
            json!({"phase":"work"}),
        ),
        event(
            2,
            brokkr_core::EventType::EffectRequested,
            json!({"phase":"work", "effect_id":"source", "input_digest":brokkr_core::canonical::sha256_hex(&input)}),
        ),
        event(
            3,
            brokkr_core::EventType::EffectStarted,
            json!({"effect_id":"source", "attempt_id":"settled", "driver":"panel"}),
        ),
        event(4, brokkr_core::EventType::EffectIndeterminate, terminal),
    ]
}

fn panel_fixture() -> (tempfile::TempDir, Engine, RunState) {
    let body = SeatBody::Invocation(Invocation::Panel {
        members: vec![
            member("a", vec!["unused".into()]),
            member("b", vec!["unused".into()]),
        ],
        aggregate: Aggregate::ReviewPanel,
    });
    let (dir, runtime) = engine(body);
    let initial = state(Some("work"), Cursor::RequestEffect);
    git_commit(&runtime.repo, "base");
    (dir, runtime, initial)
}

#[test]
fn the_rule_preserves_the_aggregate_and_checks_identity_and_selection() {
    let (_dir, runtime, initial) = panel_fixture();
    let result = json!({"result":"residual", "inputs":{"has_security_residual":true,"max_residual_severity":"low"}, "notes":"kept"});
    let events = subject(&runtime, &initial, result.clone());
    let attempts = facts::read_visit(&events, &initial, u64::MAX).unwrap();
    let mut facts = runtime.carry_facts(&events, &initial, attempts);
    let plan = carry_plan(&facts, &[Tag("b".into())]).unwrap();
    assert_eq!(plan.fresh(), [Tag("b".into())]);
    assert_eq!(plan.carried()[0].attempt_id, "settled");
    let clean = json!({"result":"clean"});
    let fresh = || {
        vec![(
            "b".into(),
            report(
                AttemptOutcome::Succeeded {
                    result: clean.clone(),
                },
                "",
            ),
        )]
    };
    let full = vec![
        (
            "a".into(),
            report(
                AttemptOutcome::Succeeded {
                    result: result.clone(),
                },
                "",
            ),
        ),
        (
            "b".into(),
            report(
                AttemptOutcome::Succeeded {
                    result: clean.clone(),
                },
                "",
            ),
        ),
    ];
    let span = PanelSpan::open(&runtime.repo);
    let joined = outcome_under(
        Aggregate::ReviewPanel,
        Some(&plan),
        fresh(),
        &span,
        "carrying",
    )
    .unwrap();
    let complete = outcome_under(Aggregate::ReviewPanel, None, full, &span, "full").unwrap();
    assert_eq!(
        serde_json::to_value(joined).unwrap(),
        serde_json::to_value(complete).unwrap()
    );
    assert!(
        matches!(carry_plan(&facts, &[Tag("unknown".into())]).unwrap_err().0, Refusal::UnknownMember { name, .. } if name == "unknown")
    );
    assert!(
        matches!(carry_plan(&facts, &[Tag("b".into()), Tag("b".into())]).unwrap_err().0, Refusal::MemberTwice { tag } if tag == "b")
    );
    assert!(
        matches!(carry_plan(&facts, &[Tag("a".into())]).unwrap_err().0, Refusal::Unsettled { members } if members == "b")
    );
    facts.attempts[0].input_holds = false;
    assert!(
        matches!(carry_plan(&facts, &[Tag("b".into())]).unwrap_err().0, Refusal::InputMoved { effects } if effects == "source")
    );
    facts.head = Some("b".repeat(40).try_into().unwrap());
    assert!(matches!(
        carry_plan(&facts, &[Tag("b".into())]).unwrap_err().0,
        Refusal::HeadMoved { .. }
    ));
    assert_eq!(
        carry_plan(&facts, &[Tag("b".into()), Tag("a".into())])
            .unwrap()
            .fresh(),
        [Tag("a".into()), Tag("b".into())]
    );
    let mut moved = span.clone();
    moved.tree.at_end = TreeReading::Dirty;
    assert!(
        matches!(outcome_under(Aggregate::ReviewPanel, Some(&plan), fresh(), &moved, "carrying").unwrap(), AttemptOutcome::Indeterminate { reason } if reason.contains("carrying attempt 'carrying' changed its reviewed subject"))
    );
    assert!(
        matches!(plan.join(vec![]), Err(EngineError::Invariant("complete carry plan", detail)) if detail == "missing fresh report")
    );
}

#[test]
fn the_record_reader_and_retry_scope_fail_closed() {
    for args in [
        json!({"members":null}),
        json!({"members":[]}),
        json!({"members":[1]}),
        json!({"other":[]}),
    ] {
        assert_eq!(
            serde_json::from_value::<RetryArgs>(args)
                .unwrap_err()
                .classify(),
            serde_json::error::Category::Data
        );
    }
    assert_eq!(
        serde_json::from_value::<RetryArgs>(json!({}))
            .unwrap()
            .members,
        None
    );
    let (_dir, runtime) = engine(crate::engine::tests::single_body(vec!["unused".into()]));
    let initial = state(Some("work"), Cursor::RequestEffect);
    for (field, value) in [
        ("tree", json!({"at_start":"fifth", "at_end":"clean"})),
        ("head", json!({"at_start":"invalid"})),
        ("head", json!({"at_start":null})),
        ("settled_members", json!([])),
    ] {
        let mut events = subject(&runtime, &initial, json!("opaque result"));
        events[3].payload[field] = value;
        assert!(
            matches!(facts::read_visit(&events, &initial, u64::MAX), Err(CarryRefusal(Refusal::RecordUnread { field: bad, .. })) if bad == field)
        );
    }
    let events = vec![
        event(
            1,
            brokkr_core::EventType::OperatorCommanded,
            json!({"command_id":"retry", "command":"retry", "args":{"members":["b"]}}),
        ),
        event(
            2,
            brokkr_core::EventType::OperatorAccepted,
            json!({"command_id":"retry"}),
        ),
        event(
            3,
            brokkr_core::EventType::EffectRequested,
            json!({"effect_id":"one"}),
        ),
        event(
            4,
            brokkr_core::EventType::EffectRequested,
            json!({"effect_id":"two"}),
        ),
    ];
    assert_eq!(
        facts::scope(&events, "one").unwrap(),
        Some((3, vec![Tag("b".into())]))
    );
    assert_eq!(facts::scope(&events, "two").unwrap(), None);
}

#[test]
fn shape_head_and_tree_refusals_are_ordered_before_the_inputs() {
    let (_dir, runtime, initial) = panel_fixture();
    let events = subject(&runtime, &initial, json!({"result":"clean"}));
    let mut facts = runtime.carry_facts(
        &events,
        &initial,
        facts::read_visit(&events, &initial, u64::MAX).unwrap(),
    );
    let names = [Tag("b".into())];
    let shape = std::mem::replace(
        &mut facts.shape,
        facts::PanelShape::NoPanel {
            phase: "work".into(),
        },
    );
    assert!(
        matches!(carry_plan(&facts, &[Tag("unknown".into())]).unwrap_err().0, Refusal::NoPanel { phase } if phase == "work")
    );
    facts.shape = facts::PanelShape::StepFollows {
        step: "later".into(),
        first: "first".into(),
    };
    assert!(
        matches!(carry_plan(&facts, &[Tag("unknown".into())]).unwrap_err().0, Refusal::PanelStepFollows { step, first } if step == "later" && first == "first")
    );
    facts.shape = shape;
    facts.attempts[0].succeeded = true;
    facts.attempts[0].park_reason = Some("original park".into());
    assert!(
        matches!(carry_plan(&facts, &names).unwrap_err().0, Refusal::Shape { why: Shape::Succeeded { park_reason, .. } } if park_reason == "original park")
    );
    facts.attempts[0].succeeded = false;
    let record = facts.attempts[0].record.take().unwrap();
    assert!(matches!(
        carry_plan(&facts, &names).unwrap_err().0,
        Refusal::Shape {
            why: Shape::NoAttemptEnded { .. }
        }
    ));
    facts.attempts[0].record = Some(PanelRecord {
        settled_members: BTreeMap::new(),
        ..record.clone()
    });
    assert!(matches!(
        carry_plan(&facts, &names).unwrap_err().0,
        Refusal::Shape {
            why: Shape::NoRecord { .. }
        }
    ));
    facts.attempts[0].record = Some(record);
    facts.attempts[0].input_holds = false;
    let head = facts.head.take().unwrap();
    facts.tree = TreeReading::Dirty;
    assert!(matches!(
        carry_plan(&facts, &names).unwrap_err().0,
        Refusal::NoHead { .. }
    ));
    facts.head = Some(head);
    assert!(matches!(
        carry_plan(&facts, &names).unwrap_err().0,
        Refusal::TreeNotClean { .. }
    ));
    facts.tree = TreeReading::Clean;
    assert!(
        matches!(carry_plan(&facts, &names).unwrap_err().0, Refusal::InputMoved { effects } if effects == "source")
    );
}
