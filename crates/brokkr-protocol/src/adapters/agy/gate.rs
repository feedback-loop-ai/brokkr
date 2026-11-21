//! The engine's gate on an agy turn (#1200, G8): one `PreToolUse` hook in
//! the engine's HOME, and the mode of `brokkr driver agy` it runs.
//!
//! The record (`docs/evidence/adapters/agy-cli-1-5.md`, Native tools and
//! hooks) shows native web search runs with no rule and that no permission
//! rule stops it (`web-search-default`, `web-search-deny-rules`), that a
//! hook in the HOME sees every tool call's name before the call and that a
//! hook that fails denies (`hook-crash`). Its `gate.py` and the
//! `gate-prior-grant` turn are this gate: a tool outside the list is denied
//! by name, and a listed tool is answered `deny_unless_prior_grant`, so
//! what no rule or mode grants becomes a tool error the model sees and the
//! turn goes on, where agy's own soft denial would have ended it.

use std::io::{Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{Hands, LaunchError};
use crate::adapters::AdapterKind;

/// The driver's own word that turns it into the gate: `brokkr driver agy --
/// --gate <name>...`, each name a tool the gate leaves to agy's permission
/// check.
const GATE: &str = "--gate";

/// The tools a level-1 seat may call, in the spellings the hook is told
/// (the record's "The hook is told the tool's own name"): reads, writes,
/// edits, shell commands and the task list. This is the list's one home,
/// and a boxed seat is handed none of it.
const LISTED: [&str; 5] = [
    "read_file",
    "write_file",
    "edit_file",
    "run_command",
    "manage_task",
];

/// The built-in an MCP call goes through (`iso-07`, `iso-08`), and the one
/// tool a boxed seat's gate lists (`box-engine-workspace-tool`): the
/// engine HOME's document names one server, the box's hands, so it reaches
/// no other, and the gate reads its name alone.
const MCP_TOOL: &str = "call_mcp_tool";

/// The names the gate of a launch with `hands` is handed: the level-1 list
/// where they are native, and [`MCP_TOOL`] alone where they are boxed, so
/// every native call is denied by name (`box-other-natives`).
fn listed(hands: Hands<'_>) -> &'static [&'static str] {
    match hands {
        Hands::Native => &LISTED,
        Hands::Boxed(_) => &[MCP_TOOL],
    }
}

/// How long agy waits on the hook, in seconds (the record: "`timeout` is
/// seconds").
const TIMEOUT_SECONDS: u64 = 10;

/// The names a gate run of `kind` was handed after [`GATE`], or `None` where
/// the driver serves a seat.
pub(in crate::adapters) fn gate_listed(kind: AdapterKind, extra: &[String]) -> Option<&[String]> {
    match (kind, extra.split_first()) {
        (AdapterKind::Agy, Some((word, listed))) if word == GATE => Some(listed),
        _ => None,
    }
}

/// One hook call as agy writes it on the hook's stdin: the tool's own name
/// and nothing else read. The call's arguments are the model's, and are
/// never read for a name.
#[derive(Deserialize)]
struct HookCall {
    #[serde(rename = "toolCall")]
    tool_call: ToolCall,
}

#[derive(Deserialize)]
struct ToolCall {
    name: String,
}

/// The gate's answer, as agy reads a hook's stdout.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
enum Decision {
    DenyUnlessPriorGrant,
    Deny { reason: String },
}

/// The reason of a call the gate could not read: it names no tool.
const UNREAD: &str = "the engine's gate could not read the tool call, so it is denied";

/// The decision on the call `input` holds for a gate handed `listed`.
fn decided(listed: &[String], input: impl Read) -> Decision {
    match serde_json::from_reader::<_, HookCall>(input) {
        Ok(call) if listed.contains(&call.tool_call.name) => Decision::DenyUnlessPriorGrant,
        Ok(call) => Decision::Deny {
            reason: format!(
                "the engine's gate does not offer the tool {}",
                call.tool_call.name
            ),
        },
        Err(_) => Decision::Deny {
            reason: UNREAD.to_string(),
        },
    }
}

/// The gate mode: read one hook call from `input`, and write its decision
/// to `output` as one JSON line.
pub(in crate::adapters) fn gate(
    listed: &[String],
    input: impl Read,
    mut output: impl Write,
) -> std::io::Result<()> {
    let decision = decided(listed, input);
    let line = serde_json::to_string(&decision).expect("a decision serializes");
    output.write_all(format!("{line}\n").as_bytes())
}

/// `hooks.json` as agy reads it from `<HOME>/.gemini/config/`: a name of
/// the author's choosing mapped to that hook's events (the record's "The
/// documents the hook and MCP turns used", `documents/gate.hooks.json`).
#[derive(Serialize)]
struct Hooks {
    #[serde(rename = "brokkr-gate")]
    gate: Events,
}

#[derive(Serialize)]
struct Events {
    #[serde(rename = "PreToolUse")]
    pre_tool_use: [Matcher; 1],
}

#[derive(Serialize)]
struct Matcher {
    matcher: &'static str,
    hooks: [Hook; 1],
}

#[derive(Serialize)]
struct Hook {
    #[serde(rename = "type")]
    kind: &'static str,
    command: String,
    timeout: u64,
}

/// The engine HOME's `hooks.json` for a launch run by `brokkr`: one
/// `PreToolUse` hook on every tool, its command the gate mode handed the
/// list of `hands` ([`listed`]). agy runs the command with a shell (the
/// record), so every word of it is quoted for one. A `brokkr` whose path
/// is not UTF-8 names no command.
pub(super) fn hooks(brokkr: &Path, hands: Hands<'_>) -> Result<String, LaunchError> {
    let program = brokkr.to_str().ok_or(LaunchError::Unnamed)?;
    let mut words = vec![program, "driver", "agy", "--", GATE];
    words.extend(listed(hands));
    let quoted: Vec<String> = words.into_iter().map(quoted).collect();
    let hook = Hook {
        kind: "command",
        command: quoted.join(" "),
        timeout: TIMEOUT_SECONDS,
    };
    let hooks = Hooks {
        gate: Events {
            pre_tool_use: [Matcher {
                matcher: "",
                hooks: [hook],
            }],
        },
    };
    Ok(serde_json::to_string(&hooks).expect("the hooks document serializes"))
}

/// `word` as one shell word: single-quoted, each `'` closed, escaped and
/// reopened.
fn quoted(word: &str) -> String {
    format!("'{}'", word.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests;
