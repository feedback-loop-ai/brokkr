//! The GATE-MOVED-HEAD marker: a gate that moved the operated repo's
//! HEAD closes its effect indeterminate with this marker and the two
//! observed heads packed into the frozen contract's free-text `reason`.
//! Every writer and reader goes through this codec, so the bytes the
//! journal carries are spelled once.

use serde::{Deserialize, Serialize};

/// The marker, and the park reason a marked indeterminate folds to.
pub const PREFIX: &str = "GATE-MOVED-HEAD";

/// The HEAD a gate step started on and the HEAD it ended on, either
/// unreadable as `None`. Fields are declared in key order, so a reason
/// encoded from this type keeps the bytes of the map it replaced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovedHead {
    pub head_at_end: Option<String>,
    pub head_at_start: Option<String>,
}

/// The indeterminate reason that records `moved`: the marker, one space,
/// and the evidence as compact JSON.
pub fn encode(moved: &MovedHead) -> String {
    format!(
        "{PREFIX} {}",
        serde_json::to_string(moved).expect("two optional strings always serialize")
    )
}

/// `None` when `reason` does not carry the marker. A marked reason whose
/// evidence does not read as [`MovedHead`] is still marked: it yields the
/// parse error, so a reader can tell the marker from its evidence.
pub fn decode(reason: &str) -> Option<Result<MovedHead, serde_json::Error>> {
    reason
        .strip_prefix(PREFIX)?
        .strip_prefix(' ')
        .map(serde_json::from_str)
}

#[cfg(test)]
mod tests;
