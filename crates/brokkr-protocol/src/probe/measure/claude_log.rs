//! Claude Code's transcript reader (#484): the rows of its project log
//! (`~/.claude/projects/<cwd>/<session>.jsonl`), decoded closed by `type`,
//! and an attachment's by its own. The vocabulary is the one the
//! controller recorded from claude 2.1.287 on 2026-10-04, as skeletons
//! (`streams/claude-*.jsonl`): every key, number and boolean real, every
//! private string a `<key>` placeholder. A key is consumed, or inert, as
//! in `claude`: inert only when no value of it can contradict a fact the
//! verdict rests on.

use std::collections::BTreeMap;

use serde::de::IgnoredAny;
use serde::Deserialize;
use serde_json::{Map, Value};

use super::claude_message::{message_said, searched, Message, ModelUsage};
use super::read::{decode, envelope, Decoded, Empty, Given, Harness, Null, Prompt, Said, Server};
use super::Fault;

/// The keys a row of the log carries whatever its type.
const ENVELOPE: [&str; 10] = [
    "parentUuid",
    "isSidechain",
    "uuid",
    "timestamp",
    "userType",
    "entrypoint",
    "cwd",
    "sessionId",
    "version",
    "gitBranch",
];

/// Inert, every key: where and when the row was written, by which
/// version, in which session and branch; none of them says what the turn
/// listed, ran or answered. A sidechain's rows are read like the rest.
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct Envelope {
    #[serde(rename = "parentUuid")]
    _parent: Option<String>,
    #[serde(rename = "isSidechain")]
    _sidechain: Given<bool>,
    #[serde(rename = "uuid")]
    _uuid: Given<String>,
    #[serde(rename = "timestamp")]
    _timestamp: Given<String>,
    #[serde(rename = "userType")]
    _user_type: Given<String>,
    #[serde(rename = "entrypoint")]
    _entrypoint: Given<String>,
    #[serde(rename = "cwd")]
    _cwd: Given<String>,
    #[serde(rename = "sessionId")]
    _session: Given<String>,
    #[serde(rename = "version")]
    _version: Given<String>,
    #[serde(rename = "gitBranch")]
    _branch: Given<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
enum Row {
    QueueOperation {
        /// Inert: whether the prompt entered or left the queue.
        #[serde(rename = "operation")]
        _operation: Queue,
        /// Inert: only the probe's prompt reads.
        #[serde(default, rename = "content")]
        _content: Given<Prompt>,
    },
    /// The prompt's row. Every key may be absent; each is inert, the
    /// message holding only the probe's prompt.
    User {
        #[serde(default, rename = "message")]
        _message: Given<UserMessage>,
        #[serde(default, rename = "promptId")]
        _prompt_id: Given<String>,
        /// Inert: the mode the adapter passes, which the listings measure.
        #[serde(default, rename = "permissionMode")]
        _permission_mode: Given<String>,
        #[serde(default, rename = "promptSource")]
        _source: Given<String>,
        #[serde(default, rename = "turnOrigin")]
        _origin: Given<String>,
        #[serde(default, rename = "turnPosition")]
        _position: Given<Position>,
    },
    Attachment(Box<Attached>),
    AtisLatch {
        /// Inert: an opaque token of the session.
        #[serde(rename = "atis")]
        _atis: String,
    },
    LastPrompt {
        /// Inert: only the probe's prompt reads.
        #[serde(rename = "lastPrompt")]
        _prompt: Prompt,
        /// Inert: the row it follows.
        #[serde(rename = "leafUuid")]
        _leaf: String,
    },
    Assistant(Box<Assistant>),
    System(Stop),
    CostState(Box<Cost>),
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Queue {
    Enqueue,
    Dequeue,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UserMessage {
    #[serde(rename = "role")]
    _role: UserRole,
    #[serde(rename = "content")]
    _content: Prompt,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum UserRole {
    User,
}

/// Inert, every key: where the prompt sits in the session.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Position {
    #[serde(rename = "promptIndex")]
    _prompt: u64,
    #[serde(rename = "turnIndex")]
    _turn: u64,
}

/// An attachment: what it says, and, inert, how it was rendered to the
/// model, which restates it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Attached {
    attachment: Attachment,
    #[serde(default, rename = "rendered")]
    _rendered: Given<Vec<Rendered>>,
    #[serde(default, rename = "renderedRole")]
    _role: Given<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rendered {
    #[serde(rename = "content")]
    _content: String,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Attachment {
    /// Inert: the host the turn runs on.
    Environment {
        #[serde(rename = "snapshot")]
        _snapshot: Snapshot,
    },
    Model {
        identity: Identity,
        /// Inert: the identity, rendered.
        #[serde(rename = "text")]
        _text: String,
    },
    DeferredToolsDelta(Box<Deferred>),
    /// Inert, every key: the subagents it may start, each under the tools
    /// and servers listed.
    AgentListingDelta {
        #[serde(rename = "addedTypes")]
        _added: Vec<String>,
        #[serde(rename = "addedLines")]
        _lines: Vec<String>,
        #[serde(rename = "builtInTypes")]
        _built_in: Vec<String>,
        #[serde(rename = "removedTypes")]
        _removed: Empty,
        #[serde(rename = "isInitial")]
        _initial: bool,
        #[serde(rename = "showConcurrencyNote")]
        _note: bool,
    },
    McpInstructionsDelta {
        /// The servers whose instructions reached the turn.
        #[serde(rename = "addedNames")]
        added: Vec<String>,
        /// Inert: their instructions.
        #[serde(rename = "addedBlocks")]
        _blocks: Vec<String>,
        #[serde(rename = "removedNames")]
        _removed: Empty,
    },
    /// Inert, every key: skills are prompts, run under the tools listed.
    SkillListing {
        #[serde(rename = "content")]
        _content: String,
        #[serde(rename = "skillCount")]
        _count: u64,
        #[serde(rename = "isInitial")]
        _initial: bool,
        #[serde(rename = "names")]
        _names: Vec<String>,
    },
    /// Inert: a reminder to the model.
    TotalTokensReminder {
        #[serde(rename = "text")]
        _text: String,
    },
    /// Inert: the account and the repository's status.
    SessionContext {
        #[serde(rename = "context")]
        _context: Context,
    },
    /// Inert: the day.
    Date {
        #[serde(rename = "date")]
        _date: String,
    },
    /// Inert: the credential's organisation.
    CredentialOrg {
        #[serde(rename = "organizationUuid")]
        _organization: String,
    },
    /// Inert, every key: a remote session the turn is not.
    RemoteSessionChange {
        #[serde(rename = "url")]
        _url: Null,
        #[serde(rename = "commit")]
        _commit: String,
        #[serde(rename = "pr")]
        _pr: String,
        #[serde(rename = "sendUserFileHint")]
        _hint: bool,
        #[serde(rename = "managedCommit")]
        _managed_commit: bool,
        #[serde(rename = "managedPr")]
        _managed_pr: bool,
    },
    PromptSnapshot(Box<PromptSnapshot>),
}

/// Inert, every key: the host.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    #[serde(rename = "workingDirectory")]
    _cwd: String,
    #[serde(rename = "isWorktree")]
    _worktree: bool,
    #[serde(rename = "isGitRepo")]
    _git: bool,
    #[serde(rename = "additionalWorkingDirectories")]
    _more: Vec<String>,
    #[serde(rename = "platform")]
    _platform: String,
    #[serde(rename = "shell")]
    _shell: String,
    #[serde(rename = "osVersion")]
    _os: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    #[serde(rename = "modelId")]
    model: String,
    /// Inert: the model's names for people and its cutoff, `null` for a
    /// model the CLI does not know.
    #[serde(rename = "marketingName")]
    _marketing: Option<String>,
    #[serde(rename = "knowledgeCutoff")]
    _cutoff: Option<String>,
}

/// Inert, every key: who the account is and what git says.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    #[serde(rename = "userEmail")]
    _email: String,
    #[serde(rename = "gitStatus")]
    _git: String,
}

/// The tools the turn may load later, and the MCP servers it waits on,
/// each a listing: a name in any of them reached the turn.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Deferred {
    added_names: Vec<String>,
    surfaced_names: Vec<String>,
    surfaced_definitions: Vec<Definition>,
    pending_mcp_servers: Vec<String>,
    needs_auth_mcp_servers: Vec<String>,
    failed_mcp_servers: Vec<FailedServer>,
    /// Inert: the added tools, rendered.
    #[serde(rename = "addedLines")]
    _lines: Vec<String>,
    /// Inert, like the two after it: only an empty list reads.
    #[serde(rename = "removedNames")]
    _removed: Empty,
    #[serde(rename = "wireHiddenNames")]
    _hidden: Empty,
    #[serde(rename = "readdedNames")]
    _readded: Empty,
}

/// A tool surfaced with its definition: its name, read, and inert, how
/// to call it, which does not say whether it is listed.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    name: String,
    #[serde(rename = "listing")]
    _listing: String,
    #[serde(rename = "definition")]
    _definition: Schema,
}

