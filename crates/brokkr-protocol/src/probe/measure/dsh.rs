//! dsh's typed reader (#484). Its headless profile prints the final
//! message as text on stdout, and its reasoning on stderr after a
//! `dsh: reasoning:` header; its events are its session log's
//! (`session.v3.jsonl`), decoded closed by `type`. The vocabulary is the
//! one the controller recorded from dsh 0.1.5-rc.1 against the operator's
//! local model on 2026-10-04 (`streams/dsh-plain.*`, the log a skeleton:
//! every key, number and boolean real, every private string a `<key>`
//! placeholder), and commander's refusal of `--model`. A key is consumed,
//! or inert, as in `claude`: inert only when no value of it can
//! contradict a fact the verdict rests on.

use serde::de::IgnoredAny;
use serde::Deserialize;
use serde_json::{Map, Value};

use super::forms::{self, Form, Says, UNKNOWN_OPTION};
use super::read::{decode, envelope, Block, Class, Counted, Decoded, Given, Harness, Prompt, Said};
use super::Fault;

/// What dsh prints that the probe reads whole.
const FORMS: [Form; 2] = [
    UNKNOWN_OPTION,
    // Its refusal of a launch with no credential.
    Form {
        words: "error {credential} is not set",
        says: Says::Refuses(Class::Auth),
    },
];

/// The line on stderr that opens dsh's reasoning, recorded 2026-10-04.
const REASONING: &str = "dsh: reasoning:";

/// The keys an event carries whatever its type.
const ENVELOPE: [&str; 3] = ["seq", "time", "surfaceOp"];

/// Inert, every key: the event's place in the log, its time, and how a
/// view shows it.
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct Envelope {
    #[serde(rename = "seq")]
    _seq: Given<u64>,
    #[serde(rename = "time")]
    _time: Given<u64>,
    #[serde(rename = "surfaceOp")]
    _surface: Given<Surface>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Surface {
    Append,
}

#[derive(Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
enum Event {
    #[serde(rename = "session")]
    Session {
        id: String,
        /// Inert: the log's format.
        #[serde(rename = "version")]
        _version: u64,
        /// Inert, like the keys after it: when, where and how deep a
        /// delegation the session is.
        #[serde(default, rename = "createdAt")]
        _created: Given<u64>,
        #[serde(default, rename = "cwd")]
        _cwd: Given<String>,
        #[serde(default, rename = "isSeeded")]
        _seeded: Given<bool>,
        #[serde(default, rename = "delegationDepth")]
        _delegation_depth: Given<u64>,
    },
    /// Inert: the permissions, sandbox and approvals dsh runs under,
    /// which list no tool and reach no server.
    #[serde(rename = "permission/preset")]
    Preset {
        #[serde(rename = "data")]
        _data: Setting,
    },
    #[serde(rename = "sandbox/mode")]
    Sandbox {
        #[serde(rename = "data")]
        _data: Setting,
    },
    #[serde(rename = "approval/policy")]
    Approval {
        #[serde(rename = "data")]
        _data: Setting,
    },
    /// Inert: the prompt queued for the agent.
    #[serde(rename = "agent/inbox/spliced")]
    Inbox {
        #[serde(rename = "data")]
        _data: Spliced,
    },
    /// Inert: where the turn and its steps start and end.
    #[serde(rename = "turn/start")]
    TurnStart {
        #[serde(rename = "data")]
        _data: Place,
    },
    #[serde(rename = "step/start")]
    StepStart {
        #[serde(rename = "data")]
        _data: Place,
    },
    #[serde(rename = "step/end")]
    StepEnd {
        #[serde(rename = "data")]
        _data: Place,
    },
    #[serde(rename = "turn/end")]
    TurnEnd {
        #[serde(rename = "data")]
        _data: Ended,
    },
    /// Inert: the system prompt and the prompt as the model reads them,
    /// which list no tool and reach no server.
    #[serde(rename = "system/message")]
    SystemMessage {
        #[serde(rename = "data")]
        _data: IgnoredAny,
    },
    #[serde(rename = "user/message")]
    UserMessage {
        #[serde(rename = "data")]
        _data: IgnoredAny,
    },
    #[serde(rename = "request/header")]
    Header { data: Header },
    /// Inert: the model and window, restated from the header.
    #[serde(rename = "request/context")]
    Context {
        #[serde(rename = "data")]
        _data: IgnoredAny,
    },
    /// Inert: the session's title, and the request that named it, which
    /// runs no tool.
    #[serde(rename = "session/title")]
    Title {
        #[serde(rename = "data")]
        _data: IgnoredAny,
    },
    #[serde(rename = "session/title-llm-request")]
    TitleRequest {
        #[serde(rename = "data")]
        _data: IgnoredAny,
    },
    #[serde(rename = "assistant/message")]
    AssistantMessage { data: Step },
}

/// Inert, every key: a setting by its one value.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Setting {
    #[serde(default, rename = "preset")]
    _preset: Given<String>,
    #[serde(default, rename = "mode")]
    _mode: Given<String>,
    #[serde(default, rename = "policy")]
    _policy: Given<String>,
}

