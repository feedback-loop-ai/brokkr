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
//!
//! Decision 0115 records the rule: a boxed seat of a harness that does not
//! fail closed is held closed by its driver. Its turn succeeds only on its
//! own terminal evidence, a result row whose `permission_denials` is an
//! empty list ([`Shown`]), and the lost-server words count only on a result
//! of a call of the engine's tool.

use serde::de::IgnoredAny;
use serde::Deserialize;
use thiserror::Error;

use super::{own, plain};
use crate::hands::SERVER_NAME;
use crate::PreSessionRefusal;

/// The native tools a level-1 seat holds. Without `--tools` the CLI keeps
/// its 34-tool default, `Agent`, `Workflow`, the cron tools and the web
/// tools among them (the record's Tools facts).
const TOOLS: [&str; 7] = ["--tools", "Read", "Edit", "Write", "Glob", "Grep", "Bash"];

/// The meta-tool a model calls an MCP server's tool through
/// (`box-engine-workspace-tool`).
const CALL: &str = "mcp_call";

/// The MCP meta-tools, their one home: `--tools ""` still leaves them, the
/// engine's server is reached through them (Q14, Q15), and only
/// `--disallowed-tools` removes them from a seat the engine serves no
/// server (the record's Tools facts).
const META: [&str; 3] = [CALL, "mcp_get", "mcp_list"];

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
        match self {
            Hands::Native => (TOOLS.iter().map(|part| part.to_string()))
                .chain(["--disallowed-tools".to_string(), META.join(",")])
                .collect(),
            Hands::Boxed(_) => NO_TOOLS.map(String::from).to_vec(),
        }
    }

    /// `line`, one row as Qoder wrote it, read once into the typed row a
    /// boxed turn is judged by (C5, D1, D3; decision 0115): its init lists
    /// the engine's server alone, connected, and no tool but the
    /// meta-tools; no result of a call of the engine's tool reports that
    /// server lost; and its result row holds `permission_denials` as an
    /// empty list, which `shown` then records. A row of no measured shape
    /// shows nothing, so it fails. Level 1's seat reads nothing here.
    pub(super) fn check(&self, line: &[u8], shown: &mut Shown) -> Result<(), Breach> {
        match self {
            Hands::Native => Ok(()),
            Hands::Boxed(grants) => boxed(grants, line, shown),
        }
    }

    /// The breach of a boxed stream that ended, exit 0, with no stop and
    /// with `shown` holding no result row (D1): its seat's result file is
    /// never taken on it. Level 1's seat ends as its stream says.
    pub(super) fn unfinished(&self, shown: &Shown) -> Option<Breach> {
        match self {
            Hands::Native => None,
            Hands::Boxed(_) => (!shown.ended).then_some(Breach::Unfinished),
        }
    }
}

/// What a boxed stream has shown so far: the id of each `mcp_call` of the
/// engine's tool (D3), and whether its result row arrived and drew no
/// denial (D1).
#[derive(Debug, Default)]
pub(super) struct Shown {
    calls: Vec<String>,
    ended: bool,
}

/// Why a boxed turn is not a turn (C5, D1, D3). Each names only plain ids
/// ([`Seen`]), as every word of the driver's does.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum Breach {
    /// A row whose type the box's checks read is not of the shape every
    /// measured stream carries, so it shows nothing.
    #[error(
        "Qoder wrote a row that is not of the shape every measured Qoder stream writes, so \
         nothing shows the seat's hands are the box's"
    )]
    Unshaped,
    /// Init's tools are not a list of names.
    #[error(
        "Qoder's init does not list its tools as every measured Qoder stream does, so nothing \
         shows the seat holds no tool of Qoder's own"
    )]
    Unread,
    /// Init's servers are missing or not a list of named servers.
    #[error(
        "Qoder's init does not list its MCP servers as every measured Qoder stream does, so \
         nothing shows the engine's server '{}' is connected",
        SERVER_NAME
    )]
    ServersUnread,
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
        "Qoder's init lists an MCP server beside the engine's server '{}': {server}, with the \
         status {status}; a boxed turn is served the engine's server alone",
        SERVER_NAME
    )]
    Beside { server: Seen, status: Seen },
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
    #[error(
        "Qoder's stream ended before its result row, so nothing shows the boxed turn drew no \
         permission denial"
    )]
    Unfinished,
    #[error(
        "Qoder's result row carries no permission_denials, so nothing shows the boxed turn drew \
         no permission denial"
    )]
    Undeclared,
    #[error(
        "Qoder's result row carries permission_denials that is not a list, so nothing shows the \
         boxed turn drew no permission denial"
    )]
    DenialsUnread,
}