/// Inert, every key: a tool's calling convention. Its input schema is
/// any JSON schema, so it is decoded as any JSON value.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Schema {
    #[serde(rename = "name")]
    _name: String,
    #[serde(rename = "description")]
    _description: String,
    #[serde(rename = "input_schema")]
    _input: IgnoredAny,
    #[serde(rename = "eager_input_streaming")]
    _eager: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FailedServer {
    name: String,
    /// Inert: why it failed, which a listed server's status says.
    #[serde(rename = "errorCode")]
    _code: String,
    #[serde(rename = "error")]
    _error: String,
}

/// The system prompt and tools as sent: the tools a listing.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PromptSnapshot {
    #[serde(default)]
    tools: Given<Vec<SnapshotTool>>,
    /// Inert, like every key after it: the prompt's text and how it was
    /// laid out, which lists no tool and reaches no server.
    #[serde(rename = "systemPrompt")]
    _system: Vec<String>,
    #[serde(default, rename = "cliPrefix")]
    _prefix: Given<String>,
    #[serde(rename = "reminderFold")]
    _fold: bool,
    #[serde(default, rename = "systemTurns")]
    _system_turns: Given<bool>,
    #[serde(default, rename = "toolChangeHeader")]
    _header: Given<bool>,
    #[serde(default, rename = "inlineTools")]
    _inline: Given<bool>,
    #[serde(default, rename = "keptReminders")]
    _kept: Given<bool>,
    #[serde(rename = "echoWireToolInputs")]
    _echo: bool,
    #[serde(rename = "contextRendering")]
    _rendering: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotTool {
    name: String,
    #[serde(rename = "description")]
    _description: String,
    #[serde(rename = "schema")]
    _schema: Schema,
}

