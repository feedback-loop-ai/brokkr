//! Claude Code's typed reader (#484): `--output-format stream-json`
//! events, decoded closed by `type` and `subtype`, and the forms its
//! stderr prints. The vocabulary is the one the controller recorded from
//! claude 2.1.287 on 2026-10-03 (`tests/streams/claude-*`): the events
//! `system/hook_started`, `system/hook_response`, `system/init`,
//! `assistant`, `rate_limit_event` and `result/success`, and the
//! unrecognised-model line on stderr. `user` is the type its transcripts
//! open with, read with no key of its own. Every struct denies a key it
//! does not name. A key is consumed, or inert: a field named with a
//! leading `_`, decoded for its type and dropped, its comment saying why.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::{Map, Value};

use super::forms::{self, Form, Says, UNKNOWN_OPTION};
use super::read::{
    decode, Class, Counted, Decoded, Empty, Given, Harness, Never, Null, Refused, Said, Server,
    Uuid,
};
use super::Fault;

/// What claude prints that the probe reads whole.
const FORMS: [Form; 3] = [
    UNKNOWN_OPTION,
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
        /// Inert: the account's quota, which no fact rests on.
        #[serde(rename = "rate_limit_info")]
        _rate_limit_info: Quota,
        #[serde(default)]
        session_id: Given<Uuid>,
        /// Inert: the event's own id.
        #[serde(default, rename = "uuid")]
        _uuid: Given<Uuid>,
    },
    Result(Outcome),
    User {},
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
    /// Read whole as text, like stderr.
    #[serde(default)]
    output: Given<String>,
    /// Read whole as text, like stderr.
    #[serde(default)]
    stdout: Given<String>,
    /// Read whole as text, like stderr.
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
    /// Inert: how the hook exited.
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
    /// Inert: the model the turn runs; an echo of it refuses nothing.
    #[serde(rename = "model")]
    _model: Given<String>,
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
    /// Inert: a subagent runs under the tools listed.
    #[serde(rename = "agents")]
    _agents: Given<Vec<String>>,
    /// Inert: a skill is a prompt, run under the tools listed.
    #[serde(rename = "skills")]
    _skills: Given<Vec<String>>,
    /// Inert: a command is a prompt, run under the tools listed.
    #[serde(rename = "slash_commands")]
    _slash_commands: Given<Vec<String>>,
    /// Inert: the terminal's own commands.
    #[serde(rename = "terminal_slash_commands")]
    _terminal_slash_commands: Given<Vec<String>>,
    /// Inert: a plugin's MCP servers and tools are listed under
    /// `mcp_servers` and `tools`, which are read.
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
    /// Inert: no tool ran.
    #[serde(rename = "parent_tool_use_id")]
    _parent_tool_use_id: Null,
    /// Inert: when the message was sent.
    #[serde(default, rename = "timestamp")]
    _timestamp: Given<String>,
    /// Inert: the provider's request id.
    #[serde(default, rename = "request_id")]
    _request_id: Given<String>,
    /// Inert: the class the message's text states, which is read.
    #[serde(default, rename = "error")]
    _error: Given<ApiError>,
    /// Inert: as `error`.
    #[serde(default, rename = "is_api_error_message")]
    _is_api_error_message: Given<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ApiError {
    ModelNotFound,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Message {
    /// The message the usage counts.
    #[serde(default)]
    id: Given<String>,
    content: Vec<Block>,
    #[serde(default)]
    usage: Given<Usage>,
    /// Inert: the model the turn ran.
    #[serde(default, rename = "model")]
    _model: Given<String>,
    /// Inert: a tag.
    #[serde(default, rename = "type")]
    _kind: Given<MessageTag>,
    /// Inert: a tag.
    #[serde(default, rename = "role")]
    _role: Given<Role>,
    /// Inert: why the message ended, `null` while it streams.
    #[serde(rename = "stop_reason")]
    _stop_reason: Option<String>,
    /// Inert: as `stop_reason`.
    #[serde(rename = "stop_sequence")]
    _stop_sequence: Option<String>,
    #[serde(rename = "stop_details")]
    _stop_details: Null,
    #[serde(rename = "container")]
    _container: Null,
    #[serde(rename = "diagnostics")]
    _diagnostics: Null,
    #[serde(rename = "context_management")]
    _context_management: Null,
    #[serde(default, rename = "input_transformations")]
    _input_transformations: Empty,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum MessageTag {
    Message,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Role {
    Assistant,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Block {
    /// Read whole, as the reply or a refusal.
    Text { text: String },
}

/// Every key may be absent, so the struct defaults each one; those the
/// recordings show `null` are `Option`s.
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct Usage {
    input_tokens: Given<u64>,
    cache_creation_input_tokens: Given<u64>,
    cache_read_input_tokens: Given<u64>,
    output_tokens: Given<u64>,
    /// Inert: `cache_creation_input_tokens` split by lifetime.
    #[serde(rename = "cache_creation")]
    _cache_creation: Given<CacheCreation>,
    /// Inert: counts of use; the tool listing says what was offered.
    #[serde(rename = "server_tool_use")]
    _server_tool_use: Given<ServerToolUse>,
    /// Inert: the counts restated per request.
    #[serde(rename = "iterations")]
    _iterations: Option<Vec<Iteration>>,
    /// Inert: part of `output_tokens`.
    #[serde(rename = "output_tokens_details")]
    _output_tokens_details: Option<OutputDetails>,
    /// Inert: a billing tag.
    #[serde(rename = "service_tier")]
    _service_tier: Option<String>,
    /// Inert: a billing tag.
    #[serde(rename = "inference_geo")]
    _inference_geo: Option<String>,
    /// Inert: a billing tag.
    #[serde(rename = "speed")]
    _speed: Option<String>,
    #[serde(rename = "fallback_credit")]
    _fallback_credit: Null,
}

/// Inert, every key: `cache_creation_input_tokens` by lifetime.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CacheCreation {
    #[serde(rename = "ephemeral_1h_input_tokens")]
    _hour: u64,
    #[serde(rename = "ephemeral_5m_input_tokens")]
    _minutes: u64,
}

/// Inert, every key: counts of use.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ServerToolUse {
    #[serde(rename = "web_search_requests")]
    _searches: u64,
    #[serde(rename = "web_fetch_requests")]
    _fetches: u64,
}

/// Inert, every key: the usage restated for one request.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Iteration {
    #[serde(rename = "input_tokens")]
    _input: u64,
    #[serde(rename = "output_tokens")]
    _output: u64,
    #[serde(rename = "cache_read_input_tokens")]
    _cache_read: u64,
    #[serde(rename = "cache_creation_input_tokens")]
    _cache_creation: u64,
    #[serde(rename = "cache_creation")]
    _by_lifetime: CacheCreation,
    #[serde(rename = "type")]
    _kind: MessageTag,
}

/// Inert, every key: part of `output_tokens`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OutputDetails {
    #[serde(rename = "thinking_tokens")]
    _thinking: u64,
}

