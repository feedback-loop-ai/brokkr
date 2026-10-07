//! Residual findings as the journal carries them, and the word of the
//! operator annotation that closes one (decision 0047). The engine admits
//! a supersede by this derivation and the view renders it, so a readout
//! change can never move which supersedes the engine accepts (#351).

use serde_json::Value;

use crate::envelope::{EventEnvelope, EventType};
use crate::policy::SEVERITY_ORDER;

/// The phases whose rulings carry residual findings.
const RESIDUAL_PHASES: [&str; 2] = ["verify", "review"];

/// The command word of the operator annotation that closes a residual
/// finding (decision 0047 ruling 1). Deliberately NOT an
/// [`OperatorCommand`](crate::fold::OperatorCommand): those are what a
/// PARKED run admits, and a supersede is only ever written on a terminal
/// one.
pub const SUPERSEDE: &str = "supersede";

/// One residual claim a ruling carries: the input that carries it, by its
/// exact name in the evaluator's closed vocabulary, and its value.
pub struct Residual<'a> {
    /// The `transition/decided` it was read from. Its sequence number is
    /// the citation a reader can go and check.
    pub ruling: &'a EventEnvelope,
    /// The phase that ruled: `verify` or `review`.
    pub phase: &'a str,
    pub input: &'a str,
    pub value: String,
}

/// The one severity input and the two boolean inputs that carry a
/// residual claim, read through the evaluator's own closed vocabulary.
/// A severity of `none`, a false flag, an unranked severity name and any
/// key outside the vocabulary all carry no finding.
fn residual_value(key: &str, value: &Value) -> Option<String> {
    match key {
        "max_residual_severity" => {
            let name = value.as_str()?;
            let rank = SEVERITY_ORDER.iter().position(|known| *known == name)?;
            match rank {
                0 => None,
                _ => Some(name.to_string()),
            }
        }
        "has_security_residual" | "high_risk_uncovered" => match value.as_bool() {
            Some(true) => Some("true".to_string()),
            _ => None,
        },
        _ => None,
    }
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
            .filter(|from| RESIDUAL_PHASES.contains(from))
        else {
            continue;
        };
        let Some(inputs) = payload.get("inputs").and_then(Value::as_object) else {
            continue;
        };
        for (input, raw) in inputs {
            if let Some(value) = residual_value(input, raw) {
                out.push(Residual {
                    ruling: event,
                    phase,
                    input,
                    value,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
