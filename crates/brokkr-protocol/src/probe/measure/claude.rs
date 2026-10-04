//! Claude Code's typed reader (#484): `--output-format stream-json`
//! events, decoded closed by `type` and `subtype`, and the forms its
//! stderr prints. The vocabulary is the one the controller recorded from
//! claude 2.1.287 on 2026-10-03 (`tests/streams/claude-*`): the events
//! `system/hook_started`, `system/hook_response`, `system/init`,
//! `assistant`, `rate_limit_event` and `result/success`, and the
//! unrecognised-model line on stderr; its transcripts are `claude_log`'s.
//! Every struct denies a key it does not name. A key is consumed, or
//! inert: a field named with a leading `_`, decoded for its type and
//! dropped, its comment saying why. A key is inert only when no value of
//! it can contradict a fact the verdict rests on (#484): the turn's
//! reply, its failure, the tools it listed and ran, and the MCP servers
//! that reached it.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::{Map, Value};

use super::claude_message::{
    message_said, searched, stopped, usage_said, Ending, Message, ModelUsage, Usage, IN_RESULT,
};
use super::forms::{self, Form, Origin, Says, UNKNOWN_OPTION};
use super::read::{
    decode, Class, Decoded, Empty, Given, Harness, Never, Null, Refused, Said, Server, Uuid,
};
use super::Fault;

/// What claude prints that the probe reads whole.
const FORMS: [Form; 4] = [
    UNKNOWN_OPTION,
    // commander's refusal of a level `--effort` does not take.
    Form {
        words: "error option --effort level argument {level} is invalid allowed choices are \
                {levels}",
        says: Says::RefusesLevel,
    },
    // A turn's reply to an unknown model, recorded 2026-10-03.
    Form {
        words: "there's an issue with the selected model {model} it may not exist or you may not \
                have access to it run --model to pick a different model",
        says: Says::Refuses(Class::Config),
    },
    // Its refusal of a launch with no credential.
    Form {
        words: "invalid api key please run login",
        says: Says::RefusesThe(Class::Auth, "API key"),
    },
];

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Event {
    System(System),
    Assistant(Box<Assistant>),
    #[serde(rename = "rate_limit_event")]
    RateLimit {
        rate_limit_info: Quota,
        #[serde(default)]
        session_id: Given<Uuid>,
        /// Inert: the event's own id.
        #[serde(default, rename = "uuid")]
        _uuid: Given<Uuid>,
    },
    Result(Outcome),
}

#[derive(Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
enum System {
    Init(Box<Init>),
    HookStarted(Hook),
    HookResponse(Hook),
}

/// Both hook events: a response adds what the hook printed and how it
/// ended.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Hook {
    session_id: Uuid,
    /// Read whole as text, like stderr, never as the reply.
    #[serde(default)]
    output: Given<String>,
    /// Read whole as text, as `output`.
    #[serde(default)]
    stdout: Given<String>,
    /// Read whole as text, as `output`.
    #[serde(default)]
    stderr: Given<String>,
    /// Inert: the hook's run.
    #[serde(rename = "hook_id")]
    _hook_id: Uuid,
    /// Inert: which hook ran, under the scratch HOME's settings.
    #[serde(rename = "hook_name")]
    _hook_name: String,
    /// Inert: the event the hook ran on.
    #[serde(rename = "hook_event")]
    _hook_event: String,
    /// Inert: the event's own id.
    #[serde(rename = "uuid")]
    _uuid: Uuid,
    /// Inert: how the hook exited; a hook runs beside the turn, and its
    /// exit neither lists a tool nor fails the turn.
    #[serde(default, rename = "exit_code")]
    _exit_code: Given<i32>,
    /// Inert: as `exit_code`.
    #[serde(default, rename = "outcome")]
    _outcome: Given<String>,
}