/// Inert, every key: the prompt spliced into the agent's inbox, and the
/// entries taken out of it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Spliced {
    #[serde(rename = "target")]
    _target: String,
    #[serde(rename = "start")]
    _start: u64,
    #[serde(rename = "inserted")]
    _inserted: Vec<Queued>,
    #[serde(default, rename = "removedCount")]
    _removed: Given<u64>,
}

/// Inert, every key: only the probe's prompt, from the user, reads.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Queued {
    #[serde(rename = "content")]
    _content: Vec<PromptText>,
    #[serde(rename = "source")]
    _source: IgnoredAny,
    #[serde(rename = "role")]
    _role: String,
    #[serde(rename = "id")]
    _id: String,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum PromptText {
    Text {
        #[serde(rename = "text")]
        _text: Prompt,
    },
}

/// Inert, every key: a turn and a step by number.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Place {
    #[serde(rename = "turn")]
    _turn: u64,
    #[serde(default, rename = "step")]
    _step: Given<u64>,
}

/// How a turn ended: completed is the only end read, so any other leaves
/// the line unread, and the turn's number is inert.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ended {
    #[serde(rename = "reason")]
    _reason: Reason,
    #[serde(rename = "turn")]
    _turn: u64,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Reason {
    Completed,
}

/// A request's header: the model and the tools it offered, a listing;
/// and, inert, why it was sent.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    header: Request,
    #[serde(rename = "reason")]
    _reason: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Request {
    config: Route,
    tools: Vec<Tool>,
    /// Inert: which of the config's values are the adapter's defaults.
    #[serde(rename = "adapterDefaults")]
    _defaults: IgnoredAny,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Route {
    model: String,
    /// Inert: the provider serving the model, and its output limit.
    #[serde(rename = "provider")]
    _provider: String,
    #[serde(rename = "maxTokens")]
    _max: u64,
}

/// A tool offered: its name, read, and, inert, how to call it, which
/// does not say whether it is listed.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Tool {
    name: String,
    #[serde(rename = "description")]
    _description: String,
    #[serde(rename = "parameters")]
    _parameters: IgnoredAny,
}

/// One assembled assistant step. Every key may be absent, so the struct
/// defaults each one.
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct Step {
    usage: Given<Usage>,
    message: Given<StepMessage>,
    /// Inert: dsh's own turn, which never leaves 1 for one task.
    #[serde(rename = "turn")]
    _turn: Given<u64>,
    /// Inert: the step's place in the turn.
    #[serde(rename = "step")]
    _step: Given<u64>,
    /// Inert: the chunks the message was assembled from, which restate
    /// it.
    #[serde(rename = "stream")]
    _stream: Given<IgnoredAny>,
}

