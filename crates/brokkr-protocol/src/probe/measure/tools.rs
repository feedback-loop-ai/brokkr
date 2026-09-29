//! The tools and MCP servers a turn lists: which tools are the CLI's own,
//! which reach the network, what switches each off, and which MCP servers
//! reached the turn beside the hands server (#467). Pure, like `measure`.

use super::{listed, on_turn, Stream, Turn};
use crate::hands::SERVER_NAME;
use crate::probe::facts::{Capability, Fact};
use crate::probe::plan::{UserConfig, USER_SCOPE_SERVER};

/// What a native egress tool's name holds, once folded to lowercase
/// letters and digits: web search, fetch, browsing and grounding.
const EGRESS_WORDS: [&str; 4] = ["web", "fetch", "browse", "grounding"];

/// The shipped harnesses' own tools that reach no network: Claude Code's,
/// Codex's and dsh's. A name that is neither one of these nor egress by
/// [`EGRESS_WORDS`] is not recognised, and egress is then unmeasured.
const LOCAL_TOOLS: [&str; 24] = [
    "Agent",
    "Bash",
    "BashOutput",
    "Edit",
    "ExitPlanMode",
    "Glob",
    "Grep",
    "KillShell",
    "LS",
    "MultiEdit",
    "NotebookEdit",
    "NotebookRead",
    "Read",
    "SlashCommand",
    "Skill",
    "Task",
    "TodoWrite",
    "ToolSearch",
    "Write",
    "apply_patch",
    "shell",
    "update_plan",
    "view_image",
    "read_file",
];

fn tool_name(item: &serde_json::Value) -> Option<String> {
    match item {
        serde_json::Value::String(name) => Some(name.clone()),
        other => other.get("name")?.as_str().map(str::to_string),
    }
}

fn server_entry(item: &serde_json::Value) -> Option<(String, String)> {
    let name = item.get("name")?.as_str()?;
    let status = item.get("status")?.as_str()?;
    Some((name.to_string(), status.to_string()))
}

/// The MCP server a tool's `mcp__<server>__<tool>` name reaches; `None`
/// for any other name, which is then the CLI's own tool.
fn tool_server(tool: &str) -> Option<&str> {
    let (server, _) = tool.strip_prefix("mcp__")?.split_once("__")?;
    Some(server)
}

/// The CLI's own tools: every listed tool that names no MCP server. A
/// tool naming one is read by [`user_mcp`], never dropped unread.
pub(super) fn native_tools(stream: &Stream) -> Fact<Vec<String>> {
    listed(stream, "tools", tool_name).map(|tools| {
        tools
            .into_iter()
            .filter(|tool| tool_server(tool).is_none())
            .collect()
    })
}

/// What the probe knows a tool's name to be.
#[derive(Clone, Copy, PartialEq)]
enum ToolClass {
    Egress,
    Local,
    Unrecognised,
}

fn folded(tool: &str) -> String {
    tool.chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase()
}

fn tool_class(tool: &str) -> ToolClass {
    let name = folded(tool);
    if EGRESS_WORDS.iter().any(|word| name.contains(word)) {
        ToolClass::Egress
    } else if LOCAL_TOOLS.iter().any(|local| folded(local) == name) {
        ToolClass::Local
    } else {
        ToolClass::Unrecognised
    }
}

/// The plain turn's egress tools, unmeasured when it named a tool the
/// probe knows neither as egress nor as local: that tool may reach the
/// network under a name no scanner rule matches.
pub(super) fn native_egress(tools: &Fact<Vec<String>>) -> Fact<Vec<String>> {
    let Fact::Measured {
        value: listed,
        evidence,
    } = tools
    else {
        return tools.clone();
    };
    let unrecognised: Vec<&str> = listed
        .iter()
        .filter(|tool| tool_class(tool) == ToolClass::Unrecognised)
        .map(String::as_str)
        .collect();
    if !unrecognised.is_empty() {
        return Fact::unmeasured(format!(
            "the plain turn listed {}, which the probe knows neither as egress nor as local",
            unrecognised.join(", ")
        ));
    }
    let egress = listed
        .iter()
        .filter(|tool| tool_class(tool) == ToolClass::Egress)
        .cloned()
        .collect();
    Fact::measured(egress, evidence.clone())
}

/// Each of the plain turn's own tools as a native capability, with what
/// switches it off (decision 0065 ruling 4, operator ruling A of
/// 2026-09-29): the adapter's hands argv when the boxed turn listed the
/// tool no more, `unsupported` with what the probe saw when the boxed
/// turn kept it or the CLI refused the argv, and unmeasured when no
/// boxed turn was read.
pub(super) fn capabilities(
    tools: &Fact<Vec<String>>,
    boxed_tools: &Fact<Vec<String>>,
    hands: &[String],
) -> Fact<Vec<Capability>> {
    tools.clone().map(|tools| {
        tools
            .into_iter()
            .map(|tool| Capability {
                off: off_switch(&tool, boxed_tools, hands),
                tool,
            })
            .collect()
    })
}

