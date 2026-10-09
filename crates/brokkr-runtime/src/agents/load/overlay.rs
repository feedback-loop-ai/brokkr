//! Agent-level `extends` (#360; the operator's ruling of 2026-10-07 on
//! decisions 0041 and 0058): a scoped office is an OVERLAY of a library
//! office, written as its difference alone — `{"extends": "implementer",
//! "models": [...], "efforts": {...}}` — rather than a second file that
//! restates the first (decision 0071 ruling 5).
//!
//! The overlay is merged here, at the loader's edge, into the definition
//! it stands for, and that definition is then parsed by exactly the rules
//! a base faces: no second validator exists to drift from the first.
//!
//! - `models` replaces the base's chain, and `efforts` replaces its
//!   efforts whole (absent, the overlay hires none): the base's efforts
//!   are keyed by the base's candidates, which the overlay just replaced.
//! - `charter` replaces the base's charter, still contained in the same
//!   library, and `inputs` the evaluator inputs the office declares.
//! - `tools` and `hands` are the office's power. An overlay that writes
//!   either, or drops one the base declares, names it under `replaces`
//!   with its reason, so a loss can never hide inside a copy. One written
//!   without a reason is refused, and so is a reason for nothing.
//! - An overlay extends a base office, one level: a base that itself
//!   extends is refused, which also refuses every cycle.

use std::path::Path;

use serde_json::{Map, Value};

use super::{invalid, object, only_keys, read_request_source, string, LibraryError};
use crate::agents::{valid_name, NAME_GRAMMAR};

/// The two fields an overlay may change only with a stated reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Power {
    Tools,
    Hands,
}

impl Power {
    const ALL: [Power; 2] = [Power::Tools, Power::Hands];

    fn key(self) -> &'static str {
        match self {
            Power::Tools => "tools",
            Power::Hands => "hands",
        }
    }
}

/// The keys an overlay may write: what it is an overlay OF, the chain it
/// hires, and the replacements it states.
const OVERLAY_KEYS: [&str; 8] = [
    "extends", "models", "efforts", "charter", "inputs", "tools", "hands", "replaces",
];

/// The definition `source` stands for: itself when it extends nothing,
/// otherwise its base with the overlay applied.
pub(super) fn effective(root: &Path, what: &str, source: Value) -> Result<Value, LibraryError> {
    let overlay = object(&source, what)?;
    if !overlay.contains_key("extends") {
        return Ok(source);
    }
    only_keys(overlay, &OVERLAY_KEYS, what)?;
    let base_name = extended(overlay, what)?;
    let mut merged = base(root, what, &base_name)?;
    merge_chain(&mut merged, overlay);
    let reasons = replaces(overlay, what)?;
    for power in Power::ALL {
        merge_power(&mut merged, overlay, &reasons, power, what, &base_name)?;
    }
    Ok(Value::Object(merged))
}

/// The base an overlay names under `extends`: a valid agent name, and only
/// beside the `models` every overlay states.
fn extended(overlay: &Map<String, Value>, what: &str) -> Result<String, LibraryError> {
    let base_name = string(overlay, "extends", what)?;
    if !valid_name(&base_name) {
        return invalid(format!(
            "{what} 'extends' names '{base_name}', which does not match {NAME_GRAMMAR}"
        ));
    }
    if !overlay.contains_key("models") {
        return invalid(format!(
            "{what} extends '{base_name}' but writes no 'models'; an overlay states the \
             chain it hires"
        ));
    }
    Ok(base_name)
}

/// `models` and `efforts` replace the base's whole; `charter` and `inputs`
/// replace it only when the overlay writes them.
fn merge_chain(merged: &mut Map<String, Value>, overlay: &Map<String, Value>) {
    for key in ["models", "efforts"] {
        match overlay.get(key) {
            Some(value) => merged.insert(key.to_string(), value.clone()),
            None => merged.remove(key),
        };
    }
    for key in ["charter", "inputs"] {
        if let Some(value) = overlay.get(key) {
            merged.insert(key.to_string(), value.clone());
        }
    }
}

/// One power: kept when the overlay is silent, replaced or dropped only
/// under a reason, and a reason for nothing is refused.
fn merge_power(
    merged: &mut Map<String, Value>,
    overlay: &Map<String, Value>,
    reasons: &Map<String, Value>,
    power: Power,
    what: &str,
    base_name: &str,
) -> Result<(), LibraryError> {
    let key = power.key();
    match (overlay.get(key), reasons.get(key)) {
        (None, None) => {}
        (Some(_), None) => {
            return invalid(format!(
                "{what} writes '{key}' over '{base_name}' without a reason; an overlay \
                 that changes an office's {key} names it under 'replaces' with why"
            ))
        }
        (Some(value), Some(_)) => _ = merged.insert(key.to_string(), value.clone()),
        (None, Some(_)) => {
            if merged.remove(key).is_none() {
                return invalid(format!(
                    "{what} 'replaces' gives a reason for '{key}', which neither it \
                     writes nor '{base_name}' declares"
                ));
            }
        }
    }
    Ok(())
}

/// The base office's own definition, read as strictly as any definition.
fn base(root: &Path, what: &str, name: &str) -> Result<Map<String, Value>, LibraryError> {
    let path = root.join(format!("{name}.json"));
    if !path.is_file() {
        return invalid(format!(
            "{what} extends '{name}', which is not an agent in this library"
        ));
    }
    let source = read_request_source(&path)?;
    let map = object(&source, &format!("agent '{name}' ({})", path.display()))?;
    if map.contains_key("extends") {
        return invalid(format!(
            "{what} extends '{name}', which itself extends another office; an overlay \
             extends a base office, one level"
        ));
    }
    Ok(map.clone())
}

/// `replaces`: power → its reason, each a non-empty string, over the
/// closed set of powers.
fn replaces(overlay: &Map<String, Value>, what: &str) -> Result<Map<String, Value>, LibraryError> {
    let Some(raw) = overlay.get("replaces") else {
        return Ok(Map::new());
    };
    let what = format!("{what} 'replaces'");
    let reasons = object(raw, &what)?;
    only_keys(reasons, &Power::ALL.map(Power::key), &what)?;
    for key in reasons.keys() {
        string(reasons, key, &what)?;
    }
    Ok(reasons.clone())
}

#[cfg(test)]
mod tests;
