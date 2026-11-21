//! A seat's declared inputs (decision 0007's input-provenance lint).
//!
//! [`declared`] refuses in the one cause [`InputRefusal`], one variant per
//! check, so a test can name the check that fired. Its `Display` is the
//! operator's text byte for byte; [`seat_inputs`] folds it into the
//! compile refusal, the one place it becomes text (#353).

use brokkr_core::policy::{is_declarable, Machine};
use serde_json::{json, Value};

use super::super::{is_engine_owned, CompileError, ResolvedSeat};

/// Why a seat's declared inputs were refused, each field borrowed from the
/// phase name, the declaration or the phase's referenced inputs.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
enum InputRefusal<'a> {
    /// The declaration is not an array.
    #[error("seat '{phase}' inputs must be an array of strings")]
    NotStrings { phase: &'a str },
    /// The declaration names an input the journal computes.
    #[error(
        "seat '{phase}' declares engine-owned input '{name}'; \
         journal-computed truth is never accepted from a seat"
    )]
    EngineOwned { phase: &'a str, name: &'a str },
    /// The declaration names an input outside the seat-declarable vocabulary.
    #[error(
        "seat '{phase}' declares unknown input '{name}'; known: \
         the evaluator's closed vocabulary minus engine-owned"
    )]
    Unknown { phase: &'a str, name: &'a str },
    /// The phase's rules read an input the declaration lacks.
    #[error(
        "phase '{phase}' rules reference input '{needed}' but \
         seat '{phase}' does not declare it; the rule could \
         never fire from seat data"
    )]
    Undeclared { phase: &'a str, needed: &'a str },
}

/// Input provenance (decision 0007): every input the phase's rules
/// reference must be engine-computed or supplied by this seat's
/// declaration; a declaration may only name known, non-engine-owned
/// evaluator inputs. An agent-resolved seat faces this lint exactly as an
/// inline one does — its declaration is the agent's, or the 0007 default.
pub(super) fn seat_inputs(
    machine: &Machine,
    phase: &str,
    raw: &Value,
    agent_seat: &Option<ResolvedSeat>,
) -> Result<Vec<String>, CompileError> {
    let referenced = machine.seat_inputs(phase);
    let raw_inputs = match agent_seat
        .as_ref()
        .and_then(|agent_seat| agent_seat.inputs.clone())
    {
        Some(declared) => Some(json!(declared)),
        None => raw.get("inputs").cloned(),
    };
    declared(phase, raw_inputs.as_ref(), &referenced)
        .map_err(|refusal| CompileError::Invalid(refusal.to_string()))
}

/// The lint's four checks over the declaration, in their order: an
/// undeclared seat takes the referenced inputs, and a declaration's
/// non-string elements are dropped.
fn declared<'a>(
    phase: &'a str,
    raw_inputs: Option<&'a Value>,
    referenced: &'a [String],
) -> Result<Vec<String>, InputRefusal<'a>> {
    let inputs = match raw_inputs {
        None => referenced.to_vec(),
        Some(declared) => {
            let declared = declared
                .as_array()
                .map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>())
                .ok_or(InputRefusal::NotStrings { phase })?;
            refuse_names(phase, &declared)?;
            for needed in referenced {
                if !declared.contains(&needed.as_str()) {
                    return Err(InputRefusal::Undeclared { phase, needed });
                }
            }
            declared.into_iter().map(str::to_string).collect()
        }
    };
    Ok(inputs)
}

/// The declaration's names, each refused when engine-owned, then when not
/// declarable.
fn refuse_names<'a>(phase: &'a str, declared: &[&'a str]) -> Result<(), InputRefusal<'a>> {
    for &name in declared {
        if is_engine_owned(name) {
            return Err(InputRefusal::EngineOwned { phase, name });
        }
        if !is_declarable(name) {
            return Err(InputRefusal::Unknown { phase, name });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
