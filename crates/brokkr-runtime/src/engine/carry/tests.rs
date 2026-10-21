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
    let (_dir, mut facts) = clean_facts();
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

fn success(result: Value) -> AttemptReport {
    report(AttemptOutcome::Succeeded { result }, "")
}

#[test]
fn plan_construction_and_join_refuse_incomplete_sets() {
    let tags = [Tag("a".into()), Tag("b".into())];
    for entries in [
        vec![Contribution::Fresh(tags[0].clone())],
        vec![
            Contribution::Fresh(tags[1].clone()),
            Contribution::Fresh(tags[0].clone()),
        ],
        vec![
            Contribution::Fresh(tags[0].clone()),
            Contribution::Fresh(Tag("foreign".into())),
        ],
    ] {
        assert!(matches!(CarryPlan::new(&tags, entries, None, None),
            Err(EngineError::Invariant("complete carry plan", detail))
                if detail == "contributions differ from declared members"));
    }
    let repeated = [tags[0].clone(), tags[0].clone()];
    assert!(matches!(CarryPlan::new(&repeated,
        repeated.iter().cloned().map(Contribution::Fresh).collect(), None, None),
        Err(EngineError::Invariant("complete carry plan", detail)) if detail == "duplicate contribution"));
    let plan = CarryPlan::new(
        &tags,
        tags.iter().cloned().map(Contribution::Fresh).collect(),
        None,
        None,
    )
    .unwrap();
    let verdict = json!({"result":"pass"});
    let reports = || {
        vec![
            ("b".into(), success(verdict.clone())),
            ("a".into(), success(verdict.clone())),
        ]
    };
    assert_eq!(
        plan.join(reports())
            .unwrap()
            .into_iter()
            .map(|(name, _)| name)
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    for (reports, expected) in [
        (vec![], "missing fresh report"),
        (
            vec![
                ("a".into(), success(verdict.clone())),
                ("a".into(), success(verdict.clone())),
            ],
            "duplicate fresh report",
        ),
        (
            {
                let mut reports = reports();
                reports.push(("c".into(), success(verdict.clone())));
                reports
            },
            "foreign fresh report",
        ),
    ] {
        assert!(
            matches!(plan.join(reports), Err(EngineError::Invariant("complete carry plan", detail)) if detail == expected)
        );
    }
}

#[test]
fn both_aggregates_keep_opaque_and_settled_verdicts_in_declared_order() {
    let (_dir, runtime, initial) = panel_fixture();
    for (aggregate, verdict, expected) in [
        (
            Aggregate::ReviewPanel,
            json!({"result":"residual", "notes":"keep"}),
            "residual",
        ),
        (
            Aggregate::UnanimousPass,
            json!({"result":"fail", "notes":"keep"}),
            "fail",
        ),
        (
            Aggregate::ReviewPanel,
            json!("opaque"),
            "__member-schema-invalid__",
        ),
    ] {
        let events = subject(&runtime, &initial, verdict.clone());
        let facts = observed(&runtime, &initial, &events, u64::MAX);
        let plan = carry_plan(&facts, &[Tag("b".into())]).unwrap();
        let fresh = json!({"result":if matches!(aggregate, Aggregate::ReviewPanel) {"clean"} else {"pass"}});
        let span = PanelSpan::open(&runtime.repo);
        let carried = outcome_under(
            aggregate,
            Some(&plan),
            vec![("b".into(), success(fresh.clone()))],
            &span,
            "two",
        )
        .unwrap();
        let full = outcome_under(
            aggregate,
            None,
            vec![("a".into(), success(verdict)), ("b".into(), success(fresh))],
            &span,
            "one",
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(&carried).unwrap(),
            serde_json::to_value(full).unwrap()
        );
        assert!(
            matches!(carried, AttemptOutcome::Succeeded { result } if result["result"] == expected)
        );
    }
}

#[test]
fn the_span_is_checked_before_join_only_when_a_verdict_is_carried() {
    let (_dir, runtime, initial) = panel_fixture();
    let events = subject(&runtime, &initial, json!({"result":"clean"}));
    let facts = observed(&runtime, &initial, &events, u64::MAX);
    let carried = carry_plan(&facts, &[Tag("b".into())]).unwrap();
    let fresh = carry_plan(&facts, &[Tag("b".into()), Tag("a".into())]).unwrap();
    let original = PanelSpan::open(&runtime.repo);
    for change in 0..4 {
        let mut span = original.clone();
        match change {
            0 => span.head.at_end = Some("b".repeat(40).try_into().unwrap()),
            1 => span.head.at_start = None,
            2 => span.tree.at_start = TreeReading::Dirty,
            3 => span.tree.at_end = TreeReading::Dirty,
            _ => unreachable!(),
        }
        let observations =
            serde_json::to_string(&(&span, carried.subject.as_ref().unwrap())).unwrap();
        let expected = Refusal::SubjectMoved {
            attempt: "two".into(),
            observations,
        }
        .to_string();
        assert!(
            matches!(outcome_under(Aggregate::ReviewPanel, Some(&carried), vec![], &span, "two").unwrap(),
            AttemptOutcome::Indeterminate { reason } if reason == expected)
        );
        let reports = || {
            vec![
                ("a".into(), success(json!({"result":"clean"}))),
                ("b".into(), success(json!({"result":"clean"}))),
            ]
        };
        let plain = outcome_under(Aggregate::ReviewPanel, None, reports(), &span, "two").unwrap();
        let all_named = outcome_under(
            Aggregate::ReviewPanel,
            Some(&fresh),
            reports(),
            &span,
            "two",
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(plain).unwrap(),
            serde_json::to_value(all_named).unwrap()
        );
    }
}

#[test]
fn a_recovery_reads_original_sources_and_the_first_park_after_success() {
    use brokkr_core::EventType::*;
    let (_dir, runtime, initial, mut events) = clean_visit();
    events.extend([
        event(5, RunParked, json!({"reason":"original park"})),
        event(6, EffectRequested, events[1].payload.clone()),
        event(7, EffectStarted, json!({"effect_id":"source", "attempt_id":"second", "carried":[{"member":"a","attempt_id":"settled"}]})),
        event(8, EffectIndeterminate, {let mut payload = events[3].payload.clone(); payload["attempt_id"] = json!("second"); payload.as_object_mut().unwrap().remove("settled_members"); payload}),
    ]);
    let facts = observed(&runtime, &initial, &events, u64::MAX);
    assert_source(&facts, "settled");
    events[3].event_type = EffectSucceeded;
    events[7].payload.as_object_mut().unwrap().remove("tree");
    events[7].payload.as_object_mut().unwrap().remove("head");
    events.extend([
        event(9, RunParked, json!({"reason":"later admission park"})),
        event(10, EffectRequested, json!({"effect_id":"next"})),
    ]);
    for cutoff in [u64::MAX, 10] {
        let facts = observed(&runtime, &initial, &events, cutoff);
        assert!(
            matches!(carry_plan(&facts, &[Tag("b".into())]).unwrap_err().0,
            Refusal::Shape { why: Shape::Succeeded { attempt, park_reason } } if attempt == "settled" && park_reason == "original park")
        );
    }
}

#[test]
fn scope_reads_only_retry_arguments_and_keeps_the_request_bound() {
    use brokkr_core::EventType::*;
    let mut events = vec![
        event(
            1,
            OperatorCommanded,
            json!({"command_id":"r", "command":"retry", "args":{"members":["b"]}}),
        ),
        event(2, OperatorAccepted, json!({"command_id":"r"})),
        event(
            3,
            OperatorCommanded,
            json!({"command_id":"s", "command":"stop", "args":{"members":["wrong"]}}),
        ),
        event(4, OperatorAccepted, json!({"command_id":"s"})),
        event(5, EffectRequested, json!({"effect_id":"effect"})),
    ];
    assert_eq!(
        facts::scope(&events, "effect").unwrap(),
        Some((5, vec![Tag("b".into())]))
    );
    for args in [
        json!({"extra":1}),
        json!({"members":null}),
        json!({"members":[]}),
        json!({"members":"b"}),
    ] {
        events[0].payload["args"] = args;
        assert!(matches!(facts::scope(&events, "effect").unwrap_err().0,
            Refusal::RecordUnread { record: RecordId::Command(1), field, .. } if field == "args"));
    }
    events[0].payload["args"] = json!({});
    assert_eq!(facts::scope(&events, "effect").unwrap(), None);
    events[0].payload.as_object_mut().unwrap().remove("args");
    assert_eq!(facts::scope(&events, "effect").unwrap(), None);
}

#[test]
fn exact_step_tags_and_all_named_retries_obey_declared_order_before_identity() {
    let (_dir, runtime, initial, mut events) = clean_visit();
    events[3].payload["settled_members"] = json!({"p:p:x":{"result":"clean"}});
    let mut facts = observed(&runtime, &initial, &events, u64::MAX);
    let tags = vec![Tag("p:p:x".into()), Tag("p:x".into())];
    facts.shape = PanelShape::Panel {
        step: Some("p".into()),
        tags: tags.clone(),
    };
    let plan = carry_plan(&facts, &[Tag("p:x".into())]).unwrap();
    assert_eq!(plan.fresh(), [Tag("p:x".into())]);
    assert_eq!(plan.carried()[0].member, Tag("p:p:x".into()));
    let pairs = plan
        .join(vec![("x".into(), success(json!({"result":"clean"})))])
        .unwrap();
    assert_eq!(
        pairs.into_iter().map(|(tag, _)| tag).collect::<Vec<_>>(),
        ["p:x", "x"]
    );
    assert!(
        matches!(carry_plan(&facts, &[Tag("x".into())]).unwrap_err().0,
        Refusal::UnknownMember { name, tags } if name == "x" && tags == "p:p:x, p:x")
    );
    facts.head = None;
    facts.tree = TreeReading::Dirty;
    facts.attempts[0].input_holds = false;
    facts.attempts[0].steps = vec!["p".into()];
    facts.attempts[0].succeeded = true;
    let fresh = carry_plan(&facts, &[tags[1].clone(), tags[0].clone()]).unwrap();
    assert_eq!(fresh.fresh(), tags);
    assert_eq!(fresh.carried().len(), 0);
    assert_eq!(fresh.subject, None);
}

#[test]
fn the_three_way_search_passes_over_restarts_but_not_a_finished_panel_step() {
    use brokkr_core::EventType::*;
    let (_dir, runtime, initial, mut events) = clean_visit();
    events.extend([
        event(
            5,
            EffectStarted,
            json!({"effect_id":"source","attempt_id":"restart"}),
        ),
        event(
            6,
            EffectIndeterminate,
            json!({"effect_id":"source","attempt_id":"restart","reason":"restart"}),
        ),
    ]);
    let mut facts = observed(&runtime, &initial, &events, u64::MAX);
    assert_source(&facts, "settled");
    facts.attempts[1].record = facts.attempts[0].record.clone();
    facts.attempts[1]
        .record
        .as_mut()
        .unwrap()
        .settled_members
        .clear();
    assert!(
        matches!(carry_plan(&facts, &[Tag("b".into())]).unwrap_err().0,
        Refusal::Shape { why:Shape::NoRecord { attempt } } if attempt == "restart")
    );
    facts.attempts[1].record = None;
    facts.attempts[1].steps = vec!["p".into()];
    facts.shape = PanelShape::Panel {
        step: Some("p".into()),
        tags: vec![Tag("p:a".into()), Tag("p:b".into())],
    };
    assert!(
        matches!(carry_plan(&facts, &[Tag("p:b".into())]).unwrap_err().0,
        Refusal::Shape { why:Shape::EndedAtLaterStep { attempt, .. } } if attempt == "restart")
    );
    facts.attempts[1].steps.clear();
    facts.attempts[0].record = None;
    assert!(
        matches!(carry_plan(&facts, &[Tag("p:b".into())]).unwrap_err().0,
        Refusal::Shape { why:Shape::NoAttemptEnded { phase, .. } } if phase == "work")
    );
}

#[test]
fn identity_names_both_ends_in_source_order_before_the_tree_and_inputs() {
    let (_dir, mut facts) = clean_facts();
    let old = facts.head.clone().unwrap();
    let new: Head = "b".repeat(40).try_into().unwrap();
    facts.head = Some(new.clone());
    facts.tree = TreeReading::Unrun;
    facts.attempts[0].input_holds = false;
    let observed =
        serde_json::to_string(&Some(&facts.attempts[0].record.as_ref().unwrap().span.head))
            .unwrap();
    assert!(
        matches!(carry_plan(&facts, &[Tag("b".into())]).unwrap_err().0,
        Refusal::HeadMoved { observations } if observations == format!("settled: {observed}; now {}", serde_json::to_string(&new).unwrap()))
    );
    facts.head = Some(old);
    let span = &mut facts.attempts[0].record.as_mut().unwrap().span;
    span.tree.at_start = TreeReading::Refused;
    span.tree.at_end = TreeReading::Dirty;
    let observed = serde_json::to_string(&Some(&span.tree)).unwrap();
    assert!(
        matches!(carry_plan(&facts, &[Tag("b".into())]).unwrap_err().0,
        Refusal::TreeNotClean { observations } if observations == format!("settled: {observed}; now \"unrun\""))
    );
}

fn observed(
    runtime: &Engine,
    initial: &RunState,
    events: &[EventEnvelope],
    cutoff: u64,
) -> CarryFacts {
    runtime.carry_facts(
        events,
        initial,
        facts::read_visit(events, initial, cutoff).unwrap(),
    )
}

fn clean_visit() -> (tempfile::TempDir, Engine, RunState, Vec<EventEnvelope>) {
    let (dir, runtime, initial) = panel_fixture();
    let events = subject(&runtime, &initial, json!({"result":"clean"}));
    (dir, runtime, initial, events)
}

fn clean_facts() -> (tempfile::TempDir, CarryFacts) {
    let (dir, runtime, initial, events) = clean_visit();
    (dir, observed(&runtime, &initial, &events, u64::MAX))
}

fn assert_source(facts: &CarryFacts, source: &str) {
    assert_eq!(
        carry_plan(facts, &[Tag("b".into())]).unwrap().carried()[0].attempt_id,
        source
    );
}
