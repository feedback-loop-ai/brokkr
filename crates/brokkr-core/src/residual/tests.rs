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
