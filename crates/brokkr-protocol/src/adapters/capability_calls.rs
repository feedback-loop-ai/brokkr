//! One harness tool call, read at the edge (decision 0065 slice two, CC1
//! and CC2): its whole tool identity and the harness's own call id, typed
//! once from the event before anything is clamped for display.
//!
//! This module is the one home of the observation type and its wire
//! encoding (design D9). The adapters read calls into it here, and the
//! engine's private wire edge decodes the same type, from the same
//! encoding, under [`OBSERVATION_KEY`]. The legacy lowering is its
//! production consumer today, and it writes exactly the `tool` field the
//! rows have always carried: no call id, no server, no new key. Emitting
//! the observation itself waits until every engine consumer is installed,
//! so this module adds no outward fact.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The private checkpoint key an observation rides under from the driver
/// to the engine, which consumes it before any append. The store never
/// admits it.
pub const OBSERVATION_KEY: &str = "observation";

/// The longest tool name a legacy row shows, in characters.
const SHOWN_CHARS: usize = 80;

/// How Claude and dsh name an MCP tool, as U0 measured both:
/// `mcp__<server>__<tool>`.
const MCP_PREFIX: &str = "mcp__";
const MCP_SEPARATOR: &str = "__";

/// Codex's item type for an MCP call. It is a category, never a tool name:
/// the server and tool ride beside it on the item.
const CODEX_MCP_ITEM: &str = "mcp_tool_call";

/// The telemetry format that reported a call. Lanetally's wrapper speaks
/// Claude's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Claude,
    Codex,
    Dsh,
}

/// What a call's telemetry says it called, encoded with its variant under
/// `kind`.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Tool {
    /// A tool the harness named, whole: Claude's and dsh's `name`, or the
    /// item type of any Codex item that is not an MCP call. An empty name
    /// is kept as reported; what it identifies is the consumer's question.
    Named { name: String },
    /// An MCP call, by the server and tool its telemetry names.
    Mcp { server: String, tool: String },
    /// An MCP call whose telemetry does not identify its server and tool
    /// (CC2's typed limitation). `reported` is the harness's own text for
    /// it, from which no server or tool is guessed.
    McpUnidentified { reported: String },
    /// The event names no tool at all.
    Missing,
}

/// One tool call, as its harness measured it. Only this module reads one
/// from a harness event; any other crate decodes one from its encoding.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub format: Format,
    /// The harness's own id for the call: Claude's `toolu_*`, Codex's
    /// `item_N`, dsh's `callId`. `None` (encoded `null`) when the event
    /// carries none.
    pub call: Option<String>,
    pub tool: Tool,
}

impl Observation {
    /// One `tool_use` block of a Claude `assistant` message.
    pub(super) fn claude(block: &Value) -> Observation {
        Observation {
            format: Format::Claude,
            call: text(block, "/id"),
            tool: named(text(block, "/name")),
        }
    }

    /// One Codex `item.started` or `item.completed` event. The pair shares
    /// its item id; counting it once is not this reading's to decide.
    pub(super) fn codex(event: &Value) -> Observation {
        let tool = match text(event, "/item/type") {
            Some(kind) if kind == CODEX_MCP_ITEM => {
                match (text(event, "/item/server"), text(event, "/item/tool")) {
                    (Some(server), Some(tool)) if !server.is_empty() && !tool.is_empty() => {
                        Tool::Mcp { server, tool }
                    }
                    _ => Tool::McpUnidentified { reported: kind },
                }
            }
            Some(name) => Tool::Named { name },
            None => Tool::Missing,
        };
        Observation {
            format: Format::Codex,
            call: text(event, "/item/id"),
            tool,
        }
    }

    /// One dsh transcript `tool/call` event.
    pub(super) fn dsh(event: &Value) -> Observation {
        Observation {
            format: Format::Dsh,
            call: text(event, "/data/callId"),
            tool: named(text(event, "/data/name")),
        }
    }

    /// The `tool` a legacy row carries for this call, clamped for display;
    /// `None` where Claude's row carries no tool and dsh writes no row.
    /// Each format shows what it always showed: Codex its item category,
    /// `unknown` when there is none, Claude and dsh the name as reported.
    /// The call id is read and dropped here, because no legacy row has it.
    pub(super) fn legacy_tool(self) -> Option<String> {
        let Self {
            format,
            call: _unrecorded,
            tool,
        } = self;
        let shown = match (format, tool) {
            (Format::Codex, Tool::Named { name: kind }) => kind,
            (Format::Codex, Tool::Mcp { .. }) => CODEX_MCP_ITEM.to_string(),
            (Format::Codex, Tool::McpUnidentified { reported }) => reported,
            (Format::Codex, Tool::Missing) => "unknown".to_string(),
            (Format::Claude | Format::Dsh, Tool::Named { name }) if name.is_empty() => return None,
            (Format::Claude | Format::Dsh, Tool::Missing) => return None,
            (Format::Claude | Format::Dsh, Tool::Named { name }) => name,
            (Format::Claude | Format::Dsh, Tool::McpUnidentified { reported }) => reported,
            (Format::Claude | Format::Dsh, Tool::Mcp { server, tool }) => {
                format!("{MCP_PREFIX}{server}{MCP_SEPARATOR}{tool}")
            }
        };
        Some(shown.chars().take(SHOWN_CHARS).collect())
    }
}

fn text(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// A Claude or dsh tool name. One under the MCP prefix is an MCP call, split
/// at the first separator after it; one that does not split into a server
/// and a tool is kept whole as an unidentified MCP call, never as a native
/// tool of that name.
fn named(name: Option<String>) -> Tool {
    let Some(name) = name else {
        return Tool::Missing;
    };
    let Some(rest) = name.strip_prefix(MCP_PREFIX) else {
        return Tool::Named { name };
    };
    match rest.split_once(MCP_SEPARATOR) {
        Some((server, tool)) if !server.is_empty() && !tool.is_empty() => Tool::Mcp {
            server: server.to_string(),
            tool: tool.to_string(),
        },
        _ => Tool::McpUnidentified { reported: name },
    }
}

#[cfg(test)]
mod tests;
