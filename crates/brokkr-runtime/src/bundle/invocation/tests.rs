use super::super::{body_manifest, SeatBody, SequenceStep, StepBody};
use super::*;
use crate::agents::Lowering;

fn link(model: &str, effort: Option<&str>) -> Candidate {
    Candidate {
        agent: "worker".into(),
        model: model.into(),
        effort: effort.map(str::to_string),
        provider: "claude".into(),
        argv: vec!["driver".into()],
        hands_fragment: Vec::new(),
        harness: Default::default(),
        resume: Default::default(),
        hands_notice: None,
        lowering: Lowering::Unavailable,
    }
}

fn chain() -> Vec<Candidate> {
    vec![link("opus", Some("high")), link("sonnet", None)]
}

fn single<C>(candidates: Vec<Candidate>, class: C) -> Invocation<C> {
    Invocation::Single {
        role_path: "role.md".into(),
        command: vec!["driver".into()],
        candidates,
        written: Vec::new(),
        class,
    }
}

/// An agent member and an inline one, written out of name order.
fn panel<C>(aggregate: Aggregate) -> Invocation<C> {
    let member = |name: &str, candidates| PanelMember {
        name: name.into(),
        role_path: "role.md".into(),
        command: vec!["driver".into()],
        candidates,
        written: Vec::new(),
        class: SeatClass::Gate,
    };
    Invocation::Panel {
        members: vec![member("b", chain()), member("a", Vec::new())],
        aggregate,
    }
}

fn step(name: &str, results: &[&str], body: Invocation<()>) -> SequenceStep {
    SequenceStep {
        name: name.into(),
        class: SeatClass::Work,
        results: results.iter().map(|result| result.to_string()).collect(),
        body: StepBody::Invocation(body),
    }
}

fn rendered(body: SeatBody) -> String {
    serde_json::to_string(&body_manifest(&body)).unwrap()
}

/// The manifest each invocation form renders, seat and step alike, frozen
/// as `body_manifest` rendered it on d5dbb162, before `Invocation` (#349
/// ask 2): the manifest bytes are the contract.
#[test]
fn invocation_renders_the_manifest_it_did() {
    let seat = |invocation| rendered(SeatBody::Invocation(invocation));
    assert_eq!(
        seat(single(Vec::new(), SeatClass::Work)),
        r#"{"driver":"inline"}"#
    );
    assert_eq!(
        seat(single(chain(), SeatClass::Work)),
        r#"{"agent":"worker","candidates":[{"effort":"high","model":"opus","provider":"claude"},{"effort":null,"model":"sonnet","provider":"claude"}]}"#
    );
    assert_eq!(
        seat(panel(Aggregate::ReviewPanel)),
        r#"{"aggregate":"review-panel","panel":{"a":{"driver":"inline"},"b":{"agent":"worker","candidates":[{"effort":"high","model":"opus","provider":"claude"},{"effort":null,"model":"sonnet","provider":"claude"}]}}}"#
    );
    let steps = vec![
        step("draft", &["drafted"], single(chain(), ())),
        step("inline", &["drafted"], single(Vec::new(), ())),
        step("judge", &["pass", "fail"], panel(Aggregate::UnanimousPass)),
    ];
    assert_eq!(
        rendered(SeatBody::Sequence { steps }),
        r#"{"sequence":[{"body":{"agent":"worker","candidates":[{"effort":"high","model":"opus","provider":"claude"},{"effort":null,"model":"sonnet","provider":"claude"}]},"name":"draft","results":["drafted"]},{"body":{"driver":"inline"},"name":"inline","results":["drafted"]},{"body":{"aggregate":"unanimous-pass","panel":{"a":{"driver":"inline"},"b":{"agent":"worker","candidates":[{"effort":"high","model":"opus","provider":"claude"},{"effort":null,"model":"sonnet","provider":"claude"}]}}},"name":"judge","results":["pass","fail"]}]}"#
    );
}
