//! Residual findings as the journal carries them, and the word of the
//! operator annotation that closes one (decision 0047). The engine admits
//! a supersede by this derivation and the view renders it, so a readout
//! change can never move which supersedes the engine accepts (#351).

use serde_json::Value;

use crate::envelope::{EventEnvelope, EventType};
use crate::policy::Severity;

/// The command word of the operator annotation that closes a residual
/// finding (decision 0047 ruling 1). Deliberately NOT an
/// [`OperatorCommand`](crate::fold::OperatorCommand): those are what a
/// PARKED run admits, and a supersede is only ever written on a terminal
/// one.
pub const SUPERSEDE: &str = "supersede";

/// The phases whose rulings carry residual findings.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResidualPhase {
    Verify,
    Review,
}

impl ResidualPhase {
    /// The phase a ruling's `from` names, or `None` for a phase that
    /// carries no residuals.
    fn named(from: &str) -> Option<ResidualPhase> {
        match from {
            "verify" => Some(ResidualPhase::Verify),
            "review" => Some(ResidualPhase::Review),
            _ => None,
        }
    }

    /// The phase name the table and every surface write for it.
    pub fn as_str(self) -> &'static str {
        match self {
            ResidualPhase::Verify => "verify",
            ResidualPhase::Review => "review",
        }
    }
}

/// What a residual claim states, one variant per evaluator input that
/// can carry one: the severity above `none`, or one of the two flags
/// set true.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Claim {
    MaxSeverity(Severity),
    SecurityResidual,
    HighRiskUncovered,
}

impl Claim {
    /// The claim the evaluator input `key` carries with `value`. A
    /// severity of `none`, a false flag, an unranked severity name and
    /// any key outside the vocabulary all carry no finding.
    fn of(key: &str, value: &Value) -> Option<Claim> {
        match key {
            "max_residual_severity" => value
                .as_str()
                .and_then(Severity::named)
                .filter(|severity| *severity != Severity::None)
                .map(Claim::MaxSeverity),
            "has_security_residual" => {
                (value.as_bool() == Some(true)).then_some(Claim::SecurityResidual)
            }
            "high_risk_uncovered" => {
                (value.as_bool() == Some(true)).then_some(Claim::HighRiskUncovered)
            }
            _ => None,
        }
    }

    /// The input that carries it, by its exact name in the evaluator's
    /// closed vocabulary.
    pub fn input(self) -> &'static str {
        match self {
            Claim::MaxSeverity(_) => "max_residual_severity",
            Claim::SecurityResidual => "has_security_residual",
            Claim::HighRiskUncovered => "high_risk_uncovered",
        }
    }

    /// Its value as the journal wrote it: the severity's name, or `true`
    /// for a flag.
    pub fn value(self) -> &'static str {
        match self {
            Claim::MaxSeverity(severity) => severity.name(),
            Claim::SecurityResidual | Claim::HighRiskUncovered => "true",
        }
    }
}

/// One residual claim a ruling carries.
pub struct Residual<'a> {
    /// The `transition/decided` it was read from. Its sequence number is
    /// the citation a reader can go and check.
    pub ruling: &'a EventEnvelope,
    /// The phase that ruled.
    pub phase: ResidualPhase,
    pub claim: Claim,
}

/// Every residual claim in a run's journal, in journal order and then in
/// input-name order: the STRUCTURED rule inputs of every
/// `transition/decided` ruled from `verify` or `review`. Reviewer prose
/// lives in free-text notes and is never re-read here as a typed finding
/// — deriving structure from prose is repair (decision 0001).
pub fn residuals(events: &[EventEnvelope]) -> Vec<Residual<'_>> {
    let mut out = Vec::new();
    for event in events {
        if event.event_type != EventType::TransitionDecided {
            continue;
        }
        let payload = &event.payload;
        let Some(phase) = payload
            .get("from")
            .and_then(Value::as_str)
            .and_then(ResidualPhase::named)
        else {
            continue;
        };
        let Some(inputs) = payload.get("inputs").and_then(Value::as_object) else {
            continue;
        };
        for (input, raw) in inputs {
            if let Some(claim) = Claim::of(input, raw) {
                out.push(Residual {
                    ruling: event,
                    phase,
                    claim,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
