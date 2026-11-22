//! A qoder seat's hands (#1235; decision 0075 ruling 4; decisions 0043 and
//! 0046): level 1's native launch or the box's, decided once from the
//! engine's set ([`Hands::of`]), and what a boxed turn must show before any
//! of it counts. A child module because its parent nears the production
//! file ceiling.
//!
//! The facts are the controller's second measurement, on Qoder CLI 1.1.95
//! (`docs/evidence/adapters/qoder-cli-1-1.md`, Second measurement):
//!
//! - under `--permission-mode dont_ask --tools ""`, one grant for the
//!   engine's tool and the MCP isolation, init's `tools` is the three MCP
//!   meta-tools, its `mcp_servers` is the engine's server `connected`, and
//!   the model reaches the hands tool through `mcp_call`
//!   (`box-engine-workspace-tool`);
//! - no tool of Qoder's own exists in that launch: a call that names one is
//!   an entry in `result.permission_denials` and the turn still ends
//!   `success` (`box-other-natives`), as a call to an engine tool the seat
//!   holds no grant for is (`box-ungranted-mcp-tool`);
//! - Qoder does not fail closed: a server that does not start is
//!   `disconnected` in init (`box-server-does-not-start`), a server that
//!   dies mid-turn is a tool error in Qoder's own words (`box-server-dies`),
//!   and each turn still ends `success`, exit 0. The driver fails closed on
//!   both, and on any denial.

use serde::Deserialize;
use serde_json::{json, Map, Value};
use thiserror::Error;

use super::{own, plain};
use crate::hands::SERVER_NAME;
use crate::PreSessionRefusal;

/// The native tools a level-1 seat holds. Without `--tools` the CLI keeps
/// its 34-tool default, `Agent`, `Workflow`, the cron tools and the web
/// tools among them (the record's Tools facts).
const TOOLS: [&str; 7] = ["--tools", "Read", "Edit", "Write", "Glob", "Grep", "Bash"];

/// The MCP meta-tools: `--tools ""` still leaves them, and the engine's
/// server is reached through them (Q14, Q15, `box-engine-workspace-tool`).
const META: [&str; 3] = ["mcp_call", "mcp_get", "mcp_list"];

/// The meta-tools denied to a seat the engine serves no server: only
/// `--disallowed-tools` removes them (the record's Tools facts).
const META_DENIED: [&str; 2] = ["--disallowed-tools", "mcp_call,mcp_get,mcp_list"];

/// A boxed launch's tool list: no tool of Qoder's own, only the meta-tools
/// (`box-other-natives`).
const NO_TOOLS: [&str; 2] = ["--tools", ""];

/// The status init gives a server Qoder reached.
const CONNECTED: &str = "connected";

/// Whether a qoder launch's hands are the box's: the one value that decides
/// its permission mode, its tool list, whether the meta-tools are denied,
/// whether the seat's own `--allowed-tools` is admitted, and whether its
/// turn is checked as a boxed one (C6). Made once, from the engine's set,
/// by [`Hands::of`]; nothing else derives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Hands {
    /// Level 1's unboxed seat: no engine server, Qoder's six tools.
    Native,
    /// The box's hands server alone, and the grant of each tool it serves.
    Boxed(Vec<String>),
}

impl Hands {
    /// The hands of a launch the engine's set grants `grants`: the set
    /// grants a tool only where it serves the box's hands server
    /// ([`crate::native_controls::mcp::qoder::served`]), and refuses where
    /// the sealed inputs box hands it does not serve, so an empty grant is
    /// level 1's seat and any other is the box's.
    pub(super) fn of(grants: Vec<String>) -> Hands {
        match grants.is_empty() {
            true => Hands::Native,
            false => Hands::Boxed(grants),
        }
    }

