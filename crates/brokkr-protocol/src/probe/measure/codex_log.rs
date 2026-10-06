//! Codex's transcript reader (#484): the rows of its rollout
//! (`~/.codex/sessions/<date>/rollout-*.jsonl`), decoded closed by `type`,
//! and a payload's by its own. The vocabulary is the one the controller
//! recorded from codex 0.160.0 on 2026-10-04, as skeletons
//! (`streams/codex-*.jsonl`): every key, number and boolean real, every
//! private string a `<key>` placeholder. A rollout lists no tool. A key
//! is consumed, or inert, as in `claude`: inert only when no value of it
//! can contradict a fact the verdict rests on.

use serde::de::IgnoredAny;
use serde::Deserialize;
use serde_json::{Map, Value};

use super::codex;
use super::read::{decode, envelope, Decoded, Empty, Given, Harness, Null, Prompt, Said};
use super::Fault;

/// The keys every row carries.
const ENVELOPE: [&str; 2] = ["timestamp", "ordinal"];

/// Inert, every key: when the row was written, and its place.
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct Envelope {
    #[serde(rename = "timestamp")]
    _timestamp: Given<String>,
    #[serde(rename = "ordinal")]
    _ordinal: Given<u64>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Row {
    SessionMeta {
        #[serde(default, rename = "payload")]
        _payload: Given<Box<SessionMeta>>,
    },
    EventMsg {
        payload: Box<EventMsg>,
    },
    ResponseItem {
        payload: Box<ResponseItem>,
        #[serde(default, rename = "metadata")]
        _metadata: Given<Metadata>,
    },
    WorldState {
        #[serde(rename = "payload")]
        _payload: Box<World>,
    },
    TurnContext {
        payload: Box<TurnContext>,
    },
    /// Inert: token counts, restated on stdout's `turn.completed`.
    TokenUsageRecord {
        #[serde(rename = "payload")]
        _payload: Box<UsageRecord>,
    },
}

/// Inert, every key: who started the session, where and with which
/// version, and the instructions it was given, none of which lists a
/// tool or reaches a server.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionMeta {
    #[serde(rename = "creator_user_id")]
    _user: String,
    #[serde(rename = "creator_account_id")]
    _account: String,
    #[serde(rename = "session_id")]
    _session: String,
    #[serde(rename = "id")]
    _id: String,
    #[serde(rename = "timestamp")]
    _timestamp: String,
    #[serde(rename = "cwd")]
    _cwd: String,
    #[serde(rename = "runtime_workspace_roots")]
    _roots: Vec<String>,
    #[serde(rename = "originator")]
    _originator: String,
    #[serde(rename = "cli_version")]
    _version: String,
    #[serde(rename = "source")]
    _source: String,
    #[serde(rename = "thread_source")]
    _thread_source: String,
    #[serde(rename = "model_provider")]
    _provider: String,
    #[serde(rename = "base_instructions")]
    _instructions: Instructions,
    #[serde(rename = "history_mode")]
    _history: String,
    #[serde(rename = "context_window")]
    _window: WindowId,
    #[serde(rename = "git")]
    _git: Bare,
}

/// Inert, every key: the instructions' text and the model they were
/// written for.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Instructions {
    #[serde(rename = "text")]
    _text: String,
    #[serde(rename = "provenance")]
    _provenance: Provenance,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Provenance {
    #[serde(rename = "type")]
    _kind: String,
    #[serde(rename = "model")]
    _model: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WindowId {
    #[serde(rename = "window_id")]
    _id: String,
}

/// An object the recordings only ever showed empty: a key is not one the
/// reader names.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bare {}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum EventMsg {
    /// Inert, every key: the turn's start and its window.
    TaskStarted {
        #[serde(rename = "turn_id")]
        _turn: String,
        #[serde(rename = "root_turn_id")]
        _root: String,
        #[serde(rename = "started_at")]
        _started: u64,
        #[serde(rename = "model_context_window")]
        _window: u64,
        #[serde(rename = "collaboration_mode_kind")]
        _mode: String,
    },
    ItemCompleted {
        item: Item,
        /// Inert, like the keys after it: which turn, and when.
        #[serde(rename = "thread_id")]
        _thread: String,
        #[serde(rename = "turn_id")]
        _turn: String,
        #[serde(rename = "started_at_ms")]
        _started: u64,
        #[serde(rename = "completed_at_ms")]
        _completed: u64,
    },
    TokenCount {
        rate_limits: Limits,
        /// Inert: token counts, restated on stdout's `turn.completed`.
        #[serde(rename = "info")]
        _info: TokenInfo,
    },
    TaskComplete(Box<TaskComplete>),
}