/// A reply's row: its message, its failure, and, inert, the request it
/// answered and the effort it ran at, which no listing rests on.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Assistant {
    message: Message,
    /// Any value is a failure, so which one is inert.
    #[serde(default)]
    error: Given<String>,
    /// `true` is a failure.
    #[serde(default)]
    is_api_error_message: Given<bool>,
    /// Any status is a failure.
    #[serde(default)]
    api_error_status: Given<u16>,
    #[serde(default, rename = "apiBlockIndex")]
    _block: Given<u64>,
    #[serde(default, rename = "requestId")]
    _request: Given<String>,
    #[serde(default, rename = "advisorModel")]
    _advisor: Given<String>,
    #[serde(default, rename = "effort")]
    _effort: Given<String>,
    #[serde(default, rename = "perTurnEffort")]
    _per_turn: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
enum Stop {
    StopHookSummary(Box<HookSummary>),
}

/// The hooks that ran as the turn stopped: one that kept the turn from
/// ending is a failure, and each other key is inert, the hooks' own
/// record, which lists no tool.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct HookSummary {
    prevented_continuation: bool,
    #[serde(rename = "hookCount")]
    _count: u64,
    #[serde(rename = "hookInfos")]
    _infos: Vec<HookInfo>,
    #[serde(rename = "hookErrors")]
    _errors: Empty,
    #[serde(rename = "hookAdditionalContext")]
    _context: Empty,
    #[serde(rename = "stopReason")]
    _reason: String,
    #[serde(rename = "hasOutput")]
    _output: bool,
    #[serde(rename = "level")]
    _level: String,
    #[serde(rename = "toolUseID")]
    _tool_use: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HookInfo {
    #[serde(rename = "command")]
    _command: String,
    #[serde(rename = "durationMs")]
    _duration: u64,
}

/// The session's running cost: each model's searches, and, inert, the
/// totals the result restates.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Cost {
    model_usage: BTreeMap<String, ModelUsage>,
    #[serde(rename = "totalCostUSD")]
    _cost: f64,
    #[serde(rename = "totalAPIDuration")]
    _api: u64,
    #[serde(rename = "totalAPIDurationWithoutRetries")]
    _api_once: u64,
    #[serde(rename = "totalToolDuration")]
    _tools: u64,
    #[serde(rename = "totalLinesAdded")]
    _added: u64,
    #[serde(rename = "totalLinesRemoved")]
    _removed: u64,
    #[serde(rename = "totalDuration")]
    _duration: u64,
    #[serde(rename = "startTime")]
    _start: u64,
    #[serde(rename = "hasUnknownModelCost")]
    _unknown: bool,
}