    /// The permission mode: level 1's `accept_edits`, or `dont_ask` for the
    /// box, under which nothing is approved that no grant names
    /// (`box-ungranted-mcp-tool`).
    pub(super) fn mode(&self) -> [String; 2] {
        let mode = match self {
            Hands::Native => "accept_edits",
            Hands::Boxed(_) => "dont_ask",
        };
        ["--permission-mode", mode].map(String::from)
    }

    /// Whether the seat's own `--allowed-tools` is admitted: a boxed seat
    /// holds no tool of Qoder's own to grant (C4).
    pub(super) fn admits_seat_grant(&self) -> bool {
        match self {
            Hands::Native => true,
            Hands::Boxed(_) => false,
        }
    }

    /// What the launch carries behind the seat's own controls: nothing for
    /// level 1's seat, and the engine's grants as the one `--allowed-tools`
    /// value for the box's.
    pub(super) fn granted(&self) -> Vec<String> {
        match self {
            Hands::Native => Vec::new(),
            Hands::Boxed(grants) => vec!["--allowed-tools".to_string(), grants.join(",")],
        }
    }

    /// The tool lists the launch ends with: level 1's six tools with the
    /// meta-tools denied, or no tool of Qoder's own beside the meta-tools.
    pub(super) fn tools(&self) -> Vec<String> {
        let tools = match self {
            Hands::Native => [&TOOLS[..], &META_DENIED].concat(),
            Hands::Boxed(_) => NO_TOOLS.to_vec(),
        };
        tools.iter().map(|part| part.to_string()).collect()
    }

    /// `row`, as Qoder wrote it, read for what a boxed turn must show
    /// (C5): its init lists the engine's server alone, connected, and no
    /// tool but the meta-tools; no tool result reports that server lost;
    /// and its result records no permission denial. Level 1's seat reads
    /// nothing here.
    pub(super) fn check(&self, row: &Map<String, Value>) -> Result<(), Breach> {
        match self {
            Hands::Native => Ok(()),
            Hands::Boxed(_) => boxed(row),
        }
    }
}

/// Why a boxed turn is not a turn (C5). Each names only plain ids
/// ([`Seen`]), as every word of the driver's does.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum Breach {
    /// Init's tools or servers are not lists of the shape every measured
    /// stream carries, so nothing confirms the box.
    #[error(
        "Qoder's init does not list its tools and MCP servers as every measured Qoder stream \
         does, so nothing shows the seat's hands are the box's"
    )]
    Unread,
    #[error(
        "Qoder's init does not list the engine's server '{}', and a boxed turn runs only with \
         it connected (box-server-does-not-start)",
        SERVER_NAME
    )]
    Unlisted,
    #[error(
        "Qoder's init lists the engine's server '{}' with a status other than connected: \
         {status}; a boxed turn runs only with it connected (box-server-does-not-start)",
        SERVER_NAME
    )]
    Unconnected { status: Seen },
    #[error(
        "Qoder's init lists an MCP server beside the engine's server '{}': {server}; a boxed \
         turn is served the engine's server alone",
        SERVER_NAME
    )]
    Beside { server: Seen },
    #[error(
        "Qoder's init lists a tool beside its MCP meta-tools: {tool}; a boxed seat holds no \
         tool of Qoder's own"
    )]
    Native { tool: Seen },
    #[error(
        "Qoder reported mid-turn that the engine's server '{}' is not connected \
         (box-server-dies)",
        SERVER_NAME
    )]
    Lost,
    #[error(
        "Qoder's result records a permission denial: a boxed seat called a tool it does not \
         hold"
    )]
    Denied,
}

impl Breach {
    /// The refusal the turn ends with: a refusal to start where nothing of
    /// the turn was folded, and otherwise a lost seat, as every stop is.
    pub(super) fn refusal(&self) -> PreSessionRefusal {
        let token = match self {
            Breach::Unread
            | Breach::Unlisted
            | Breach::Unconnected { .. }
            | Breach::Beside { .. } => "box_unserved",
            Breach::Native { .. } => "box_native",
            Breach::Lost => "box_server_lost",
            Breach::Denied => "box_denied",
        };
        own(token, self.to_string())
    }
}