#[derive(Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
enum Item {
    /// Inert: only the probe's prompt reads.
    UserMessage {
        #[serde(rename = "id")]
        _id: String,
        #[serde(rename = "content")]
        _content: Vec<UserText>,
    },
    AgentMessage {
        content: Vec<AgentText>,
        /// Inert: the item's own id, and its phase, of one value.
        #[serde(rename = "id")]
        _id: String,
        #[serde(rename = "phase")]
        _phase: Phase,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum UserText {
    Text {
        #[serde(rename = "text")]
        _text: Prompt,
        #[serde(rename = "text_elements")]
        _elements: Empty,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
enum AgentText {
    /// Read whole, as the reply.
    Text { text: String },
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    FinalAnswer,
}

/// The account's limits: one reached is a failure, and every other key
/// is inert, a measure of the limit that fails nothing.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Limits {
    rate_limit_reached_type: Option<String>,
    #[serde(rename = "limit_id")]
    _id: String,
    #[serde(rename = "limit_name")]
    _name: Null,
    #[serde(rename = "primary")]
    _primary: Window,
    #[serde(rename = "secondary")]
    _secondary: Option<Window>,
    #[serde(rename = "credits")]
    _credits: Credits,
    #[serde(rename = "individual_limit")]
    _individual: Null,
    #[serde(rename = "spend_control_reached")]
    _spend: Null,
    #[serde(rename = "plan_type")]
    _plan: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Window {
    #[serde(rename = "used_percent")]
    _used: f64,
    #[serde(rename = "window_minutes")]
    _minutes: u64,
    #[serde(rename = "resets_at")]
    _resets: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Credits {
    #[serde(rename = "has_credits")]
    _has: bool,
    #[serde(rename = "unlimited")]
    _unlimited: bool,
    #[serde(rename = "balance")]
    _balance: String,
}

/// Inert, every key: token counts.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TokenInfo {
    #[serde(rename = "total_token_usage")]
    _total: Tokens,
    #[serde(rename = "last_token_usage")]
    _last: Tokens,
    #[serde(rename = "model_context_window")]
    _window: u64,
}

/// Inert, every key: token counts, none of them a use.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Tokens {
    #[serde(rename = "input_tokens")]
    _input: u64,
    #[serde(rename = "cached_input_tokens")]
    _cached: u64,
    #[serde(rename = "cache_write_input_tokens")]
    _written: u64,
    #[serde(rename = "output_tokens")]
    _output: u64,
    #[serde(rename = "reasoning_output_tokens")]
    _reasoning: u64,
    #[serde(rename = "total_tokens")]
    _total: u64,
}

/// The task's end: its last message, read whole, and its error, a
/// failure; the rest inert timings.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskComplete {
    last_agent_message: Option<String>,
    #[serde(default)]
    error: Given<TaskError>,
    #[serde(rename = "turn_id")]
    _turn: String,
    #[serde(rename = "started_at")]
    _started: u64,
    #[serde(rename = "completed_at")]
    _completed: u64,
    #[serde(rename = "duration_ms")]
    _duration: u64,
    #[serde(default, rename = "time_to_first_token_ms")]
    _first_token: Given<u64>,
}

/// Inert, every key: the error is a failure whatever it says.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskError {
    #[serde(rename = "message")]
    _message: String,
    #[serde(rename = "codex_error_info")]
    _info: String,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum ResponseItem {
    Message {
        role: Role,
        content: Vec<Content>,
        /// Inert: the item's own id, its phase, of one value, and its
        /// turn and kinds.
        #[serde(rename = "id")]
        _id: String,
        #[serde(default, rename = "phase")]
        _phase: Given<Phase>,
        #[serde(rename = "internal_chat_message_metadata_passthrough")]
        _passthrough: Passthrough,
    },
}

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Role {
    Developer,
    User,
    Assistant,
}

/// A message's text: the assistant's is read whole, as the reply; the
/// developer's and the user's are the instructions and the prompt codex
/// gave the model, inert, since none lists a tool or reaches a server.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Content {
    InputText { text: String },
    OutputText { text: String },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Passthrough {
    #[serde(rename = "turn_id")]
    _turn: String,
    #[serde(rename = "create_time")]
    _created: f64,
    #[serde(rename = "content_item_kinds")]
    _kinds: Vec<String>,
}

/// Inert, every key, save an MCP attribution, which reads only as none:
/// who wrote the message and where it is kept.
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct Metadata {
    #[serde(rename = "client_authored")]
    _client: Given<bool>,
    #[serde(rename = "mcp_attribution")]
    _mcp: Given<Attribution>,
    #[serde(rename = "retained_source")]
    _retained: Given<Retained>,
    #[serde(rename = "user_input_order")]
    _order: Given<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Attribution {
    #[serde(rename = "status")]
    _status: Unattributed,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Unattributed {
    None,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Retained {
    #[serde(rename = "id")]
    _id: RetainedId,
    #[serde(rename = "revision")]
    _revision: String,
    #[serde(rename = "complete")]
    _complete: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RetainedId {
    #[serde(rename = "message_id")]
    _message: String,
    #[serde(rename = "turn_id")]
    _turn: String,
    #[serde(rename = "role")]
    _role: Role,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct World {
    #[serde(rename = "state")]
    _state: State,
    /// Inert: whether the state is whole.
    #[serde(rename = "full")]
    _full: bool,
}

/// The state codex gave the model: an app's or a plugin's instructions
/// would be a connector's reach, so each reads only as absent; the rest
/// inert, the model's instructions and environment.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    #[serde(rename = "apps_instructions")]
    _apps: Absent,
    #[serde(rename = "plugins_instructions")]
    _plugins: Absent,
    #[serde(rename = "agents_md")]
    _agents_md: Bare,
    #[serde(rename = "collaboration_mode")]
    _collaboration: Collaboration,
    #[serde(rename = "context_window_guidance")]
    _guidance: String,
    #[serde(rename = "environments")]
    _environments: IgnoredAny,
    #[serde(rename = "environments_instructions")]
    _environments_instructions: bool,
    #[serde(rename = "git_attribution")]
    _git: bool,
    #[serde(rename = "host_skills")]
    _skills: IgnoredAny,
    #[serde(rename = "managed_developer_instructions")]
    _managed: Bare,
    #[serde(rename = "model")]
    _model: String,
    #[serde(rename = "multi_agent_mode")]
    _multi_agent: IgnoredAny,
    #[serde(default, rename = "multi_agent_usage_hint")]
    _hint: Given<String>,
    #[serde(rename = "permissions")]
    _permissions: Permissions,
    #[serde(rename = "persistent_mode")]
    _persistent: Bare,
    #[serde(rename = "realtime")]
    _realtime: IgnoredAny,
    #[serde(rename = "skills")]
    _skill_instructions: IgnoredAny,
}

/// A flag the recordings only ever showed `false`: `true` is not one the
/// reader names.
#[derive(Debug)]
struct Absent;

impl<'de> Deserialize<'de> for Absent {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Absent, D::Error> {
        match bool::deserialize(deserializer)? {
            false => Ok(Absent),
            true => Err(serde::de::Error::custom("not false")),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Collaboration {
    #[serde(rename = "mode")]
    _mode: String,
    #[serde(rename = "model")]
    _model: String,
    #[serde(rename = "instructions")]
    _instructions: String,
}

/// The commands the model may run unasked: only none reads.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Permissions {
    #[serde(rename = "instructions")]
    _instructions: String,
    #[serde(rename = "approved_command_prefixes")]
    _approved: Empty,
}

/// The turn's settings: the model it ran, and, inert, the sandbox and
/// approvals it ran under, which list no tool and reach no server, as
/// codex gives them.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TurnContext {
    model: String,
    #[serde(rename = "disabled_plugin_ids")]
    _disabled: Empty,
    #[serde(rename = "turn_id")]
    _turn: String,
    #[serde(rename = "root_turn_id")]
    _root: String,
    #[serde(rename = "cwd")]
    _cwd: String,
    #[serde(rename = "workspace_roots")]
    _roots: Vec<String>,
    #[serde(rename = "current_date")]
    _date: String,
    #[serde(rename = "timezone")]
    _timezone: String,
    #[serde(rename = "approval_policy")]
    _approval: String,
    #[serde(rename = "approvals_reviewer")]
    _reviewer: String,
    #[serde(rename = "sandbox_policy")]
    _sandbox: IgnoredAny,
    #[serde(rename = "permission_profile")]
    _profile: IgnoredAny,
    #[serde(rename = "active_permission_profile")]
    _active: IgnoredAny,
    #[serde(default, rename = "comp_hash")]
    _hash: Given<String>,
    #[serde(rename = "personality")]
    _personality: String,
    #[serde(rename = "collaboration_mode")]
    _collaboration: IgnoredAny,
    #[serde(rename = "multi_agent_version")]
    _multi_agent: String,
    #[serde(rename = "realtime_active")]
    _realtime: bool,
    #[serde(rename = "effort")]
    _effort: String,
    #[serde(rename = "summary")]
    _summary: String,
}

/// Inert, every key: token counts.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UsageRecord {
    #[serde(rename = "thread_id")]
    _thread: String,
    #[serde(rename = "turn_id")]
    _turn: String,
    #[serde(rename = "session_id")]
    _session: String,
    #[serde(rename = "root_turn_id")]
    _root: String,
    #[serde(rename = "response_id")]
    _response: String,
    #[serde(rename = "usage")]
    _usage: Tokens,
    #[serde(rename = "turn_token_usage")]
    _turn_usage: Tokens,
    #[serde(rename = "thread_token_usage")]
    _thread_usage: Tokens,
}

/// One row of codex's rollout, decoded, or why it is unread.
pub(super) fn row(mut fields: Map<String, Value>) -> Result<Decoded, Fault> {
    let _: Envelope = envelope(Harness::Codex, &mut fields, &ENVELOPE)?;
    let (label, said) = match decode(Harness::Codex, fields)? {
        Row::SessionMeta { .. } => ("session_meta", Vec::new()),
        Row::EventMsg { payload } => event_said(*payload)?,
        Row::ResponseItem { payload, .. } => ("response_item", item_said(*payload)?),
        Row::WorldState { .. } => ("world_state", Vec::new()),
        Row::TurnContext { payload } => ("turn_context", vec![Said::Model(payload.model)]),
        Row::TokenUsageRecord { .. } => ("token_usage_record", Vec::new()),
    };
    Ok(Decoded {
        label: label.to_string(),
        said,
    })
}

/// What an event row says: the only strings it reads are the agent's,
/// the turn's own answer (#484).
fn event_said(event: EventMsg) -> Result<(&'static str, Vec<Said>), Fault> {
    let mut said = Vec::new();
    let label = match event {
        EventMsg::TaskStarted { .. } => "event_msg/task_started",
        EventMsg::ItemCompleted { item, .. } => {
            if let Item::AgentMessage { content, .. } = item {
                for AgentText::Text { text } in &content {
                    said.extend(codex::answer(text)?);
                }
            }
            "event_msg/item_completed"
        }
        EventMsg::TokenCount { rate_limits, .. } => {
            if rate_limits.rate_limit_reached_type.is_some() {
                said.push(Said::Failed {
                    at: "/payload/rate_limits/rate_limit_reached_type",
                });
            }
            "event_msg/token_count"
        }
        EventMsg::TaskComplete(complete) => {
            if let Some(text) = &complete.last_agent_message {
                said.extend(codex::answer(text)?);
            }
            if complete.error.0.is_some() {
                said.push(Said::Failed {
                    at: "/payload/error",
                });
            }
            "event_msg/task_complete"
        }
    };
    Ok((label, said))
}

/// A message: the assistant's text read whole, as the turn's answer, and
/// every other role's inert.
fn item_said(item: ResponseItem) -> Result<Vec<Said>, Fault> {
    let ResponseItem::Message { role, content, .. } = item;
    let mut said = Vec::new();
    for part in content.iter().filter(|_| role == Role::Assistant) {
        let (Content::InputText { text } | Content::OutputText { text }) = part;
        said.extend(codex::answer(text)?);
    }
    Ok(said)
}
