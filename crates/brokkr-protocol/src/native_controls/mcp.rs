//! The engine's one MCP server transport and the checks that read MCP out
//! of a parsed command (decision 0065 slice two, U1a): the box's hands as
//! the engine binds them, the effects a hands fragment must carry and
//! nothing beside, and the authored server a recipe may not configure. The
//! single `brokkr` hands server is the only one; this module admits no
//! other server.

use super::grammar::{self, Command, Effect, ListKind};
use super::State;

/// One value as the body of a TOML basic string (rebuild unit 13-fix-c,
/// R2): the ONE encoder every value interpolated into a Codex TOML
/// assignment passes through, for the value emitted and the value expected
/// alike. `\` and `"` are escaped, each control character TOML forbids in
/// a basic string (U+0000 to U+001F and U+007F) is its short escape or
/// `\uXXXX`, and every other scalar value stands as itself, so what a TOML
/// reader decodes is `value` exactly: a literal `/` stays those six
/// characters and never decodes as a slash.
pub(super) fn toml_basic(value: &str) -> String {
    let mut body = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => body.push_str("\\\\"),
            '"' => body.push_str("\\\""),
            '\u{8}' => body.push_str("\\b"),
            '\t' => body.push_str("\\t"),
            '\n' => body.push_str("\\n"),
            '\u{c}' => body.push_str("\\f"),
            '\r' => body.push_str("\\r"),
            c if c <= '\u{1f}' || c == '\u{7f}' => {
                body.push_str(&format!("\\u{:04X}", u32::from(c)))
            }
            c => body.push(c),
        }
    }
    body
}

/// What the engine binds the box's hands to at spawn (decision 0043;
/// rebuild unit 13-fix-b, R1): its own executable, which the harness spawns
/// as `brokkr hands serve`, the site's workdir and its typed hands
/// declaration. The transport's values are produced from these, never read
/// from a record or a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transport<'a> {
    pub brokkr: &'a std::path::Path,
    pub workdir: &'a std::path::Path,
    pub spec: &'a crate::hands::HandsSpec,
}

impl Transport<'_> {
    /// The MCP document a Claude or LaneTally harness is handed.
    fn document(&self) -> String {
        crate::hands::mcp_config(self.brokkr, self.workdir, self.spec).to_string()
    }

    /// The server's argument vector as a TOML array, as a Codex harness is
    /// handed it: each argument a basic string [`toml_basic`] encodes.
    fn arguments(&self) -> String {
        let quoted: Vec<String> = crate::hands::serve_args(self.workdir, self.spec)
            .iter()
            .map(|arg| format!("\"{}\"", toml_basic(arg)))
            .collect();
        format!("[{}]", quoted.join(","))
    }

    /// The executable as the body of the TOML basic string a Codex
    /// fragment quotes it in.
    fn command(&self) -> String {
        toml_basic(&self.brokkr.to_string_lossy())
    }

    /// An adapter's measured `hands.workspace` fragment with the engine's
    /// tokens expanded as the spawn expands them: `{hands_mcp_json}` to the
    /// document, `{hands_args_toml}` to the arguments and `{brokkr}`, which
    /// stands in a TOML basic string, to the executable that string
    /// encodes (rebuild unit 13-fix-c, R2). `None` where the executable or
    /// the workdir is not UTF-8: no provider value represents that path
    /// exactly, and a lossy one would bind another.
    pub fn expand(&self, fragment: &[String]) -> Option<Vec<String>> {
        self.brokkr.to_str()?;
        self.workdir.to_str()?;
        let (document, arguments, command) = (self.document(), self.arguments(), self.command());
        Some(
            fragment
                .iter()
                .map(|part| {
                    part.replace("{hands_mcp_json}", &document)
                        .replace("{hands_args_toml}", &arguments)
                        .replace("{brokkr}", &command)
                })
                .collect(),
        )
    }

    /// The capability-bearing options, other than tool lists and a class,
    /// that the box's hands carry at `harness`, and nothing beside them: for
    /// Claude and LaneTally strict MCP and exactly this document, and for
    /// Codex this server's command, its arguments and its approving tool
    /// mode. `None` where no hands transport is supported.
    fn effects(&self, harness: &str) -> Option<Vec<(&'static str, Vec<String>)>> {
        let control = |name: &'static str, value: String| (name, vec![value]);
        let key = |part: &str| format!("mcp_servers.{}.{part}", crate::hands::SERVER_NAME);
        match harness {
            "claude" | "lanetally" => Some(vec![
                ("--strict-mcp-config", Vec::new()),
                control("--mcp-config", self.document()),
            ]),
            "codex" => Some(vec![
                control(
                    "--config",
                    format!("{}=\"{}\"", key("command"), self.command()),
                ),
                control("--config", format!("{}={}", key("args"), self.arguments())),
                control(
                    "--config",
                    format!("{}=\"approve\"", key("default_tools_approval_mode")),
                ),
            ]),
            _ => None,
        }
    }

    /// Whether the hands' read `state` carries exactly this transport's own
    /// server at `harness`, and nothing beside it (rebuild unit 13-fix-b,
    /// R1): every effect [`Self::effects`] owes, and as many controls as
    /// it owes. A harness with no hands transport carries none.
    pub(super) fn carried_by(&self, harness: &str, state: &State) -> bool {
        let carried: Vec<(&str, &Vec<String>)> = state.controls().collect();
        self.effects(harness).is_some_and(|owed| {
            carried.len() == owed.len()
                && owed
                    .iter()
                    .all(|(name, values)| carried.contains(&(*name, values)))
        })
    }
}