fn off_switch(tool: &str, boxed_tools: &Fact<Vec<String>>, hands: &[String]) -> Fact<Vec<String>> {
    match boxed_tools {
        Fact::Measured { value: left, .. } if left.iter().any(|kept| kept == tool) => {
            Fact::Unsupported {
                evidence: format!("the hands argv left {tool}"),
            }
        }
        Fact::Measured { evidence, .. } => Fact::measured(
            hands.to_vec(),
            format!("the boxed turn listed no {tool}: {evidence}"),
        ),
        Fact::Unmeasured { why } => Fact::unmeasured(format!("the boxed turn was not read: {why}")),
        Fact::Unsupported { evidence } => Fact::Unsupported {
            evidence: evidence.clone(),
        },
    }
}

pub(super) fn mcp_server(stream: &Stream) -> Fact<String> {
    listed(stream, "mcp_servers", server_entry).map(|servers| {
        servers
            .into_iter()
            .find(|(name, _)| name == SERVER_NAME)
            .map_or_else(|| "not listed".to_string(), |(_, status)| status)
    })
}

/// Whether an MCP server other than the hands server reached a turn: a
/// listed tool that names one is a reach even where no server listing
/// names it (#467). With no such tool, the turn's server listing says,
/// and a user-scope configuration the probe could not plant leaves the
/// question open.
pub(super) fn user_mcp(turn: &Turn, config: &UserConfig) -> Fact<bool> {
    on_turn(turn, |stream| match (foreign_tool(stream), config) {
        (Some(reached), _) => reached,
        (None, UserConfig::Unknown(why)) => Fact::unmeasured(*why),
        (None, UserConfig::Planted { .. }) => listed(stream, "mcp_servers", server_entry)
            .map(|servers| servers.iter().any(|(name, _)| name != SERVER_NAME)),
    })
}

/// The first listed tool of an MCP server other than the hands server,
/// read as that server having reached the turn.
fn foreign_tool(stream: &Stream) -> Option<Fact<bool>> {
    let tools = listed(stream, "tools", tool_name);
    let (tool, server) = tools.value()?.iter().find_map(|tool| {
        tool_server(tool)
            .filter(|server| *server != SERVER_NAME)
            .map(|server| (tool, server))
    })?;
    Some(Fact::measured(
        true,
        format!(
            "{}, among them {tool} of the MCP server {server}",
            tools.account()
        ),
    ))
}

/// Whether the operator's user-scope configuration is kept out of a turn
/// (#467): read from the boxed turn when it showed which MCP servers it
/// reached, since the hands argv is what isolates it, else from the plain
/// turn.
pub(super) fn config_isolation(unboxed: &Fact<bool>, boxed: &Fact<bool>) -> Fact<bool> {
    let other = format!("an MCP server other than {SERVER_NAME}");
    let none = format!(
        "no MCP server other than {SERVER_NAME}, the planted user-scope server \
         {USER_SCOPE_SERVER} included,"
    );
    match (boxed, unboxed) {
        (Fact::Measured { value: true, .. }, _) => Fact::measured(
            false,
            format!("{other} reached the boxed turn: {}", boxed.account()),
        ),
        (Fact::Measured { value: false, .. }, _) => Fact::measured(
            true,
            format!("{none} reached the boxed turn: {}", boxed.account()),
        ),
        (_, Fact::Measured { value: true, .. }) => Fact::measured(
            false,
            format!(
                "{other} reached the plain turn, and no boxed turn showed it kept out: {}",
                boxed.account()
            ),
        ),
        (_, Fact::Measured { value: false, .. }) => Fact::measured(
            true,
            format!("{none} reached the plain turn: {}", unboxed.account()),
        ),
        (_, Fact::Unmeasured { .. } | Fact::Unsupported { .. }) => Fact::unmeasured(format!(
            "no turn showed whether a user-scope MCP server loads: {}",
            unboxed.account()
        )),
    }
}

/// Whether the hands argv left any native egress tool behind, a tool it
/// does not recognise counted as egress.
pub(super) fn egress_off(
    native_egress: &Fact<Vec<String>>,
    boxed_tools: &Fact<Vec<String>>,
) -> Fact<bool> {
    let Some(egress) = native_egress.value() else {
        return Fact::unmeasured(format!(
            "the plain turn's native egress was not read: {}",
            native_egress.account()
        ));
    };
    if egress.is_empty() {
        return Fact::measured(true, "the plain turn listed no native egress tool");
    }
    match boxed_tools {
        Fact::Measured { value: left, .. } => {
            let kept: Vec<&String> = left
                .iter()
                .filter(|tool| tool_class(tool) != ToolClass::Local)
                .collect();
            let evidence = if kept.is_empty() {
                format!("the hands argv removed {}", egress.join(", "))
            } else {
                format!(
                    "the hands argv left {}",
                    kept.iter()
                        .map(|t| t.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            Fact::measured(kept.is_empty(), evidence)
        }
        Fact::Unmeasured { why } => Fact::unmeasured(format!("the boxed turn was not read: {why}")),
        Fact::Unsupported { evidence } => Fact::Unsupported {
            evidence: evidence.clone(),
        },
    }
}