/// A name Qoder wrote, as the driver's words name it: quoted where it is a
/// plain id, and never otherwise (H2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Seen {
    Plain(String),
    Unplain,
}

impl Seen {
    fn of(text: &str) -> Seen {
        match plain(text) {
            true => Seen::Plain(text.to_string()),
            false => Seen::Unplain,
        }
    }
}

impl std::fmt::Display for Seen {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Seen::Plain(text) => write!(formatter, "'{text}'"),
            Seen::Unplain => formatter.write_str("a value that is not a plain id"),
        }
    }
}

/// The two members of init a boxed turn is judged by, typed where they are
/// read.
#[derive(Deserialize)]
struct Init {
    tools: Vec<String>,
    mcp_servers: Vec<Server>,
}

/// One server init lists.
#[derive(Deserialize)]
struct Server {
    name: String,
    status: String,
}

/// [`Hands::check`] for the box: init, each tool result and the result
/// row, each by its own rule; any other row shows nothing.
fn boxed(row: &Map<String, Value>) -> Result<(), Breach> {
    let text = |key: &str| row.get(key).and_then(Value::as_str);
    match (text("type"), text("subtype")) {
        (Some("system"), Some("init")) => init(row),
        (Some("user"), _) => kept(row),
        (Some("result"), _) => undenied(row),
        _ => Ok(()),
    }
}

/// An init row shows the engine's server alone and connected, then no
/// tool but the meta-tools: the check that a later Qoder which leaves a
/// native tool under `--tools ""` fails instead of running unboxed.
fn init(row: &Map<String, Value>) -> Result<(), Breach> {
    let members = json!({"tools": row.get("tools"), "mcp_servers": row.get("mcp_servers")});
    let init = Init::deserialize(&members).map_err(|_| Breach::Unread)?;
    if let Some(other) = (init.mcp_servers.iter()).find(|server| server.name != SERVER_NAME) {
        return Err(Breach::Beside {
            server: Seen::of(&other.name),
        });
    }
    match init.mcp_servers.as_slice() {
        [] => return Err(Breach::Unlisted),
        [engine] if engine.status != CONNECTED => {
            return Err(Breach::Unconnected {
                status: Seen::of(&engine.status),
            })
        }
        [_] => {}
        [_, second, ..] => {
            return Err(Breach::Beside {
                server: Seen::of(&second.name),
            })
        }
    }
    match init
        .tools
        .iter()
        .find(|tool| !META.contains(&tool.as_str()))
    {
        Some(tool) => Err(Breach::Native {
            tool: Seen::of(tool),
        }),
        None => Ok(()),
    }
}

/// A user row carries no tool result that is an error reading, as Qoder
/// writes it, that the engine's server is not connected (`box-server-dies`).
/// Qoder's words are the only sign it gives: the turn goes on, with no
/// denial and no error of its own.
fn kept(row: &Map<String, Value>) -> Result<(), Breach> {
    let lost = format!("Error: MCP server '{SERVER_NAME}' is not connected");
    let blocks = (row.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_array);
    let reported = blocks.into_iter().flatten().any(|block| {
        block.get("type").and_then(Value::as_str) == Some("tool_result")
            && block.get("is_error") == Some(&Value::Bool(true))
            && block.get("content").and_then(Value::as_str) == Some(lost.as_str())
    });
    match reported {
        true => Err(Breach::Lost),
        false => Ok(()),
    }
}

/// A result row records no permission denial: in a boxed launch each is a
/// call to a tool the seat does not hold (`box-other-natives`,
/// `box-ungranted-mcp-tool`), and the turn fails on it rather than ending
/// `success` as Qoder ends it.
fn undenied(row: &Map<String, Value>) -> Result<(), Breach> {
    match row.get("permission_denials") {
        None => Ok(()),
        Some(Value::Array(denials)) if denials.is_empty() => Ok(()),
        Some(_) => Err(Breach::Denied),
    }
}
