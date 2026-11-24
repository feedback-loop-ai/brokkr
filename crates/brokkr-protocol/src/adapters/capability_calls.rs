//! One harness tool call, read at the edge (decision 0065 slice two, CC1
//! and CC2): its whole tool identity and the harness's own call id, typed
//! once from the event before anything is clamped for display.
//!
//! This module is the one home of the observation type and its wire
//! encoding (design D9). The adapters read calls into it here, and the
//! engine's private wire edge decodes the same type, from the same
//! encoding, under [`OBSERVATION_KEY`]. Every engine consumer is installed
//! (U4b, U4e and U4f), so each row a call lowers to carries its
//! observation beside the `tool` the row has always shown (U4f2). The
//! engine takes the observation off before any append and alone writes
//! the call's public attribution; the store never sees it.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::row::{mcp_separator, Row};

/// The private checkpoint key an observation rides under from the driver
/// to the engine, which consumes it before any append. The store never
/// admits it.
pub const OBSERVATION_KEY: &str = "observation";

/// The longest tool name a legacy row shows, in characters.
const SHOWN_CHARS: usize = 80;

/// How Claude and dsh name an MCP tool, as U0 measured both:
/// `mcp__<server>__<tool>`.
pub(super) const MCP_PREFIX: &str = "mcp__";
pub(super) const MCP_SEPARATOR: &str = "__";

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
    pub(super) fn claude(block: &Row<'_>) -> Observation {
        Observation {
            format: Format::Claude,
            call: block.text("/id"),
            tool: block.tool("/name"),
        }
    }

    /// One Codex `item.started` or `item.completed` event. The pair shares
    /// its item id; counting it once is not this reading's to decide.
    pub(super) fn codex(event: &Row<'_>) -> Observation {
        let tool = if codex_item_is(event, CODEX_MCP_ITEM) {
            match (event.text("/item/server"), event.text("/item/tool")) {
                (Some(server), Some(tool)) if !server.is_empty() && !tool.is_empty() => {
                    Tool::Mcp { server, tool }
                }
                _ => Tool::McpUnidentified {
                    reported: event.text(CODEX_ITEM_TYPE).unwrap_or_default(),
                },
            }
        } else {
            match event.text(CODEX_ITEM_TYPE) {
                Some(name) => Tool::Named { name },
                None => Tool::Missing,
            }
        };
        Observation {
            format: Format::Codex,
            call: event.text("/item/id"),
            tool,
        }
    }

    pub(super) fn onto_codex(self, event: &Row<'_>, row: Value) -> Value {
        let shown = event
            .text(CODEX_ITEM_TYPE)
            .unwrap_or_else(|| "unknown".into());
        self.onto_with_tool(row, Some(shown.chars().take(SHOWN_CHARS).collect()))
    }

    /// One dsh transcript `tool/call` event.
    pub(super) fn dsh(event: &Value) -> Observation {
        Observation {
            format: Format::Dsh,
            call: text(event, "/data/callId"),
            tool: named(text(event, "/data/name")),
        }
    }

    /// The tool display for Claude and dsh, clamped separately from the
    /// full observation sent to the engine.
    pub(super) fn shown_tool(&self) -> Option<String> {
        let shown = match &self.tool {
            Tool::Named { name } if name.is_empty() => return None,
            Tool::Missing => return None,
            Tool::Named { name } => name.clone(),
            Tool::McpUnidentified { reported } => reported.clone(),
            Tool::Mcp { server, tool } => format!("{MCP_PREFIX}{server}{MCP_SEPARATOR}{tool}"),
        };
        Some(shown.chars().take(SHOWN_CHARS).collect())
    }

    /// `row`, the object this call's event lowers to, carrying the call:
    /// the `tool` [`Self::shown_tool`] shows, where it shows one, and the
    /// observation itself under [`OBSERVATION_KEY`], whole and unclamped,
    /// for the engine to consume before any append (U4f2).
    pub(super) fn onto(self, row: Value) -> Value {
        let shown = self.shown_tool();
        self.onto_with_tool(row, shown)
    }

    fn onto_with_tool(self, mut row: Value, shown: Option<String>) -> Value {
        if let Some(tool) = shown {
            row["tool"] = Value::String(tool);
        }
        row[OBSERVATION_KEY] = serde_json::to_value(self).expect("an observation encodes");
        row
    }
}

fn text(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_string)
}

const CODEX_ITEM_TYPE: &str = "/item/type";

/// Compare the Codex item category before masking its displayed spelling.
pub(super) fn codex_item_is(event: &Row<'_>, kind: &str) -> bool {
    event.is(CODEX_ITEM_TYPE, kind)
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
    match mcp_separator(rest) {
        Some(separator) => Tool::Mcp {
            server: rest[..separator.start].to_string(),
            tool: rest[separator.end..].to_string(),
        },
        None => Tool::McpUnidentified { reported: name },
    }
}

#[cfg(test)]
mod tests;