/// The step's message: its text, read whole, and the model that served
/// it. Every key may be absent, so the struct defaults each one.
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct StepMessage {
    content: Given<Vec<Content>>,
    source: Given<Source>,
    /// Inert: a tag, and the message's own id.
    #[serde(rename = "role")]
    _role: Given<String>,
    #[serde(rename = "id")]
    _id: Given<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Content {
    /// Inert: the model's thinking, which runs nothing.
    Reasoning {
        #[serde(rename = "text")]
        _text: String,
    },
    Text {
        text: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Source {
    model: String,
    /// Inert, like the keys after it: who served the step, and the state
    /// a replay restores.
    #[serde(default, rename = "kind")]
    _kind: Given<String>,
    #[serde(default, rename = "provider")]
    _provider: Given<String>,
    #[serde(default, rename = "replayState")]
    _replay: Given<IgnoredAny>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Usage {
    input_tokens: u64,
    output_tokens: u64,
    #[serde(default)]
    cache_read_tokens: Given<u64>,
    #[serde(default)]
    reasoning_tokens: Given<u64>,
    #[serde(default)]
    total_tokens: Given<u64>,
}

/// One dsh event, decoded, or why it is unread: labelled by its type,
/// which decoding it has shown to be one the reader names.
pub(super) fn event(mut fields: Map<String, Value>) -> Result<Decoded, Fault> {
    let _: Envelope = envelope(Harness::Dsh, &mut fields, &ENVELOPE)?;
    let label = fields
        .get("type")
        .and_then(Value::as_str)
        .map(str::to_string);
    let said = match decode(Harness::Dsh, fields)? {
        Event::Session { id, .. } => vec![Said::Session { key: "id", id }],
        Event::Header { data } => header_said(data.header),
        Event::AssistantMessage { data } => step_said(data)?,
        Event::Preset { .. }
        | Event::Sandbox { .. }
        | Event::Approval { .. }
        | Event::Inbox { .. }
        | Event::TurnStart { .. }
        | Event::StepStart { .. }
        | Event::StepEnd { .. }
        | Event::TurnEnd { .. }
        | Event::SystemMessage { .. }
        | Event::UserMessage { .. }
        | Event::Context { .. }
        | Event::Title { .. }
        | Event::TitleRequest { .. } => Vec::new(),
    };
    Ok(Decoded {
        label: label.unwrap_or_default(),
        said,
    })
}

fn header_said(request: Request) -> Vec<Said> {
    let names = request.tools.into_iter().map(|tool| tool.name).collect();
    vec![
        Said::Model(request.config.model),
        Said::Tools {
            at: "/data/header/tools",
            names,
        },
    ]
}

fn step_said(step: Step) -> Result<Vec<Said>, Fault> {
    let mut said = Vec::new();
    let message = step.message.0.unwrap_or_default();
    for content in message.content.0.iter().flatten() {
        if let Content::Text { text } = content {
            said.extend(forms::read(text, &FORMS).ok_or(Fault::Unrecognised)?);
        }
    }
    said.extend(message.source.0.map(|source| Said::Model(source.model)));
    let usage = step.usage.0.map(|usage| {
        Counted::of(
            "/data/usage",
            None,
            [
                ("inputTokens", Some(usage.input_tokens)),
                ("outputTokens", Some(usage.output_tokens)),
                ("cacheReadTokens", usage.cache_read_tokens.0),
                ("reasoningTokens", usage.reasoning_tokens.0),
                ("totalTokens", usage.total_tokens.0),
            ],
        )
    });
    said.extend(usage);
    Ok(said)
}

/// One line of dsh's text, read whole against its forms, `block` saying
/// whether its reasoning has opened: the header opens it, and a line in
/// it that no form reads is the model's thinking, inert, as the reply
/// is there, since thinking runs nothing and answers nothing.
pub(super) fn text(line: &str, block: &mut Block) -> Option<Vec<Said>> {
    if line.trim() == REASONING {
        *block = Block::Reasoning;
        return Some(Vec::new());
    }
    let read = forms::read(line, &FORMS);
    match block {
        Block::Plain => read,
        Block::Reasoning => {
            let said = read.unwrap_or_default().into_iter();
            Some(said.filter(|said| *said != Said::Reply).collect())
        }
    }
}