/// One row of claude's log, decoded, or why it is unread.
pub(super) fn row(mut fields: Map<String, Value>) -> Result<Decoded, Fault> {
    let _: Envelope = envelope(Harness::Claude, &mut fields, &ENVELOPE)?;
    let (label, said) = match decode(Harness::Claude, fields)? {
        Row::QueueOperation { .. } => ("queue-operation", Vec::new()),
        Row::User { .. } => ("user", Vec::new()),
        Row::Attachment(attached) => ("attachment", attachment_said(attached.attachment)),
        Row::AtisLatch { .. } => ("atis-latch", Vec::new()),
        Row::LastPrompt { .. } => ("last-prompt", Vec::new()),
        Row::Assistant(assistant) => ("assistant", assistant_said(*assistant)?),
        Row::System(Stop::StopHookSummary(summary)) => {
            let failed = summary.prevented_continuation;
            let at = "/preventedContinuation";
            let said = failed.then_some(Said::Failed { at }).into_iter().collect();
            ("system/stop_hook_summary", said)
        }
        Row::CostState(cost) => ("cost-state", searched(&cost.model_usage)),
    };
    Ok(Decoded {
        label: label.to_string(),
        said,
    })
}

/// A reply's row: its message, whose text is read unless the row states
/// its failure, and that failure.
fn assistant_said(assistant: Assistant) -> Result<Vec<Said>, Fault> {
    let failures = [
        (assistant.error.0.is_some(), "/error"),
        (
            assistant.is_api_error_message.0 == Some(true),
            "/isApiErrorMessage",
        ),
        (assistant.api_error_status.0.is_some(), "/apiErrorStatus"),
    ];
    let failed = failures.iter().any(|(failed, _)| *failed);
    let mut said = message_said(assistant.message, failed)?;
    let stated = failures.into_iter().filter(|(failed, _)| *failed);
    said.extend(stated.map(|(_, at)| Said::Failed { at }));
    Ok(said)
}

/// A name listing of tools at `at`, when it names any.
fn tools(at: &'static str, names: Vec<String>) -> Option<Said> {
    (!names.is_empty()).then_some(Said::Tools { at, names })
}

/// A listing of MCP servers at `at`, each given `status`, when it names
/// any.
fn servers(at: &'static str, names: Vec<String>, status: &'static str) -> Option<Said> {
    let servers: Vec<Server> = names
        .into_iter()
        .map(|name| Server { name, status })
        .collect();
    (!servers.is_empty()).then_some(Said::Servers { at, servers })
}

fn attachment_said(attachment: Attachment) -> Vec<Said> {
    match attachment {
        Attachment::Model { identity, .. } => vec![Said::Model(identity.model)],
        Attachment::DeferredToolsDelta(deferred) => deferred_said(*deferred),
        Attachment::McpInstructionsDelta { added, .. } => {
            servers("/attachment/addedNames", added, "connected")
                .into_iter()
                .collect()
        }
        Attachment::PromptSnapshot(snapshot) => {
            let listed = snapshot.tools.0.into_iter().flatten();
            let names = listed.map(|tool| tool.name).collect();
            tools("/attachment/tools", names).into_iter().collect()
        }
        Attachment::Environment { .. }
        | Attachment::AgentListingDelta { .. }
        | Attachment::SkillListing { .. }
        | Attachment::TotalTokensReminder { .. }
        | Attachment::SessionContext { .. }
        | Attachment::Date { .. }
        | Attachment::CredentialOrg { .. }
        | Attachment::RemoteSessionChange { .. } => Vec::new(),
    }
}

fn deferred_said(deferred: Deferred) -> Vec<Said> {
    let defined = deferred.surfaced_definitions.into_iter();
    let failed = deferred.failed_mcp_servers.into_iter();
    [
        tools("/attachment/addedNames", deferred.added_names),
        tools("/attachment/surfacedNames", deferred.surfaced_names),
        tools(
            "/attachment/surfacedDefinitions",
            defined.map(|definition| definition.name).collect(),
        ),
        servers(
            "/attachment/pendingMcpServers",
            deferred.pending_mcp_servers,
            "pending",
        ),
        servers(
            "/attachment/needsAuthMcpServers",
            deferred.needs_auth_mcp_servers,
            "needs-auth",
        ),
        servers(
            "/attachment/failedMcpServers",
            failed.map(|server| server.name).collect(),
            "failed",
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}
