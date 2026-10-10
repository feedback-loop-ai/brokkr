//! `forced_crew` (#360; decision 0041's addendum of 2026-10-07): a recipe
//! whose crew must not fall back — a wager's arm, the standby hedge,
//! review-first's named firing — says so at its root, naming the decision
//! that forces it. Its forced seats hire overlays whose chain holds exactly
//! the forced models, and the roster test reads this declaration in place
//! of a list of recipe names. The compiler admits the reason and nothing
//! else: a forced crew is a property of the offices it hires, never a
//! switch the engine reads at run time.

use serde_json::{Map, Value};

use super::{invalid, CompileError};

/// Admit one layer's `forced_crew`: absent, or a reason that is a string
/// with something in it. Anything else is refused naming the layer, never
/// echoing the value.
pub(super) fn admit(layer: &str, document: &Map<String, Value>) -> Result<(), CompileError> {
    match document.get("forced_crew") {
        None => Ok(()),
        Some(Value::String(reason)) if !reason.trim().is_empty() => Ok(()),
        Some(_) => Err(invalid(format!(
            "recipe {} declares 'forced_crew' without a reason; a forced crew names the \
             decision that forces it, as a non-empty string (decision 0041's addendum of \
             2026-10-07)",
            crate::bundle::bounded_site(layer)
        ))),
    }
}

#[cfg(test)]
mod tests;