/// The first part of the box's whole transport that the typed hands
/// `hands` do not carry at `provider` (rebuild unit 13-fix-c, R1): strict
/// MCP and the MCP document for Claude and LaneTally, whose include
/// restriction and workspace allowance the composer writes itself; the
/// sandbox class and the server's three bindings for Codex.
pub(super) fn untransported(provider: &str, hands: &Command) -> Option<&'static str> {
    let named = |name: &str| hands.nodes.iter().any(|node| node.name() == name);
    let bound = |part: &str| {
        let key = format!("mcp_servers.{}.{part}", crate::hands::SERVER_NAME);
        hands
            .nodes
            .iter()
            .filter(|node| node.name() == "--config")
            .flat_map(|node| &node.values)
            .any(|value| grammar::config_key(value) == key)
    };
    let owed: Vec<(bool, &'static str)> = match provider {
        "claude" | "lanetally" => vec![
            (
                named("--strict-mcp-config"),
                "strict MCP configuration ('--strict-mcp-config')",
            ),
            (named("--mcp-config"), "MCP document ('--mcp-config')"),
        ],
        "codex" => vec![
            (named("--sandbox"), "sandbox class ('--sandbox')"),
            (
                bound("command"),
                "server command binding ('mcp_servers.brokkr.command')",
            ),
            (
                bound("args"),
                "server arguments binding ('mcp_servers.brokkr.args')",
            ),
            (
                bound("default_tools_approval_mode"),
                "server approval binding ('mcp_servers.brokkr.default_tools_approval_mode')",
            ),
        ],
        _ => Vec::new(),
    };
    owed.into_iter()
        .find(|(present, _)| !present)
        .map(|(_, what)| what)
}

/// The first AUTHORED effect that configures a capability server, loads a
/// plugin, or admits a server's tools — as the NAME of what was written,
/// never a value (decision 0066 rulings 4 and 6; second council H1 and
/// H2). Independent of any native inventory and of any grant: a recipe's
/// driver command is recipe data, and only the realm grants a capability.
///
/// The judgment is on the parsed STRUCTURE, so it does not enumerate
/// spellings:
///
/// - any assignment into the `mcp_servers` table or under it, in all five
///   Codex config spellings including the attached `-cKEY=VALUE`, however
///   the key is quoted or spaced;
/// - any option whose modelled effect is LOADING — `--mcp-config`,
///   `--settings`, `--plugin-dir`, `--agents`, a Codex `--profile` — because
///   each loads a document that can configure a server, and a document the
///   engine cannot classify is not a channel a recipe may open;
/// - any value of any INCLUDE or ALLOW list that names an `mcp__` tool or
///   carries a wildcard, judged on every value of every occurrence rather
///   than on the first.
///
/// A DENY list is never here: subtraction narrows access and is not an
/// admission (second council M1).
///
/// There is deliberately no exception for a server named `brokkr`: the
/// engine's own hands arrive in the OTHER part, and a name proves nothing.
pub fn authored_server_conflict(command: &Command) -> Option<String> {
    for node in &command.nodes {
        match node.spec.effect {
            Effect::Config => {
                let key = grammar::config_key(node.values.first().map_or("", String::as_str));
                if grammar::config_under(&key, "mcp_servers") {
                    return Some(format!("{} mcp_servers", node.spelling));
                }
            }
            Effect::Load => return Some(node.spelling.clone()),
            Effect::List(ListKind::Include | ListKind::Allow) => {
                for tool in grammar::node_patterns(node)
                    .into_iter()
                    .map(grammar::tool_name)
                {
                    if tool.starts_with("mcp__") {
                        return Some(format!("{} mcp__*", node.name()));
                    }
                    if tool.contains('*') {
                        return Some(format!("{} *", node.name()));
                    }
                }
            }
            _ => {}
        }
    }
    None
}
