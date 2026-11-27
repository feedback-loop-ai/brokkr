//! A gate that moved the operated repo's HEAD or changed its tracked
//! tree closes its effect indeterminate with a marker and evidence
//! packed into the frozen contract's free-text `reason`.
//! Every writer and reader goes through this codec, so the bytes the
//! journal carries are spelled once.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// The marker, and the park reason a marked indeterminate folds to.
pub const PREFIX: &str = "GATE-MOVED-HEAD";

/// The marker for changed or unreadable tracked tree content.
pub const TREE_PREFIX: &str = "GATE-CHANGED-TREE";

const DRIVER_STDERR_SUFFIX: &str = "; driver stderr: ";

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
    encode_marked(PREFIX, moved)
}

#[derive(Serialize)]
#[serde(untagged)]
enum TreeEvidence<'a> {
    Changed {
        omitted: usize,
        paths: &'a [String],
    },
    Unreadable {
        end_unreadable: bool,
        start_unreadable: bool,
    },
}

/// Record changed tracked paths and the number beyond the evidence limit.
pub fn encode_tree_change(paths: &[String], omitted: usize) -> String {
    encode_marked(TREE_PREFIX, &TreeEvidence::Changed { omitted, paths })
}

/// Record which endpoint readings were unreadable.
pub fn encode_tree_unreadable(start: bool, end: bool) -> String {
    encode_marked(
        TREE_PREFIX,
        &TreeEvidence::Unreadable {
            end_unreadable: end,
            start_unreadable: start,
        },
    )
}

fn encode_marked(prefix: &str, evidence: &impl Serialize) -> String {
    let evidence = serde_json::to_string(evidence).expect("gate evidence always serializes");
    format!("{prefix} {evidence}")
}

/// `None` when `reason` does not carry the marker. A marked reason whose
/// evidence does not read as `T` is still marked: it yields the parse
/// error, so a reader can tell the marker from its evidence. `T` is the
/// reader's domain: [`MovedHead`] reads the heads [`encode`] writes, while
/// a `serde_json::Value` reader keeps any JSON a journal carries. Either
/// reader accepts the driver-stderr evidence suffix after the JSON.
pub fn decode<T: DeserializeOwned>(reason: &str) -> Option<Result<T, serde_json::Error>> {
    decode_marked(PREFIX, reason)
}

/// Read a tracked-tree marker and its evidence, with the same suffix as [decode].
pub fn decode_tree<T: DeserializeOwned>(reason: &str) -> Option<Result<T, serde_json::Error>> {
    decode_marked(TREE_PREFIX, reason)
}

fn decode_marked<T: DeserializeOwned>(
    prefix: &str,
    reason: &str,
) -> Option<Result<T, serde_json::Error>> {
    let rest = reason.strip_prefix(prefix)?.strip_prefix(' ')?;
    let mut json = serde_json::Deserializer::from_str(rest);
    Some(T::deserialize(&mut json).and_then(|value| {
        let trailing = &rest[json.into_iter::<T>().byte_offset()..];
        if trailing.is_empty() || trailing.starts_with(DRIVER_STDERR_SUFFIX) {
            Ok(value)
        } else {
            // Let the JSON parser handle trailing whitespace and report other text.
            serde_json::from_str(rest)
        }
    }))
}

#[cfg(test)]
mod tests;