/// Every key may be absent, so the struct defaults each one.
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct Init {
    session_id: Given<Uuid>,
    tools: Given<Vec<String>>,
    mcp_servers: Given<Vec<ServerEntry>>,
    model: Given<String>,
    /// An entry `mcp__<server>__<prompt>` is that server's prompt, so it
    /// reached the turn; every other is the CLI's or a prompt run under
    /// the tools listed.
    slash_commands: Given<Vec<String>>,
    /// Inert: the scratch repository the turn runs in.
    #[serde(rename = "cwd")]
    _cwd: Given<String>,
    /// Inert: the permission mode the adapter passes.
    #[serde(rename = "permissionMode")]
    _permission_mode: Given<String>,
    /// Inert: where the credential came from.
    #[serde(rename = "apiKeySource")]
    _api_key_source: Given<String>,
    /// Inert: `--version` measures the version.
    #[serde(rename = "claude_code_version")]
    _claude_code_version: Given<String>,
    /// Inert: the event's own id.
    #[serde(rename = "uuid")]
    _uuid: Given<Uuid>,
    /// Inert: an output style, which reaches nothing.
    #[serde(rename = "output_style")]
    _output_style: Given<String>,
    /// Inert: where memory is kept.
    #[serde(rename = "memory_paths")]
    _memory_paths: Given<String>,
    /// Inert: the SDK's socket.
    #[serde(rename = "messaging_socket_path")]
    _messaging_socket_path: Given<String>,
    /// Inert: a subagent runs under the tools listed, and reaches only
    /// the servers listed, so no name of one contradicts either listing.
    #[serde(rename = "agents")]
    _agents: Given<Vec<String>>,
    /// Inert: a skill is a prompt, run under the tools listed, so no name
    /// of one contradicts a listing.
    #[serde(rename = "skills")]
    _skills: Given<Vec<String>>,
    /// Inert: the terminal's own commands, which no headless turn runs.
    #[serde(rename = "terminal_slash_commands")]
    _terminal_slash_commands: Given<Vec<String>>,
    /// Inert: a plugin reaches a turn only through the MCP servers and
    /// tools it adds, which `mcp_servers` and `tools` list and the probe
    /// reads, so no plugin's name contradicts a listing.
    #[serde(rename = "plugins")]
    _plugins: Given<Vec<Plugin>>,
    /// Inert: the SDK protocol's capabilities, not the turn's.
    #[serde(rename = "capabilities")]
    _capabilities: Given<Vec<String>>,
    /// Inert: a preference.
    #[serde(rename = "analytics_disabled")]
    _analytics_disabled: Given<bool>,
    /// Inert: a preference.
    #[serde(rename = "product_feedback_disabled")]
    _product_feedback_disabled: Given<bool>,
    /// Inert: a preference.
    #[serde(rename = "per_turn_effort_active")]
    _per_turn_effort_active: Given<bool>,
    /// Inert: a preference.
    #[serde(rename = "fast_mode_state")]
    _fast_mode_state: Given<String>,
    /// Inert: why the preference is off.
    #[serde(rename = "fast_mode_disabled_reason")]
    _fast_mode_disabled_reason: Given<String>,
    /// Inert: a preference.
    #[serde(rename = "view_mode")]
    _view_mode: Given<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ServerEntry {
    name: String,
    status: Status,
    /// Inert: where the server was configured; its name is what reached.
    #[serde(default, rename = "source")]
    _source: Given<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Status {
    Connected,
    Failed,
    NeedsAuth,
    Pending,
}

impl Status {
    fn as_str(&self) -> &'static str {
        match self {
            Status::Connected => "connected",
            Status::Failed => "failed",
            Status::NeedsAuth => "needs-auth",
            Status::Pending => "pending",
        }
    }
}

/// Inert, every key: a plugin's servers and tools are listed apart.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Plugin {
    #[serde(rename = "name")]
    _name: String,
    #[serde(rename = "path")]
    _path: String,
    #[serde(rename = "source")]
    _source: String,
    #[serde(default, rename = "version")]
    _version: Given<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Assistant {
    message: Message,
    #[serde(default)]
    session_id: Given<Uuid>,
    /// Inert: the event's own id.
    #[serde(default, rename = "uuid")]
    _uuid: Given<Uuid>,
    /// Inert: `null` is the only value read, so no tool ran.
    #[serde(rename = "parent_tool_use_id")]
    _parent_tool_use_id: Null,
    /// Inert: when the message was sent.
    #[serde(default, rename = "timestamp")]
    _timestamp: Given<String>,
    /// Inert: the provider's request id.
    #[serde(default, rename = "request_id")]
    _request_id: Given<String>,
    /// Any value is a failure.
    #[serde(default)]
    error: Given<ApiError>,
    /// `true` is a failure.
    #[serde(default)]
    is_api_error_message: Given<bool>,
}

/// The class of an API error claude names a message by.
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ApiError {
    ModelNotFound,
}

/// The account's quota: a rejected turn failed, and every other key is
/// inert, a measure of the quota that fails nothing.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Quota {
    status: QuotaStatus,
    #[serde(rename = "resetsAt")]
    _resets_at: u64,
    #[serde(rename = "rateLimitType")]
    _kind: String,
    #[serde(rename = "utilization")]
    _utilization: f64,
    #[serde(rename = "isUsingOverage")]
    _overage: bool,
    #[serde(default, rename = "surpassedThreshold")]
    _threshold: Given<f64>,
    #[serde(default, rename = "unifiedWindows")]
    _windows: Given<BTreeMap<String, Window>>,
}

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum QuotaStatus {
    Allowed,
    AllowedWarning,
    Rejected,
}

