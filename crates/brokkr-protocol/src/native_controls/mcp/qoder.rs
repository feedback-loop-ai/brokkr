//! Qoder's arm of the engine's MCP set (#1178), beside Claude's in its
//! parent: the measured isolation fragment a qoder launch carries, in its
//! one home, and what the engine's set gives that launch, written by the
//! set's one builder (`document`), never by a copy of it. A child module
//! because its parent stands at the production file ceiling.
//!
//! The facts are the controller's measurement of Qoder CLI 1.1.92 on Linux
//! (`docs/evidence/adapters/qoder-cli-1-1.md`, MCP isolation (SI2)):
//!
//! - `--strict-mcp-config` alone still starts the user-scope server inside
//!   the turn's own process tree (Q03 to Q06);
//! - `--setting-sources ""` alone still starts the project's `.mcp.json`
//!   (Q09);
//! - together, beside an empty or an engine file, they started no planted
//!   sentinel, cold or replacement (Q07, Q10, Q11, Q14, Q15);
//! - the engine's server answered `tools/call` only under an
//!   `--allowed-tools` grant naming its tool `mcp__<server>__<tool>`
//!   (Q12 to Q15).

use std::collections::BTreeMap;

use serde_json::Value;

use super::super::listed::hands_grant;
use super::{document, Transport};

/// The isolation every qoder launch carries, with `file` the engine's MCP
/// document: the strict flag, the file and the empty setting sources, all
/// three together. This is their one home: no launch path writes the strict
/// flag without the sources, because Q03 to Q06 show that leaks the
/// user-scope server.
pub(crate) fn isolation(file: &str) -> [String; 5] {
    [
        "--strict-mcp-config",
        "--mcp-config",
        file,
        "--setting-sources",
        "",
    ]
    .map(String::from)
}

/// What the engine's set gives one qoder launch: the document its file
/// holds, and the grant of each tool that document serves, as Qoder names
/// it. No grant is no server, so the launch denies the MCP meta-tools.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Served {
    pub(crate) document: Value,
    pub(crate) grants: Vec<String>,
}

/// The engine's set at a qoder launch: the empty document with no grant,
/// or the box's `hands` and the grant of their one tool, the tool Claude's
/// listing grants them by. `None` where a path the hands name is not UTF-8,
/// since no document value represents it exactly ([`Transport::expand`]).
/// Qoder carries no broker: its broker grant is unmeasured, so the edge
/// refuses a brokered set before it reaches here.
pub(crate) fn served(hands: Option<Transport<'_>>) -> Option<Served> {
    let Some(hands) = hands else {
        let empty = document(std::path::Path::new(""), &BTreeMap::new());
        return Some(Served {
            document: empty,
            grants: Vec::new(),
        });
    };
    hands.expand(&[])?;
    Some(Served {
        document: hands.document(),
        grants: vec![hands_grant()],
    })
}

#[cfg(test)]
mod tests;