/// Inert, every key: the account's quota.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Quota {
    #[serde(rename = "status")]
    _status: String,
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
    /// Inert: the text in `result` says what failed.
    #[serde(rename = "is_error")]
    _is_error: Given<bool>,
    /// Inert: as `is_error`.
    #[serde(rename = "api_error_status")]
    _api_error_status: Option<u16>,
    /// Inert: `usage` and `total_cost_usd` restated per model.
    #[serde(rename = "modelUsage")]
    _model_usage: Given<BTreeMap<String, ModelUsage>>,
    /// Inert: why the turn ended.
    #[serde(rename = "stop_reason")]
    _stop_reason: Given<String>,
    /// Inert: as `stop_reason`.
    #[serde(rename = "terminal_reason")]
    _terminal_reason: Given<String>,
    /// Inert: no subagent ran.
    #[serde(rename = "subagent_stats")]
    _subagent_stats: Given<Subagents>,
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
    /// Inert, like each count and timing after it.
    #[serde(rename = "num_turns")]
    _num_turns: Given<u64>,
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

/// Inert, every key: one model's share of the turn.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelUsage {
    #[serde(rename = "inputTokens")]
    _input: u64,
    #[serde(rename = "outputTokens")]
    _output: u64,
    #[serde(rename = "cacheReadInputTokens")]
    _cache_read: u64,
    #[serde(rename = "cacheCreationInputTokens")]
    _cache_creation: u64,
    #[serde(rename = "webSearchRequests")]
    _searches: u64,
    #[serde(rename = "costUSD")]
    _cost: f64,
    #[serde(rename = "contextWindow")]
    _context_window: u64,
    #[serde(rename = "maxOutputTokens")]
    _max_output: u64,
    #[serde(rename = "thinkingTokens")]
    _thinking: u64,
    #[serde(rename = "canonicalModel")]
    _canonical_model: String,
    #[serde(rename = "provider")]
    _provider: String,
    #[serde(rename = "costBasis")]
    _cost_basis: String,
}