/// Inert, every key: one window of the quota.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Window {
    #[serde(rename = "utilization")]
    _utilization: f64,
    #[serde(rename = "resetsAt")]
    _resets_at: u64,
}

#[derive(Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
enum Outcome {
    Success(Box<Finished>),
}

/// A turn's result. Every key may be absent, so the struct defaults each
/// one; the one the recordings show `null` is an `Option`.
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct Finished {
    session_id: Given<Uuid>,
    total_cost_usd: Given<f64>,
    usage: Given<Usage>,
    /// Read whole, as the reply or a refusal.
    result: Given<String>,
    /// `true` is a failure.
    is_error: Given<bool>,
    /// Any status is a failure.
    api_error_status: Option<u16>,
    /// Each model's share of the turn: its searches are a tool run.
    #[serde(rename = "modelUsage")]
    model_usage: Given<BTreeMap<String, ModelUsage>>,
    /// Why the turn ended: a tool use is a tool run.
    stop_reason: Given<Ending>,
    /// How the turn ended: an API error is a failure.
    terminal_reason: Given<Terminal>,
    /// Counts of subagents: one spawned is a tool run, since a turn
    /// spawns one only through a tool (#484).
    subagent_stats: Given<Subagents>,
    /// Inert: an empty list is the only value read.
    #[serde(rename = "permission_denials")]
    _permission_denials: Empty,
    /// Inert: a preference.
    #[serde(rename = "fast_mode_state")]
    _fast_mode_state: Given<String>,
    /// Inert: why the preference is off.
    #[serde(rename = "fast_mode_disabled_reason")]
    _fast_mode_disabled_reason: Given<String>,
    /// Inert: the event's own id.
    #[serde(rename = "uuid")]
    _uuid: Given<Uuid>,
    /// None taken is a failure, and more than one a tool run: a turn
    /// that runs no tool answers in one (#484).
    num_turns: Given<u64>,
    /// Inert, like each timing and count after it: how long, or in what
    /// order, a turn ran says nothing of what it did.
    #[serde(rename = "duration_ms")]
    _duration_ms: Given<u64>,
    #[serde(rename = "duration_api_ms")]
    _duration_api_ms: Given<u64>,
    #[serde(rename = "ttft_ms")]
    _ttft_ms: Given<u64>,
    #[serde(rename = "ttft_stream_ms")]
    _ttft_stream_ms: Given<u64>,
    #[serde(rename = "time_to_request_ms")]
    _time_to_request_ms: Given<u64>,
    #[serde(rename = "first_content_frame_ms")]
    _first_content_frame_ms: Given<u64>,
    #[serde(rename = "queued_turn_count")]
    _queued_turn_count: Given<u64>,
    #[serde(rename = "result_index")]
    _result_index: Given<u64>,
}

