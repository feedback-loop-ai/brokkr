//! Agy's arm of the engine's MCP set (#1200), beside Claude's and Qoder's:
//! the document the engine writes to an agy launch's HOME, by the set's one
//! builder (`document`), and the rule each engine tool is granted by. A
//! child module because its parent stands at the production file ceiling.
//!
//! The facts are the controller's measurement of Antigravity CLI 1.5.13 on
//! Linux (`docs/evidence/adapters/agy-cli-1-5.md`, MCP isolation (SI2)): a
//! server in the HOME's `.gemini/config/mcp_config.json` starts cold and on
//! a resume (`iso-07`, `iso-09`); the model reaches it through the built-in
//! `call_mcp_tool`, and a call needs an `mcp(<server>/<tool>)` allow rule,
//! without which the server still starts and the call is soft-denied
//! (`iso-08`).

use std::collections::BTreeMap;

use serde_json::Value;

use super::{document, Transport};
use crate::hands::{SERVER_NAME, TOOL_NAME};

/// What the engine's set gives one agy launch: the document its HOME holds,
/// and the allow rule of each tool that document serves, as Agy spells a
/// rule. No rule is no server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Served {
    pub(crate) document: Value,
    pub(crate) rules: Vec<String>,
}

/// The engine's set at an agy launch: the box's `hands` and the rule of
/// their one tool, or, with no hands, the empty document and no rule.
/// `None` where a path the hands name is not UTF-8, since no document value
/// represents it exactly ([`Transport::expand`]). Agy carries no broker:
/// its broker grant is unmeasured, so the edge refuses a brokered set first.
pub(crate) fn served(hands: Option<Transport<'_>>) -> Option<Served> {
    let rules: Vec<String> = hands.iter().map(|_| hands_rule()).collect();
    let document = match hands {
        Some(hands) => hands.expand(&[]).map(|_| hands.document())?,
        None => document(std::path::Path::new(""), &BTreeMap::new()),
    };
    Some(Served { document, rules })
}

/// The allow rule of the typed hands' one tool, `mcp(<server>/<tool>)`: the
/// one spelling an agy seat is allowed the box's hands by (`iso-07`).
fn hands_rule() -> String {
    format!("mcp({SERVER_NAME}/{TOOL_NAME})")
}
