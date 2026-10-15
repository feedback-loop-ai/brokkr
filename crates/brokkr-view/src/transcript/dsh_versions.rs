//! The DSH session format versions the reader admits, and what version
//! four reads differently from version three (proposed decision 0086;
//! change `admit-dsh-session-v4`, design D3 to D6). Everything else
//! version four reads exactly as version three: the V3-to-V4 edge of the
//! 0.2.0-rc.2 writer rewrites no event type and keeps every field outside
//! its named conversions, so the version-three vocabulary and payload
//! mapping carry over, and only the conversions below change a reading.

use serde_json::Value;

use super::{
    content_blocks, content_text, dsh_message_blocks, dsh_part, DshBlock, DshDirection, Part, Role,
    RowOutcome, DSH_QUIET_THREE,
};

/// The DSH on-disk format versions this reader admits (design D3). The set
/// is closed and exactly `{0, 3, 4}`; the enum makes every dispatch site
/// exhaustive so a further version cannot be added silently.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum DshVersion {
    Zero,
    Three,
    Four,
}

impl DshVersion {
    /// The version a header's `version` member names, read from its parsed
    /// value and its recorded token's digit signature together (design
    /// D3): zero with an empty signature, three or four with that one
    /// digit. Any other pair names no admitted version.
    pub(super) fn admitted(parsed: f64, signature: &str) -> Option<DshVersion> {
        [
            (0.0, "", DshVersion::Zero),
            (3.0, "3", DshVersion::Three),
            (4.0, "4", DshVersion::Four),
        ]
        .into_iter()
        .find(|&(value, digits, _)| parsed == value && signature == digits)
        .map(|(_, _, version)| version)
    }
}

/// The names version four adds to version three's quiet vocabulary
/// (design D4): `developer/message`, native to version four, whose
/// tool-addition and tool-removal blocks are the writer's notices of a
/// changed tool set, not a turn anyone spoke.
const QUIET_FOUR_ADDS: &[&str] = &["developer/message"];

/// Version four's quiet vocabulary: version three's, which the edge
/// carries over unrenamed, and the names it adds.
pub(super) fn quiet_four(kind: &str) -> bool {
    DSH_QUIET_THREE.contains(&kind) || QUIET_FOUR_ADDS.contains(&kind)
}

/// A message row's blocks under `version`: version four reads version
/// three's content mapping save the retired wrapper block.
pub(super) fn message_blocks(
    data: &Value,
    role: Role,
    version: DshVersion,
) -> RowOutcome<DshBlock> {
    match (version, data.get("content")) {
        (DshVersion::Zero | DshVersion::Three, _) | (DshVersion::Four, Some(Value::String(_))) => {
            dsh_message_blocks(data, role)
        }
        (DshVersion::Four, content) => content_blocks(content, role, four_part),
    }
}

/// A `tool/result` message's blocks under `version`: version three's
/// wrapper block inside the message, or version four's tool-role message.
pub(super) fn result_blocks(message: &Value, version: DshVersion) -> RowOutcome<DshBlock> {
    match version {
        DshVersion::Zero | DshVersion::Three => dsh_message_blocks(message, Role::Tool),
        DshVersion::Four => tool_role_result(message),
    }
}

/// Whether a content member is the `tool-result` wrapper version four
/// retires from every content list it interprets.
fn retired_wrapper(part: &Value) -> bool {
    part.get("type").and_then(Value::as_str) == Some("tool-result")
}

/// One version-four content member: version three's mapping, save the
/// retired wrapper, which the writer refuses wherever it reads content and
/// so is a counted unrecognized member here, never a result (design D5).
fn four_part(part: &Value) -> Part<DshBlock> {
    if retired_wrapper(part) {
        return Part::Unrecognized;
    }
    dsh_part(part)
}

/// A version-four `tool/result` message (design D5): role `tool`, the call
/// it settles named by `toolCallId`, its output the message's own
/// `content`, rendered as version three renders the wrapper's. A message
/// whose content still holds a retired wrapper is one the writer refuses,
/// so the row is counted and supplies no result.
fn tool_role_result(message: &Value) -> RowOutcome<DshBlock> {
    let content = message.get("content");
    let members = content.and_then(Value::as_array);
    if members.is_some_and(|members| members.iter().any(retired_wrapper)) {
        return RowOutcome::nothing(true);
    }
    let id = message
        .get("toolCallId")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty());
    let output = content.map(content_text).unwrap_or_default();
    RowOutcome::one(DshBlock::tool(id, DshDirection::Result, output), Role::Tool)
}

#[cfg(test)]
mod tests;
