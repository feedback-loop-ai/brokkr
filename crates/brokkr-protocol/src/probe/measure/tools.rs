//! The tools and MCP servers a turn lists: which tools are the CLI's own,
//! which reach the network, what switches each off, and which MCP servers
//! reached the turn beside the hands server (#467). Pure, like `measure`.

use super::listing::Listing;
use super::{on_turn, Stream, Streams, Turn};
use crate::hands::SERVER_NAME;
use crate::probe::facts::{Capability, Fact};
use crate::probe::plan::{Plan, UserConfig, USER_SCOPE_SERVER};
use crate::probe::{Native, NativePower};

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
pub(super) fn native_tools(streams: &Streams) -> Fact<Vec<String>> {
    Listing::read(&streams.all, "tools", tool_name)
        .fact()
        .map(|tools| {
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

/// Each native capability the adapter declares, keyed as a realm grants
/// it and mapped to its tools, with what switches it off (decision 0065
/// rulings 1 and 4, operator ruling A of 2026-09-29): the declared OFF
/// controls when the turn under them listed none of its tools,
/// `unsupported` with what the probe saw when that turn kept one or the
/// CLI refused the controls, and unmeasured when no control was tried or
/// its turn was not read. A plain turn that lists a tool the probe does
/// not know as local, and no declared capability maps, leaves the
/// inventory unmeasured, since no realm can grant that tool.
pub(super) fn capabilities(
    plan: &Plan,
    tools: &Fact<Vec<String>>,
    off_tools: &Fact<Vec<String>>,
) -> Fact<Vec<Capability>> {
    let powers = match &plan.native {
        Native::Known { powers, .. } => powers,
        Native::Unmeasured(why) => {
            return Fact::unmeasured(format!(
                "the adapter declares its native capabilities unmeasured: {why}"
            ))
        }
    };
    let Fact::Measured {
        value: listed,
        evidence,
    } = tools
    else {
        return Fact::unmeasured(format!(
            "the plain turn's tools were not read: {}",
            tools.account()
        ));
    };
    let unmapped: Vec<&str> = listed
        .iter()
        .filter(|tool| tool_class(tool) != ToolClass::Local)
        .filter(|tool| !powers.iter().any(|power| power.tools.contains(tool)))
        .map(String::as_str)
        .collect();
    if !unmapped.is_empty() {
        return Fact::unmeasured(format!(
            "the plain turn listed {}, which no declared native capability maps, so no realm \
             can grant it",
            unmapped.join(", ")
        ));
    }
    let each = powers.iter().map(|power| Capability {
        capability: power.capability.clone(),
        tools: power.tools.clone(),
        off: off_switch(power, &plan.off, off_tools),
    });
    Fact::measured(each.collect(), evidence.clone())
}

fn off_switch(
    power: &NativePower,
    off: &[String],
    off_tools: &Fact<Vec<String>>,
) -> Fact<Vec<String>> {
    match off_tools {
        Fact::Measured {
            value: left,
            evidence,
        } => match left.iter().find(|tool| power.tools.contains(tool)) {
            Some(kept) => Fact::Unsupported {
                evidence: format!("the declared OFF controls left {kept}: {evidence}"),
            },
            None => Fact::measured(
                off.to_vec(),
                format!(
                    "the turn under the declared OFF controls listed none of {}: {evidence}",
                    power.tools.join(", ")
                ),
            ),
        },
        Fact::Unmeasured { why } => Fact::unmeasured(format!(
            "the turn under the declared OFF controls was not read: {why}"
        )),
        Fact::Unsupported { evidence } => Fact::Unsupported {
            evidence: evidence.clone(),
        },
    }
}

/// The hands server's status, unmeasured when the turn's listings gave it
/// more than one.
pub(super) fn mcp_server(streams: &Streams) -> Fact<String> {
    let statuses = Listing::read(&streams.all, "mcp_servers", server_entry)
        .fact()
        .map(|servers| {
            servers
                .into_iter()
                .filter(|(name, _)| name == SERVER_NAME)
                .map(|(_, status)| status)
                .collect::<Vec<_>>()
        });
    match statuses {
        Fact::Measured { value, evidence } if value.len() > 1 => Fact::unmeasured(format!(
            "the turn's listings gave {SERVER_NAME} the statuses {}: {evidence}",
            value.join(", ")
        )),
        other => other.map(|statuses| {
            statuses
                .into_iter()
                .next()
                .unwrap_or_else(|| "not listed".to_string())
        }),
    }
}

/// Whether an MCP server other than the hands server reached a turn
/// (#467), read from both listings by [`Listing`]'s invariant and from
/// stderr's text: a reach read anywhere is a reach, whatever else went
/// unread and however the turn ended, so a turn the CLI refused, or one
/// that never finished, still shows the reach it printed (#484). With
/// none read, a turn that failed says what `on_turn` says of it; in one
/// that was read, any value either listing left unread, or a user-scope
/// configuration the probe could not plant, leaves the question open, and
/// only then does the server listing, read whole, say no.
pub(super) fn user_mcp(turn: &Turn, config: &UserConfig) -> Fact<bool> {
    let streams = turn.streams();
    let tools = Listing::read(streams, "tools", tool_name);
    let servers = Listing::read(streams, "mcp_servers", server_entry);
    match reach(&tools, &servers).or_else(|| text_reach(streams)) {
        Some(reached) => Fact::measured(true, reached),
        None => on_turn(turn, |_| no_reach(&tools, servers, config)),
    }
}

/// Each value the turn's two listings hold that their reader could not
/// name, which the verdict's evidence counts as unread (#484).
pub(super) fn unread_values(turn: &Turn) -> Vec<String> {
    let streams = turn.streams();
    let mut values = Listing::read(streams, "tools", tool_name).unread_values();
    values.extend(Listing::read(streams, "mcp_servers", server_entry).unread_values());
    values
}

/// The first line of text, stderr's, that names an MCP server other than
/// the hands server.
fn text_reach(streams: &[Stream]) -> Option<String> {
    streams.iter().find_map(|stream| {
        let (line, named) = stream.reaches.first()?;
        Some(format!("line {line} of {} names {named}", stream.source))
    })
}

/// What a line of text says of the MCP servers and tools that reached a
/// turn (#484): the scanner refuses what it does not recognise.
pub(super) enum Said {
    /// It names no listing, MCP server or tool, or only the hands
    /// server's.
    Nothing,
    /// It names an MCP server other than the hands server, as this says.
    Reach(String),
    /// It names a listing, JSON it does not hold whole, or an MCP server
    /// or tool whose name it does not spell whole.
    Unread,
}

/// The words a line of text names a listing of MCP servers by, folded to
/// lowercase.
const SERVER_LISTINGS: [&str; 4] = ["mcp_servers", "mcpservers", "mcp servers", "mcp-servers"];

/// How a line names one MCP server: `MCP server <name>`, any case.
const MCP_SERVER: &str = "mcp server";

/// What `text` says: a reach when it names the planted server, a tool of
/// another server or another `MCP server <name>`; else unread when it
/// names any listing or a server or tool it does not spell whole.
pub(super) fn said(text: &str) -> Said {
    if text.contains(USER_SCOPE_SERVER) {
        return Said::Reach(format!(
            "the planted user-scope MCP server {USER_SCOPE_SERVER}"
        ));
    }
    let mut unread = names_a_listing(text);
    for mention in mentions(text) {
        match mention {
            Some((server, named)) if server != SERVER_NAME => return Said::Reach(named),
            Some(_) => {}
            None => unread = true,
        }
    }
    if unread {
        Said::Unread
    } else {
        Said::Nothing
    }
}

/// Whether `text` names a listing of MCP servers or tools, or starts JSON
/// it does not hold whole, being no one JSON object.
fn names_a_listing(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let tools = lower.match_indices("tools").any(|(at, word)| {
        lower[at + word.len()..]
            .trim_start_matches(|c: char| c.is_whitespace() || matches!(c, '"' | '\''))
            .starts_with([':', '=', '['])
    });
    tools
        || SERVER_LISTINGS.iter().any(|words| lower.contains(words))
        || lower.trim_start().starts_with(['{', '['])
}

/// A name as a line spells it: letters, digits, `_`, `-` and `.`.
fn spelled(text: &str) -> String {
    text.chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        .collect()
}

/// Each MCP server `text` names, by a tool it spells `mcp__<server>__…`
/// or as `MCP server <name>`: the server and how the line names it, or
/// `None` where the spelling names no server whole.
fn mentions(text: &str) -> Vec<Option<(String, String)>> {
    let tools = text.match_indices("mcp__").map(|(at, _)| {
        let tool = spelled(&text[at..]);
        let server = tool_server(&tool)?.to_string();
        let named = format!("{tool} of the MCP server {server}");
        Some((server, named))
    });
    let lower = text.to_ascii_lowercase();
    let servers = lower.match_indices(MCP_SERVER).filter_map(|(at, word)| {
        let rest = &text[at + word.len()..];
        // `MCP servers` is a listing, which `names_a_listing` reads.
        if rest.starts_with(|c: char| c.is_ascii_alphanumeric()) {
            return None;
        }
        let name = spelled(rest.trim_start_matches(|c: char| {
            c.is_whitespace() || matches!(c, ':' | '"' | '\'' | '`')
        }));
        Some((!name.is_empty()).then(|| (name.clone(), format!("the MCP server {name}"))))
    });
    tools.chain(servers).collect()
}

/// Where an MCP server other than the hands server was read reaching the
/// turn: a listed tool that names one, even where no server listing does,
/// else a listed server.
fn reach(tools: &Listing<String>, servers: &Listing<(String, String)>) -> Option<String> {
    let tool = tools.entries().iter().find_map(|entry| {
        let server = tool_server(&entry.value).filter(|server| *server != SERVER_NAME)?;
        Some(format!(
            "{} of the MCP server {server} was listed by {}",
            entry.value, entry.at
        ))
    });
    tool.or_else(|| {
        let entry = servers
            .entries()
            .iter()
            .find(|entry| entry.value.0 != SERVER_NAME)?;
        Some(format!(
            "the MCP server {} was listed {} by {}",
            entry.value.0, entry.value.1, entry.at
        ))
    })
}

/// No reach was read: measured `false` only when both listings were read
/// whole and the planted server's listing was given.
fn no_reach(
    tools: &Listing<String>,
    servers: Listing<(String, String)>,
    config: &UserConfig,
) -> Fact<bool> {
    let unread: Vec<String> = [tools.unread(), servers.unread()]
        .into_iter()
        .flatten()
        .collect();
    if !unread.is_empty() {
        return Fact::unmeasured(format!(
            "no MCP server other than {SERVER_NAME} was read reaching the turn, but {}",
            unread.join(", and ")
        ));
    }
    match config {
        UserConfig::Unknown(why) => Fact::unmeasured(*why),
        UserConfig::Planted { .. } => servers.fact().map(|_| false),
    }
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

/// Whether the declared OFF controls left any native egress tool behind,
/// a tool the probe does not recognise counted as egress.
pub(super) fn egress_off(
    native_egress: &Fact<Vec<String>>,
    off_tools: &Fact<Vec<String>>,
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
    match off_tools {
        Fact::Measured { value: left, .. } => {
            let kept: Vec<&String> = left
                .iter()
                .filter(|tool| tool_class(tool) != ToolClass::Local)
                .collect();
            let evidence = if kept.is_empty() {
                format!("the declared OFF controls removed {}", egress.join(", "))
            } else {
                format!(
                    "the declared OFF controls left {}",
                    kept.iter()
                        .map(|t| t.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            Fact::measured(kept.is_empty(), evidence)
        }
        Fact::Unmeasured { why } => Fact::unmeasured(format!(
            "the turn under the declared OFF controls was not read: {why}"
        )),
        Fact::Unsupported { evidence } => Fact::Unsupported {
            evidence: evidence.clone(),
        },
    }
}