/// Inert, every key: counts of subagents, none of which ran.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Subagents {
    #[serde(rename = "spawned")]
    _spawned: u64,
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
        Event::RateLimit { session_id, .. } => {
            Ok(labelled("rate_limit_event", sessions(session_id)))
        }
        Event::Result(outcome) => outcome_said(outcome),
        Event::User {} => Ok(labelled("user", Vec::new())),
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
    if let Some(names) = init.tools.0 {
        said.push(Said::Tools {
            at: "/tools",
            names,
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

/// A hook event: its session, and what the hook printed, read whole.
fn hook_said(label: &str, hook: Hook) -> Result<Decoded, Fault> {
    let mut said = sessions(Given(Some(hook.session_id)));
    for text in [hook.output.0, hook.stdout.0, hook.stderr.0]
        .iter()
        .flatten()
    {
        said.extend(texts(text)?);
    }
    Ok(labelled(label, said))
}

/// A string the event carries, read whole by claude's forms.
fn texts(text: &str) -> Result<Vec<Said>, Fault> {
    forms::read(text, &FORMS).ok_or(Fault::Unrecognised)
}

fn usage_said(at: &'static str, message: Option<String>, usage: Usage) -> Said {
    Counted::of(
        at,
        message,
        [
            ("input_tokens", usage.input_tokens.0),
            (
                "cache_creation_input_tokens",
                usage.cache_creation_input_tokens.0,
            ),
            ("cache_read_input_tokens", usage.cache_read_input_tokens.0),
            ("output_tokens", usage.output_tokens.0),
        ],
    )
}

fn assistant_said(assistant: Assistant) -> Result<Decoded, Fault> {
    let message = assistant.message;
    let mut said = sessions(assistant.session_id);
    for Block::Text { text } in &message.content {
        said.extend(texts(text)?);
    }
    if let Some(usage) = message.usage.0 {
        said.push(usage_said("/message/usage", message.id.0, usage));
    }
    Ok(labelled("assistant", said))
}

fn outcome_said(outcome: Outcome) -> Result<Decoded, Fault> {
    let (label, outcome) = match outcome {
        Outcome::Success(finished) => ("result/success", *finished),
    };
    let mut said = sessions(outcome.session_id);
    if let Some(text) = &outcome.result.0 {
        said.extend(texts(text)?);
    }
    if let Some(usage) = outcome.usage.0 {
        said.push(usage_said("/usage", None, usage));
    }
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
        return forms::read(line, &FORMS);
    };
    let unrecognised: Unrecognised = decode(Harness::Claude, super::strict::object(json)?).ok()?;
    Some(vec![Said::Refusal(Refused {
        class: Class::Config,
        object: unrecognised.model,
    })])
}