/// How a turn ended.
#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Terminal {
    Completed,
    ApiError,
}

/// Counts of subagents: how many were spawned, consumed, and every other
/// key inert, a count of those spawned.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Subagents {
    spawned: u64,
    #[serde(rename = "requested")]
    _requested: Requested,
    #[serde(rename = "started_in_background")]
    _in_background: u64,
    #[serde(rename = "max_depth")]
    _max_depth: u64,
    #[serde(rename = "spawned_by_subagents")]
    _by_subagents: u64,
    #[serde(rename = "completed")]
    _completed: u64,
    #[serde(rename = "failed")]
    _failed: u64,
    #[serde(rename = "killed")]
    _killed: Killed,
    #[serde(rename = "refused")]
    _refused: Declined,
    #[serde(rename = "by_type")]
    _by_type: BTreeMap<String, Never>,
}

/// Inert, every key: a count.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Requested {
    #[serde(rename = "background")]
    _background: u64,
    #[serde(rename = "foreground")]
    _foreground: u64,
    #[serde(rename = "unset")]
    _unset: u64,
}

/// Inert, every key: a count.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Killed {
    #[serde(rename = "parent")]
    _parent: u64,
    #[serde(rename = "user")]
    _user: u64,
    #[serde(rename = "system")]
    _system: u64,
}

/// Inert, every key: a count.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Declined {
    #[serde(rename = "depth_limit")]
    _depth_limit: u64,
    #[serde(rename = "concurrency_limit")]
    _concurrency_limit: u64,
    #[serde(rename = "budget")]
    _budget: u64,
}

/// One claude event, decoded, or why it is unread.
pub(super) fn event(fields: Map<String, Value>) -> Result<Decoded, Fault> {
    match decode(Harness::Claude, fields)? {
        Event::System(System::Init(init)) => Ok(init_said(*init)),
        Event::System(System::HookStarted(hook)) => hook_said("system/hook_started", hook),
        Event::System(System::HookResponse(hook)) => hook_said("system/hook_response", hook),
        Event::Assistant(assistant) => assistant_said(*assistant),
        Event::RateLimit {
            rate_limit_info,
            session_id,
            ..
        } => {
            let mut said = sessions(session_id);
            let rejected = rate_limit_info.status == QuotaStatus::Rejected;
            said.extend(failures([(rejected, "/rate_limit_info/status")]));
            Ok(labelled("rate_limit_event", said))
        }
        Event::Result(outcome) => outcome_said(outcome),
    }
}

fn labelled(label: &str, said: Vec<Said>) -> Decoded {
    Decoded {
        label: label.to_string(),
        said,
    }
}

fn sessions(id: Given<Uuid>) -> Vec<Said> {
    let session = |id: Uuid| Said::Session {
        key: "session_id",
        id: id.0,
    };
    id.0.map(session).into_iter().collect()
}

fn init_said(init: Init) -> Decoded {
    let mut said = sessions(init.session_id);
    said.extend(init.model.0.map(Said::Model));
    if let Some(names) = init.tools.0 {
        said.push(Said::Tools {
            at: "/tools",
            names,
        });
    }
    let prompts = init.slash_commands.0.into_iter().flatten();
    let served: Vec<String> = prompts.filter(|name| name.starts_with("mcp__")).collect();
    if !served.is_empty() {
        said.push(Said::Tools {
            at: "/slash_commands",
            names: served,
        });
    }
    if let Some(entries) = init.mcp_servers.0 {
        let servers = entries.into_iter().map(|entry| Server {
            name: entry.name,
            status: entry.status.as_str(),
        });
        said.push(Said::Servers {
            at: "/mcp_servers",
            servers: servers.collect(),
        });
    }
    labelled("system/init", said)
}