impl Breach {
    /// The refusal the turn ends with: a refusal to start where nothing of
    /// the turn was folded, and otherwise a lost seat, as every stop is.
    pub(super) fn refusal(&self) -> PreSessionRefusal {
        let token = match self {
            Breach::Unshaped => "box_unread",
            Breach::Unread
            | Breach::ServersUnread
            | Breach::Unlisted
            | Breach::Unconnected { .. }
            | Breach::Beside { .. } => "box_unserved",
            Breach::Native { .. } => "box_native",
            Breach::Lost => "box_server_lost",
            Breach::Denied => "box_denied",
            Breach::Unfinished => "box_unfinished",
            Breach::Undeclared | Breach::DenialsUnread => "box_undeclared",
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

/// One stdout row as a boxed turn reads it, deserialized once from the
/// bytes Qoder wrote: every row of a measured stream is of this shape, and
/// a row of a type no check reads is [`Row::Other`].
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Row {
    System(System),
    Assistant {
        message: Message,
    },
    User {
        message: Message,
    },
    Result(Ended),
    #[serde(other)]
    Other,
}

/// A system row: init, or a subtype no check reads (`artifacts_update`,
/// `hook_diagnostics`).
#[derive(Deserialize)]
#[serde(tag = "subtype", rename_all = "lowercase")]
enum System {
    Init(Init),
    #[serde(other)]
    Other,
}

/// The two members of init a boxed turn is judged by.
#[derive(Deserialize)]
struct Init {
    #[serde(default)]
    tools: Member<Vec<String>>,
    #[serde(default)]
    mcp_servers: Member<Vec<Server>>,
}

/// One server init lists.
#[derive(Deserialize)]
struct Server {
    name: String,
    status: String,
}

/// A message's content: a list of blocks in every measured stream.
#[derive(Deserialize)]
struct Message {
    content: Vec<Block>,
}

/// The two blocks the lost-server check reads (D3): a call, and a result
/// that names the call it answers.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Block {
    ToolUse {
        id: String,
        name: String,
        input: Called,
    },
    ToolResult {
        tool_use_id: String,
        #[serde(default)]
        is_error: bool,
        #[serde(default)]
        content: Option<Answer>,
    },
    #[serde(other)]
    Other,
}

/// What a meta-tool call names: the server tool `mcp_call` calls.
#[derive(Deserialize)]
struct Called {
    #[serde(rename = "toolName")]
    tool_name: Option<String>,
}

/// A tool result's content: Qoder's text, or blocks no check reads.
#[derive(Deserialize)]
#[serde(untagged)]
enum Answer {
    Text(String),
    Other(IgnoredAny),
}

/// The member of the result row the box is judged by (D1).
#[derive(Deserialize)]
struct Ended {
    #[serde(default)]
    permission_denials: Member<Vec<IgnoredAny>>,
}

/// A member a check reads: of its measured type, of another, or absent,
/// each told apart (D1).
#[derive(Deserialize, Default)]
#[serde(untagged)]
enum Member<T> {
    Read(T),
    Unread(IgnoredAny),
    #[serde(skip)]
    #[default]
    Absent,
}

/// [`Hands::check`] for the box `grants` serves: init, each call, each
/// tool result and the result row, each by its own rule; any other row
/// shows nothing.
fn boxed(grants: &[String], line: &[u8], shown: &mut Shown) -> Result<(), Breach> {
    let row = serde_json::from_slice::<Row>(line).map_err(|_| Breach::Unshaped)?;
    match row {
        Row::System(System::Init(found)) => init(found),
        Row::Assistant { message } => {
            shown.called(grants, message.content);
            Ok(())
        }
        Row::User { message } => shown.kept(&message.content),
        Row::Result(ended) => {
            undenied(ended)?;
            shown.ended = true;
            Ok(())
        }
        Row::System(System::Other) | Row::Other => Ok(()),
    }
}

/// An init row shows the engine's server alone and connected, then no
/// tool but the meta-tools: the check that a later Qoder which leaves a
/// native tool under `--tools ""` fails instead of running unboxed.
fn init(init: Init) -> Result<(), Breach> {
    let Member::Read(servers) = init.mcp_servers else {
        return Err(Breach::ServersUnread);
    };
    let beside = |server: &Server| Breach::Beside {
        server: Seen::of(&server.name),
        status: Seen::of(&server.status),
    };
    if let Some(other) = (servers.iter()).find(|server| server.name != SERVER_NAME) {
        return Err(beside(other));
    }
    match servers.as_slice() {
        [] => return Err(Breach::Unlisted),
        [engine] if engine.status != CONNECTED => {
            return Err(Breach::Unconnected {
                status: Seen::of(&engine.status),
            })
        }
        [_] => {}
        [_, second, ..] => return Err(beside(second)),
    }
    let Member::Read(tools) = init.tools else {
        return Err(Breach::Unread);
    };
    match tools.iter().find(|tool| !META.contains(&tool.as_str())) {
        Some(tool) => Err(Breach::Native {
            tool: Seen::of(tool),
        }),
        None => Ok(()),
    }
}

impl Shown {
    /// Each call in `content` of the engine's tool through `mcp_call`,
    /// its tool one of `grants`, recorded by its id.
    fn called(&mut self, grants: &[String], blocks: Vec<Block>) {
        for block in blocks {
            if let Block::ToolUse { id, name, input } = block {
                let engine = (input.tool_name).is_some_and(|tool| grants.contains(&tool));
                if name == CALL && engine {
                    self.calls.push(id);
                }
            }
        }
    }

    /// `blocks` carry no result of a call [`Shown::called`] recorded
    /// that is an error reading, as Qoder writes it, that the engine's
    /// server is not connected (`box-server-dies`). Qoder's words are the
    /// only sign it gives: the turn goes on, with no denial and no error of
    /// its own. The same words on a result of any other call are not this
    /// sign (D3).
    fn kept(&self, blocks: &[Block]) -> Result<(), Breach> {
        let lost = format!("Error: MCP server '{SERVER_NAME}' is not connected");
        let reported = blocks.iter().any(|block| match block {
            Block::ToolResult {
                tool_use_id,
                is_error,
                content: Some(Answer::Text(text)),
            } => *is_error && *text == lost && self.calls.contains(tool_use_id),
            Block::ToolResult { .. } | Block::ToolUse { .. } | Block::Other => false,
        });
        match reported {
            true => Err(Breach::Lost),
            false => Ok(()),
        }
    }
}

/// A result row holds `permission_denials` as a list, and an empty one: in
/// a boxed launch each denial is a call to a tool the seat does not hold
/// (`box-other-natives`, `box-ungranted-mcp-tool`), and the turn fails on
/// it rather than ending `success` as Qoder ends it. A row without the
/// member, or with one that is no list, shows nothing (D1).
fn undenied(ended: Ended) -> Result<(), Breach> {
    match ended.permission_denials {
        Member::Read(denials) if denials.is_empty() => Ok(()),
        Member::Read(_) => Err(Breach::Denied),
        Member::Unread(_) => Err(Breach::DenialsUnread),
        Member::Absent => Err(Breach::Undeclared),
    }
}
