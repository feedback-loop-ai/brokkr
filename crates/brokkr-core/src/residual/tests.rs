use super::*;
use crate::envelope_builder::EnvelopeBuilder;
use serde_json::json;

fn event(seq: u64, event_type: EventType, payload: Value) -> EventEnvelope {
    EnvelopeBuilder::new(event_type, payload).seq(seq).build()
}

fn ruled(seq: u64, from: &str, inputs: Value) -> EventEnvelope {
    event(
        seq,
        EventType::TransitionDecided,
        json!({"from": from, "rule_id": "R", "inputs": inputs}),
    )
}

/// Each claim as `(seq, phase, claim)`, the facts the engine admits a
/// supersede by and the view renders.
fn claims(events: &[EventEnvelope]) -> Vec<(u64, ResidualPhase, Claim)> {
    residuals(events)
        .into_iter()
        .map(|residual| (residual.ruling.seq, residual.phase, residual.claim))
        .collect()
}

#[test]
fn the_rulings_of_verify_and_review_carry_the_claims_in_their_closed_vocabulary() {
    let events = vec![
        ruled(
            1,
            "verify",
            json!({"max_residual_severity": "high", "has_security_residual": true,
                   "high_risk_uncovered": true, "tests_passed": true}),
        ),
        ruled(2, "review", json!({"max_residual_severity": "low"})),
    ];
    assert_eq!(
        claims(&events),
        vec![
            (1, ResidualPhase::Verify, Claim::SecurityResidual),
            (1, ResidualPhase::Verify, Claim::HighRiskUncovered),
            (1, ResidualPhase::Verify, Claim::MaxSeverity(Severity::High)),
            (2, ResidualPhase::Review, Claim::MaxSeverity(Severity::Low)),
        ]
    );
}

/// The words each claim renders to are the journal's own: the phase as
/// `from` named it, the input by its evaluator name, and the value as it
/// was written.
#[test]
fn a_claim_renders_to_the_words_the_journal_wrote() {
    let words = |phase: ResidualPhase, claim: Claim| (phase.as_str(), claim.input(), claim.value());
    assert_eq!(
        [
            words(ResidualPhase::Verify, Claim::SecurityResidual),
            words(ResidualPhase::Review, Claim::HighRiskUncovered),
            words(
                ResidualPhase::Review,
                Claim::MaxSeverity(Severity::Critical)
            ),
        ],
        [
            ("verify", "has_security_residual", "true"),
            ("review", "high_risk_uncovered", "true"),
            ("review", "max_residual_severity", "critical"),
        ]
    );
}

#[test]
fn no_claim_without_a_residual_ruling_that_states_one() {
    let events = vec![
        // Not a ruling, though it reads like one.
        event(
            1,
            EventType::OperatorCommanded,
            json!({"from": "verify", "inputs": {"max_residual_severity": "high"}}),
        ),
        // A phase that carries no residuals, and a ruling naming no phase.
        ruled(2, "implement", json!({"max_residual_severity": "high"})),
        event(
            3,
            EventType::TransitionDecided,
            json!({"inputs": {"max_residual_severity": "high"}}),
        ),
        // Inputs that are not a map.
        ruled(4, "verify", json!(["max_residual_severity"])),
        // `none`, an unranked name, a non-string severity, false and
        // non-boolean flags, and a key outside the vocabulary.
        ruled(
            5,
            "review",
            json!({"max_residual_severity": "none", "has_security_residual": false,
                   "high_risk_uncovered": "true", "residual": "high"}),
        ),
        ruled(6, "verify", json!({"max_residual_severity": "severe"})),
        ruled(7, "verify", json!({"max_residual_severity": 4})),
    ];
    assert_eq!(claims(&events), Vec::new());
}

fn superseding(seq: u64, args: Value) -> EventEnvelope {
    event(
        seq,
        EventType::OperatorCommanded,
        json!({"command": "supersede", "operator": "op", "args": args}),
    )
}

