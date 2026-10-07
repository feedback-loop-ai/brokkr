//! Codex's typed reader (#484): `exec --json` events, decoded closed by
//! `type`, and the forms codex prints. The vocabulary is the one the
//! controller recorded from codex on 2026-10-03 (`tests/streams/codex-*`):
//! `thread.started`, `turn.started`, `item.completed` (an `agent_message`
//! or an `error` item), `turn.completed`, `error` and `turn.failed`, and
//! the notice on stderr every launch prints; its rollouts are
//! `codex_log`'s. A key is consumed, or inert, as in `claude`.

use serde::Deserialize;
use serde_json::{Map, Value};

use super::forms::{self, Form, Origin, Says};
use super::read::{decode, Class, Counted, Decoded, Given, Harness, Said, Uuid};
use super::Fault;

/// What codex prints that the probe reads whole.
const FORMS: [Form; 6] = [
    // serde's refusal of a level `model_reasoning_effort` does not take.
    Form {
        words: "error loading config unknown variant {level} expected one of {levels}",
        says: Says::RefusesLevel,
    },
    // On stderr, at every launch, recorded 2026-10-03.
    Form {
        words: "reading additional input from stdin",
        says: Says::Nothing,
    },
    // The provider's refusal of an unknown model, recorded 2026-10-03.
    Form {
        words: "the {model} model is not supported when using codex with a chatgpt account",
        says: Says::Refuses(Class::Config),
    },
    // The warning before it, which refuses nothing, recorded 2026-10-03.
    Form {
        words: "model metadata for {model} not found defaulting to fallback metadata this can \
                degrade performance and cause issues",
        says: Says::Nothing,
    },
    // A config key it does not know.
    Form {
        words: "error loading config unknown key {key}",
        says: Says::Refuses(Class::Control),
    },
    // Its refusal of a launch with no credential.
    Form {
        words: "not logged in run codex login",
        says: Says::RefusesThe(Class::Auth, "login"),
    },
];

#[derive(Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
enum Event {
    #[serde(rename = "thread.started")]
    ThreadStarted { thread_id: Uuid },
    #[serde(rename = "turn.started")]
    TurnStarted {},
    #[serde(rename = "item.completed")]
    ItemCompleted { item: Item },
    #[serde(rename = "turn.completed")]
    TurnCompleted { usage: Usage },
    /// A failure, its message read whole, as a refusal.
    #[serde(rename = "error")]
    Error { message: String },
    /// A failure, as `error`.
    #[serde(rename = "turn.failed")]
    TurnFailed { error: Failure },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Item {
    AgentMessage {
        /// Inert: the item's own id.
        #[serde(rename = "id")]
        _id: String,
        /// Read whole, as the reply.
        text: String,
    },
    Error {
        /// Inert: the item's own id.
        #[serde(rename = "id")]
        _id: String,
        /// Read whole, as a refusal or a warning, never as the reply.
        message: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Usage {
    input_tokens: u64,
    output_tokens: u64,
    #[serde(default)]
    cached_input_tokens: Given<u64>,
    #[serde(default)]
    cache_write_input_tokens: Given<u64>,
    #[serde(default)]
    reasoning_output_tokens: Given<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Failure {
    /// Read whole, as a refusal.
    message: String,
}

/// The provider's error, which codex prints as JSON inside a message.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApiFailure {
    #[serde(rename = "type")]
    _kind: ErrorTag,
    /// Inert: the HTTP status of an event that is a failure whatever it
    /// holds; the message says what was refused.
    #[serde(rename = "status")]
    _status: u16,
    error: ApiError,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ErrorTag {
    Error,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApiError {
    /// Inert: the provider's class, as `status`.
    #[serde(rename = "type")]
    _kind: String,
    /// Read whole, as a refusal.
    message: String,
}

/// One codex event, decoded, or why it is unread.
pub(super) fn event(fields: Map<String, Value>) -> Result<Decoded, Fault> {
    let (label, said) = match decode(Harness::Codex, fields)? {
        Event::ThreadStarted { thread_id } => (
            "thread.started",
            vec![Said::Session {
                key: "thread_id",
                id: thread_id.0,
            }],
        ),
        Event::TurnStarted {} => ("turn.started", Vec::new()),
        Event::ItemCompleted {
            item: Item::AgentMessage { text, .. },
        } => ("item.completed", answer(&text)?),
        Event::ItemCompleted {
            item: Item::Error { message: text, .. },
        } => ("item.completed", message(&text)?),
        Event::TurnCompleted { usage } => ("turn.completed", vec![usage_said(usage)]),
        Event::Error { message: text } => ("error", failed(message(&text)?, "/message")),
        Event::TurnFailed { error } => (
            "turn.failed",
            failed(message(&error.message)?, "/error/message"),
        ),
    };
    Ok(Decoded {
        label: label.to_string(),
        said,
    })
}

/// What a failure's message says, and the failure, stated at `at`.
fn failed(mut said: Vec<Said>, at: &'static str) -> Vec<Said> {
    said.push(Said::Failed { at });
    said
}

fn usage_said(usage: Usage) -> Said {
    Counted::of(
        "/usage",
        None,
        [
            ("input_tokens", Some(usage.input_tokens)),
            ("cached_input_tokens", usage.cached_input_tokens.0),
            ("cache_write_input_tokens", usage.cache_write_input_tokens.0),
            ("output_tokens", Some(usage.output_tokens)),
            ("reasoning_output_tokens", usage.reasoning_output_tokens.0),
        ],
    )
}

/// A message a failure or an error item carries, read whole, never as
/// the reply: the provider's error, when it is one as JSON, by the
/// message it holds, and otherwise as text.
fn message(text: &str) -> Result<Vec<Said>, Fault> {
    let inner = super::strict::object(text)
        .map(|fields| decode::<ApiFailure>(Harness::Codex, fields))
        .transpose()?;
    let text = inner
        .as_ref()
        .map_or(text, |failure| &failure.error.message);
    self::text(text, Origin::Aside).ok_or(Fault::Unrecognised)
}

/// The turn's own answer, an agent message's text, read whole.
pub(super) fn answer(text: &str) -> Result<Vec<Said>, Fault> {
    self::text(text, Origin::Answer).ok_or(Fault::Unrecognised)
}

/// One line of codex's text, from `origin`, read whole against its
/// forms.
pub(super) fn text(line: &str, origin: Origin) -> Option<Vec<Said>> {
    forms::read(line, origin, &FORMS)
}