/// A hook event: its session, and what the hook printed, read whole,
/// never as the reply: a hook is not the turn's answer (#484).
fn hook_said(label: &str, hook: Hook) -> Result<Decoded, Fault> {
    let mut said = sessions(Given(Some(hook.session_id)));
    for text in [hook.output.0, hook.stdout.0, hook.stderr.0]
        .iter()
        .flatten()
    {
        said.extend(texts(text, Origin::Aside)?);
    }
    Ok(labelled(label, said))
}

/// A string the event carries, from `origin`, read whole by claude's
/// forms.
pub(super) fn texts(text: &str, origin: Origin) -> Result<Vec<Said>, Fault> {
    forms::read(text, origin, &FORMS).ok_or(Fault::Unrecognised)
}

/// A failure at each pointer whose test holds.
fn failures<const N: usize>(tests: [(bool, &'static str); N]) -> impl Iterator<Item = Said> {
    let failed = tests.into_iter().filter(|(failed, _)| *failed);
    failed.map(|(_, at)| Said::Failed { at })
}

/// A tool run, naming no tool, at each pointer whose test holds.
fn runs<const N: usize>(tests: [(bool, &'static str); N]) -> impl Iterator<Item = Said> {
    let ran = tests.into_iter().filter(|(ran, _)| *ran);
    ran.map(|(_, at)| Said::Ran { tool: None, at })
}

fn assistant_said(assistant: Assistant) -> Result<Decoded, Fault> {
    let mut said = sessions(assistant.session_id);
    said.extend(message_said(assistant.message, false)?);
    said.extend(failures([
        (assistant.error.0.is_some(), "/error"),
        (
            assistant.is_api_error_message.0 == Some(true),
            "/is_api_error_message",
        ),
    ]));
    Ok(labelled("assistant", said))
}

fn outcome_said(outcome: Outcome) -> Result<Decoded, Fault> {
    let (label, outcome) = match outcome {
        Outcome::Success(finished) => ("result/success", *finished),
    };
    let mut said = sessions(outcome.session_id);
    if let Some(text) = &outcome.result.0 {
        said.extend(texts(text, Origin::Answer)?);
    }
    said.extend(failures([
        (outcome.is_error.0 == Some(true), "/is_error"),
        (outcome.api_error_status.is_some(), "/api_error_status"),
        (outcome.num_turns.0 == Some(0), "/num_turns"),
        (
            outcome.terminal_reason.0 == Some(Terminal::ApiError),
            "/terminal_reason",
        ),
    ]));
    let spawned = outcome.subagent_stats.0.map_or(0, |stats| stats.spawned);
    said.extend(runs([
        (spawned > 0, "/subagent_stats/spawned"),
        (outcome.num_turns.0 > Some(1), "/num_turns"),
    ]));
    if let Some(usage) = outcome.usage.0 {
        said.extend(usage_said(&IN_RESULT, None, usage));
    }
    said.extend(stopped(outcome.stop_reason.0.as_ref(), IN_RESULT.stop));
    let models = outcome.model_usage.0.unwrap_or_default();
    said.extend(searched(&models));
    if outcome.total_cost_usd.0.is_some() {
        said.push(Said::Cost {
            at: "/total_cost_usd",
        });
    }
    Ok(labelled(label, said))
}

/// What claude prints on stderr for a model it does not know, after the
/// tag `[claude-code:unrecognized_model]`, recorded 2026-10-03.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Unrecognised {
    model: String,
    /// Inert: which caller asked.
    #[serde(rename = "query_source")]
    _query_source: String,
}

/// One line of claude's text: its tagged unrecognised-model line, whose
/// object is the model it names, or a form read whole.
pub(super) fn text(line: &str) -> Option<Vec<Said>> {
    let Some(json) = line
        .trim()
        .strip_prefix("[claude-code:unrecognized_model] ")
    else {
        return forms::read(line, Origin::Aside, &FORMS);
    };
    let unrecognised: Unrecognised = decode(Harness::Claude, super::strict::object(json)?).ok()?;
    Some(vec![Said::Refusal(Refused {
        class: Class::Config,
        object: unrecognised.model,
    })])
}