#[test]
fn a_supersede_reads_its_mark_and_the_findings_it_closes() {
    let annotation = superseding(
        9,
        json!({"by": {"realm": "east", "run_id": "other", "seq": 4}, "findings": [2, "x", 3],
               "reason": "fixed there"}),
    );
    assert_eq!(
        closing_of(&annotation),
        Some((
            Mark {
                seq: 9,
                by: Citation {
                    realm: Some("east".into()),
                    run_id: "other".into(),
                    seq: 4,
                },
                reason: Some("fixed there".into()),
                operator: Some("op".into()),
                recorded_at: "2026-01-01T00:00:00Z".into(),
            },
            vec![2, 3],
        ))
    );
    let bare = event(
        9,
        EventType::OperatorCommanded,
        json!({"command": "supersede", "args": {"by": {"run_id": "other", "seq": 4}, "findings": []}}),
    );
    let (mark, findings) = closing_of(&bare).unwrap();
    assert_eq!(
        (mark.by.realm, mark.reason, mark.operator),
        (None, None, None)
    );
    assert_eq!(findings, Vec::<u64>::new());
}

#[test]
fn no_closing_without_a_supersede_whose_citation_reads() {
    let cited = json!({"by": {"run_id": "other", "seq": 4}, "findings": [2]});
    let retry = event(
        9,
        EventType::OperatorCommanded,
        json!({"command": "retry", "args": cited}),
    );
    let ruling = event(
        9,
        EventType::TransitionDecided,
        json!({"command": "supersede", "args": cited}),
    );
    let unread = [
        json!({"findings": [2]}),
        json!({"by": {"seq": 4}, "findings": [2]}),
        json!({"by": {"run_id": "other"}, "findings": [2]}),
        json!({"by": {"run_id": "other", "seq": 4}, "findings": 2}),
    ];
    assert_eq!(closing_of(&retry), None);
    assert_eq!(closing_of(&ruling), None);
    for args in unread {
        assert_eq!(closing_of(&superseding(9, args.clone())), None, "{args}");
    }
}

#[test]
fn a_finding_closed_twice_carries_the_last_mark() {
    let first = superseding(
        5,
        json!({"by": {"run_id": "a", "seq": 1}, "findings": [2, 3]}),
    );
    let last = superseding(6, json!({"by": {"run_id": "b", "seq": 1}, "findings": [3]}));
    let marks = closings(&[first, last, ruled(7, "verify", json!({}))]);
    let by: Vec<(u64, u64, &str)> = marks
        .iter()
        .map(|(finding, mark)| (*finding, mark.seq, mark.by.run_id.as_str()))
        .collect();
    assert_eq!(by, vec![(2, 5, "a"), (3, 6, "b")]);
}

#[test]
fn the_open_severity_is_the_highest_an_unclosed_finding_states() {
    let open = |findings: &[(Option<Severity>, bool)]| open_severity(findings.iter().copied());
    assert_eq!(
        open(&[(Some(Severity::Low), false), (Some(Severity::High), false)]),
        Some(Severity::High)
    );
    assert_eq!(
        open(&[(Some(Severity::Low), false), (Some(Severity::High), true)]),
        Some(Severity::Low),
        "a closed finding is open no longer"
    );
    assert_eq!(open(&[(None, false), (Some(Severity::None), false)]), None);
    assert_eq!(open(&[]), None);
    assert_eq!(
        Claim::MaxSeverity(Severity::Medium).severity(),
        Some(Severity::Medium)
    );
    assert_eq!(Claim::SecurityResidual.severity(), None);
    assert_eq!(Claim::HighRiskUncovered.severity(), None);
}

/// A claim reads back from the input and value it words, and from no
/// other pair: a flag's value under another flag's input, or a severity
/// under a flag's, names no claim (decision 0089 ruling 2).
#[test]
fn a_claim_reads_back_from_its_own_words_alone() {
    let claims = [
        Claim::MaxSeverity(Severity::High),
        Claim::SecurityResidual,
        Claim::HighRiskUncovered,
    ];
    for claim in claims {
        assert_eq!(Claim::named(claim.input(), claim.value()), Some(claim));
    }
    let unstated = [
        ("max_residual_severity", "none"),
        ("high_risk_uncovered", "high"),
        ("high_risk_uncovered", "yes"),
        ("has_security_residual", "yes"),
        ("unknown_flag", "true"),
    ];
    for (input, value) in unstated {
        assert_eq!(Claim::named(input, value), None, "{input}: {value}");
    }
}
